//! 接続画面と詳細フォーム。Coreへのコマンド発行はserviceに委譲する。
use super::*;

impl Workspace {
    fn toggle_details(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.details = !self.details;
        // 一度作った入力欄を保持し、開閉だけで未保存の入力を失わないようにする。
        if self.details && self.detail_inputs.is_none() {
            self.detail_inputs = Some(DetailInputs {
                client_id: cx.new(|cx| InputState::new(window, cx).placeholder("Client ID")),
                credentials: vec![],
            });
            self.init_runtime_inputs(window, cx);
            self.watch_detail_inputs(cx);
        }
        self.detail_motion.set(
            self.details as u8 as f32,
            if self.reduced { 0. } else { 0.35 },
            Instant::now(),
        );
        cx.notify();
    }

    pub(super) fn connections(&self, cx: &mut Context<Self>) -> AnyElement {
        let now = Instant::now();
        let progress = self.detail_motion.value(now);
        v_flex()
            .child(self.heading("接続設定"))
            .child(self.row(
                "Discord",
                "",
                if self.runtime.is_some() {
                    self.note(if self.discord {
                        "接続済み"
                    } else {
                        "未接続"
                    })
                } else {
                    self.toggle(Toggle::DemoDiscord, cx)
                },
            ))
            .child(self.row(
                "XSOverlay",
                "",
                if self.runtime.is_some() {
                    self.note(if self.overlay {
                        "接続済み"
                    } else {
                        "未接続"
                    })
                } else {
                    self.toggle(Toggle::DemoOverlay, cx)
                },
            ))
            .when(self.runtime.is_some(), |e| {
                e.child(self.runtime_connection_actions(cx))
            })
            .child(self.allowlist_form(cx))
            .child(
                Button::new("details")
                    .ghost()
                    .justify_start()
                    .h(px(48.))
                    .px_0()
                    .small()
                    .icon(
                        Icon::new(IconName::ChevronRight)
                            .rotate(radians(progress * std::f32::consts::FRAC_PI_2)),
                    )
                    .label("詳細設定")
                    .on_click(cx.listener(|this, _, window, cx| this.toggle_details(window, cx))),
            )
            .when_some(
                self.detail_inputs
                    .as_ref()
                    .filter(|_| progress > 0. || self.detail_motion.active(now)),
                |e, inputs| {
                    e.child(self.reveal(
                        self.connection_details(inputs, cx),
                        progress,
                        self.detail_height.clone(),
                    ))
                },
            )
            .into_any_element()
    }

    fn connection_details(&self, inputs: &DetailInputs, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .gap_4()
            .p_5()
            .bg(rgb(self.p().surface))
            .rounded(px(8.))
            .child("OAuth2 / Client ID")
            .child(
                Input::new(&inputs.client_id)
                    .w_full()
                    .with_size(px(40.))
                    .min_h(px(40.))
                    .flex_shrink_0()
                    .text_size(px(13.))
                    .disabled(!self.details),
            )
            .child(self.note(if self.runtime.is_some() {
                "Secret・Tokenは空欄で現在の値を保持します。"
            } else {
                "デモ入力です。実際の認証情報は入力しないでください。"
            }))
            .when(self.runtime.is_none(), |e| {
                e.child(self.row(
                    "Botによる状態判別",
                    "Botは状態判別用です。通知はDiscordから受信します。",
                    self.toggle(Toggle::DemoBot, cx),
                ))
            })
            .when(self.runtime.is_some(), |e| {
                e.children(self.runtime_credential_fields())
            })
            .opacity(self.detail_motion.value(Instant::now()))
            .into_any_element()
    }
}
