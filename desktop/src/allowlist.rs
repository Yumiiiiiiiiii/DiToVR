//! 現行版と同じ「貼り付け→追加→個別削除」の許可リスト。
//! 入力中の文字と登録済みIDを分け、保存するのは一覧のIDだけにする。
use super::*;
use crate::runtime::channel_id_from_input;

const LIMIT: usize = 32;

fn addition(entries: &[String], text: &str) -> Result<String, &'static str> {
    let id = channel_id_from_input(text)
        .ok_or("IDまたはDiscordのメッセージリンクを入力してください。")?;
    if entries.contains(&id) {
        return Err("登録済みです。");
    }
    if entries.len() >= LIMIT {
        return Err("登録は32件までです。");
    }
    Ok(id)
}

impl Workspace {
    pub(super) fn clear_allowlist_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.allowlist_error = None;
        self.allowlist_input
            .update(cx, |s, cx| s.set_value("", window, cx));
    }

    pub(super) fn add_allowed_channel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.runtime_busy {
            return;
        }
        match addition(&self.allowlist, &self.allowlist_input.read(cx).value()) {
            Ok(id) => {
                self.allowlist.push(id);
                self.clear_allowlist_input(window, cx);
            }
            Err(message) => self.allowlist_error = Some(message),
        }
        self.allowlist_input.update(cx, |s, cx| s.focus(window, cx));
        cx.notify();
    }

    pub(super) fn allowlist_form(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.p();
        v_flex()
            .gap_3()
            .mt_4()
            .pt_4()
            .pb_4()
            .border_t_1()
            .border_b_1()
            .border_color(rgb(p.line))
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(18.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("許可リスト"),
                    )
                    .child(self.note(format!("{} / {LIMIT}件", self.allowlist.len()))),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .child("チャンネルIDまたはメッセージリンク"),
            )
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        Input::new(&self.allowlist_input)
                            .flex_1()
                            .min_w_0()
                            .with_size(px(40.))
                            .h(px(40.))
                            .min_h(px(40.))
                            .text_size(px(13.))
                            .disabled(self.runtime_busy),
                    )
                    .child(
                        Button::new("add-channel")
                            .h(px(40.))
                            .icon(IconName::Plus)
                            .label("追加")
                            .disabled(self.runtime_busy || self.allowlist.len() >= LIMIT)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.add_allowed_channel(window, cx)
                            })),
                    ),
            )
            .when_some(self.allowlist_error, |e, message| {
                e.child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(if self.dark { 0xffb4ab } else { 0xaa3026 }))
                        .child(message),
                )
            })
            .when(self.receive.read(cx).selected != 0, |e| {
                e.child(self.note("登録した会話は、ミュート中も受信します。"))
            })
            .when(self.allowlist.is_empty(), |e| {
                e.child(self.note("登録なし"))
            })
            .children(self.allowlist.iter().map(|id| {
                let channel = id.clone();
                h_flex()
                    .gap_3()
                    .justify_between()
                    .py_1()
                    .child(div().min_w_0().text_size(px(13.)).child(id.clone()))
                    .child(
                        Button::new(SharedString::from(format!("remove-channel-{id}")))
                            .ghost()
                            .small()
                            .label("削除")
                            .disabled(self.runtime_busy)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.allowlist.retain(|entry| entry != &channel);
                                this.allowlist_error = None;
                                cx.notify();
                            })),
                    )
            }))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{LIMIT, addition};
    use crate::runtime::channel_id_from_input;

    #[test]
    fn accepts_current_id_range_and_discord_links() {
        for text in [
            " 123 ",
            "https://discord.com/channels/@me/123/456",
            "https://canary.discord.com/channels/999/123/",
            "https://ptb.discord.com/channels/999/123/456?jump=1#message",
        ] {
            assert_eq!(channel_id_from_input(text).as_deref(), Some("123"));
        }
        assert_eq!(
            channel_id_from_input("18446744073709551615").as_deref(),
            Some("18446744073709551615")
        );
    }

    #[test]
    fn rejects_invalid_ids_and_untrusted_links() {
        for text in [
            "",
            "0",
            "01",
            "18446744073709551616",
            "１２３",
            "1,2",
            "http://discord.com/channels/@me/123/456",
            "https://discord.com.example.com/channels/@me/123/456",
            "https://user@discord.com/channels/@me/123/456",
            "https://discord.com/channels/@me/+123/456",
            "https://discord.com/channels/@me/123/not-a-message",
            "https://discord.com/channels/@me/123//",
            "https://discord.com/channels/@me/%31%32%33/456",
        ] {
            assert!(channel_id_from_input(text).is_none(), "accepted {text}");
        }
    }

    #[test]
    fn rejects_duplicates_and_overflow_without_changing_entries() {
        let mut entries = vec!["123".to_owned()];
        assert!(addition(&entries, "https://discord.com/channels/@me/123/456").is_err());
        entries = (1..=LIMIT).map(|id| id.to_string()).collect();
        assert!(addition(&entries, "999").is_err());
        assert_eq!(entries.len(), LIMIT);
        entries.pop();
        assert_eq!(addition(&entries, "999").as_deref(), Ok("999"));
    }
}
