//! Experimental host adapter. Production Core stays unchanged.
//! All three renderers use these same services, settings, validation and audio.
use ditovr_core::{
    audio,
    config::{self, Config, ThemePreference},
    diagnostics::{Diagnostics, Source},
    discord::{
        notifier::DiscordRPCNotifier,
        presence,
        receive::{BotStatus, ReceiveControl, ReceiveMode},
    },
    system,
    vr::{self, XsOverlaySender},
};
pub use ditovr_core::{config::ThemePreference as Theme, discord::receive::ReceiveMode as Receive};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
    time::Duration,
};
use tokio::sync::Mutex;

/// メッセージリンクからチャンネルIDだけを取り出す。URLへの通信は行わない。
/// 現行React版と同じホスト・パス・u64範囲を受け付ける。
pub fn channel_id_from_input(text: &str) -> Option<String> {
    let text = text.trim();
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit());
    let id = if digits(text) {
        text.to_owned()
    } else {
        let url = reqwest::Url::parse(text).ok()?;
        if url.scheme() != "https"
            || !matches!(
                url.host_str()?,
                "discord.com" | "canary.discord.com" | "ptb.discord.com"
            )
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return None;
        }
        let path = url.path().strip_suffix('/').unwrap_or(url.path());
        let parts: Vec<_> = path.split('/').collect();
        match parts.as_slice() {
            ["", "channels", group, id] if *group == "@me" || digits(group) => (*id).to_owned(),
            ["", "channels", group, id, message]
                if (*group == "@me" || digits(group)) && digits(message) =>
            {
                (*id).to_owned()
            }
            _ => return None,
        }
    };
    if !digits(&id) || id.starts_with('0') || id.len() > 20 || id.parse::<u64>().is_err() {
        return None;
    }
    Some(id)
}

#[derive(Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub discord: u8,
    pub overlay: bool,
    pub running: bool,
    pub sound_only: bool,
    pub sound: String,
    pub volume: f64,
    pub theme: ThemePreference,
    pub receive: ReceiveMode,
    pub client_id: String,
    pub redirect_uri: String,
    pub secret_set: bool,
    pub allowlist: String,
    pub bot_set: bool,
    pub bot_guild: String,
    pub bot_user: String,
    pub bot_status: BotStatus,
    pub auto_exit: bool,
    pub timeout: f64,
    pub max_title: usize,
    pub max_content: usize,
    pub message: String,
    pub error: bool,
    pub preview: bool,
    pub received: u64,
    pub sent: u64,
    pub tests: u64,
    pub onboarding: bool,
}

// Secret fields are input only. Never derive Debug, Serialize or return Config.
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Patch {
    pub sound_only: Option<bool>,
    pub sound: Option<String>,
    pub volume: Option<f64>,
    pub theme: Option<ThemePreference>,
    pub receive: Option<ReceiveMode>,
    pub client_id: Option<String>,
    pub secret: Option<String>,
    pub redirect_uri: Option<String>,
    pub allowlist: Option<String>,
    pub bot_token: Option<String>,
    pub bot_guild: Option<String>,
    pub bot_user: Option<String>,
    pub auto_exit: Option<bool>,
    pub timeout: Option<f64>,
    pub max_title: Option<usize>,
    pub max_content: Option<usize>,
}

struct Services {
    sender: XsOverlaySender,
    overlay: Arc<AtomicBool>,
    tasks: Vec<tokio::task::JoinHandle<()>>,
    _instance: Instance,
}
impl Services {
    async fn stop(self) {
        for task in &self.tasks {
            task.abort();
        }
        for task in self.tasks {
            let _ = task.await;
        }
    }
}
struct Inner {
    cfg: Config,
    services: Option<Services>,
    message: String,
    error: bool,
    blocked: bool,
}
#[derive(Clone)]
pub struct Runtime {
    executor: Arc<tokio::runtime::Runtime>,
    inner: Arc<Mutex<Inner>>,
    path: Arc<PathBuf>,
    preview: bool,
    /// -debug: XSOverlayへ送らず、Discord受信の診断だけを行う。
    debug: bool,
    discord: Arc<AtomicU8>,
    sound_only: Arc<AtomicBool>,
    control: ReceiveControl,
    whitelist: Arc<AtomicU8>,
    diagnostics: Diagnostics,
    pub exit: Arc<AtomicBool>,
}
pub static OPTIONS: std::sync::OnceLock<Options> = std::sync::OnceLock::new();

/// 起動引数から決まる動作。プレビューは実設定の読み書きと外部接続をしない。
#[derive(Clone, Default)]
pub struct Options {
    pub preview: bool,
    pub setup: bool,
    pub launch_mode: Option<bool>,
    pub debug: bool,
}

impl Runtime {
    pub fn new(options: &Options) -> Result<Self, String> {
        let (path, problem) = if options.preview {
            (
                std::env::temp_dir().join("ditovr-preview-no-writes/config.json"),
                None,
            )
        } else {
            let appdata =
                PathBuf::from(std::env::var_os("APPDATA").ok_or("AppDataが見つかりません。")?);
            let directory = appdata.join(crate::paths::CONFIG_DIRECTORY);
            std::fs::create_dir_all(&directory).map_err(|_| "設定の保存先を作成できません。")?;
            let path = directory.join("config.json");
            let sources = crate::paths::config_sources(&appdata);
            let refs: Vec<&std::path::Path> = sources.iter().map(|p| p.as_path()).collect();
            let problem = crate::paths::migrate_config(&path, &refs).err().map(|_| {
                "旧設定を移行できませんでした。以前の保存先と新しい保存先を確認してください。"
                    .to_string()
            });
            (path, problem)
        };
        let mut runtime = Self::at(path, options.preview, problem)?;
        runtime.debug = options.debug;
        if let Some(enabled) = options.launch_mode {
            runtime.apply_launch_mode(enabled);
        }
        Ok(runtime)
    }
    /// --sound-only / --normal。保存に失敗したら現在の設定を変えずに理由を表示する。
    fn apply_launch_mode(&self, enabled: bool) {
        let result = self.executor.block_on(async {
            let mut inner = self.inner.lock().await;
            if inner.blocked {
                return Err("設定ファイルを確認してください。".to_string());
            }
            let mut next = inner.cfg.clone();
            next.notification_sound_only = enabled;
            next.validate().map_err(String::from)?;
            if !self.preview {
                config::save_sound_only_at(&self.path, enabled).map_err(|_| {
                    "通知モードを保存できませんでした。保存先を確認してください。".to_string()
                })?;
            }
            inner.cfg = next;
            self.sound_only.store(enabled, Ordering::SeqCst);
            Ok(())
        });
        if let Err(message) = result {
            self.executor.block_on(async {
                let mut inner = self.inner.lock().await;
                inner.message = message;
                inner.error = true;
            });
        }
    }
    /// 通常起動では画面表示と同時に受信を開始する(設定中の--setupでは停止のまま)。
    pub fn autostart(&self) {
        if self.preview {
            return;
        }
        let _ = self.execute("connect", Patch::default());
    }
    fn at(path: PathBuf, preview: bool, problem: Option<String>) -> Result<Self, String> {
        let loaded = if preview {
            Ok(Self::preview_config())
        } else {
            config::setup_defaults(&path)
        };
        let (cfg, blocked) = match loaded {
            Ok(c) => (c, false),
            Err(_) => (Config::default(), true),
        };
        if !preview {
            config::set_config_path(path.clone()).map_err(|_| "設定の保存先を設定できません。")?;
        }
        let executor = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(|_| "バックグラウンド処理を開始できません。")?;
        let message = if blocked {
            "設定を復号・読み込みできませんでした。元のファイルは上書きしません。".to_string()
        } else if let Some(problem) = problem {
            problem
        } else if preview {
            "プレビュー · 外部接続なし / 設定は保存しません".to_string()
        } else {
            "設定はこのPCに保存されます。".to_string()
        };
        let error = blocked;
        Ok(Self {
            executor: Arc::new(executor),
            path: Arc::new(path),
            preview,
            debug: false,
            discord: Arc::new(AtomicU8::new(if preview { 2 } else { 0 })),
            sound_only: Arc::new(AtomicBool::new(cfg.notification_sound_only)),
            control: ReceiveControl::new(&cfg),
            whitelist: Arc::new(AtomicU8::new(0)),
            diagnostics: Diagnostics::default(),
            exit: Arc::new(AtomicBool::new(false)),
            inner: Arc::new(Mutex::new(Inner {
                cfg,
                services: None,
                message,
                error,
                blocked,
            })),
        })
    }
    #[cfg(test)]
    pub fn preview_for_test() -> Self {
        Self::at(
            std::env::temp_dir().join("ditovr-repro/config.json"),
            true,
            None,
        )
        .unwrap()
    }
    fn preview_config() -> Config {
        Config {
            client_id: "123456789012345678".into(),
            client_secret: "preview-only".into(),
            theme: ThemePreference::Dark,
            notification_sound: "default".into(),
            notification_volume: 0.6,
            auto_exit_with_vr: false,
            ..Config::default()
        }
    }
    pub fn view(&self) -> View {
        self.executor.block_on(async {
            let inner = self.inner.lock().await;
            let c = &inner.cfg;
            let d = self.diagnostics.snapshot();
            View {
                discord: self.discord.load(Ordering::Relaxed),
                overlay: inner
                    .services
                    .as_ref()
                    .is_some_and(|s| s.overlay.load(Ordering::Relaxed)),
                running: inner.services.is_some(),
                sound_only: c.notification_sound_only,
                sound: c.notification_sound.clone(),
                volume: c.notification_volume * 100.,
                theme: c.theme,
                receive: c.receive_mode,
                client_id: if c.client_id == "YOUR_CLIENT_ID_HERE" {
                    String::new()
                } else {
                    c.client_id.clone()
                },
                secret_set: !c.client_secret.is_empty()
                    && c.client_secret != "YOUR_CLIENT_SECRET_HERE",
                redirect_uri: c.redirect_uri.clone(),
                allowlist: c.whitelist_channel_ids.join(", "),
                bot_set: !c.bot_token.is_empty(),
                bot_guild: c.bot_guild_id.clone(),
                bot_user: c.bot_user_id.clone(),
                bot_status: self.control.bot_status(),
                auto_exit: c.auto_exit_with_vr,
                timeout: c.notification_timeout,
                max_title: c.max_title_length,
                max_content: c.max_content_length,
                message: inner.message.clone(),
                error: inner.error,
                preview: self.preview,
                received: d.live.received,
                sent: d.live.sent,
                onboarding: c.needs_onboarding(),
                tests: d.tests.validated + d.tests.sent,
            }
        })
    }
    /// 受信サービスが動いているか。Call from a worker, never the GUI thread.
    pub fn services_running(&self) -> bool {
        self.executor
            .block_on(async { self.inner.lock().await.services.is_some() })
    }
    /// Call from a worker, never the GUI thread. Commands serialize against OAuth/settings changes.
    pub fn execute(&self, action: &str, patch: Patch) -> Result<View, String> {
        let result = self.executor.block_on(self.dispatch(action, patch));
        self.executor.block_on(async {
            let mut inner = self.inner.lock().await;
            match &result {
                Ok(msg) => {
                    inner.message = msg.clone();
                    inner.error = false;
                }
                Err(msg) => {
                    inner.message = msg.clone();
                    inner.error = true;
                }
            }
        });
        result.map(|_| self.view())
    }
    async fn dispatch(&self, action: &str, patch: Patch) -> Result<String, String> {
        let mut inner = self.inner.lock().await;
        if inner.blocked {
            return Err("設定ファイルを確認してください。復号失敗時の上書きは行いません。".into());
        }
        match action {
            "save" => {
                let mut c = if !self.preview && self.path.exists() {
                    config::setup_defaults(&self.path)
                        .map_err(|_| "設定を読み込めません。保存は中止しました。")?
                } else {
                    inner.cfg.clone()
                };
                apply_patch(&mut c, patch);
                if self.preview {
                    let mut validation = c.clone();
                    validation.client_id = "123".into();
                    validation.client_secret = "preview-only".into();
                    validation.validate().map_err(String::from)?;
                } else {
                    c.validate().map_err(String::from)?;
                }
                let sound = c.notification_sound.clone();
                let directory = self.path.parent().unwrap().join("sound-cache");
                tokio::task::spawn_blocking(move || audio::prepare_sound(&sound, &directory))
                    .await
                    .map_err(|_| "音声の検証処理に失敗しました。")?
                    .map_err(String::from)?;
                let running = inner.services.is_some();
                // Await the old OAuth task before replacing credentials. Otherwise
                // an in-flight refresh could write old tokens into the new profile.
                if let Some(s) = inner.services.take() {
                    s.stop().await;
                }
                self.discord.store(0, Ordering::SeqCst);
                if !self.preview {
                    std::fs::create_dir_all(self.path.parent().unwrap())
                        .map_err(|_| "設定フォルダーを作成できません。")?;
                    config::save_setup_at(&self.path, &c).map_err(
                        |_| "設定を保存できません。既存設定は維持し、受信を停止しました。",
                    )?;
                    c = config::setup_defaults(&self.path)
                        .map_err(|_| "保存した設定を読み込めません。")?;
                }
                inner.cfg = c;
                self.control.configure(&inner.cfg);
                self.sound_only
                    .store(inner.cfg.notification_sound_only, Ordering::SeqCst);
                if running {
                    self.connect(&mut inner).await?;
                }
                Ok(if self.preview {
                    "測定用の変更を反映しました。保存・外部送信なし。"
                } else {
                    "比較用設定を保存しました。"
                }
                .into())
            }
            "onboarding" => {
                if !self.preview {
                    config::complete_onboarding_at(&self.path).map_err(
                        |_| "初回案内の完了を保存できませんでした。保存先を確認してください。",
                    )?;
                }
                inner.cfg.onboarding_completed = Some(true);
                Ok("初回案内を完了しました。".into())
            }
            "connect" => {
                self.connect(&mut inner).await?;
                Ok(
                    "接続を開始しました。Discord側の認証画面が開いた場合は確認してください。"
                        .into(),
                )
            }
            "pause" => {
                if let Some(s) = inner.services.take() {
                    s.stop().await;
                }
                self.discord.store(0, Ordering::SeqCst);
                Ok("通知受信を停止しました。".into())
            }
            "test" => {
                let s = inner
                    .services
                    .as_ref()
                    .filter(|s| s.overlay.load(Ordering::SeqCst))
                    .ok_or("XSOverlayに接続してから試してください。")?;
                let sound = inner.cfg.notification_sound.clone();
                let directory = self.path.parent().unwrap().join("sound-cache");
                let sound =
                    tokio::task::spawn_blocking(move || audio::prepare_sound(&sound, &directory))
                        .await
                        .map_err(|_| "音声変換に失敗しました。")?
                        .map_err(String::from)?;
                if s.sender
                    .with_trace(self.diagnostics.trace(Source::OverlayTest))
                    .send_notification(
                        "DiToVR",
                        "これはテスト通知です",
                        None,
                        &sound,
                        inner.cfg.notification_volume,
                        inner.cfg.notification_timeout,
                    )
                    .await
                {
                    Ok("テスト通知を送信キューに追加しました。VRで確認してください。".into())
                } else {
                    Err("送信キューへ追加できませんでした。接続を確認してください。".into())
                }
            }
            "audition" => {
                let mut c = inner.cfg.clone();
                apply_patch(&mut c, patch);
                drop(inner);
                tokio::task::spawn_blocking(move || audition(&c))
                    .await
                    .map_err(|_| "音の再生処理に失敗しました。")??;
                Ok("このPCでの再生処理が完了しました。VRへの送信はありません。".into())
            }
            "diagnose" => {
                let mut c = inner.cfg.clone();
                c.log_message_content = false;
                drop(inner);
                let http = http_client()?;
                let (sender, rx) =
                    vr::create_diagnostic_channel(http.clone(), self.sound_only.clone());
                let task = tokio::spawn(vr::receive_diagnostic_loop(rx));
                let mut notifier = DiscordRPCNotifier::new(c, sender, http)
                    .with_trace(self.diagnostics.trace(Source::Simulation));
                notifier.inject_test_notification(false, "1").await;
                // 送信側（notifier 内）を閉じると、受信ループはキューを処理し終えて自然に終了する。
                drop(notifier);
                task.await
                    .map_err(|_| "模擬受信の検証処理が中断されました。")?;
                Ok("模擬受信を共通Coreで検証しました。実受信・VR送信とは別に集計します。".into())
            }
            _ => Err("未対応の操作です。".into()),
        }
    }
    async fn connect(&self, inner: &mut Inner) -> Result<(), String> {
        if self.preview {
            return Err("測定モードでは外部サービスに接続しません。".into());
        }
        if inner.services.is_some() {
            return Ok(());
        }
        inner.cfg.validate().map_err(String::from)?;
        let instance = Instance::acquire()?;
        let mut c = config::load_from_path(&self.path, true)
            .map_err(|_| "比較用設定を読み込めません。先に保存してください。")?;
        c.log_message_content = false;
        let sound = c.notification_sound.clone();
        let directory = self.path.parent().unwrap().join("sound-cache");
        c.notification_sound =
            tokio::task::spawn_blocking(move || audio::prepare_sound(&sound, &directory))
                .await
                .map_err(|_| "音声変換に失敗しました。")?
                .map_err(String::from)?;
        self.control.configure(&c);
        let http = http_client()?;
        let (sender, rx, overlay) = if self.debug {
            let (sender, rx) = vr::create_diagnostic_channel(http.clone(), self.sound_only.clone());
            (sender, rx, Arc::new(AtomicBool::new(false)))
        } else {
            vr::create_xsoverlay_channel(http.clone(), self.sound_only.clone())
        };
        let overlay_copy = overlay.clone();
        let sender_copy = sender.clone();
        let debug = self.debug;
        let mut tasks = vec![tokio::spawn(async move {
            if debug {
                vr::receive_diagnostic_loop(rx).await;
            } else {
                vr::xsoverlay_loop(rx, overlay_copy, sender_copy).await;
            }
        })];
        let mut notifier = DiscordRPCNotifier::new(c.clone(), sender.clone(), http.clone())
            .with_status(self.discord.clone())
            .with_receive_control(self.control.clone(), self.whitelist.clone())
            .with_trace(self.diagnostics.trace(Source::Live));
        tasks.push(tokio::spawn(async move {
            notifier.start().await;
        }));
        if c.receive_mode == ReceiveMode::Auto {
            tasks.push(tokio::spawn(presence::run(
                c.clone(),
                http,
                self.control.clone(),
            )));
        }
        // --debug は XSOverlay・SteamVR に連動しない受信テスト専用。VR 終了による自動終了は行わない。
        if c.auto_exit_with_vr && !self.debug {
            let (tx, mut rx) = tokio::sync::mpsc::channel(1);
            let exit = self.exit.clone();
            tasks.push(tokio::spawn(system::steamvr_monitor_loop(tx)));
            tasks.push(tokio::spawn(async move {
                if rx.recv().await.is_some() {
                    exit.store(true, Ordering::SeqCst);
                }
            }));
        }
        inner.services = Some(Services {
            sender,
            overlay,
            tasks,
            _instance: instance,
        });
        Ok(())
    }
}

fn apply_patch(c: &mut Config, p: Patch) {
    if let Some(v) = p.sound_only {
        c.notification_sound_only = v;
    }
    if let Some(v) = p.sound {
        c.notification_sound = v;
    }
    if let Some(v) = p.volume {
        c.notification_volume = v / 100.;
    }
    if let Some(v) = p.theme {
        c.theme = v;
    }
    if let Some(v) = p.receive {
        c.receive_mode = v;
    }
    if let Some(v) = p.client_id {
        c.client_id = v.trim().into();
    }
    if let Some(v) = p.secret.filter(|v| !v.trim().is_empty()) {
        c.client_secret = v.trim().into();
    }
    if let Some(v) = p.redirect_uri {
        c.redirect_uri = v.trim().into();
    }
    if let Some(v) = p.allowlist {
        c.whitelist_channel_ids = v
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|v| !v.is_empty())
            .map(str::to_owned)
            .collect();
    }
    if let Some(v) = p.bot_token.filter(|v| !v.trim().is_empty()) {
        c.bot_token = v.trim().strip_prefix("Bot ").unwrap_or(v.trim()).into();
    }
    if let Some(v) = p.bot_guild {
        c.bot_guild_id = v.trim().into();
    }
    if let Some(v) = p.bot_user {
        c.bot_user_id = v.trim().into();
    }
    if let Some(v) = p.auto_exit {
        c.auto_exit_with_vr = v;
    }
    if let Some(v) = p.timeout {
        c.notification_timeout = v;
    }
    if let Some(v) = p.max_title {
        c.max_title_length = v;
    }
    if let Some(v) = p.max_content {
        c.max_content_length = v;
    }
}
fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .https_only(true)
        .build()
        .map_err(|_| "HTTP接続を準備できません。".into())
}
fn audition(c: &Config) -> Result<(), String> {
    use rodio::{Decoder, OutputStreamBuilder, Sink, Source, source::SineWave};
    if c.notification_sound.is_empty()
        || c.notification_volume <= 0.
        || !c.notification_volume.is_finite()
    {
        return Err("通知音を選び、音量を1%以上にしてください。".into());
    }
    audio::validate_sound_path(&c.notification_sound).map_err(String::from)?;
    let mut stream = OutputStreamBuilder::open_default_stream()
        .map_err(|_| "Windowsの再生デバイスを開けません。")?;
    stream.log_on_drop(false);
    let sink = Sink::connect_new(stream.mixer());
    sink.set_volume(c.notification_volume.clamp(0., 1.) as f32);
    if c.notification_sound == "default" {
        sink.append(
            SineWave::new(880.)
                .take_duration(Duration::from_millis(180))
                .amplify(0.12),
        );
        sink.append(
            SineWave::new(1174.66)
                .take_duration(Duration::from_millis(240))
                .amplify(0.10),
        );
    } else {
        let input =
            std::fs::File::open(&c.notification_sound).map_err(|_| "音声ファイルを開けません。")?;
        sink.append(
            Decoder::try_from(input)
                .map_err(|_| "音声をデコードできません。")?
                .take_duration(Duration::from_secs(4)),
        );
    }
    sink.sleep_until_end();
    Ok(())
}

// 実接続は1プロセスだけ。
struct Instance(usize);
impl Instance {
    fn acquire() -> Result<Self, String> {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError},
            System::Threading::CreateMutexW,
        };
        let name: Vec<u16> = "Local\\DiToVR-Live\0".encode_utf16().collect();
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return Err("多重接続防止を準備できません。".into());
        }
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            unsafe { CloseHandle(handle) };
            return Err("別のDiToVRがすでに接続中です。".into());
        }
        Ok(Self(handle as usize))
    }
}
impl Drop for Instance {
    fn drop(&mut self) {
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0 as _) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stopped_services_report_not_running() {
        // 更新が受信を止める前の状態を記録するために使う。プレビューは接続しない。
        assert!(!Runtime::preview_for_test().services_running());
    }
    #[test]
    fn public_view_and_simulation_do_not_expose_credentials_or_count_as_live() {
        let r = Runtime::at(
            std::env::temp_dir().join("ditovr-preview-no-writes/config.json"),
            true,
            None,
        )
        .unwrap();
        r.executor.block_on(async {
            let mut i = r.inner.lock().await;
            i.cfg.client_secret = "secret-marker".into();
            i.cfg.access_token = "access-marker".into();
            i.cfg.refresh_token = "refresh-marker".into();
            i.cfg.bot_token = "bot-marker".into();
        });
        let serialized = serde_json::to_string(&r.view()).unwrap();
        for marker in [
            "secret-marker",
            "access-marker",
            "refresh-marker",
            "bot-marker",
        ] {
            assert!(!serialized.contains(marker));
        }
        assert!(r.execute("connect", Patch::default()).is_err());
        let v = r.execute("diagnose", Patch::default()).unwrap();
        assert_eq!(v.received, 0);
        assert_eq!(v.sent, 0);
        assert!(v.tests > 0);
    }
    #[test]
    fn saving_preserves_tokens_encrypts_secrets_and_rejects_invalid_changes() {
        let dir = std::env::temp_dir().join(format!("ditovr-runtime-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let c = Config {
            client_id: "123".into(),
            client_secret: "fixture-secret".into(),
            notification_volume: 0.6,
            notification_sound: "default".into(),
            ..Config::default()
        };
        config::save_setup_at(&path, &c).unwrap();
        let r = Runtime::at(path.clone(), false, None).unwrap();
        config::save_tokens("fixture-access", "fixture-refresh").unwrap();
        r.execute(
            "save",
            Patch {
                volume: Some(42.),
                secret: Some(String::new()),
                ..Patch::default()
            },
        )
        .unwrap();
        let saved = config::setup_defaults(&path).unwrap();
        assert_eq!(saved.notification_volume, 0.42);
        assert_eq!(saved.access_token, "fixture-access");
        assert_eq!(saved.refresh_token, "fixture-refresh");
        let bytes = std::fs::read(&path).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(!text.contains("fixture-secret"));
        assert!(!text.contains("fixture-access"));
        assert!(
            r.execute(
                "save",
                Patch {
                    volume: Some(-1.),
                    ..Patch::default()
                }
            )
            .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        drop(r);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn invalid_modes_and_duplicate_allowlist_use_core_validation() {
        let mut c = Config {
            client_id: "123".into(),
            client_secret: "test-only".into(),
            ..Config::default()
        };
        apply_patch(
            &mut c,
            Patch {
                sound_only: Some(true),
                ..Patch::default()
            },
        );
        assert!(c.validate().is_err());
        c.notification_sound_only = false;
        apply_patch(
            &mut c,
            Patch {
                allowlist: Some("123, 123".into()),
                ..Patch::default()
            },
        );
        assert!(c.validate().is_err());
    }
}
