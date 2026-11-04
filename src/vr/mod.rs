use crate::diagnostics::{Outcome, Trace};
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use serde::Serialize;
use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::time::{interval, sleep, timeout, Instant};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

const XSOVERLAY_WS_URL: &str = "ws://127.0.0.1:42070/?client=DiToVR";
const SENDER_NAME: &str = "DiToVR";
const QUEUE_CAPACITY: usize = 64;
const MAX_ICON_BYTES: usize = 256 * 1024;
const MAX_ICON_CACHE: usize = 64;
const SEND_TIMEOUT: Duration = Duration::from_secs(5);

/// XSOverlay loads custom Ogg clips through UnityWebRequest, which needs a URL.
/// Keep the built-in sound names unchanged and escape local file paths as file URLs.
pub fn notification_audio_path(sound: &str) -> io::Result<String> {
    if matches!(sound, "" | "default" | "error" | "warning") {
        return Ok(sound.into());
    }
    let path = Path::new(sound);
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "通知音に使う Ogg ファイルを「参照」から選んでください。",
        ));
    }
    if !path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("ogg"))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "通知音の準備ができていません。設定で音声ファイルを選び直してください。",
        ));
    }
    reqwest::Url::from_file_path(path)
        .map(String::from)
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "通知音のファイルパスを確認してください。",
            )
        })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NotificationData<'a> {
    #[serde(rename = "type")]
    msg_type: i32,
    timeout: f64,
    height: i32,
    opacity: f64,
    volume: f64,
    audio_path: &'a str,
    title: &'a str,
    content: &'a str,
    use_base64_icon: bool,
    icon: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WsMessage {
    sender: &'static str,
    target: &'static str,
    command: &'static str,
    json_data: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AudioData<'a> {
    audio_path: &'a str,
    volume: f64,
    spatial: bool,
}

pub struct QueuedNotification {
    pub(crate) title: String,
    pub(crate) content: String,
    icon_url: String,
    sound_path: String,
    sound_only: Arc<AtomicBool>,
    volume: f64,
    duration: f64,
    expires: Instant,
    gate: Option<crate::discord::receive::NotificationGate>,
    trace: Option<Trace>,
}

impl QueuedNotification {
    fn finish(&mut self, outcome: Outcome) {
        if let Some(trace) = self.trace.take() {
            trace.record(outcome);
        }
    }
    /// Exercises queued policy and output constraints without network, file reads or playback.
    pub fn validate_without_sending(mut self) {
        if self.expires <= Instant::now() {
            self.finish(Outcome::DropExpired);
            return;
        }
        if !self.allowed() {
            self.finish(Outcome::DropPolicy);
            return;
        }
        // Diagnostics keep original MP3/WAV paths and deliberately skip conversion.
        // Validate the source setting without claiming that it was decoded or played.
        let valid = if self.sound_only.load(Ordering::SeqCst) {
            !self.sound_path.is_empty()
                && self.volume.is_finite()
                && self.volume > 0.0
                && crate::audio::validate_sound_path(&self.sound_path).is_ok()
        } else {
            self.to_json("").is_ok()
        };
        let outcome = if valid {
            Outcome::Validated
        } else {
            Outcome::DropInvalid
        };
        self.finish(outcome);
    }
    pub(crate) fn allowed(&self) -> bool {
        self.gate.as_ref().is_none_or(|gate| gate.allows())
    }
    fn to_json(&self, icon: &str) -> Result<String, serde_json::Error> {
        self.to_json_for_mode(icon, self.sound_only.load(Ordering::SeqCst))
    }
    fn to_json_for_mode(&self, icon: &str, sound_only: bool) -> Result<String, serde_json::Error> {
        let audio_path = if self.sound_path.is_empty() || self.volume <= 0.0 {
            String::new()
        } else {
            notification_audio_path(&self.sound_path).unwrap_or_else(|error| {
                // An old unsupported sound setting must not discard the message itself.
                eprintln!("⚠️ [XSOverlay] 通知音を使用できません: {error}");
                String::new()
            })
        };
        if sound_only && audio_path.is_empty() {
            return Err(serde_json::Error::io(io::Error::new(
                io::ErrorKind::InvalidInput,
                "通知音のみを使うには、通知音を選び音量を上げてください。",
            )));
        }
        let data = NotificationData {
            msg_type: 1,
            timeout: self.duration,
            height: if self.content.contains('\n') {
                225
            } else {
                175
            },
            opacity: 1.0,
            volume: if audio_path.is_empty() {
                0.0
            } else {
                self.volume.clamp(0.0, 1.0)
            },
            audio_path: &audio_path,
            title: &self.title,
            content: &self.content,
            use_base64_icon: !icon.is_empty(),
            icon: if icon.is_empty() { "default" } else { icon },
        };
        serde_json::to_string(&WsMessage {
            sender: SENDER_NAME,
            target: "xsoverlay",
            command: if sound_only {
                "PlayAudio"
            } else {
                "SendNotification"
            },
            json_data: if sound_only {
                serde_json::to_string(&AudioData {
                    audio_path: &audio_path,
                    volume: data.volume,
                    spatial: false,
                })?
            } else {
                serde_json::to_string(&data)?
            },
        })
    }
}

impl Drop for QueuedNotification {
    fn drop(&mut self) {
        self.finish(Outcome::DropStopped);
    }
}

#[derive(Clone)]
pub struct XsOverlaySender {
    tx: Sender<QueuedNotification>,
    http_client: Client,
    is_connected: Arc<AtomicBool>,
    sound_only: Arc<AtomicBool>,
    gate: Option<crate::discord::receive::NotificationGate>,
    trace: Option<Trace>,
    diagnostic_only: bool,
}

impl XsOverlaySender {
    pub fn with_sound_only(&self, sound_only: Arc<AtomicBool>) -> Self {
        let mut sender = self.clone();
        sender.sound_only = sound_only;
        sender
    }
    pub fn sound_only_mode(&self) -> bool {
        self.sound_only.load(Ordering::SeqCst)
    }
    pub fn with_trace(&self, trace: Trace) -> Self {
        let mut sender = self.clone();
        sender.trace = Some(trace);
        sender
    }
    fn record(&self, outcome: Outcome) {
        if let Some(trace) = &self.trace {
            trace.record(outcome);
        }
    }
    pub fn with_notification_gate(&self, gate: crate::discord::receive::NotificationGate) -> Self {
        let mut sender = self.clone();
        sender.gate = Some(gate);
        sender
    }

    async fn get_base64_icon(&self, url: &str) -> String {
        if !allowed_icon_url(url) {
            return String::new();
        }
        let fetch = async {
            let mut response = self.http_client.get(url).send().await.ok()?;
            if !response.status().is_success()
                || response
                    .content_length()
                    .is_some_and(|size| size > MAX_ICON_BYTES as u64)
                || !response
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|h| h.to_str().ok())
                    .is_some_and(|s| s.starts_with("image/"))
            {
                return None;
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.ok()? {
                if bytes.len() + chunk.len() > MAX_ICON_BYTES {
                    return None;
                }
                bytes.extend_from_slice(&chunk);
            }
            if bytes.is_empty() {
                return None;
            }
            Some(base64::engine::general_purpose::STANDARD.encode(bytes))
        };
        timeout(Duration::from_secs(1), fetch)
            .await
            .ok()
            .flatten()
            .unwrap_or_default()
    }

    /// Queue immediately. Icon downloads happen in the overlay task, not in RPC.
    pub async fn send_notification(
        &self,
        title: &str,
        content: &str,
        icon_url: Option<&str>,
        sound_path: &str,
        volume: f64,
        duration: f64,
    ) -> bool {
        if self.gate.as_ref().is_some_and(|gate| !gate.allows()) {
            self.record(Outcome::DropPolicy);
            return false;
        }
        if !self.diagnostic_only && !self.is_connected.load(Ordering::SeqCst) {
            self.record(Outcome::DropNoOverlay);
            eprintln!("⚠️ XSOverlay 未接続のため通知を表示できませんでした。");
            return false;
        }
        let mut notification = QueuedNotification {
            title: title.into(),
            content: content.into(),
            icon_url: icon_url.unwrap_or("").into(),
            sound_path: sound_path.into(),
            sound_only: self.sound_only.clone(),
            volume,
            duration,
            expires: Instant::now() + Duration::from_secs(15),
            gate: self.gate.clone(),
            trace: self.trace.clone(),
        };
        match self.tx.try_reserve() {
            Ok(permit) => {
                self.record(Outcome::Queued);
                permit.send(notification);
                true
            }
            Err(_) => {
                notification.finish(Outcome::DropFull);
                eprintln!("⚠️ 通知キューが満杯、または送信タスクが終了しています。");
                false
            }
        }
    }
}

fn allowed_icon_url(url: &str) -> bool {
    reqwest::Url::parse(url).ok().is_some_and(|u| {
        u.scheme() == "https"
            && matches!(
                u.host_str(),
                Some("cdn.discordapp.com" | "media.discordapp.net")
            )
            && u.port().is_none()
            && u.username().is_empty()
            && u.password().is_none()
    })
}

pub fn create_xsoverlay_channel(
    http_client: Client,
    sound_only: Arc<AtomicBool>,
) -> (
    XsOverlaySender,
    Receiver<QueuedNotification>,
    Arc<AtomicBool>,
) {
    let (tx, rx) = mpsc::channel(QUEUE_CAPACITY);
    let is_connected = Arc::new(AtomicBool::new(false));
    let sender = XsOverlaySender {
        tx,
        http_client,
        is_connected: is_connected.clone(),
        sound_only,
        gate: None,
        trace: None,
        diagnostic_only: false,
    };
    (sender, rx, is_connected)
}

/// A local receive-check queue. Never represents an XSOverlay connection.
pub fn create_diagnostic_channel(
    http_client: Client,
    sound_only: Arc<AtomicBool>,
) -> (XsOverlaySender, Receiver<QueuedNotification>) {
    let (mut sender, rx, _) = create_xsoverlay_channel(http_client, sound_only);
    sender.diagnostic_only = true;
    (sender, rx)
}

/// Validate incoming messages in memory without sockets, images or VR output.
pub async fn receive_diagnostic_loop(mut rx: Receiver<QueuedNotification>) {
    while let Some(note) = rx.recv().await {
        note.validate_without_sending();
    }
}

struct ConnectionGuard(Arc<AtomicBool>);
impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

pub async fn xsoverlay_loop(
    rx: Receiver<QueuedNotification>,
    is_connected: Arc<AtomicBool>,
    sender: XsOverlaySender,
) {
    xsoverlay_loop_at(rx, is_connected, sender, XSOVERLAY_WS_URL).await;
}

async fn xsoverlay_loop_at(
    mut rx: Receiver<QueuedNotification>,
    is_connected: Arc<AtomicBool>,
    sender: XsOverlaySender,
    url: &str,
) {
    let _guard = ConnectionGuard(is_connected.clone());
    let mut icon_cache = HashMap::<String, String>::new();
    loop {
        match timeout(SEND_TIMEOUT, connect_async(url)).await {
            Ok(Ok((ws_stream, _))) => {
                is_connected.store(true, Ordering::SeqCst);
                println!("✅ [XSOverlay] 接続しました。Discord の接続状態は別途表示されます。");
                if !sender.sound_only_mode() {
                    sender
                        .send_notification(
                            SENDER_NAME,
                            "XSOverlay に接続しました",
                            None,
                            "",
                            0.0,
                            5.0,
                        )
                        .await;
                }
                let (mut ws_tx, mut ws_rx) = ws_stream.split();
                let mut ping_interval = interval(Duration::from_secs(10));
                loop {
                    tokio::select! {
                        queued = rx.recv() => {
                            let Some(mut notification) = queued else { return };
                            if notification.expires <= Instant::now() { notification.finish(Outcome::DropExpired); continue; }
                            if !notification.allowed() { notification.finish(Outcome::DropPolicy); continue; }
                            let icon = if sender.sound_only_mode() { String::new() } else if let Some(cached) = icon_cache.get(&notification.icon_url) {
                                cached.clone()
                            } else {
                                let icon = sender.get_base64_icon(&notification.icon_url).await;
                                if !icon.is_empty() {
                                    if icon_cache.len() >= MAX_ICON_CACHE { icon_cache.clear(); }
                                    icon_cache.insert(notification.icon_url.clone(), icon.clone());
                                }
                                icon
                            };
                            // The user may switch modes during an icon download.
                            if notification.expires <= Instant::now() { notification.finish(Outcome::DropExpired); continue; }
                            if !notification.allowed() { notification.finish(Outcome::DropPolicy); continue; }
                            let sound_only = sender.sound_only_mode();
                            if sound_only {
                                if notification.sound_path.is_empty() || notification.volume <= 0.0 {
                                    notification.finish(Outcome::DropInvalid);
                                    continue;
                                }
                                if matches!(notification.sound_path.as_str(), "default" | "error" | "warning") {
                                    let Some(path) = crate::system::xsoverlay_sound_file(&notification.sound_path) else {
                                        eprintln!("⚠️ XSOverlay の標準音を読み込めません。通知音のファイルを選んでください。");
                                        notification.finish(Outcome::DropInvalid);
                                        continue;
                                    };
                                    notification.sound_path = path.to_string_lossy().into();
                                }
                            }
                            let Ok(json) = notification.to_json_for_mode(&icon, sound_only) else { notification.finish(Outcome::DropInvalid); continue };
                            if !matches!(timeout(SEND_TIMEOUT,
                                ws_tx.send(Message::Text(json.into()))).await, Ok(Ok(()))) {
                                notification.finish(Outcome::DropSendFailed);
                                break;
                            }
                            notification.finish(Outcome::Sent);
                        }
                        _ = ping_interval.tick() => {
                            if !matches!(timeout(SEND_TIMEOUT,
                                ws_tx.send(Message::Ping(Vec::new().into()))).await, Ok(Ok(()))) {
                                break;
                            }
                        }
                        event = ws_rx.next() => {
                            match event {
                                Some(Ok(Message::Ping(bytes))) => {
                                    if !matches!(timeout(SEND_TIMEOUT,
                                        ws_tx.send(Message::Pong(bytes))).await, Ok(Ok(()))) {
                                        break;
                                    }
                                }
                                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                                _ => {}
                            }
                        }
                    }
                }
                is_connected.store(false, Ordering::SeqCst);
                eprintln!("⚠️ [XSOverlay] 切断されました。3 秒後に再接続します。");
            }
            _ => {
                is_connected.store(false, Ordering::SeqCst);
                eprintln!("⚠️ [XSOverlay] 接続待機中。XSOverlay と通知用 WebSocket の設定を確認してください。");
            }
        }
        // Do not replay messages from a disconnected session.
        while let Ok(mut notification) = rx.try_recv() {
            notification.finish(Outcome::DropDisconnected);
        }
        sleep(Duration::from_secs(3)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{Diagnostics, Source};

    #[tokio::test]
    async fn pending_notifications_switch_to_audio_only_without_visual_payloads() {
        use tokio::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let peer = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            loop {
                if let Some(Ok(Message::Text(bytes))) = ws.next().await {
                    return serde_json::from_str::<serde_json::Value>(&bytes).unwrap();
                }
            }
        });
        let mode = Arc::new(AtomicBool::new(false));
        let (sender, rx, connected) =
            create_xsoverlay_channel(Client::new(), Arc::new(AtomicBool::new(false)));
        let sender = sender.with_sound_only(mode.clone());
        connected.store(true, Ordering::SeqCst);
        let path = r"C:\通知音\sound #1 %.ogg";
        assert!(
            sender
                .send_notification(
                    "private-name",
                    "private-message",
                    Some("https://cdn.discordapp.com/private-icon.png"),
                    path,
                    0.6,
                    7.0
                )
                .await
        );
        // The mode is read immediately before sending, including already queued events.
        mode.store(true, Ordering::SeqCst);
        let task = tokio::spawn(async move {
            xsoverlay_loop_at(rx, connected, sender, &format!("ws://{address}")).await;
        });
        let envelope = timeout(Duration::from_secs(3), peer)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(envelope["command"], "PlayAudio");
        let data: serde_json::Value =
            serde_json::from_str(envelope["jsonData"].as_str().unwrap()).unwrap();
        assert_eq!(data.as_object().unwrap().len(), 3);
        assert_eq!(data["volume"], 0.6);
        assert_eq!(data["spatial"], false);
        assert_eq!(
            reqwest::Url::parse(data["audioPath"].as_str().unwrap())
                .unwrap()
                .to_file_path()
                .unwrap(),
            Path::new(path)
        );
        for absent in [
            "title",
            "content",
            "icon",
            "timeout",
            "private-name",
            "private-message",
            "private-icon",
        ] {
            assert!(!envelope.to_string().contains(absent));
        }
        task.abort();
        let _ = task.await;
    }

    #[tokio::test]
    async fn audio_only_keeps_policy_and_expiry_checks_and_rejects_silent_events() {
        use crate::{
            config::Config,
            discord::receive::{ReceiveControl, ReceiveMode},
        };
        let log = Diagnostics::default();
        let (sender, mut rx) =
            create_diagnostic_channel(Client::new(), Arc::new(AtomicBool::new(false)));
        let control = ReceiveControl::new(&Config::default());
        let sender = sender
            .with_sound_only(Arc::new(AtomicBool::new(true)))
            .with_trace(log.trace(Source::Simulation))
            .with_notification_gate(control.gate(Some("100"), false));
        assert!(
            sender
                .send_notification("test", "test", None, "default", 0.5, 5.0)
                .await
        );
        let mut expired = rx.recv().await.unwrap();
        expired.expires = Instant::now();
        expired.validate_without_sending();
        assert!(
            sender
                .send_notification("test", "test", None, "default", 0.5, 5.0)
                .await
        );
        control.set_mode(ReceiveMode::Whitelist);
        rx.recv().await.unwrap().validate_without_sending();
        assert!(
            !sender
                .send_notification("test", "test", None, "default", 0.5, 5.0)
                .await
        );
        control.set_mode(ReceiveMode::Normal);
        for (sound, volume) in [("", 0.5), ("default", 0.0), (r"C:\unsupported.txt", 0.5)] {
            assert!(
                sender
                    .send_notification("test", "test", None, sound, volume, 5.0)
                    .await
            );
            rx.recv().await.unwrap().validate_without_sending();
        }
        for sound in ["default", r"C:\original.mp3", r"C:\original.wav"] {
            assert!(
                sender
                    .send_notification("test", "test", None, sound, 0.5, 5.0)
                    .await
            );
            rx.recv().await.unwrap().validate_without_sending();
        }
        let snapshot = log.snapshot();
        assert_eq!(snapshot.tests.sent, 0);
        assert_eq!(snapshot.tests.validated, 3);
        for outcome in [
            Outcome::DropExpired,
            Outcome::DropPolicy,
            Outcome::DropInvalid,
        ] {
            assert!(snapshot.events.iter().any(|event| event.outcome == outcome));
        }
    }

    #[tokio::test]
    async fn custom_audio_reaches_websocket_as_an_escaped_file_url() {
        use tokio::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let peer = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            loop {
                if let Some(Ok(Message::Text(bytes))) = ws.next().await {
                    let envelope: serde_json::Value = serde_json::from_str(&bytes).unwrap();
                    let data: serde_json::Value =
                        serde_json::from_str(envelope["jsonData"].as_str().unwrap()).unwrap();
                    if data["title"] == "audio-test" {
                        return (envelope, data);
                    }
                }
            }
        });
        let (sender, rx, connected) =
            create_xsoverlay_channel(Client::new(), Arc::new(AtomicBool::new(false)));
        let loop_sender = sender.clone();
        let loop_connected = connected.clone();
        let task = tokio::spawn(async move {
            xsoverlay_loop_at(rx, loop_connected, loop_sender, &format!("ws://{address}")).await;
        });
        timeout(Duration::from_secs(3), async {
            while !connected.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let path = r"C:\通知音\My sound #1 100%.OGG";
        assert!(
            sender
                .send_notification("audio-test", "test", None, path, 0.65, 8.0)
                .await
        );
        let (envelope, data) = timeout(Duration::from_secs(3), peer)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(envelope["command"], "SendNotification");
        assert_eq!(data["volume"], 0.65);
        assert_eq!(data["timeout"], 8.0);
        let url = reqwest::Url::parse(data["audioPath"].as_str().unwrap()).unwrap();
        assert_eq!(url.scheme(), "file");
        assert!(url.fragment().is_none());
        assert!(url.query().is_none());
        assert_eq!(url.to_file_path().unwrap(), Path::new(path));
        task.abort();
        let _ = task.await;
    }

    #[tokio::test]
    async fn builtin_and_muted_sounds_preserve_xsoverlay_semantics() {
        let (sender, mut rx, connected) =
            create_xsoverlay_channel(Client::new(), Arc::new(AtomicBool::new(false)));
        connected.store(true, Ordering::SeqCst);
        for (sound, volume, expected_path, expected_volume) in [
            ("default", 0.7, "default", 0.7),
            ("error", 0.5, "error", 0.5),
            ("warning", 0.5, "warning", 0.5),
            ("", 0.7, "", 0.0),
            (r"C:\sound.ogg", 0.0, "", 0.0),
            (r"C:\sound.mp3", 0.7, "", 0.0),
        ] {
            assert!(
                sender
                    .send_notification("test", "test", None, sound, volume, 5.0)
                    .await
            );
            let queued = rx.recv().await.unwrap();
            let envelope: serde_json::Value =
                serde_json::from_str(&queued.to_json("").unwrap()).unwrap();
            let data: serde_json::Value =
                serde_json::from_str(envelope["jsonData"].as_str().unwrap()).unwrap();
            assert_eq!(data["audioPath"], expected_path);
            assert_eq!(data["volume"], expected_volume);
        }
        for path in [
            "file",
            "relative.ogg",
            r"C:\sound.mp3",
            r"C:\sound.wav",
            "https://example.com/sound.ogg",
        ] {
            assert!(notification_audio_path(path).is_err());
        }
    }

    #[tokio::test]
    async fn debug_send_records_websocket_success_without_exposing_payloads() {
        use tokio::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let peer = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            let mut messages = Vec::new();
            while messages.is_empty() {
                if let Some(Ok(Message::Text(bytes))) = ws.next().await {
                    messages.push(bytes);
                }
            }
            messages
        });
        let sound_only = Arc::new(AtomicBool::new(true));
        let log = Diagnostics::default();
        let (sender, rx, connected) = create_xsoverlay_channel(Client::new(), sound_only);
        let sender = sender.with_trace(log.trace(Source::System));
        let loop_sender = sender.clone();
        let loop_connected = connected.clone();
        let task = tokio::spawn(async move {
            xsoverlay_loop_at(rx, loop_connected, loop_sender, &format!("ws://{address}")).await;
        });
        timeout(Duration::from_secs(3), async {
            while !connected.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let mut notifier = crate::discord::notifier::DiscordRPCNotifier::new(
            crate::config::Config {
                notification_sound: r"C:\test.ogg".into(),
                notification_volume: 0.5,
                ..crate::config::Config::default()
            },
            sender,
            Client::new(),
        )
        .with_trace(log.trace(Source::Live));
        notifier.inject_test_notification(false, "100").await;
        let messages = timeout(Duration::from_secs(3), peer)
            .await
            .unwrap()
            .unwrap();
        timeout(Duration::from_secs(3), async {
            while log.snapshot().live.sent != 1 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let snapshot = log.snapshot();
        assert_eq!(snapshot.live.received, 1);
        assert_eq!(snapshot.live.queued, 1);
        assert_eq!(snapshot.live.sent, 1);
        assert_eq!(snapshot.tests.received, 0);
        let json: serde_json::Value = serde_json::from_str(&messages[0]).unwrap();
        let data: serde_json::Value =
            serde_json::from_str(json["jsonData"].as_str().unwrap()).unwrap();
        assert_eq!(json["command"], "PlayAudio");
        assert!(data.get("title").is_none());
        assert!(data.get("content").is_none());
        let metadata = serde_json::to_string(&snapshot).unwrap();
        assert!(!metadata.contains("テスト用サンプル"));
        assert!(!metadata.contains("新しいメッセージ"));
        assert!(!metadata.contains("channel"));
        task.abort();
        let _ = task.await;
    }

    #[tokio::test]
    async fn debug_queue_reports_disconnection_and_final_policy_changes() {
        use crate::{
            config::Config,
            discord::receive::{ReceiveControl, ReceiveMode},
        };
        let log = Diagnostics::default();
        let (sender, mut rx, connected) =
            create_xsoverlay_channel(Client::new(), Arc::new(AtomicBool::new(false)));
        let control = ReceiveControl::new(&Config::default());
        let sender = sender
            .with_trace(log.trace(Source::Live))
            .with_notification_gate(control.gate(Some("100"), false));
        assert!(
            !sender
                .send_notification("dummy", "dummy", None, "", 0.0, 5.0)
                .await
        );
        connected.store(true, Ordering::SeqCst);
        assert!(
            sender
                .send_notification("dummy", "dummy", None, "", 0.0, 5.0)
                .await
        );
        control.set_mode(ReceiveMode::Whitelist);
        rx.recv().await.unwrap().validate_without_sending();
        let snapshot = log.snapshot();
        assert_eq!(snapshot.live.dropped, 2);
        assert_eq!(snapshot.live.sent, 0);
        assert!(snapshot
            .events
            .iter()
            .any(|e| e.outcome == Outcome::DropPolicy));
        assert!(snapshot
            .events
            .iter()
            .any(|e| e.outcome == Outcome::DropNoOverlay));
    }

    #[test]
    fn icon_downloads_only_use_discord_https_hosts() {
        assert!(allowed_icon_url(
            "https://cdn.discordapp.com/avatars/1/2.png"
        ));
        for url in [
            "http://cdn.discordapp.com/a.png",
            "https://127.0.0.1/a.png",
            "https://cdn.discordapp.com.evil.example/a.png",
            "file:///C:/secret",
            "https://user:pass@cdn.discordapp.com/a.png",
        ] {
            assert!(!allowed_icon_url(url));
        }
    }

    #[tokio::test]
    async fn sound_only_toggle_hides_queued_payload_and_bounds_queue() {
        let sound_only = Arc::new(AtomicBool::new(false));
        let (sender, mut rx, connected) =
            create_xsoverlay_channel(Client::new(), sound_only.clone());
        connected.store(true, Ordering::SeqCst);
        assert!(
            sender
                .send_notification(
                    "Alice",
                    "Private Server / #private-channel\nsecret",
                    None,
                    r"C:\sound.ogg",
                    0.8,
                    8.0
                )
                .await
        );
        let queued = rx.recv().await.unwrap();
        let original: serde_json::Value =
            serde_json::from_str(&queued.to_json("").unwrap()).unwrap();
        let data: serde_json::Value =
            serde_json::from_str(original["jsonData"].as_str().unwrap()).unwrap();
        assert_eq!(data["height"], 225);
        sound_only.store(true, Ordering::SeqCst);
        let envelope: serde_json::Value =
            serde_json::from_str(&queued.to_json("").unwrap()).unwrap();
        let data: serde_json::Value =
            serde_json::from_str(envelope["jsonData"].as_str().unwrap()).unwrap();
        assert_eq!(envelope["command"], "PlayAudio");
        assert!(data.get("title").is_none());
        assert!(data.get("timeout").is_none());
        assert_eq!(data["volume"], 0.8);
        assert!(!envelope.to_string().contains("secret"));
        assert!(!envelope.to_string().contains("Private Server"));
        assert!(!envelope.to_string().contains("private-channel"));
        for _ in 0..QUEUE_CAPACITY {
            assert!(
                sender
                    .send_notification("Alice", "secret", None, r"C:\sound.ogg", 0.8, 5.0)
                    .await
            );
        }
        assert!(
            !sender
                .send_notification("Alice", "secret", None, r"C:\sound.ogg", 0.8, 5.0)
                .await
        );
        let queued = rx.recv().await.unwrap();
        let payload = queued.to_json("").unwrap();
        assert!(!payload.contains("Alice"));
        assert!(!payload.contains("secret"));
        connected.store(false, Ordering::SeqCst);
        assert!(
            !sender
                .send_notification("Alice", "secret", None, r"C:\sound.ogg", 0.8, 5.0)
                .await
        );
    }
}
