//! スイッチの状態と変更処理。デモ用BotとVR連動終了は別の設定として扱う。
use super::*;

#[derive(Clone, Copy)]
pub(super) enum Toggle {
    DemoDiscord,
    DemoOverlay,
    AutoExit,
    Theme,
    ReducedMotion,
    Pause,
    DemoBot,
}

impl Toggle {
    const ALL: [Self; 7] = [
        Self::DemoDiscord,
        Self::DemoOverlay,
        Self::AutoExit,
        Self::Theme,
        Self::ReducedMotion,
        Self::Pause,
        Self::DemoBot,
    ];
}

impl Workspace {
    fn toggle_value(&self, setting: Toggle) -> bool {
        match setting {
            Toggle::DemoDiscord => self.discord,
            Toggle::DemoOverlay => self.overlay,
            Toggle::AutoExit => self.auto_exit,
            Toggle::Theme => self.dark,
            Toggle::ReducedMotion => self.reduced,
            Toggle::Pause => self.paused,
            Toggle::DemoBot => self.demo_bot,
        }
    }

    /// 保存・取り消し・実状態の読み込み後も、つまみの表示を同じ経路で同期する。
    pub(super) fn sync_toggles(&mut self, now: Instant) {
        for setting in Toggle::ALL {
            let value = self.toggle_value(setting);
            self.toggles[setting as usize].set(value as u8 as f32, now, self.reduced);
        }
    }

    pub(super) fn toggle_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let now = Instant::now();
        self.dark = !self.dark;
        self.theme_swap.set(0., 0., now);
        self.theme_swap.set(1., 0.3, now);
        self.sync_toggles(now);
        theme::apply(self.dark, window, cx);
        cx.notify();
    }

    pub(super) fn toggle_pause(&mut self, cx: &mut Context<Self>) {
        if self.runtime.is_some() {
            self.runtime_action(if self.paused { "connect" } else { "pause" }, cx);
        } else {
            self.paused = !self.paused;
            self.sync_toggles(Instant::now());
            cx.notify();
        }
    }

    pub(super) fn toggle(&self, setting: Toggle, cx: &mut Context<Self>) -> AnyElement {
        let p = self.p();
        let t = self.toggles[setting as usize]
            .value(Instant::now())
            .clamp(0., 1.);
        Button::new(("toggle", setting as usize))
            .ghost()
            .disabled(matches!(setting, Toggle::DemoBot) && !self.details)
            .w(px(36.))
            .h(px(28.))
            .px_0()
            .child(
                div()
                    .w(px(34.))
                    .h(px(20.))
                    .rounded_full()
                    .relative()
                    .bg(rgb(motion::mix(p.input, p.blue, t)))
                    .child(
                        div()
                            .absolute()
                            .top(px(2.))
                            .left(px(2. + t * 14.))
                            .size(px(16.))
                            .rounded_full()
                            .bg(rgb(if self.dark {
                                motion::mix(p.text, p.on_blue, t)
                            } else {
                                0xffffff
                            })),
                    ),
            )
            .on_click(cx.listener(move |this, _, window, cx| {
                match setting {
                    Toggle::DemoDiscord => this.discord = !this.discord,
                    Toggle::DemoOverlay => this.overlay = !this.overlay,
                    Toggle::AutoExit => this.auto_exit = !this.auto_exit,
                    Toggle::DemoBot => this.demo_bot = !this.demo_bot,
                    Toggle::Theme => {
                        this.toggle_theme(window, cx);
                        return;
                    }
                    Toggle::Pause => {
                        this.toggle_pause(cx);
                        return;
                    }
                    Toggle::ReducedMotion => {
                        this.reduced = !this.reduced;
                        cx.set_global(MotionSettings(this.reduced));
                    }
                }
                let now = Instant::now();
                this.sync_toggles(now);
                if this.reduced {
                    this.nav_motion.finish(now);
                    this.mode_motion.finish(now);
                    this.volume_motion.finish(now);
                    this.panel.finish_movement(now);
                    this.detail_motion.finish();
                    this.file_motion.finish();
                }
                cx.notify();
            }))
            .into_any_element()
    }
}
