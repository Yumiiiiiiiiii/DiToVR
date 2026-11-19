//! 選択された③の通知画面と、変更時だけ表示する保存欄。
//! 試聴は編集中の設定、通知テストはCoreに保存した設定を使用する。
use super::*;

impl Workspace {
    pub(super) fn refined_home(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let p = self.p();
        let stacked = window.viewport_size().width < px(900.);
        v_flex()
            .gap(px(20.))
            .child(
                v_flex().gap(px(4.)).child(
                    div()
                        .text_size(px(25.))
                        .line_height(px(36.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("通知の設定"),
                ),
            )
            .child(
                div()
                    .flex()
                    .w_full()
                    .gap(px(if stacked { 20. } else { 28. }))
                    .when(stacked, |e| e.flex_col())
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .child(self.section("01", "通知の届け方", ""))
                            .child(self.modes(cx))
                            .child(
                                h_flex()
                                    .mt(px(12.))
                                    .gap_4()
                                    .justify_between()
                                    .when(!stacked, |e| e.flex_col().items_start().gap_2())
                                    .child(
                                        v_flex()
                                            .flex_shrink_0()
                                            .child(
                                                div()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .child("受信する通知"),
                                            )
                                            .when(self.receive.read(cx).selected != 0, |e| {
                                                e.child(self.note("一覧は「接続」で編集"))
                                            }),
                                    )
                                    .child(
                                        div()
                                            .w(px(if stacked { 292. } else { 280. }))
                                            .child(Select::new(&self.receive).w_full()),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .border_color(rgb(p.line))
                            .when(stacked, |e| e.border_t_1().pt(px(18.)))
                            .when(!stacked, |e| e.border_l_1().pl(px(28.)))
                            .child(self.refined_audio(window, cx)),
                    ),
            )
            .into_any_element()
    }

    fn refined_audio(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let p = self.p();
        let stacked = window.viewport_size().width < px(900.);
        let file_name = self
            .file
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "ファイルは未選択".into());
        let external = self.runtime_view.as_ref().is_some_and(|v| !v.preview);
        v_flex()
            .min_w_0()
            .child(self.section("02", "通知音とテスト", ""))
            .child(
                h_flex()
                    .justify_between()
                    .gap_4()
                    .when(!stacked, |e| e.flex_col().items_start().gap_2())
                    .child(div().font_weight(FontWeight::MEDIUM).child("音源"))
                    .child(
                        div()
                            .w(px(if stacked { 292. } else { 280. }))
                            .child(Select::new(&self.source).disabled(self.playing).w_full()),
                    ),
            )
            .when(
                self.file_motion.value(Instant::now()) > 0.
                    || self.file_motion.active(Instant::now()),
                |e| {
                    e.child(
                        self.reveal(
                            v_flex()
                                .pt_3()
                                .pb_1()
                                .gap_2()
                                .child(
                                    div()
                                        .text_size(px(12.))
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
                                                .small()
                                                .icon(Icon::empty().path("ditovr/file-audio.svg"))
                                                .label(if self.picking {
                                                    "選択中…"
                                                } else {
                                                    "ファイルを選ぶ"
                                                })
                                                .disabled(
                                                    self.picking
                                                        || self.playing
                                                        || self.sound != Sound::File,
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
                    .mt(px(14.))
                    .gap(px(8.))
                    .child(
                        h_flex().justify_between().child("音量").child(
                            div()
                                .text_color(rgb(p.blue))
                                .child(format!("{:.0}%", self.volume_value)),
                        ),
                    )
                    .child(
                        div()
                            .id("volume-keyboard")
                            .relative()
                            .track_focus(&self.volume_focus.clone().tab_index(0).tab_stop(true))
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
                                e.child(
                                    div()
                                        .absolute()
                                        .inset(px(-4.))
                                        .border_2()
                                        .border_color(rgb(p.blue))
                                        .rounded(px(8.)),
                                )
                            }),
                    )
                    .child(
                        h_flex().justify_end().gap_3().flex_wrap().child(
                            h_flex()
                                .gap_2()
                                .child(
                                    Button::new("listen")
                                        .ghost()
                                        .small()
                                        .icon(Icon::empty().path("ditovr/play.svg"))
                                        .label("音を試聴")
                                        .disabled(self.playing || self.runtime_busy)
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.test(true, cx)),
                                        ),
                                )
                                .child(
                                    Button::new("test")
                                        .small()
                                        .icon(IconName::Bell)
                                        .label("通知を試す")
                                        .disabled(self.playing || self.runtime_busy)
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.test(false, cx)),
                                        ),
                                ),
                        ),
                    ),
            )
            .child(div().mt(px(8.)).child(self.note(if external {
                "試聴は編集中の設定、通知テストは保存した設定を使います。"
            } else {
                "試聴は最大4秒。デモでは外部に通知を送りません。"
            })))
            .into_any_element()
    }

    pub(super) fn refined_footer(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.p();
        let progress = self.footer_motion.value(Instant::now()).clamp(0., 1.);
        // 高さは一度だけ確保し、中身だけを動かしてスクロール領域の再計算を避ける。
        let offset = if self.reduced {
            0.
        } else {
            8. * (1. - progress)
        };
        div()
            .flex_shrink_0()
            .h(px(72.))
            .overflow_hidden()
            .child(
                h_flex()
                    .relative()
                    .top(px(offset))
                    .h(px(72.))
                    .w_full()
                    .justify_between()
                    .gap_4()
                    .px_7()
                    .border_t_1()
                    .border_color(rgb(p.line))
                    .bg(rgb(p.canvas))
                    .opacity(progress)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(12.))
                            .text_color(rgb(p.muted))
                            .child(
                                v_flex().gap(px(2.)).child(
                                    h_flex()
                                        .gap_2()
                                        .child(div().size(px(5.)).rounded_full().bg(rgb(p.blue)))
                                        .child(
                                            div()
                                                .text_size(px(13.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(rgb(p.text))
                                                .child(if self.runtime_saving {
                                                    "保存しています…"
                                                } else {
                                                    "未保存の変更"
                                                }),
                                        ),
                                ),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .flex_shrink_0()
                            .child(
                                Button::new("discard-draft")
                                    .label("取り消す")
                                    .disabled(self.runtime_busy || !self.footer_dirty)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.discard_draft(window, cx)
                                    })),
                            )
                            .child(
                                Button::new("save-runtime")
                                    .primary()
                                    .label("保存")
                                    .disabled(self.runtime_busy || !self.footer_dirty)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if this.runtime.is_some() {
                                            this.runtime_action("save", cx);
                                        } else {
                                            this.commit_draft(cx);
                                            this.feedback(
                                                "変更を反映しました（この起動中のみ）。",
                                                false,
                                                cx,
                                            );
                                        }
                                    })),
                            ),
                    ),
            )
            .into_any_element()
    }

    pub(super) fn refined_notice(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.p();
        h_flex()
            .pt_2()
            .gap_3()
            .opacity(self.feedback_motion.value(Instant::now()))
            .child(
                div()
                    .flex_1()
                    .text_size(px(12.))
                    .line_height(px(20.))
                    .text_color(rgb(if self.error {
                        if self.dark { 0xffb4ab } else { 0xaa3026 }
                    } else {
                        p.muted
                    }))
                    .child(self.message.clone()),
            )
            .child(
                Button::new("dismiss-notice")
                    .ghost()
                    .small()
                    .label("閉じる")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.notice = false;
                        cx.notify();
                    })),
            )
            .into_any_element()
    }
}
