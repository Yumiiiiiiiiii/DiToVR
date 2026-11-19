//! 保存済みの値と編集中の値を比較し、保存欄と取り消しを管理する。
//! Secret・Tokenは複製もDebug／Serializeも行わず、入力の有無だけで変更を判定する。
//! 数値も文字列で比較し、入力途中や不正な値を「未変更」として扱わない。
use super::*;

#[derive(Clone, PartialEq)]
pub(super) struct SettingsDraft {
    sound_only: bool,
    sound: Sound,
    file: Option<PathBuf>,
    volume: f32,
    dark: bool,
    reduced: bool,
    auto_exit: bool,
    demo_bot: bool,
    receive: usize,
    client_id: String,
    allowlist: Vec<String>,
    redirect_uri: String,
    bot_guild: String,
    bot_user: String,
    numbers: Vec<String>,
}

impl Workspace {
    fn capture_draft(&self, cx: &App) -> SettingsDraft {
        let view = self.runtime_view.as_ref();
        let inputs = self.detail_inputs.as_ref();
        let credential = |index: usize, fallback: String| {
            inputs
                .and_then(|i| i.credentials.get(index))
                .map(|i| i.read(cx).value().to_string())
                .unwrap_or(fallback)
        };
        SettingsDraft {
            sound_only: self.sound_only,
            sound: self.sound,
            file: if self.sound == Sound::File {
                self.file.clone()
            } else {
                None
            },
            volume: self.volume_value,
            dark: self.dark,
            reduced: self.reduced,
            auto_exit: self.auto_exit,
            demo_bot: self.demo_bot,
            receive: self.receive.read(cx).selected,
            client_id: inputs
                .map(|i| i.client_id.read(cx).value().to_string())
                .unwrap_or_else(|| view.map(|v| v.client_id.clone()).unwrap_or_default()),
            allowlist: self.allowlist.clone(),
            redirect_uri: credential(1, view.map(|v| v.redirect_uri.clone()).unwrap_or_default()),
            bot_guild: credential(3, view.map(|v| v.bot_guild.clone()).unwrap_or_default()),
            bot_user: credential(4, view.map(|v| v.bot_user.clone()).unwrap_or_default()),
            numbers: if self.runtime_numbers.is_empty() {
                view.map(|v| {
                    vec![
                        v.timeout.to_string(),
                        v.max_title.to_string(),
                        v.max_content.to_string(),
                    ]
                })
                .unwrap_or_default()
            } else {
                self.runtime_numbers
                    .iter()
                    .map(|i| i.read(cx).value().to_string())
                    .collect()
            },
        }
    }

    pub(super) fn commit_draft(&mut self, cx: &App) {
        self.saved_draft = Some(self.capture_draft(cx));
    }

    pub(super) fn is_dirty(&self, cx: &App) -> bool {
        let secret_edited = self.detail_inputs.as_ref().is_some_and(|inputs| {
            [0, 2].into_iter().any(|i| {
                inputs
                    .credentials
                    .get(i)
                    .is_some_and(|input| !input.read(cx).value().is_empty())
            })
        });
        secret_edited
            || self
                .saved_draft
                .as_ref()
                .is_some_and(|saved| *saved != self.capture_draft(cx))
    }

    pub(super) fn discard_draft(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(saved) = self.saved_draft.clone() else {
            return;
        };
        let now = Instant::now();
        self.reduced = saved.reduced;
        self.sound_only = saved.sound_only;
        self.file = saved.file;
        self.set_sound(saved.sound);
        self.volume_value = saved.volume;
        self.volume_motion.set(saved.volume, now, self.reduced);
        self.mode_motion
            .set(saved.sound_only as u8 as f32, now, self.reduced);
        self.auto_exit = saved.auto_exit;
        self.demo_bot = saved.demo_bot;
        self.allowlist = saved.allowlist;
        self.clear_allowlist_input(window, cx);
        self.dark = saved.dark;
        cx.set_global(MotionSettings(self.reduced));
        theme::apply(self.dark, window, cx);
        self.receive.update(cx, |s, cx| {
            s.selected = saved.receive;
            s.close(cx);
        });
        self.source.update(cx, |s, cx| {
            s.selected = match saved.sound {
                Sound::Silent => 0,
                Sound::Chime => 1,
                Sound::File => 2,
            };
            s.close(cx);
        });
        self.sync_toggles(now);
        if let Some(inputs) = &self.detail_inputs {
            inputs
                .client_id
                .update(cx, |s, cx| s.set_value(saved.client_id, window, cx));
            // 空欄は保存済み認証情報の保持を意味する。復元のために秘密値を読み出さない。
            for (input, value) in inputs.credentials.iter().zip([
                String::new(),
                saved.redirect_uri,
                String::new(),
                saved.bot_guild,
                saved.bot_user,
            ]) {
                input.update(cx, |s, cx| s.set_value(value, window, cx));
            }
        }
        for (input, value) in self.runtime_numbers.iter().zip(saved.numbers) {
            input.update(cx, |s, cx| s.set_value(value, window, cx));
        }
        self.notice = false;
        self.error = false;
        cx.notify();
    }

    pub(super) fn watch_number_inputs(&mut self, cx: &mut Context<Self>) {
        self.number_subscriptions = self
            .runtime_numbers
            .iter()
            .map(|input| {
                cx.subscribe(input, |_, _, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        cx.notify();
                    }
                })
            })
            .collect();
    }

    pub(super) fn watch_detail_inputs(&mut self, cx: &mut Context<Self>) {
        let Some(inputs) = &self.detail_inputs else {
            return;
        };
        self.detail_subscriptions = std::iter::once(&inputs.client_id)
            .chain(inputs.credentials.iter())
            .map(|input| {
                cx.subscribe(input, |_, _, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        cx.notify();
                    }
                })
            })
            .collect();
    }
}
