#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! GPUIホストの起動・画面状態・描画ループ。
//! connectionsは接続フォーム、settingsはスイッチ、draftは保存前の編集管理、
//! serviceは共通Coreとの非同期連携を担当する。

mod allowlist;
mod audio;
mod connections;
mod controls;
mod draft;
mod motion;
mod paths;
mod refinement;
mod runtime;
mod service;
mod settings;
mod shell;
mod theme;
mod update;
mod update_view;

use audio::Sound;
use controls::{Button, Choice, ChoiceEvent, MotionSettings, Select};
use draft::SettingsDraft;
use gpui::{prelude::*, *};
use gpui_component::{
    Icon, IconName, Root, Sizable, TitleBar, h_flex,
    input::{Input, InputEvent, InputState},
    scroll::{Scrollbar, ScrollbarShow},
    slider::{Slider, SliderEvent, SliderState},
    v_flex,
};
use motion::{Fade, Panel, Spring};
use settings::Toggle;
use std::{cell::Cell, path::PathBuf, rc::Rc, time::Instant};
use theme::Palette;

const PAGES: [&str; 4] = ["通知", "接続", "動作設定", "診断"];
const NAMES: [&str; 3] = ["整列", "余白", "小窓"];

type UpdateKey = (
    crate::update::Phase,
    u64,
    bool,
    Option<String>,
    Option<String>,
);

struct DetailInputs {
    client_id: Entity<InputState>,
    credentials: Vec<Entity<InputState>>,
}

struct Workspace {
    // Coreの状態とUIの編集中データを分け、状態の定期取得で入力を消さない。
    runtime: Option<crate::runtime::Runtime>,
    runtime_view: Option<crate::runtime::View>,
    runtime_busy: bool,
    runtime_saving: bool,
    runtime_reload: bool,
    updater: Option<crate::update::Updater>,
    update_key: Option<UpdateKey>,
    update_gate: std::sync::Arc<crate::update::UpdateGate>,
    runtime_tray: Option<crate::shell::TraySession>,
    runtime_numbers: Vec<Entity<InputState>>,
    number_subscriptions: Vec<Subscription>,
    detail_subscriptions: Vec<Subscription>,
    allowlist: Vec<String>,
    allowlist_input: Entity<InputState>,
    allowlist_error: Option<&'static str>,
    saved_draft: Option<SettingsDraft>,
    footer_dirty: bool,
    footer_motion: Fade,
    notice: bool,
    design: usize,
    look_b: bool,
    intro: Fade,
    // pageは選択先、panel.visibleは退出アニメーション後に切り替わる表示中ページ。
    page: usize,
    panel: Panel,
    nav_motion: Spring,
    mode_motion: Spring,
    detail_motion: Fade,
    file_motion: Fade,
    feedback_motion: Fade,
    theme_swap: Fade,
    volume_motion: Spring,
    volume_value: f32,
    toggles: [Spring; 7],
    nav_hover: [Fade; 4],
    mode_hover: [Fade; 2],
    mode_width: Rc<Cell<f32>>,
    detail_height: Rc<Cell<f32>>,
    file_height: Rc<Cell<f32>>,
    dark: bool,
    reduced: bool,
    paused: bool,
    sound_only: bool,
    sound: Sound,
    file: Option<PathBuf>,
    volume: Entity<SliderState>,
    volume_focus: FocusHandle,
    scroll: ScrollHandle,
    receive: Entity<Choice>,
    source: Entity<Choice>,
    detail_inputs: Option<DetailInputs>,
    inactive_input_focus: Option<FocusHandle>,
    details: bool,
    discord: bool,
    overlay: bool,
    auto_exit: bool,
    demo_bot: bool,
    playing: bool,
    picking: bool,
    message: String,
    error: bool,
    tests: usize,
    _subscriptions: Vec<Subscription>,
}

impl Workspace {
    fn new(design: usize, look_b: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        theme::apply(true, window, cx);
        let reduced = theme::windows_reduces_motion();
        let volume = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max(100.)
                .step(1.)
                .default_value(60.)
        });
        cx.set_global(MotionSettings(reduced));
        let receive = cx.new(|cx| {
            Choice::new(
                vec!["すべての通知", "許可リストのみ", "Discordの状態で自動判別"],
                0,
                cx,
            )
        });
        let source = cx.new(|cx| Choice::new(vec!["オフ", "標準の通知音", "音声ファイル"], 1, cx));
        let allowlist_input = cx.new(|cx| InputState::new(window, cx));
        let allowlist_subscription = cx.subscribe_in(
            &allowlist_input,
            window,
            |this, _, event: &InputEvent, window, cx| match event {
                InputEvent::PressEnter { secondary: false } => this.add_allowed_channel(window, cx),
                InputEvent::Change => {
                    this.allowlist_error = None;
                    cx.notify();
                }
                _ => {}
            },
        );
        let source_subscription = cx.subscribe(&source, |this, _, event: &ChoiceEvent, cx| {
            this.set_sound(match event.0 {
                0 => Sound::Silent,
                2 => Sound::File,
                _ => Sound::Chime,
            });
            cx.notify();
        });
        let receive_subscription = cx.subscribe(&receive, |_, _, _: &ChoiceEvent, cx| cx.notify());
        let subscription = cx.subscribe(&volume, |this, _, event: &SliderEvent, cx| {
            let SliderEvent::Change(value) = event;
            this.volume_value = value.end();
            // ドラッグはポインターへ即時追従し、キー操作だけをばねで補間する。
            this.volume_motion.snap(value.end(), Instant::now());
            cx.notify();
        });
        let activation_subscription = cx.observe_window_activation(window, |this, window, cx| {
            if window.is_window_active() {
                if let Some(focus) = this.inactive_input_focus.take() {
                    window.focus(&focus);
                }
            } else if this
                .runtime_numbers
                .iter()
                .any(|input| input.focus_handle(cx).is_focused(window))
                || this.allowlist_input.focus_handle(cx).is_focused(window)
                || this.detail_inputs.as_ref().is_some_and(|inputs| {
                    inputs.client_id.focus_handle(cx).is_focused(window)
                        || inputs
                            .credentials
                            .iter()
                            .any(|input| input.focus_handle(cx).is_focused(window))
                })
            {
                // gpui-component 0.5.1 hides an inactive input's cursor but keeps
                // its 500 ms blink task alive. Blur cancels it via on_blur;
                // restore the same field on activation, retaining text/selection.
                this.inactive_input_focus = window.focused(cx);
                window.blur();
            }
        });
        let mut view = Self {
            runtime: None,
            runtime_view: None,
            runtime_busy: false,
            runtime_saving: false,
            runtime_reload: false,
            updater: None,
            update_key: None,
            update_gate: Default::default(),
            runtime_tray: None,
            runtime_numbers: vec![],
            number_subscriptions: vec![],
            detail_subscriptions: vec![],
            allowlist: vec![],
            allowlist_input,
            allowlist_error: None,
            saved_draft: None,
            footer_dirty: false,
            footer_motion: Fade::new(0.),
            notice: false,
            design,
            look_b,
            intro: {
                let mut f = Fade::new(if look_b { 0. } else { 1. });
                if look_b {
                    f.set(1., 0.5, Instant::now());
                }
                f
            },
            page: 0,
            panel: Panel::new(),
            nav_motion: Spring::selection(0.),
            mode_motion: Spring::selection(0.),
            detail_motion: Fade::new(0.),
            file_motion: Fade::new(if design == 3 { 0. } else { 1. }),
            feedback_motion: Fade::new(1.),
            theme_swap: Fade::new(1.),
            volume_motion: Spring::new(60., 400., 32.),
            volume_value: 60.,
            toggles: [true, false, false, true, reduced, false, false].map(Spring::thumb),
            nav_hover: [Fade::new(0.); 4],
            mode_hover: [Fade::new(0.); 2],
            mode_width: Rc::new(Cell::new(704.)),
            detail_height: Rc::new(Cell::new(348.)),
            file_height: Rc::new(Cell::new(82.)),
            dark: true,
            reduced,
            paused: false,
            sound_only: false,
            sound: Sound::Chime,
            file: None,
            volume,
            volume_focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            receive,
            source,
            detail_inputs: None,
            inactive_input_focus: None,
            details: false,
            discord: true,
            overlay: false,
            auto_exit: false,
            demo_bot: false,
            playing: false,
            picking: false,
            message: "設定はこのウィンドウの中だけで保持されます。".into(),
            error: false,
            tests: 0,
            _subscriptions: vec![
                subscription,
                source_subscription,
                receive_subscription,
                activation_subscription,
                allowlist_subscription,
            ],
        };
        view.init_runtime(window, cx);
        view.commit_draft(cx);
        view
    }

    fn p(&self) -> Palette {
        Palette::new(self.dark)
    }

    fn note(&self, text: impl Into<SharedString>) -> AnyElement {
        div()
            .text_size(px(12.))
            .text_color(rgb(self.p().muted))
            .line_height(px(21.6))
            .child(text.into())
            .into_any_element()
    }

    fn section(&self, number: &str, title: &str, description: &str) -> AnyElement {
        v_flex()
            .gap_2()
            .mb(px(if self.design == 3 { 10. } else { 22. }))
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(self.p().blue))
                            .child(number.to_owned()),
                    )
                    .child(
                        div()
                            .text_size(px(18.))
                            .line_height(px(28.8))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title.to_owned()),
                    ),
            )
            .when(!description.is_empty(), |e| {
                e.child(self.note(description.to_owned()))
            })
            .into_any_element()
    }

    fn heading(&self, text: &str) -> AnyElement {
        div()
            .text_size(px(25.))
            .line_height(px(37.5))
            .font_weight(FontWeight::SEMIBOLD)
            .mb_6()
            .child(text.to_owned())
            .into_any_element()
    }

    fn feedback(&mut self, text: impl Into<String>, error: bool, cx: &mut Context<Self>) {
        self.message = text.into();
        self.error = error;
        self.notice = true;
        let now = Instant::now();
        self.feedback_motion.set(0., 0., now);
        self.feedback_motion.set(1., 0.15, now);
        cx.notify();
    }

    fn set_sound(&mut self, sound: Sound) {
        let now = Instant::now();
        self.sound = sound;
        self.file_motion.set(
            if self.design != 3 || sound == Sound::File {
                1.
            } else {
                0.
            },
            if self.reduced { 0. } else { 0.35 },
            now,
        );
    }

    // The inner panel retains its natural layout. Only the outer clip changes
    // height, so text and icons never stretch. Measurements follow wrapped Japanese.
    fn reveal(&self, content: AnyElement, progress: f32, measured: Rc<Cell<f32>>) -> AnyElement {
        let height = measured.get();
        div()
            .w_full()
            .overflow_hidden()
            .when(progress < 0.9999, |e| e.h(px(height * progress)))
            .child(
                div().relative().w_full().child(content).child(
                    canvas(
                        |_, _, _| (),
                        move |bounds, _, _, _| {
                            measured.set(bounds.size.height.into());
                        },
                    )
                    .absolute()
                    .size_full(),
                ),
            )
            .into_any_element()
    }

    fn pick_file(&mut self, cx: &mut Context<Self>) {
        if self.picking {
            return;
        }
        self.picking = true;
        cx.notify();
        cx.spawn(async move |view, cx| {
            // rfd's async Windows implementation owns a fresh STA thread. GPUI's
            // shared executor may already be MTA, where the sync picker fails.
            let selected = rfd::AsyncFileDialog::new()
                .set_title("通知音を選ぶ")
                .add_filter("音声ファイル", &["mp3", "wav", "ogg"])
                .pick_file()
                .await
                .map(|file| file.path().to_path_buf());
            let _ = view.update(cx, |this, cx| {
                this.picking = false;
                if let Some(path) = selected {
                    this.file = Some(path);
                    this.set_sound(Sound::File);
                    this.feedback("音声ファイルを選びました。", false, cx);
                } else {
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn test(&mut self, audio_only: bool, cx: &mut Context<Self>) {
        if self.runtime.is_some() {
            self.runtime_action(if audio_only { "audition" } else { "test" }, cx);
            return;
        }
        if self.playing {
            return;
        }
        let volume = self.volume_value;
        let sound = self.sound;
        if sound == Sound::Silent && (audio_only || self.sound_only) {
            self.feedback(
                "音のみで試すには、標準音か音声ファイルを選んでください。",
                true,
                cx,
            );
            return;
        }
        if sound != Sound::Silent && volume <= 0. {
            self.feedback("音量が0%です。音量を上げてから試してください。", true, cx);
            return;
        }
        if sound == Sound::File && self.file.is_none() {
            self.feedback(
                "音声ファイルが未選択です。「ファイルを選ぶ」から選んでください。",
                true,
                cx,
            );
            return;
        }
        if self.paused && !audio_only {
            self.feedback(
                "デモを一時停止中です。再開してから通知を試してください。",
                true,
                cx,
            );
            return;
        }
        if sound == Sound::Silent {
            self.tests += 1;
            self.feedback(
                "表示テストが完了しました。音はオフです。外部には送信していません。",
                false,
                cx,
            );
            return;
        }
        let file = self.file.clone();
        self.playing = true;
        self.feedback("このPCで試聴しています…（最大4秒）", false, cx);
        cx.spawn(async move |view, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { audio::play(sound, file, volume) })
                .await;
            let _ = view.update(cx, |this, cx| {
                this.playing = false;
                match result {
                    Ok(()) => {
                        this.tests += 1;
                        this.feedback(
                            "再生処理が完了しました。Discord・XSOverlayには送信していません。",
                            false,
                            cx,
                        );
                    }
                    Err(message) => this.feedback(message, true, cx),
                }
            });
        })
        .detach();
    }

    // Retain the production VR monogram; only the tile and optical padding
    // adapt to this UI. Notification actions still use the bell symbol.
    fn app_mark(&self, size: f32) -> AnyElement {
        let p = self.p();
        div()
            .size(px(size))
            .flex_shrink_0()
            .rounded(px(size / 4.))
            .bg(rgb(p.brand))
            .border_1()
            .border_color(rgb(p.brand_edge))
            .flex()
            .items_center()
            .justify_center()
            .child(
                svg()
                    .path("ditovr/vr-mark.svg")
                    .w(px(size * 0.74))
                    .h(px(size * 0.55))
                    .text_color(rgb(0xffffff)),
            )
            .into_any_element()
    }

    fn header(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.p();
        h_flex()
            .w_full()
            .justify_between()
            .gap_3()
            .flex_wrap()
            .child(
                h_flex().gap_3().child(self.app_mark(36.)).child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_size(px(21.))
                                .line_height(px(27.3))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("DiToVR"),
                        )
                        .when(self.design != 3, |e| {
                            e.child(
                                div()
                                    .text_size(px(11.))
                                    .line_height(px(17.6))
                                    .text_color(rgb(p.muted))
                                    .child(format!(
                                        "案{} / {}",
                                        self.design,
                                        NAMES[self.design - 1]
                                    )),
                            )
                        }),
                ),
            )
            // Keep the preferred theme action in the header; connection
            // start/stop still belongs to the Connection page in design 3.
            .child(
                h_flex()
                    .gap(px(6.))
                    .when(self.design != 3, |e| {
                        e.child(
                            Button::new("pause")
                                .ghost()
                                .disabled(self.runtime_busy)
                                .font_size(12.)
                                .icon(Icon::empty().path(if self.paused {
                                    "ditovr/play.svg"
                                } else {
                                    "ditovr/pause.svg"
                                }))
                                .label(if self.paused {
                                    if self.runtime.is_some() {
                                        "接続開始"
                                    } else {
                                        "再開"
                                    }
                                } else {
                                    "一時停止"
                                })
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.toggle_pause(cx);
                                })),
                        )
                    })
                    .child(
                        Button::new("theme")
                            .ghost()
                            .disabled(self.runtime_saving)
                            .font_size(12.)
                            .icon(
                                Icon::new(if self.dark {
                                    IconName::Sun
                                } else {
                                    IconName::Moon
                                })
                                .rotate(radians(
                                    if self.reduced {
                                        0.
                                    } else {
                                        (self.theme_swap.value(Instant::now()) - 1.)
                                            * std::f32::consts::FRAC_PI_2
                                    },
                                )),
                            )
                            .label(if self.dark { "ライト" } else { "ダーク" })
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.toggle_theme(window, cx);
                            })),
                    ),
            )
            .into_any_element()
    }

    fn pill(&self, label: &str, on: bool) -> AnyElement {
        let p = self.p();
        h_flex()
            .gap_2()
            .px(px(12.))
            .py(px(5.))
            .rounded_full()
            .border_1()
            .border_color(rgb(p.line))
            .child(
                div()
                    .size(px(8.))
                    .rounded_full()
                    .bg(rgb(if on { p.good } else { p.muted })),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .line_height(px(18.))
                    .text_color(rgb(p.muted))
                    .child(format!(
                        "{label}  {}",
                        if on { "接続済み" } else { "未接続" }
                    )),
            )
            .into_any_element()
    }

    /// 案B: タイトルと接続状態を一枚のカードにまとめ、常に最上部へ表示する。
    fn header_b(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.p();
        let now = Instant::now();
        let enter = self.intro.value(now);
        div()
            .w_full()
            .opacity(enter)
            .relative()
            .top(px((1. - enter) * 8.))
            .rounded(px(16.))
            .border_1()
            .border_color(rgb(p.line))
            .bg(rgb(p.surface))
            .px(px(20.))
            .py(px(16.))
            .child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .items_center()
                    .gap_3()
                    .flex_wrap()
                    .child(
                        h_flex().gap_3().child(self.app_mark(36.)).child(
                            v_flex()
                                .child(
                                    div()
                                        .text_size(px(21.))
                                        .line_height(px(27.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child("DiToVR"),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(18.))
                                        .text_color(rgb(p.muted))
                                        .child("VRのままDiscordの通知に気づく"),
                                ),
                        ),
                    )
                    .child(
                        Button::new("theme")
                            .ghost()
                            .disabled(self.runtime_saving)
                            .font_size(12.)
                            .icon(
                                Icon::new(if self.dark {
                                    IconName::Sun
                                } else {
                                    IconName::Moon
                                })
                                .rotate(radians(
                                    if self.reduced {
                                        0.
                                    } else {
                                        (self.theme_swap.value(now) - 1.)
                                            * std::f32::consts::FRAC_PI_2
                                    },
                                )),
                            )
                            .label(if self.dark { "ライト" } else { "ダーク" })
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.toggle_theme(window, cx);
                            })),
                    ),
            )
            .child(
                h_flex()
                    .mt(px(12.))
                    .gap(px(8.))
                    .flex_wrap()
                    .child(self.pill("Discord", self.discord))
                    .child(self.pill("XSOverlay", self.overlay))
                    .when(self.paused, |e| e.child(self.note("一時停止中"))),
            )
            .into_any_element()
    }

    fn nav(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.p();
        let now = Instant::now();
        h_flex()
            .relative()
            .w_full()
            .pb(px(10.))
            .gap(px(8.))
            .border_b_1()
            .border_color(rgb(p.line))
            .children(PAGES.iter().enumerate().map(|(i, name)| {
                let hover = self.nav_hover[i].value(now);
                div()
                    .w(px(if i == 2 { 84. } else { 58. }))
                    .flex_shrink_0()
                    .child(
                        Button::new(("nav", i))
                            .ghost()
                            .small()
                            .rounded_none()
                            .w_full()
                            .label(*name)
                            .bg(Hsla::from(rgb(p.hover)).opacity(hover))
                            .text_color(rgb(if self.page == i { p.text } else { p.muted }))
                            .on_hover(cx.listener(move |this, hovered, _, cx| {
                                this.nav_hover[i].set(*hovered as u8 as f32, 0.2, Instant::now());
                                cx.notify();
                            }))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if this.page != i {
                                    let now = Instant::now();
                                    this.page = i;
                                    this.scroll.set_offset(point(px(0.), px(0.)));
                                    this.nav_motion.set(i as f32, now, this.reduced);
                                    this.receive.update(cx, |s, cx| s.close(cx));
                                    this.source.update(cx, |s, cx| s.close(cx));
                                    this.panel.select(i, now, this.reduced);
                                    cx.notify();
                                }
                            })),
                    )
            }))
            .child(
                div()
                    .absolute()
                    .bottom_0()
                    .left(px(nav_geometry(self.nav_motion.value(now)).0))
                    .w(px(nav_geometry(self.nav_motion.value(now)).1))
                    .h(px(2.))
                    .bg(rgb(p.blue)),
            )
            .into_any_element()
    }

    fn status(&self) -> AnyElement {
        let p = self.p();
        h_flex()
            .gap(px(18.))
            .flex_wrap()
            .pt(px(if self.design == 3 { 12. } else { 20. }))
            .child(
                div()
                    .text_size(px(12.))
                    .px(px(7.))
                    .py(px(3.))
                    .line_height(px(20.))
                    .rounded(px(4.))
                    .bg(rgb(p.hover))
                    .text_color(rgb(p.muted))
                    .child(if self.runtime_view.as_ref().is_some_and(|v| v.preview) {
                        "プレビュー"
                    } else {
                        "通常"
                    }),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(div().size(px(5.)).rounded_full().bg(rgb(if self.discord {
                        p.good
                    } else {
                        p.muted
                    })))
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(20.))
                            .text_color(rgb(p.muted))
                            .child(if self.discord {
                                "Discord  接続済み"
                            } else {
                                "Discord  未接続"
                            }),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(div().size(px(5.)).rounded_full().bg(rgb(if self.overlay {
                        p.good
                    } else {
                        p.muted
                    })))
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(20.))
                            .text_color(rgb(p.muted))
                            .child(if self.overlay {
                                "XSOverlay  接続済み"
                            } else {
                                "XSOverlay  未接続"
                            }),
                    ),
            )
            .when(self.paused, |e| {
                e.child(self.note(if self.runtime.is_some() {
                    "停止中"
                } else {
                    "一時停止中"
                }))
            })
            .into_any_element()
    }

    fn modes(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.p();
        let now = Instant::now();
        let t = self.mode_motion.value(now);
        let horizontal = self.design == 3;
        let width = (self.mode_width.get() - 10.) / 2.;
        let measure = self.mode_width.clone();
        let view = cx.entity().downgrade();
        div()
            .relative()
            .flex()
            .gap(px(10.))
            .w_full()
            .when(!horizontal, |e| e.flex_col())
            .child(
                div()
                    .absolute()
                    .rounded(px(7.))
                    .bg(rgb(p.hover))
                    .border_1()
                    .border_color(rgb(p.blue))
                    .when(horizontal, |e| {
                        e.left(px((width + 10.) * t))
                            .top_0()
                            .w(px(width))
                            .h(px(54.))
                    })
                    .when(!horizontal, |e| {
                        e.top(px(70. * t)).left_0().w_full().h(px(62.))
                    }),
            )
            .children(
                [false, true]
                    .into_iter()
                    .enumerate()
                    .map(|(i, sound_only)| {
                        let fill = if i == 0 { 1. - t } else { t };
                        let hover = self.mode_hover[i].value(now);
                        Button::new(("mode", i))
                            .ghost()
                            .flex_1()
                            .min_w_0()
                            .small()
                            .pr(px(35.))
                            .rounded(px(7.))
                            .relative()
                            .h(px(if horizontal { 54. } else { 64. }))
                            .bg(Hsla::from(rgb(p.hover)).opacity(hover * 0.35))
                            .border_1()
                            .border_color(Hsla::from(rgb(p.line)).opacity(1. - fill.clamp(0., 1.)))
                            .label(if self.design == 3 {
                                if sound_only {
                                    "通知音のみ"
                                } else {
                                    "表示＋通知音"
                                }
                            } else if sound_only {
                                "音だけ届ける"
                            } else {
                                "表示と音で届ける"
                            })
                            .icon(if sound_only {
                                Icon::empty().path("ditovr/headphones.svg")
                            } else {
                                Icon::new(IconName::Bell)
                            })
                            .overlay(
                                div()
                                    .absolute()
                                    .right(px(15.))
                                    .size(px(16.))
                                    .rounded_full()
                                    .border_1()
                                    .border_color(rgb(motion::mix(p.input, p.blue, fill)))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        div()
                                            .size(px(8. * fill.clamp(0., 1.)))
                                            .rounded_full()
                                            .bg(rgb(p.blue)),
                                    ),
                            )
                            .on_hover(cx.listener(move |this, hovered, _, cx| {
                                this.mode_hover[i].set(*hovered as u8 as f32, 0.2, Instant::now());
                                cx.notify();
                            }))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.sound_only = sound_only;
                                this.mode_motion.set(i as f32, Instant::now(), this.reduced);
                                cx.notify();
                            }))
                    }),
            )
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        let width: f32 = bounds.size.width.into();
                        if (measure.replace(width) - width).abs() > 0.1 {
                            let view = view.clone();
                            window.on_next_frame(move |_, cx| {
                                let _ = view.update(cx, |_, cx| cx.notify());
                            });
                        }
                    },
                )
                .absolute()
                .size_full(),
            )
            .into_any_element()
    }

    fn notifications(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .flex_1()
            .min_w_0()
            .child(self.section("01", "届け方", "VRでの過ごし方に合わせて、通知を選ぶ。"))
            .child(self.modes(cx))
            .child(div().mt_3().child(self.note(if self.sound_only {
                "本文を表示せず、通知音だけを鳴らします。"
            } else {
                "XSOverlayに通知を表示し、通知音を鳴らします。"
            })))
            .child(
                v_flex()
                    .gap_3()
                    .mt(px(if self.design == 3 { 18. } else { 28. }))
                    .child(div().font_weight(FontWeight::MEDIUM).child("受信する通知"))
                    .child(Select::new(&self.receive).w_full())
                    .child(self.note("許可リストや自動判別の設定は「接続」から。")),
            )
            .into_any_element()
    }

    fn audio_panel(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let p = self.p();
        let volume = self.volume_value;
        let file_name = self
            .file
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "ファイルは未選択".into());
        v_flex()
            .flex_1()
            .min_w_0()
            .child(self.section("02", "通知音", "いつもの音を、心地よい音量で。"))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .line_height(px(20.8))
                    .mb_3()
                    .child("音源"),
            )
            .child(Select::new(&self.source).disabled(self.playing).w_full())
            .when(
                self.file_motion.value(Instant::now()) > 0.
                    || self.file_motion.active(Instant::now()),
                |e| {
                    e.child(
                        self.reveal(
                            v_flex()
                                .pt_4()
                                .pb_1()
                                .gap_2()
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .text_color(rgb(p.muted))
                                        .truncate()
                                        .child(file_name),
                                )
                                .child(
                                    h_flex()
                                        .gap_3()
                                        .flex_wrap()
                                        .child(
                                            Button::new("pick")
                                                .icon(Icon::empty().path("ditovr/file-audio.svg"))
                                                .label(if self.picking {
                                                    "選択中…"
                                                } else {
                                                    "ファイルを選ぶ"
                                                })
                                                .disabled(
                                                    self.picking
                                                        || self.playing
                                                        || (self.design == 3
                                                            && self.sound != Sound::File),
                                                )
                                                .on_click(
                                                    cx.listener(|this, _, _, cx| {
                                                        this.pick_file(cx)
                                                    }),
                                                ),
                                        )
                                        .child(self.note("MP3 / WAV / Ogg · 16 MiBまで")),
                                )
                                .into_any_element(),
                            self.file_motion.value(Instant::now()),
                            self.file_height.clone(),
                        ),
                    )
                },
            )
            .child(
                v_flex()
                    .mt(px(if self.design == 3 { 16. } else { 28. }))
                    .gap_3()
                    .child(
                        h_flex()
                            .justify_between()
                            .child("音量")
                            .child(div().text_color(rgb(p.blue)).child(format!("{volume:.0}%"))),
                    )
                    .child(
                        div()
                            .id("volume-keyboard")
                            .track_focus(&self.volume_focus.clone().tab_index(0).tab_stop(true))
                            .mt(px(4.))
                            .on_key_down(cx.listener(|this, e: &KeyDownEvent, _, cx| {
                                let value = match e.keystroke.key.as_str() {
                                    "left" | "down" => this.volume_value - 1.,
                                    "right" | "up" => this.volume_value + 1.,
                                    "home" => 0.,
                                    "end" => 100.,
                                    _ => return,
                                }
                                .clamp(0., 100.);
                                this.volume_value = value;
                                this.volume_motion.set(value, Instant::now(), this.reduced);
                                cx.notify();
                                cx.stop_propagation();
                            }))
                            .child(Slider::new(&self.volume).horizontal())
                            .when(self.volume_focus.is_focused(window), |e| {
                                e.border_1().border_color(rgb(p.blue)).rounded(px(8.))
                            }),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(
                                        Button::new("quieter")
                                            .ghost()
                                            .size(px(36.))
                                            .px_0()
                                            .label("−")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.volume_value =
                                                    (this.volume_value - 5.).max(0.);
                                                this.volume_motion.set(
                                                    this.volume_value,
                                                    Instant::now(),
                                                    this.reduced,
                                                );
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Button::new("louder")
                                            .ghost()
                                            .size(px(36.))
                                            .px_0()
                                            .label("＋")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.volume_value =
                                                    (this.volume_value + 5.).min(100.);
                                                this.volume_motion.set(
                                                    this.volume_value,
                                                    Instant::now(),
                                                    this.reduced,
                                                );
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .child(
                                Button::new("listen")
                                    .ghost()
                                    .icon(Icon::empty().path("ditovr/play.svg"))
                                    .label("音を試聴")
                                    .disabled(self.playing)
                                    .on_click(cx.listener(|this, _, _, cx| this.test(true, cx))),
                            ),
                    ),
            )
            .child(
                div()
                    .mt(px(10.))
                    .child(self.note("このPCで最大4秒間、ローカル再生します。")),
            )
            .into_any_element()
    }

    fn home(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        if self.design == 3 {
            return self.refined_home(window, cx);
        }
        let p = self.p();
        let stacked = window.viewport_size().width < px(820.);
        let columns = div()
            .flex()
            .gap(px(if stacked {
                24.
            } else if self.design == 2 {
                44.
            } else {
                32.
            }))
            .w_full()
            .when(stacked, |e| e.flex_col())
            .child(self.notifications(cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .when(self.design == 2, |e| {
                        e.p_6().rounded(px(12.)).bg(rgb(p.surface))
                    })
                    .when(self.design == 1 && !stacked, |e| {
                        e.border_l_1().border_color(rgb(p.line)).pl_7()
                    })
                    .when(stacked, |e| e.border_t_1().border_color(rgb(p.line)).pt_6())
                    .child(self.audio_panel(window, cx)),
            );
        v_flex()
            .gap_6()
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(if self.design == 2 { 28. } else { 25. }))
                            .line_height(px(37.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("通知を、あなたのペースで。"),
                    )
                    .when(self.design != 3, |e| {
                        e.child(self.note("VRに集中するための、小さな調整。"))
                    }),
            )
            .child(columns)
            .child(
                div()
                    .mt_2()
                    .pt_6()
                    .border_t_1()
                    .border_color(rgb(p.line))
                    .child(
                        h_flex()
                            .justify_between()
                            .gap_4()
                            .flex_wrap()
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child("このPCで動きを確認")
                                    .child(self.note(if self.runtime.is_some() {
                                        "保存した設定でXSOverlayにテスト通知を送ります。"
                                    } else {
                                        "試作版です。VRやDiscordへの送信は行いません。"
                                    })),
                            )
                            .child(
                                Button::new("test")
                                    .primary()
                                    .icon(IconName::Bell)
                                    .label(if self.playing {
                                        "試聴中…"
                                    } else {
                                        "通知を試す"
                                    })
                                    .disabled(self.playing)
                                    .on_click(cx.listener(|this, _, _, cx| this.test(false, cx))),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn row(&self, title: &str, description: &str, action: impl IntoElement) -> AnyElement {
        h_flex()
            .w_full()
            .justify_between()
            .gap_4()
            .py(px(if self.design == 3 { 18. } else { 24. }))
            .border_b_1()
            .border_color(rgb(self.p().line))
            .child(
                v_flex()
                    .flex_1()
                    .gap_2()
                    .child(title.to_owned())
                    .when(!description.is_empty(), |e| {
                        e.child(self.note(description.to_owned()))
                    }),
            )
            .child(action)
            .into_any_element()
    }

    fn behavior(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        self.ensure_runtime_numbers(window, cx);
        v_flex()
            .gap_0()
            .child(self.heading(if self.design == 3 {
                "動作設定"
            } else {
                "使い心地を、自分らしく"
            }))
            .when(self.design != 3, |e| {
                e.child(self.row(
                    "ダークテーマ",
                    "黒を基調とするダークと、明るいライト。",
                    self.toggle(Toggle::Theme, cx),
                ))
            })
            .child(self.row(
                "動きを控える",
                "移動や伸縮のアニメーションを抑えます。",
                self.toggle(Toggle::ReducedMotion, cx),
            ))
            // In the selected runtime UI, connection start/stop has one home:
            // the Connection page. Keep the standalone demo's pause control.
            .when(self.design != 3 || self.runtime.is_none(), |e| {
                e.child(self.row(
                    "通知の一時停止",
                    if self.runtime.is_some() {
                        "Discordからの通知受信を停止・再開します。"
                    } else {
                        "デモの通知テストを一時停止。音の試聴は利用できます。"
                    },
                    self.toggle(Toggle::Pause, cx),
                ))
            })
            .when(self.runtime.is_some(), |e| {
                e.child(self.row(
                    "SteamVR終了時にアプリも終了",
                    "",
                    self.toggle(Toggle::AutoExit, cx),
                ))
                .child(self.runtime_number_fields())
            })
            .child(self.update_section(cx))
            .child(div().mt_5().child(self.note(
                if self.runtime_view.as_ref().is_some_and(|v| !v.preview) {
                    "×でトレイに格納します。通知の転送は続きます。"
                } else {
                    "プレビューの設定は、この起動中だけ保持します。"
                },
            )))
            .into_any_element()
    }

    fn diagnostics(&self, cx: &mut Context<Self>) -> AnyElement {
        let uses_saved_settings = self.runtime_view.as_ref().is_some_and(|v| !v.preview);
        v_flex()
            .gap_0()
            .child(self.heading(if self.design == 3 {
                "診断"
            } else {
                "いまの状態を確認"
            }))
            .child(self.row(
                "描画",
                "GPUI 0.2.2 / gpui-component 0.5.1",
                self.note("ネイティブ"),
            ))
            .when(self.runtime.is_none(), |e| {
                e.child(self.row("テスト回数", "", self.note(format!("{} 回", self.tests))))
            })
            .child(self.row(
                "保存・外部接続",
                if uses_saved_settings {
                    "このPCに保存した設定を使用します。"
                } else {
                    "設定はこの起動中のみ保持し、外部には接続しません。"
                },
                self.note(if uses_saved_settings {
                    "利用中"
                } else {
                    "利用なし"
                }),
            ))
            .when(self.runtime.is_some(), |e| {
                e.child(self.runtime_diagnostics(cx))
            })
            .child(self.row(
                "アニメーション",
                "操作時だけ描画します。",
                self.note(if self.reduced { "抑制中" } else { "有効" }),
            ))
            // 画面サイズの確認は開発・撮影用。保存した設定で動く通常起動では表示しない（--ui-preview では表示）。
            .when(!uses_saved_settings, |e| {
                e.child(
                    self.row(
                        "画面サイズの確認",
                        "",
                        h_flex()
                            .gap_2()
                            .flex_wrap()
                            .child(Button::new("compact-window").label("小さく").on_click(
                                cx.listener(|_, _, window, _| {
                                    window.resize(size(px(640.), px(640.)));
                                }),
                            ))
                            .child(Button::new("standard-window").label("標準に戻す").on_click(
                                cx.listener(|this, _, window, _| {
                                    window.resize(size(
                                        px(match this.design {
                                            2 => 1120.,
                                            3 => 760.,
                                            _ => 1040.,
                                        }),
                                        px(if this.runtime.is_some() { 800. } else { 900. }),
                                    ));
                                }),
                            ))
                            .when(self.design == 3, |e| {
                                e.child(Button::new("wide-window").label("広く").on_click(
                                    cx.listener(|_, _, window, _| {
                                        window.resize(size(px(1000.), px(800.)));
                                    }),
                                ))
                            }),
                    ),
                )
            })
            // Coreの累積値はUIだけでリセットできないため、デモにだけ表示する。
            .when(self.runtime.is_none(), |e| {
                e.child(
                    Button::new("reset")
                        .w(px(148.))
                        .mt_6()
                        .label("テスト回数をリセット")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.tests = 0;
                            this.feedback("テスト回数をリセットしました。", false, cx);
                        })),
                )
            })
            .into_any_element()
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.runtime_reload {
            // Coreからの保存結果を反映してから編集基準を更新し、保存欄の点滅を防ぐ。
            self.runtime_reload = false;
            if let Some(v) = self.runtime_view.clone() {
                self.apply_runtime(&v, window, cx);
                self.commit_draft(cx);
            }
        }
        let p = self.p();
        let now = Instant::now();
        let dirty = self.is_dirty(cx);
        // 編集の有無と更新の確定の判断は、入力を変更するのと同じUIスレッドで行う（UpdateGate）。
        let _applying_update = self.update_gate.publish_editing(dirty);
        if dirty != self.footer_dirty {
            if dirty && !self.error && !self.runtime_busy {
                self.notice = false;
            }
            self.footer_dirty = dirty;
            self.footer_motion
                .set(dirty as u8 as f32, if dirty { 0.28 } else { 0.18 }, now);
        }
        self.panel.advance(now, self.reduced);
        let active = self.panel.active(now)
            || self.footer_motion.active(now)
            || self.nav_motion.active(now)
            || self.mode_motion.active(now)
            || self.detail_motion.active(now)
            || self.file_motion.active(now)
            || self.feedback_motion.active(now)
            || self.theme_swap.active(now)
            || self.intro.active(now)
            || self.volume_motion.active(now)
            || self.toggles.iter().any(|s| s.active(now))
            || self.nav_hover.iter().any(|s| s.active(now))
            || self.mode_hover.iter().any(|s| s.active(now));
        // 静止時はフレームを要求しない。常駐中にアニメーションを回し続けないため。
        if active {
            window.request_animation_frame();
        }
        let displayed_volume = self.volume_motion.value(now).clamp(0., 100.);
        if (self.volume.read(cx).value().end() - displayed_volume).abs() > 0.0001 {
            self.volume.update(cx, |slider, cx| {
                slider.set_value(displayed_volume, window, cx)
            });
        }
        let content = match self.panel.visible {
            1 => self.connections(cx),
            2 => self.behavior(window, cx),
            3 => self.diagnostics(cx),
            _ => self.home(window, cx),
        };
        let offset = self.panel.offset.value(now);
        let content = div()
            .id("animated-page")
            .relative()
            .left(px(offset))
            .w_full()
            .opacity(self.panel.opacity.value(now))
            .child(content)
            .when(self.runtime_saving, |e| {
                e.capture_any_mouse_down(|_, _, cx| cx.stop_propagation())
                    .capture_any_mouse_up(|_, _, cx| cx.stop_propagation())
                    .capture_key_down(|_, _, cx| cx.stop_propagation())
            })
            .when(self.panel.visible != self.page, |e| {
                e.capture_any_mouse_down(|_, _, cx| cx.stop_propagation())
                    .capture_any_mouse_up(|_, _, cx| cx.stop_propagation())
                    .capture_key_down(|_, _, cx| cx.stop_propagation())
            });
        div()
            .size_full()
            .bg(rgb(p.canvas))
            .text_color(rgb(p.text))
            .font_family("Yu Gothic UI")
            .text_size(px(14.))
            .line_height(px(22.4))
            .child(
                v_flex()
                    .size_full()
                    .when(self.design == 3, |e| {
                        e.child(
                            TitleBar::new()
                                .h(px(38.))
                                .bg(rgb(p.canvas))
                                .border_color(rgb(p.line))
                                .child(
                                    h_flex().gap_2().child(self.app_mark(18.)).child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(rgb(p.muted))
                                            .child("DiToVR"),
                                    ),
                                ),
                        )
                    })
                    .child(
                        v_flex()
                            .px(px(if self.design == 2 { 40. } else { 28. }))
                            .pt(px(if self.design == 3 { 20. } else { 24. }))
                            .child(
                                div()
                                    .mb(px(if self.design == 3 { 14. } else { 20. }))
                                    .child(if self.look_b {
                                        self.header_b(cx)
                                    } else {
                                        self.header(cx)
                                    }),
                            )
                            .child(self.nav(cx))
                            .when(!self.look_b, |e| e.child(self.status()))
                            .when(
                                self.runtime_view.as_ref().is_some_and(|v| v.onboarding),
                                |e| e.child(self.onboarding_banner(cx)),
                            )
                            .when(self.design == 3 && self.notice, |e| {
                                e.child(self.refined_notice(cx))
                            }),
                    )
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .min_h_0()
                            .child(
                                div()
                                    .id("page-scroll")
                                    .size_full()
                                    .overflow_y_scroll()
                                    .track_scroll(&self.scroll)
                                    .px(px(if self.design == 2 { 40. } else { 28. }))
                                    .pt(px(if self.design == 3 { 22. } else { 30. }))
                                    .pb(px(if self.design == 3 { 22. } else { 30. }))
                                    .child(if self.look_b {
                                        let enter = self.intro.value(now);
                                        div()
                                            .relative()
                                            .top(px((1. - enter) * 10.))
                                            .opacity(enter)
                                            .rounded(px(16.))
                                            .border_1()
                                            .border_color(rgb(p.line))
                                            .bg(rgb(p.surface))
                                            .p(px(22.))
                                            .child(content)
                                            .into_any_element()
                                    } else {
                                        content.into_any_element()
                                    }),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .right_0()
                                    .bottom_0()
                                    .w(px(16.))
                                    .child(
                                        Scrollbar::vertical(&self.scroll)
                                            .scrollbar_show(ScrollbarShow::Always),
                                    ),
                            ),
                    )
                    .when(
                        self.design == 3
                            && (self.footer_motion.value(now) > 0.
                                || self.footer_motion.active(now)),
                        |e| e.child(self.refined_footer(cx)),
                    )
                    .when(self.design != 3, |e| {
                        e.child(
                            v_flex()
                                .gap_2()
                                .px_7()
                                .py(px(14.))
                                .when(self.runtime.is_some(), |e| e.min_h(px(104.)))
                                .border_t_1()
                                .border_color(rgb(p.line))
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .line_height(px(22.))
                                        .text_color(rgb(if self.error {
                                            if self.dark { 0xffb4ab } else { 0xaa3026 }
                                        } else {
                                            p.muted
                                        }))
                                        .opacity(self.feedback_motion.value(now))
                                        .child(self.message.clone()),
                                )
                                .child(
                                    h_flex()
                                        .justify_between()
                                        .child(self.note(if self.runtime.is_some() {
                                            "3 / GPUI · 実処理比較"
                                        } else {
                                            "GPUI デザイン試作 · 設定は保存されません"
                                        }))
                                        .when(self.runtime.is_some(), |e| {
                                            e.child(
                                                Button::new("save-runtime")
                                                    .primary()
                                                    .label("保存")
                                                    .disabled(self.runtime_busy)
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.runtime_action("save", cx)
                                                    })),
                                            )
                                        })
                                        .child(self.note(format!(
                                            "案{} / {}",
                                            self.design,
                                            NAMES[self.design - 1]
                                        ))),
                                ),
                        )
                    }),
            )
    }
}

fn main() {
    ditovr_core::system::init_display_scaling();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help") {
        ditovr_core::system::attach_console();
        println!(
            "DiToVR\n--setup: 設定を開く\n--sound-only / --normal: 通知音のみ / 通常表示を指定\n-debug / --debug: XSOverlay を使わない Discord 受信診断\n--ui-preview: ダミー表示（実設定・外部接続なし）"
        );
        return;
    }
    let has = |name: &str| args.iter().any(|a| a == name);
    let options = runtime::Options {
        preview: has("--ui-preview"),
        setup: has("--setup"),
        launch_mode: if has("--sound-only") {
            Some(true)
        } else if has("--normal") {
            Some(false)
        } else {
            None
        },
        debug: has("-debug") || has("--debug"),
    };
    // 2回目の起動は既存のウィンドウを復帰して終了する。プレビューは別のインスタンス名。
    let _instance = match shell::single_instance(options.preview) {
        Some(guard) => guard,
        None => return,
    };
    runtime::OPTIONS.set(options).ok();
    Application::new()
        .with_assets(PreviewAssets)
        .run(move |cx: &mut App| {
            gpui_component::init(cx);
            cx.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            let bounds = Bounds::centered(None, size(px(760.), px(820.)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(620.), px(580.))),
                    titlebar: Some(TitlebarOptions {
                        appears_transparent: true,
                        title: Some(shell::WINDOW_TITLE.into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                move |window, cx| {
                    let view = cx.new(|cx| Workspace::new(3, false, window, cx));
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .expect("画面を開けませんでした。");
            cx.activate(true);
        });
}

struct PreviewAssets;
impl AssetSource for PreviewAssets {
    fn load(&self, path: &str) -> anyhow::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        let asset: Option<&'static [u8]> = match path {
            "ditovr/vr-mark.svg" => Some(include_bytes!("../assets/vr-mark.svg")),
            "ditovr/headphones.svg" => Some(include_bytes!("../assets/headphones.svg")),
            "ditovr/play.svg" => Some(include_bytes!("../assets/play.svg")),
            "ditovr/pause.svg" => Some(include_bytes!("../assets/pause.svg")),
            "ditovr/file-audio.svg" => Some(include_bytes!("../assets/file-audio.svg")),
            _ => None,
        };
        if let Some(bytes) = asset {
            Ok(Some(std::borrow::Cow::Borrowed(bytes)))
        } else {
            gpui_component_assets::Assets.load(path)
        }
    }
    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        gpui_component_assets::Assets.list(path)
    }
}

fn nav_geometry(value: f32) -> (f32, f32) {
    let starts = [0., 66., 132., 224.];
    let widths = [58., 58., 84., 58.];
    let value = value.clamp(0., 3.);
    let i = (value.floor() as usize).min(2);
    let t = value - i as f32;
    (
        starts[i] + (starts[i + 1] - starts[i]) * t,
        widths[i] + (widths[i + 1] - widths[i]) * t,
    )
}
