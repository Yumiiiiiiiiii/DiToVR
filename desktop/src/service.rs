//! GPUIと共通Coreの境界。ブロックする処理はワーカーで実行し、UIにはViewだけを返す。
use super::*;
use crate::runtime::{Patch, Receive, Runtime, Theme, View};
use std::time::Duration;

// 作成時のプレースホルダーと表示ラベルを同じ定義にする。boolは伏字入力の指定。
const CREDENTIAL_FIELDS: [(&str, bool); 5] = [
    ("Client Secret（空欄なら保持）", true),
    ("Redirect URI", false),
    ("Bot Token（空欄なら保持）", true),
    ("サーバーID", false),
    ("自分のユーザーID", false),
];

impl Workspace {
    /// 接続状態の取得では編集中の設定を上書きしない。
    fn update_runtime_status(&mut self, view: &View) {
        self.discord = view.discord == 2;
        self.overlay = view.overlay;
        self.paused = !view.running;
        self.tests = view.tests as usize;
        self.runtime_view = Some(view.clone());
    }

    pub(super) fn ensure_runtime_numbers(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.runtime_numbers.is_empty() {
            return;
        }
        let Some(view) = self.runtime_view.as_ref() else {
            return;
        };
        self.runtime_numbers = [
            view.timeout.to_string(),
            view.max_title.to_string(),
            view.max_content.to_string(),
        ]
        .into_iter()
        .map(|value| cx.new(|cx| InputState::new(window, cx).default_value(value)))
        .collect();
        self.watch_number_inputs(cx);
    }
    pub(super) fn init_runtime(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let options = crate::runtime::OPTIONS.get().cloned().unwrap_or_default();
        match Runtime::new(&options) {
            Ok(runtime) => {
                // 自動接続は音声変換などで状態ロックを長く保持する。先に初期表示用の状態を読み、
                // 起動直後の画面（UIスレッド）がその完了を待たないようにする。
                let v = runtime.view();
                if !options.setup {
                    let starter = runtime.clone();
                    std::thread::spawn(move || starter.autostart());
                }
                self.apply_runtime(&v, window, cx);
                self.message = v.message.clone();
                self.error = v.error;
                self.runtime = Some(runtime.clone());
                self.init_updater(&runtime);
                self.init_runtime_tray(window, cx);
                use raw_window_handle::{HasWindowHandle, RawWindowHandle};
                let handle = match HasWindowHandle::window_handle(window).map(|h| h.as_raw()) {
                    Ok(RawWindowHandle::Win32(h)) => h.hwnd.get() as usize,
                    _ => 0,
                };
                if handle != 0 {
                    crate::shell::tag_window(handle, options.preview);
                }
                cx.spawn(async move |weak, cx| {
                    loop {
                        cx.background_executor().timer(Duration::from_secs(1)).await;
                        if handle != 0 && !crate::shell::is_visible(handle) {
                            if runtime_exit(&weak, cx) {
                                break;
                            }
                            continue;
                        }
                        let runtime = runtime.clone();
                        let view = cx
                            .background_executor()
                            .spawn(async move { runtime.view() })
                            .await;
                        let result = weak.update(cx, |this, cx| {
                            let update_changed = this.refresh_update_status();
                            if this.runtime_view.as_ref() != Some(&view) {
                                this.update_runtime_status(&view);
                                cx.notify();
                            } else if update_changed {
                                cx.notify();
                            }
                        });
                        if result.is_err() {
                            break;
                        }
                        if runtime_exit(&weak, cx) {
                            break;
                        }
                    }
                })
                .detach();
            }
            Err(message) => self.feedback(message, true, cx),
        }
    }
    /// 初期読み込みと保存・取込の成功時だけ、フォーム全体と表示状態を更新する。
    pub(super) fn apply_runtime(&mut self, v: &View, window: &mut Window, cx: &mut Context<Self>) {
        self.update_runtime_status(v);
        self.auto_exit = v.auto_exit;
        self.dark = v.theme != Theme::Light;
        self.sync_toggles(Instant::now());
        self.volume_value = v.volume as f32;
        self.volume_motion.snap(v.volume as f32, Instant::now());
        self.volume
            .update(cx, |s, cx| s.set_value(v.volume as f32, window, cx));
        theme::apply(self.dark, window, cx);
        self.sound_only = v.sound_only;
        self.mode_motion = Spring::selection(v.sound_only as u8 as f32);
        self.set_sound(if v.sound.is_empty() {
            Sound::Silent
        } else if v.sound == "default" {
            Sound::Chime
        } else {
            Sound::File
        });
        self.file = if !v.sound.is_empty() && v.sound != "default" {
            Some(PathBuf::from(&v.sound))
        } else {
            None
        };
        self.receive.update(cx, |s, cx| {
            s.selected = match v.receive {
                Receive::Normal => 0,
                Receive::Whitelist => 1,
                Receive::Auto => 2,
            };
            cx.notify();
        });
        self.source.update(cx, |s, cx| {
            s.selected = match self.sound {
                Sound::Silent => 0,
                Sound::Chime => 1,
                Sound::File => 2,
            };
            cx.notify();
        });
        self.allowlist = v
            .allowlist
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|id| !id.is_empty())
            .map(str::to_owned)
            .collect();
        self.clear_allowlist_input(window, cx);
        if let Some(inputs) = self.detail_inputs.as_ref() {
            inputs
                .client_id
                .update(cx, |s, cx| s.set_value(v.client_id.clone(), window, cx));
            for (i, value) in [
                String::new(),
                v.redirect_uri.clone(),
                String::new(),
                v.bot_guild.clone(),
                v.bot_user.clone(),
            ]
            .into_iter()
            .enumerate()
            {
                if let Some(input) = inputs.credentials.get(i) {
                    input.update(cx, |s, cx| s.set_value(value, window, cx));
                }
            }
        }
        self.ensure_runtime_numbers(window, cx);
    }
    pub(super) fn init_runtime_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(view) = self.runtime_view.clone() else {
            return;
        };
        if let Some(inputs) = self.detail_inputs.as_mut() {
            inputs
                .client_id
                .update(cx, |s, cx| s.set_value(view.client_id.clone(), window, cx));
            inputs.credentials = CREDENTIAL_FIELDS
                .into_iter()
                .map(|(label, masked)| {
                    cx.new(|cx| {
                        InputState::new(window, cx)
                            .placeholder(label)
                            .masked(masked)
                    })
                })
                .collect();
            for (i, value) in [
                (1, view.redirect_uri),
                (3, view.bot_guild),
                (4, view.bot_user),
            ] {
                inputs.credentials[i].update(cx, |s, cx| s.set_value(value, window, cx));
            }
        }
    }
    pub(super) fn runtime_credential_fields(&self) -> Vec<AnyElement> {
        let Some(inputs) = self.detail_inputs.as_ref() else {
            return vec![];
        };
        CREDENTIAL_FIELDS
            .into_iter()
            .zip(&inputs.credentials)
            .map(|((name, _), input)| {
                v_flex()
                    .gap_2()
                    .child(name)
                    .child(
                        Input::new(input)
                            .w_full()
                            .min_h(px(40.))
                            .disabled(!self.details),
                    )
                    .into_any_element()
            })
            .collect()
    }
    fn runtime_patch(&self, cx: &App) -> Patch {
        let mut patch = Patch {
            volume: Some(self.volume_value as f64),
            sound_only: Some(self.sound_only),
            sound: Some(match self.sound {
                Sound::Silent => String::new(),
                Sound::Chime => "default".into(),
                Sound::File => self
                    .file
                    .as_ref()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            }),
            theme: Some(if self.dark { Theme::Dark } else { Theme::Light }),
            auto_exit: Some(self.auto_exit),
            allowlist: Some(self.allowlist.join(", ")),
            receive: Some(match self.receive.read(cx).selected {
                1 => Receive::Whitelist,
                2 => Receive::Auto,
                _ => Receive::Normal,
            }),
            ..Patch::default()
        };
        if let Some(inputs) = &self.detail_inputs {
            patch.client_id = Some(inputs.client_id.read(cx).value().to_string());
            // 認証情報は入力から直接Patchへ渡し、表示用Viewや複製した配列へ入れない。
            if let [secret, redirect_uri, bot_token, bot_guild, bot_user] =
                inputs.credentials.as_slice()
            {
                patch.secret = Some(secret.read(cx).value().to_string());
                patch.redirect_uri = Some(redirect_uri.read(cx).value().to_string());
                patch.bot_token = Some(bot_token.read(cx).value().to_string());
                patch.bot_guild = Some(bot_guild.read(cx).value().to_string());
                patch.bot_user = Some(bot_user.read(cx).value().to_string());
            }
        }
        if self.runtime_numbers.len() == 3 {
            patch.timeout = Some(
                self.runtime_numbers[0]
                    .read(cx)
                    .value()
                    .parse()
                    .unwrap_or(f64::NAN),
            );
            patch.max_title = Some(
                self.runtime_numbers[1]
                    .read(cx)
                    .value()
                    .parse()
                    .unwrap_or(0),
            );
            patch.max_content = Some(
                self.runtime_numbers[2]
                    .read(cx)
                    .value()
                    .parse()
                    .unwrap_or(0),
            );
        }
        patch
    }
    pub(super) fn runtime_number_fields(&self) -> AnyElement {
        v_flex()
            .gap_3()
            .py_4()
            .children(
                [
                    "通知の表示時間（1〜30秒）",
                    "タイトル上限（10〜200文字）",
                    "本文上限（10〜2000文字）",
                ]
                .into_iter()
                .zip(&self.runtime_numbers)
                .map(|(label, input)| {
                    v_flex()
                        .gap_2()
                        .child(label)
                        .child(Input::new(input).w_full())
                        .into_any_element()
                }),
            )
            .into_any_element()
    }
    pub(super) fn runtime_action(&mut self, action: &'static str, cx: &mut Context<Self>) {
        let Some(runtime) = self.runtime.clone() else {
            return;
        };
        if self.runtime_busy {
            return;
        }
        let patch = self.runtime_patch(cx);
        self.runtime_busy = true;
        self.runtime_saving = action == "save";
        self.feedback("処理中…", false, cx);
        cx.spawn(async move |weak, cx| {
            let worker = runtime.clone();
            let (result, current) = cx
                .background_executor()
                .spawn(async move {
                    let result = worker.execute(action, patch);
                    let current = worker.view();
                    (result, current)
                })
                .await;
            let _ = weak.update(cx, |this, cx| {
                this.runtime_busy = false;
                this.runtime_saving = false;
                this.update_runtime_status(&current);
                if action == "save" && result.is_ok() {
                    this.runtime_reload = true;
                    this.runtime_numbers.clear();
                    this.number_subscriptions.clear();
                    // 詳細フォームは次回開いたとき、保存結果から再構築する。
                    this.detail_inputs = None;
                    this.detail_subscriptions.clear();
                    this.details = false;
                    this.detail_motion.finish();
                    // 次の描画でapply_runtimeがフォームと保存基準をまとめて更新する。
                }
                match result {
                    Ok(v) => {
                        // 描画側で文言の前方一致を使わず、保存結果の種類で案内を選ぶ。
                        let message = if action == "save" && current.preview {
                            "変更を反映しました（この起動中のみ）。".to_owned()
                        } else {
                            v.message
                        };
                        this.feedback(message, false, cx);
                    }
                    Err(e) => this.feedback(settings_error(e), true, cx),
                }
            });
        })
        .detach();
    }
    pub(super) fn runtime_connection_actions(&self, cx: &mut Context<Self>) -> AnyElement {
        h_flex()
            .gap_3()
            .py_4()
            .child(
                Button::new("connect-runtime")
                    .primary()
                    .label(if self.paused {
                        "接続を開始"
                    } else {
                        "接続を停止"
                    })
                    .disabled(self.runtime_busy)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.runtime_action(if this.paused { "connect" } else { "pause" }, cx)
                    })),
            )
            .into_any_element()
    }
    pub(super) fn runtime_diagnostics(&self, cx: &mut Context<Self>) -> AnyElement {
        let counts = self
            .runtime_view
            .as_ref()
            .map(|v| {
                format!(
                    "実受信 {} / 実送信 {} / テスト検証 {}",
                    v.received, v.sent, v.tests
                )
            })
            .unwrap_or_default();
        v_flex()
            .gap_3()
            .py_4()
            .child(self.note(counts))
            .child(
                Button::new("diagnose-runtime")
                    .label("受信処理をテスト")
                    .disabled(self.runtime_busy)
                    .on_click(cx.listener(|this, _, _, cx| this.runtime_action("diagnose", cx))),
            )
            .child(
                Button::new("quit-runtime")
                    .label("終了")
                    .on_click(|_, _, cx| cx.quit()),
            )
            .into_any_element()
    }
    fn init_runtime_tray(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let handle = match HasWindowHandle::window_handle(window).map(|h| h.as_raw()) {
            Ok(RawWindowHandle::Win32(h)) => h.hwnd.get() as usize,
            _ => return,
        };
        let (tx, rx) = async_channel::unbounded();
        let tray = crate::shell::tray("DiToVR", move |event| {
            let _ = tx.try_send(event);
        });
        let Ok(tray) = tray else {
            return;
        };
        self.runtime_tray = Some(tray);
        window.on_window_should_close(cx, move |_, _| {
            crate::shell::set_visible(handle, false);
            false
        });
        cx.spawn(async move |weak, cx| {
            loop {
                let event = rx.recv().await;
                let Ok(event) = event else {
                    break;
                };
                if weak
                    .update(cx, |_, cx| match event {
                        crate::shell::ShellEvent::Show => crate::shell::set_visible(handle, true),
                        crate::shell::ShellEvent::Quit => cx.quit(),
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }
}
// Coreの設定キーを画面の項目名へ置き換える。エラーの理由や範囲はそのまま残す。
fn settings_error(mut message: String) -> String {
    for (key, label) in [
        ("NOTIFICATION_TIMEOUT", "通知の表示時間"),
        ("MAX_TITLE_LENGTH", "タイトル上限"),
        ("MAX_CONTENT_LENGTH", "本文上限"),
    ] {
        message = message.replace(key, label);
    }
    message
}

fn runtime_exit(weak: &WeakEntity<Workspace>, cx: &mut AsyncApp) -> bool {
    weak.update(cx, |this, cx| {
        if this
            .runtime
            .as_ref()
            .is_some_and(|r| r.exit.load(std::sync::atomic::Ordering::Relaxed))
        {
            cx.quit();
            true
        } else {
            false
        }
    })
    .unwrap_or(true)
}
