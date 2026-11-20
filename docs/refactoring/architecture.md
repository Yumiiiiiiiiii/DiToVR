# 構成（v0.4.1 時点・GPUI 版）

`ARCHITECTURE.md` は Tauri 時代の記述で、現行コードとは一致しない。現行の構成は次のとおり。

```
Discord デスクトップ ─(名前付きパイプ/RPC)→ src/discord/notifier.rs ┐
Bot Gateway ───────────(プレゼンス)────────→ src/discord/presence.rs ┤→ receive.rs（受信条件）→ 有界キュー → src/vr/mod.rs → XSOverlay
GPUI 画面(desktop/src/main.rs) ⇄ service.rs ⇄ runtime.rs（Core の接続・設定・音声）
                                  update.rs / update_view.rs（署名付き更新。Store 版は無効）
```

| 領域 | ファイル | 責務 |
| --- | --- | --- |
| Core | `src/discord/ipc.rs` | Windows 名前付きパイプ、フレーム上限、タイムアウト、PING/PONG |
| Core | `src/discord/notifier.rs` | OAuth 認可、購読、再接続、自分の通知・重複の除外（`SeenIds`） |
| Core | `src/discord/presence.rs`, `receive.rs` | Bot のプレゼンス判定と受信ポリシー |
| Core | `src/vr/mod.rs` | XSOverlay WebSocket、送信直前の再確認、期限切れ破棄 |
| Core | `src/config/` | 設定、DPAPI、原子的保存 |
| desktop | `runtime.rs` | Core の起動・停止・保存。専用 Tokio ランタイムを持ち、画面からは同期の `execute`/`view` で使う |
| desktop | `service.rs`, `main.rs` | GPUI の状態、背景スレッドとの橋渡し、画面描画 |
| desktop | `update.rs`, `update_view.rs` | 更新確認・ダウンロード・署名検証・適用。独自の Tokio ランタイムで動く |

## 非同期実行の規則（今回の修正で明文化）
- `Runtime::execute` / `apply_*` は内部で `block_on` するため、**どの Tokio ランタイムのタスク上でも直接呼ばない**。UI のワーカースレッドか `spawn_blocking` から呼ぶ（`update.rs` の `call_host`）。
- 更新の `busy` は `BusyGuard`（Drop）で解除し、失敗・panic でも残さない。
- 更新サーバーの応答は署名検証前に上限を設ける（フィード 1 MiB、インストーラー 200 MiB）。

## 今回の変更で変わった点
- 通知の重複 ID は、自分の投稿・空本文の判定を通った後に記録する（空イベントが ID を消費しない）。
- PING への PONG 書込は、受信期限の残り時間（最低 250 ms、最大 5 秒）で打ち切る。
- 旧保存先の更新設定は、`Updater` が読む前に移行する。
