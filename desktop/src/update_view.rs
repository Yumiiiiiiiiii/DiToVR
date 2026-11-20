//! 更新の設定表示と初回案内。更新処理本体は update.rs、接続処理は runtime.rs が担当する。
use super::*;
use crate::update::{Host, Phase, Updater};
use std::sync::{Arc, atomic::AtomicBool};

/// 更新の適用前に、編集中の設定がなく受信を止められるかを確認する。
pub(super) struct UpdateHost {
    pub runtime: crate::runtime::Runtime,
    pub editing: Arc<AtomicBool>,
    /// 更新のために受信を止めたとき、止める前に動いていたか。ユーザーが止めていた受信は再開しない。
    was_running: AtomicBool,
}
impl UpdateHost {
    pub fn new(runtime: crate::runtime::Runtime, editing: Arc<AtomicBool>) -> Self {
        Self {
            runtime,
            editing,
            was_running: AtomicBool::new(false),
        }
    }
}
impl Host for UpdateHost {
    fn prepare(&self) -> bool {
        use std::sync::atomic::Ordering::SeqCst;
        if self.editing.load(SeqCst) {
            return false;
        }
        let running = self.runtime.services_running();
        let paused = self.runtime.execute("pause", Default::default()).is_ok();
        if paused {
            self.was_running.store(running, SeqCst);
        }
        paused
    }
    fn can_install(&self) -> bool {
        !self.editing.load(std::sync::atomic::Ordering::SeqCst)
    }
    fn cancel(&self) {
        if self
            .was_running
            .swap(false, std::sync::atomic::Ordering::SeqCst)
        {
            let _ = self.runtime.execute("connect", Default::default());
        }
    }
}

fn phase_text(phase: Phase, version: Option<&str>, downloaded: u64, total: Option<u64>) -> String {
    let version = version.unwrap_or("");
    match phase {
        Phase::Idle => "確認していません。".into(),
        Phase::Checking => "更新を確認しています…".into(),
        Phase::Current => "最新の版です。".into(),
        Phase::Available => format!("新しい版 {version} があります。"),
        Phase::Downloading => match total {
            Some(total) if total > 0 => format!(
                "{version} をダウンロード中 ({}%)",
                downloaded.min(total) * 100 / total
            ),
            _ => format!("{version} をダウンロード中…"),
        },
        Phase::Waiting => format!("{version} を適用できるまで待機しています。"),
        Phase::Installing => format!("{version} を適用しています。"),
        Phase::Error => "更新に失敗しました。".into(),
    }
}

impl Workspace {
    /// 更新の状態が変わったか。変わっていたら記録して true。
    pub(super) fn refresh_update_status(&mut self) -> bool {
        let Some(updater) = &self.updater else {
            return false;
        };
        let s = updater.snapshot();
        let key = (
            s.phase,
            s.downloaded,
            s.auto_update,
            s.message.clone(),
            s.version.clone(),
        );
        if self.update_key.as_ref() == Some(&key) {
            return false;
        }
        self.update_key = Some(key);
        true
    }

    pub(super) fn init_updater(&mut self, runtime: &crate::runtime::Runtime) {
        let options = crate::runtime::OPTIONS.get().cloned().unwrap_or_default();
        let (directory, sources) = match std::env::var_os("APPDATA") {
            Some(appdata) if !options.preview => {
                let appdata = PathBuf::from(appdata);
                let sources = [
                    appdata
                        .join(crate::paths::PREVIOUS_CONFIG_DIRECTORY)
                        .join("updates.json"),
                    appdata
                        .join(crate::paths::TAURI_IDENTIFIER)
                        .join("updates.json"),
                ];
                (
                    appdata.join(crate::paths::CONFIG_DIRECTORY),
                    sources.to_vec(),
                )
            }
            _ => (
                std::env::temp_dir().join("ditovr-preview-no-writes"),
                vec![],
            ),
        };
        let path = directory.join("updates.json");
        // 旧保存先の設定は、Updater が読む前に移行する（読込後だと今回の起動に反映されない）。
        let migration_failed = !options.preview && !crate::update::store_managed() && {
            let refs: Vec<&std::path::Path> = sources.iter().map(|p| p.as_path()).collect();
            crate::update::migrate_preferences_from(&refs, &path).is_err()
        };
        let updater = Updater::new(path.clone(), options.preview, options.debug);
        if migration_failed {
            updater.report_migration_failure();
        }
        updater.start(Arc::new(UpdateHost::new(
            runtime.clone(),
            self.editing.clone(),
        )));
        self.updater = Some(updater);
    }

    /// 動作設定ページの更新欄。Store版は更新をStoreに任せる旨だけ表示する。
    pub(super) fn update_section(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(updater) = self.updater.clone() else {
            return div().into_any_element();
        };
        let s = updater.snapshot();
        if s.store_managed {
            return self.row(
                "更新",
                "更新は Microsoft Store が管理します。Microsoft Store の「ライブラリ」で確認できます。",
                self.note(format!("v{}", s.current_version)),
            );
        }
        let detail = match (&s.message, &s.disabled_reason) {
            (Some(message), _) => message.clone(),
            (None, Some(reason)) if s.phase == Phase::Idle => reason.clone(),
            _ => phase_text(s.phase, s.version.as_deref(), s.downloaded, s.total),
        };
        let busy = matches!(
            s.phase,
            Phase::Checking | Phase::Downloading | Phase::Waiting | Phase::Installing
        );
        let toggle_updater = updater.clone();
        let check_updater = updater;
        let auto = s.auto_update;
        let host = Arc::new(UpdateHost::new(
            self.runtime.clone().expect("runtime"),
            self.editing.clone(),
        ));
        self.row(
            "自動アップデート",
            &format!("v{} · {detail}", s.current_version),
            h_flex()
                .gap_2()
                .child(
                    Button::new("update-check")
                        .label("今すぐ確認")
                        .disabled(busy || !s.can_check)
                        .on_click(cx.listener(move |_, _, _, cx| {
                            check_updater.check_now(host.clone());
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("update-auto")
                        .label(if auto {
                            "自動: オン"
                        } else {
                            "自動: オフ"
                        })
                        .disabled(s.phase == Phase::Installing)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Err(message) = toggle_updater.set_auto_update(!auto) {
                                this.feedback(message, true, cx);
                            }
                            cx.notify();
                        })),
                ),
        )
    }

    /// 初回案内。設定が未完了のときだけ通知画面の上に出す。
    pub(super) fn onboarding_banner(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.p();
        v_flex()
            .mt_3()
            .gap_2()
            .p_4()
            .rounded(px(10.))
            .border_1()
            .border_color(rgb(p.blue))
            .child(div().font_weight(FontWeight::SEMIBOLD).child("最初の接続を整えましょう"))
            .child(self.note(
                "Discord の開発者ポータルでアプリを作り、「接続」でクライアント ID と Secret を入力すると VR に通知が届きます。通知の本文や認証情報は外部へ送りません。",
            ))
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("onboarding-portal")
                            .label("開発者ポータルを開く")
                            .on_click(|_, _, _| {
                                crate::shell::open_url("https://discord.com/developers/applications")
                            }),
                    )
                    .child(
                        Button::new("onboarding-connect")
                            .primary()
                            .label("接続設定を開く")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.finish_onboarding(cx);
                                this.go_to_page(1, cx);
                            })),
                    )
                    .child(
                        Button::new("onboarding-later")
                            .ghost()
                            .label("あとで")
                            .on_click(cx.listener(|this, _, _, cx| this.finish_onboarding(cx))),
                    ),
            )
            .into_any_element()
    }

    /// Runtime::execute は状態ロックを待つため、UIスレッドでは呼ばず背景で実行して結果だけ反映する。
    pub(super) fn finish_onboarding(&mut self, cx: &mut Context<Self>) {
        let Some(runtime) = self.runtime.clone() else {
            cx.notify();
            return;
        };
        cx.spawn(async move |weak, cx| {
            let view = cx
                .background_executor()
                .spawn(async move { runtime.execute("onboarding", Default::default()).ok() })
                .await;
            let _ = weak.update(cx, |this, cx| {
                if let Some(view) = view {
                    this.runtime_view = Some(view);
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn go_to_page(&mut self, page: usize, cx: &mut Context<Self>) {
        if self.page == page {
            return;
        }
        let now = Instant::now();
        self.page = page;
        self.scroll.set_offset(point(px(0.), px(0.)));
        self.nav_motion.set(page as f32, now, self.reduced);
        self.receive.update(cx, |s, cx| s.close(cx));
        self.source.update(cx, |s, cx| s.close(cx));
        self.panel.select(page, now, self.reduced);
        cx.notify();
    }
}
