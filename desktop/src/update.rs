//! 通常版の署名付き自動更新。Store版では通信・保存・インストーラー起動を行わない。
//! 配布済みTauri版と同じフィード・署名(minisign)・バージョン結合を使い、互換性を保つ。
use base64::Engine;
use minisign_verify::{PublicKey, Signature};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const CHECK_INTERVAL: Duration = Duration::from_secs(12 * 60 * 60);
const REPOSITORY_PATH: &str = "/Yumiiiiiiiiii/DiToVR/releases/download/";
const LEGACY_REPOSITORY_PATH: &str = "/Yumiiiiiiiiii/discord_To_VR/releases/download/";
/// 既存の更新署名鍵(公開鍵のみ)。鍵や形式を変えると配布済みの版が更新できなくなる。
pub const PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDg5OERDNjFDNTZGOUFBRkQKUldUOXF2bFdITWFOaVVjWHBLdC9xNUFCclNoOEl4QVJaZkQxT3M1Y1k3b3Q3aEtqcVkvUXBscEkK";
pub const ENDPOINTS: [&str; 2] = [
    "https://raw.githubusercontent.com/Yumiiiiiiiiii/DiToVR/updates/latest.json",
    "https://github.com/Yumiiiiiiiiii/DiToVR/releases/latest/download/latest.json",
];
const PLATFORM: &str = "windows-x86_64";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Idle,
    Checking,
    Current,
    Available,
    Downloading,
    Waiting,
    Installing,
    Error,
}

#[derive(Clone)]
pub struct UpdateStatus {
    pub current_version: String,
    pub store_managed: bool,
    pub auto_update: bool,
    pub supported: bool,
    pub can_check: bool,
    pub disabled_reason: Option<String>,
    pub phase: Phase,
    pub version: Option<String>,
    pub notes: Option<String>,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub checked_at_ms: Option<u64>,
    pub message: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Preferences {
    auto_update: bool,
}

#[derive(Deserialize)]
struct Feed {
    version: String,
    notes: Option<String>,
    platforms: std::collections::HashMap<String, Platform>,
}
#[derive(Deserialize)]
struct Platform {
    url: String,
    signature: String,
}

pub struct Release {
    pub version: String,
    pub notes: Option<String>,
    url: reqwest::Url,
    signature: String,
}

/// 更新の適用前に呼ぶ確認。未保存の編集中などは false を返し、適用を待つ。
pub trait Host: Send + Sync + 'static {
    fn prepare(&self) -> bool;
    fn cancel(&self);
}

/// Host のコールバックは同期で Runtime::execute（block_on）に届くため、非同期タスク上で直接呼ぶと
/// 「ランタイム内でのランタイム起動」で panic する。専用のブロッキングスレッドで実行する。
async fn call_host<T: Send + 'static>(
    host: &Arc<dyn Host>,
    f: impl FnOnce(&dyn Host) -> T + Send + 'static,
) -> Option<T> {
    let host = host.clone();
    tokio::task::spawn_blocking(move || f(host.as_ref()))
        .await
        .ok()
}

/// 失敗・panic・キャンセルのどの経路でも busy を解除する。
struct BusyGuard(Arc<AtomicBool>);
impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

#[derive(Clone)]
pub struct Updater {
    status: Arc<Mutex<UpdateStatus>>,
    busy: Arc<AtomicBool>,
    write: Arc<Mutex<()>>,
    path: Arc<PathBuf>,
    network_enabled: bool,
    executor: Arc<tokio::runtime::Runtime>,
}

/// パッケージidentityがあるMSIX実行も、feature無しのビルドでStore管理として扱う。
/// 想定外のWindows APIエラーでは更新を有効にしない。
pub fn store_managed() -> bool {
    use windows_sys::Win32::{
        Foundation::APPMODEL_ERROR_NO_PACKAGE, Storage::Packaging::Appx::GetCurrentPackageFullName,
    };
    let mut length = 0;
    cfg!(feature = "microsoft-store")
        || unsafe { GetCurrentPackageFullName(&mut length, std::ptr::null_mut()) }
            != APPMODEL_ERROR_NO_PACKAGE
}

impl Updater {
    pub fn new(path: PathBuf, preview: bool, debug: bool) -> Self {
        Self::with(path, preview, debug, store_managed())
    }
    fn with(path: PathBuf, preview: bool, debug: bool, store_managed: bool) -> Self {
        let network_enabled = !store_managed && !preview && !debug && !cfg!(debug_assertions);
        let (auto_update, problem) = if store_managed {
            (false, None)
        } else if preview {
            (true, None)
        } else {
            match std::fs::read(&path) {
                Ok(bytes) => match serde_json::from_slice::<Preferences>(&bytes) {
                    Ok(prefs) => (prefs.auto_update, None),
                    Err(_) => (false, Some("更新設定を読めませんでした。自動更新を切り替えて設定を保存し直してください。".into())),
                },
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (true, None),
                Err(_) => (false, Some("更新設定を読めませんでした。保存先を確認してください。".into())),
            }
        };
        let supported = network_enabled && installed_executable();
        let disabled_reason = if store_managed {
            Some("このアプリの更新は Microsoft Store が管理します。Microsoft Store の「ライブラリ」で更新を確認できます。".into())
        } else if preview {
            Some("画面プレビューでは更新の通信・適用を行いません。".into())
        } else if debug {
            Some("-debug では Discord の受信テストだけを行います。更新は通常起動で確認してください。".into())
        } else if cfg!(debug_assertions) {
            Some("開発ビルドでは更新を適用しません。配布版を使用してください。".into())
        } else if !supported {
            Some(
                "自動適用にはインストーラー版が必要です。ZIP 版では更新の確認だけを行います。"
                    .into(),
            )
        } else {
            None
        };
        let executor = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("更新処理を開始できません。");
        Self {
            status: Arc::new(Mutex::new(UpdateStatus {
                current_version: env!("CARGO_PKG_VERSION").into(),
                store_managed,
                auto_update,
                supported,
                can_check: network_enabled,
                disabled_reason,
                phase: Phase::Idle,
                version: None,
                notes: None,
                downloaded: 0,
                total: None,
                checked_at_ms: None,
                message: problem,
            })),
            busy: Arc::new(AtomicBool::new(false)),
            write: Arc::new(Mutex::new(())),
            path: Arc::new(path),
            network_enabled,
            executor: Arc::new(executor),
        }
    }
    fn mutate(&self, action: impl FnOnce(&mut UpdateStatus)) {
        action(&mut self.status.lock().unwrap_or_else(|e| e.into_inner()));
    }
    pub fn snapshot(&self) -> UpdateStatus {
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
    pub fn report_migration_failure(&self) {
        if self.snapshot().store_managed {
            return;
        }
        self.mutate(|s| {
            s.auto_update = false;
            s.phase = Phase::Error;
            s.message = Some("以前の更新設定を移行できなかったため、自動更新を停止しました。保存先を確認し、自動アップデートを切り替えて保存し直してください。".into());
        });
    }
    fn fail(&self, message: &str) {
        self.mutate(|s| {
            s.phase = Phase::Error;
            s.message = Some(message.into());
        });
    }
    pub fn set_auto_update(&self, enabled: bool) -> Result<(), String> {
        if self.snapshot().store_managed {
            return Err("更新の設定は Microsoft Store で変更してください。".into());
        }
        let _write = self.write.lock().unwrap_or_else(|e| e.into_inner());
        if self.snapshot().phase == Phase::Installing {
            return Err("更新の適用中は切り替えできません。".into());
        }
        if self.network_enabled {
            let bytes = serde_json::to_vec_pretty(&Preferences {
                auto_update: enabled,
            })
            .unwrap();
            let temporary = self.path.with_extension("updates.tmp");
            // 秘密情報を含まない1項目の設定。config.jsonには触れない。
            std::fs::write(&temporary, bytes)
                .and_then(|_| replace_file(&temporary, &self.path))
                .map_err(|_| {
                    "更新設定を保存できませんでした。保存先を確認してください。".to_string()
                })?;
        }
        self.mutate(|s| s.auto_update = enabled);
        Ok(())
    }
    /// 手動確認。別の更新処理が進行中なら何もしない。
    pub fn check_now(&self, host: Arc<dyn Host>) {
        let this = self.clone();
        self.executor.spawn(async move {
            let _ = this.run_check(host).await;
        });
    }
    /// 起動15秒後から12時間おきに確認する。
    pub fn start(&self, host: Arc<dyn Host>) {
        if !self.network_enabled {
            return;
        }
        let this = self.clone();
        self.executor.spawn(async move {
            tokio::time::sleep(Duration::from_secs(15)).await;
            let mut next = tokio::time::Instant::now();
            let mut before = false;
            loop {
                let enabled = this.snapshot().auto_update;
                if enabled && (!before || tokio::time::Instant::now() >= next) {
                    let _ = this.run_check(host.clone()).await;
                    next = tokio::time::Instant::now() + CHECK_INTERVAL;
                }
                before = enabled;
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        });
    }
    async fn run_check(&self, host: Arc<dyn Host>) -> Result<(), String> {
        if !self.network_enabled {
            return Err(self.snapshot().disabled_reason.unwrap_or_default());
        }
        if self.busy.swap(true, Ordering::SeqCst) {
            return Err("更新処理が進行中です。".into());
        }
        let _busy = BusyGuard(self.busy.clone());
        let result = self.update_flow(host).await;
        if let Err(message) = &result {
            self.fail(message);
        }
        result
    }
    async fn update_flow(&self, host: Arc<dyn Host>) -> Result<(), String> {
        self.mutate(|s| {
            s.phase = Phase::Checking;
            s.message = None;
            s.version = None;
            s.notes = None;
            s.downloaded = 0;
            s.total = None;
        });
        let client = reqwest::Client::builder()
            .https_only(true)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|_| "更新の準備ができませんでした。")?;
        let result = fetch_release(&client, &ENDPOINTS, env!("CARGO_PKG_VERSION")).await;
        self.mutate(|s| {
            s.checked_at_ms = Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            )
        });
        let release = match result? {
            Some(release) => release,
            None => {
                self.mutate(|s| s.phase = Phase::Current);
                return Ok(());
            }
        };
        if !allowed_download(&release.url) {
            return Err("更新ファイルの配布先が正しくないため適用しませんでした。".into());
        }
        self.mutate(|s| {
            s.phase = Phase::Available;
            s.version = Some(release.version.clone());
            s.notes = release
                .notes
                .as_ref()
                .map(|n| n.chars().take(4000).collect());
        });
        let status = self.snapshot();
        if !status.auto_update || !status.supported {
            return Ok(());
        }
        self.mutate(|s| s.phase = Phase::Downloading);
        let bytes = download(&client, &release, |chunk, total| {
            self.mutate(|s| {
                s.downloaded = s.downloaded.saturating_add(chunk);
                s.total = total;
            })
        })
        .await
        .map_err(|_| "更新ファイルのダウンロードまたは署名・バージョン検証に失敗しました。現在のアプリは変更していません。".to_string())?;
        self.mutate(|s| s.phase = Phase::Waiting);
        loop {
            if !self.snapshot().auto_update {
                self.mutate(|s| s.phase = Phase::Available);
                return Ok(());
            }
            if call_host(&host, |h| h.prepare()).await.unwrap_or(false) {
                break;
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        let apply = {
            let _write = self.write.lock().unwrap_or_else(|e| e.into_inner());
            let enabled = self.snapshot().auto_update;
            self.mutate(|s| {
                s.phase = if enabled {
                    Phase::Installing
                } else {
                    Phase::Available
                }
            });
            enabled
        };
        if !apply {
            call_host(&host, |h| h.cancel()).await;
            return Ok(());
        }
        let installed = tokio::task::spawn_blocking(move || launch_installer(&bytes)).await;
        if !matches!(installed, Ok(Ok(()))) {
            call_host(&host, |h| h.cancel()).await;
            return Err(
                "更新インストーラーを起動できませんでした。通知の受信を再開しました。".into(),
            );
        }
        Ok(())
    }
}

/// 公開済みの2つのフィードを順に読み、現在より新しい版だけ返す。
pub async fn fetch_release(
    client: &reqwest::Client,
    endpoints: &[&str],
    current: &str,
) -> Result<Option<Release>, String> {
    let mut missing = true;
    for endpoint in endpoints {
        let response = match client.get(*endpoint).send().await {
            Ok(response) => response,
            Err(_) => {
                missing = false;
                continue;
            }
        };
        if !response.status().is_success() {
            continue;
        }
        let body = read_limited(response, MAX_FEED_BYTES, |_, _| {})
            .await
            .map_err(|_| "更新情報を読めませんでした。".to_string())?;
        let feed: Feed = serde_json::from_slice(&body)
            .map_err(|_| "更新情報を読めませんでした。".to_string())?;
        if !is_newer(&feed.version, current)? {
            return Ok(None);
        }
        let platform = feed
            .platforms
            .get(PLATFORM)
            .ok_or("この環境向けの更新がありません。")?;
        let url =
            reqwest::Url::parse(&platform.url).map_err(|_| "更新ファイルの場所が不正です。")?;
        return Ok(Some(Release {
            version: feed.version,
            notes: feed.notes,
            url,
            signature: platform.signature.clone(),
        }));
    }
    Err(if missing {
        "更新情報がまだ公開されていないか、配布先を利用できません。現在のアプリはそのまま使えます。"
            .into()
    } else {
        "更新を確認できませんでした。通信環境を確認して、あとで再試行してください。".into()
    })
}

fn parse_version(value: &str) -> Result<[u64; 3], String> {
    let parts: Vec<u64> = value
        .trim_start_matches('v')
        .split('.')
        .map(|part| part.parse::<u64>())
        .collect::<Result<_, _>>()
        .map_err(|_| "更新のバージョンが不正です。".to_string())?;
    <[u64; 3]>::try_from(parts).map_err(|_| "更新のバージョンが不正です。".to_string())
}
fn is_newer(candidate: &str, current: &str) -> Result<bool, String> {
    Ok(parse_version(candidate)? > parse_version(current)?)
}

const MAX_FEED_BYTES: u64 = 1024 * 1024;
const MAX_INSTALLER_BYTES: u64 = 200 * 1024 * 1024;

/// 申告された長さと累積サイズの両方を上限で止め、署名検証前のメモリ増大を防ぐ。
async fn read_limited(
    mut response: reqwest::Response,
    limit: u64,
    mut progress: impl FnMut(u64, Option<u64>),
) -> Result<Vec<u8>, String> {
    let total = response.content_length();
    if total.is_some_and(|length| length > limit) {
        return Err("download".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| "download")? {
        if bytes.len() as u64 + chunk.len() as u64 > limit {
            return Err("download".into());
        }
        bytes.extend_from_slice(&chunk);
        progress(chunk.len() as u64, total);
    }
    Ok(bytes)
}

pub async fn download(
    client: &reqwest::Client,
    release: &Release,
    progress: impl FnMut(u64, Option<u64>),
) -> Result<Vec<u8>, String> {
    let response = client
        .get(release.url.clone())
        .timeout(Duration::from_secs(300))
        .send()
        .await
        .map_err(|_| "download")?;
    if !response.status().is_success() {
        return Err("download".into());
    }
    let bytes = read_limited(response, MAX_INSTALLER_BYTES, progress).await?;
    verify(&bytes, &release.signature, PUBLIC_KEY, &release.version)?;
    Ok(bytes)
}

/// minisign署名と、署名済みtrusted commentのバージョンが通知された版と一致することを確認する。
/// 一致を必須にして、署名のない古い版や別の版への差し替えを拒否する。
pub fn verify(
    data: &[u8],
    signature: &str,
    public_key: &str,
    announced: &str,
) -> Result<(), String> {
    let decode = |value: &str| -> Result<String, String> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(value)
            .map_err(|_| "signature".to_string())?;
        String::from_utf8(bytes).map_err(|_| "signature".to_string())
    };
    let key = PublicKey::decode(&decode(public_key)?).map_err(|_| "signature".to_string())?;
    let signature = Signature::decode(&decode(signature)?).map_err(|_| "signature".to_string())?;
    key.verify(data, &signature, true)
        .map_err(|_| "signature".to_string())?;
    // 検証後でないとtrusted commentは信用できない。
    let signed = signature
        .trusted_comment()
        .split('\t')
        .find_map(|field| field.strip_prefix("version:"))
        .ok_or("signed version missing")?;
    if parse_version(signed)? == parse_version(announced)? {
        Ok(())
    } else {
        Err("signed version mismatch".into())
    }
}

/// 許可する配布先: github.com のこのリポジトリ(または旧リポジトリ)のReleaseにある -setup.exe のみ。
pub fn allowed_download(url: &reqwest::Url) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some("github.com")
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && (url.path().starts_with(REPOSITORY_PATH)
            || url.path().starts_with(LEGACY_REPOSITORY_PATH))
        && url.path().ends_with("-setup.exe")
}

/// 検証済みのNSISをパッシブで起動し、このプロセスを終了する(インストーラーが再起動する)。
fn launch_installer(bytes: &[u8]) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOW};
    let directory = std::env::temp_dir().join(format!("DiToVR-update-{}", std::process::id()));
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let file = directory.join("DiToVR-setup.exe");
    std::fs::write(&file, bytes).map_err(|e| e.to_string())?;
    let wide = |s: &std::ffi::OsStr| -> Vec<u16> { s.encode_wide().chain(Some(0)).collect() };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut parameters = String::from("/P /UPDATE /R");
    if !args.is_empty() {
        parameters.push_str(" /ARGS");
        for arg in &args {
            parameters.push(' ');
            parameters.push_str(&arg.replace('"', "\"\""));
        }
    }
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            wide("open".as_ref()).as_ptr(),
            wide(file.as_os_str()).as_ptr(),
            wide(parameters.as_ref()).as_ptr(),
            std::ptr::null(),
            SW_SHOW,
        )
    };
    if result as isize <= 32 {
        return Err("launch".into());
    }
    std::process::exit(0);
}

pub fn migrate_preferences_from(sources: &[&Path], destination: &Path) -> std::io::Result<bool> {
    if destination.try_exists()? {
        return Ok(false);
    }
    for source in sources {
        if source.try_exists()? {
            return migrate_preferences(source, destination);
        }
    }
    Ok(false)
}

pub fn migrate_preferences(previous: &Path, destination: &Path) -> std::io::Result<bool> {
    if destination.try_exists()? {
        return Ok(false);
    }
    let bytes = match std::fs::read(previous) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e),
    };
    serde_json::from_slice::<Preferences>(&bytes)
        .map_err(|_| std::io::Error::other("更新設定の形式が不正です"))?;
    let temporary = destination.with_extension("updates.tmp");
    std::fs::write(&temporary, &bytes)?;
    replace_file(&temporary, destination)?;
    Ok(true)
}

fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };
    let from: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
    if unsafe {
        MoveFileExW(
            from.as_ptr(),
            to.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    } == 0
    {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// NSISが記録した実際のインストール先と一致するときだけ自動適用する。コピーしたZIP版では更新しない。
fn installed_executable() -> bool {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_SZ, RegGetValueW};
    let key: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\DiToVR\0"
        .encode_utf16()
        .collect();
    let value: Vec<u16> = "InstallLocation\0".encode_utf16().collect();
    let mut buffer = [0u16; 32768];
    let mut length = std::mem::size_of_val(&buffer) as u32;
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &mut length,
        )
    };
    if result != 0 {
        return false;
    }
    let end = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
    let installed = PathBuf::from(String::from_utf16_lossy(&buffer[..end]).trim_matches('"'))
        .join("DiToVR.exe");
    match (
        installed.canonicalize(),
        std::env::current_exe().and_then(|p| p.canonicalize()),
    ) {
        (Ok(installed), Ok(current)) => installed == current,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    /// 指定した生のHTTP応答を1回だけ返すローカルサーバーを起動し、その応答を取得する。
    async fn serve_once(raw: Vec<u8>) -> reqwest::Response {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0u8; 1024];
            let _ = socket.read(&mut request).await;
            let _ = socket.write_all(&raw).await;
            let _ = socket.shutdown().await;
        });
        reqwest::get(format!("http://{address}/")).await.unwrap()
    }

    #[tokio::test]
    async fn bodies_are_bounded_by_declared_length_and_by_streamed_size() {
        let exact = serve_once(
            b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\nabcd".to_vec(),
        )
        .await;
        assert_eq!(read_limited(exact, 4, |_, _| {}).await.unwrap(), b"abcd");
        let declared = serve_once(
            b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nabcde".to_vec(),
        )
        .await;
        assert!(read_limited(declared, 4, |_, _| {}).await.is_err());
        // Content-Length を持たない応答（接続終了まで続く）でも累積サイズで止まる。
        let streamed =
            serve_once(b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\nabcdefgh".to_vec()).await;
        assert!(read_limited(streamed, 4, |_, _| {}).await.is_err());
    }

    struct RuntimeHost(crate::runtime::Runtime);
    impl Host for RuntimeHost {
        fn prepare(&self) -> bool {
            self.0.execute("pause", Default::default()).is_ok()
        }
        fn cancel(&self) {
            let _ = self.0.execute("connect", Default::default());
        }
    }

    /// 回帰: Host は Runtime::execute（block_on）に届く。Updater のタスク上で直接呼ぶと panic した。
    #[test]
    fn host_callbacks_run_from_an_updater_task_without_nesting_runtimes() {
        let host: Arc<dyn Host> =
            Arc::new(RuntimeHost(crate::runtime::Runtime::preview_for_test()));
        let updater_rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let keep_alive = host.clone(); // 実アプリでも Runtime は更新タスクより長く生きる
        let (prepared, cancelled) = updater_rt.block_on(async move {
            tokio::spawn(async move {
                let prepared = call_host(&host, |h| h.prepare()).await;
                let cancelled = call_host(&host, |h| h.cancel()).await;
                (prepared, cancelled)
            })
            .await
            .expect("host callbacks must not panic the updater task")
        });
        assert!(prepared.is_some() && cancelled.is_some());
        drop(keep_alive);
    }

    #[test]
    fn busy_flag_is_released_even_when_the_flow_unwinds() {
        let busy = Arc::new(AtomicBool::new(true));
        let guard = BusyGuard(busy.clone());
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _guard = guard;
            panic!("simulated failure");
        }));
        assert!(!busy.load(Ordering::SeqCst));
    }
    use super::*;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Fixture {
        public_key: String,
        signature: String,
        payload: String,
    }
    fn fixture() -> Fixture {
        serde_json::from_str(include_str!("../tests/fixtures/update-signature.json")).unwrap()
    }

    #[test]
    fn signed_download_rejects_tampering_and_version_replay() {
        let f = fixture();
        // 配布済みTauri版と同じ署名fixture: 0.2.1で署名されている。
        assert!(verify(f.payload.as_bytes(), &f.signature, &f.public_key, "0.2.1").is_ok());
        assert!(verify(f.payload.as_bytes(), &f.signature, &f.public_key, "v0.2.1").is_ok());
        let mut tampered = f.payload.as_bytes().to_vec();
        tampered.push(b'!');
        assert!(verify(&tampered, &f.signature, &f.public_key, "0.2.1").is_err());
        assert!(verify(f.payload.as_bytes(), &f.signature, &f.public_key, "0.2.2").is_err());
        assert!(verify(f.payload.as_bytes(), &f.signature, PUBLIC_KEY, "0.2.1").is_err());
    }

    #[test]
    fn only_newer_versions_are_offered_and_never_older_ones() {
        assert!(is_newer("0.3.2", "0.3.1").unwrap());
        assert!(!is_newer("0.3.1", "0.3.1").unwrap());
        assert!(!is_newer("0.2.9", "0.3.1").unwrap());
        assert!(is_newer("0.10.0", "0.9.9").unwrap());
        assert!(is_newer("x", "0.3.1").is_err());
    }

    #[test]
    fn downloads_are_limited_to_this_projects_release_installers() {
        let ok = |u: &str| allowed_download(&reqwest::Url::parse(u).unwrap());
        assert!(ok(
            "https://github.com/Yumiiiiiiiiii/DiToVR/releases/download/v0.3.1/DiToVR_0.3.1_x64-setup.exe"
        ));
        assert!(ok(
            "https://github.com/Yumiiiiiiiiii/discord_To_VR/releases/download/v0.2.2/a-setup.exe"
        ));
        assert!(!ok(
            "http://github.com/Yumiiiiiiiiii/DiToVR/releases/download/v1/a-setup.exe"
        ));
        assert!(!ok(
            "https://evil.example/Yumiiiiiiiiii/DiToVR/releases/download/v1/a-setup.exe"
        ));
        assert!(!ok(
            "https://github.com/Yumiiiiiiiiii/DiToVR/releases/download/v1/a.exe"
        ));
        assert!(!ok(
            "https://github.com/Yumiiiiiiiiii/DiToVR/releases/download/v1/a-setup.exe?x=1"
        ));
    }

    #[test]
    fn preferences_migrate_once_and_never_overwrite() {
        let dir = std::env::temp_dir().join(format!("ditovr-upd-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (old, new) = (dir.join("old.json"), dir.join("updates.json"));
        std::fs::write(&old, br#"{"auto_update":false}"#).unwrap();
        assert!(migrate_preferences_from(&[&old], &new).unwrap());
        assert_eq!(std::fs::read(&new).unwrap(), br#"{"auto_update":false}"#);
        std::fs::write(&old, br#"{"auto_update":true}"#).unwrap();
        assert!(!migrate_preferences_from(&[&old], &new).unwrap());
        assert_eq!(std::fs::read(&new).unwrap(), br#"{"auto_update":false}"#);
        std::fs::remove_file(&new).unwrap();
        std::fs::write(&old, b"{bad").unwrap();
        assert!(migrate_preferences_from(&[&old], &new).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn store_builds_never_enable_updates_or_network() {
        let dir = std::env::temp_dir().join("ditovr-store-upd.json");
        let u = Updater::with(dir, false, false, true);
        let s = u.snapshot();
        assert!(s.store_managed && !s.can_check && !s.auto_update && !s.supported);
        assert!(u.set_auto_update(true).is_err());
    }
}
