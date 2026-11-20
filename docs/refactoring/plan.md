# リファクタリング計画（refactor/internal-quality）

方針: 機能追加・UI変更なし。設定形式、DPAPI、OAuth保存、更新署名検証、Store/通常版の分離、単一起動、トレイ、`--debug`、`--ui-preview` は維持する。変更は作業単位ごとに回帰テストを付け、Core と desktop を別々に検証する。

`ARCHITECTURE.md` は Tauri 時代の記述のままで現行コードと一致しない。最終段階で現行構成に合わせて更新する。

基準状態（変更前, 2026-11-20）: fmt・clippy(-D warnings)・Core 50 テスト・desktop 16 テスト・Node 33 テストはすべて成功。

## 作業単位（担当ファイルは重ならない）

| # | 内容 | ファイル | 担当 | 状態 |
| --- | --- | --- | --- | --- |
| 1a | 更新コールバックが非同期タスク上で `block_on` に入り panic する不具合の修正、`busy` の RAII 化 | `desktop/src/update.rs` | Claude | 完了 |
| 1b | 旧保存先の更新設定を読込前に移行、更新応答のサイズ上限 | `desktop/src/update.rs`, `update_view.rs` | Claude | 完了 |
| 2 | 重複 ID を空通知の判定後に記録、ID 正規化を `SeenIds` に集約 | `src/discord/notifier.rs` | Claude | 完了 |
| 3 | PONG 書込を受信期限に含める | `src/discord/ipc.rs` | Claude | 完了 |
| 4 | 診断完了の sleep 依存を除去（送信側を閉じてキューを処理し終える） | `desktop/src/runtime.rs` | Claude | 完了 |
| 4b | 状態ロックの範囲縮小 | `desktop/src/runtime.rs` | — | 実施しない（下記） |
| 5 | 外部の送信側が全て閉じたら VR ワーカーが終了（接続中・再接続待ちとも） | `src/vr/mod.rs` | Claude | 完了 |
| 6 | 起動直後の初期表示が自動接続のロックを待たない順序に変更、初回案内の完了を背景で実行 | `desktop/src/service.rs`, `update_view.rs` | Claude | 完了 |
| 7 | 認証情報の復号を `decode_secrets` に集約 | `src/config/mod.rs` | Claude | 完了 |
| 8 | Gateway の HELLO 解釈・Close コード分類・再試行待ちの重複を抽出 | `src/discord/presence.rs` | Claude | 完了 |
| 9 | 更新の中止時、更新前に止めていた受信を再開しない（`was_running`）。インストーラー起動直前に編集中を再確認して待ち直す | `desktop/src/update.rs`, `update_view.rs`, `runtime.rs` | Claude（Codex の監査で発見） | 完了 |

Codex は `--write` を付けても環境の方針で書込とコマンド実行が拒否されたため、独立分析とレビュー（読み取りのみ）を担当した。実装とテストは Claude が行った。

## 見送り
- 速度改善の名目だけの変更、大規模な依存追加、trait の新設。
- `unwrap()` は長さ・範囲が保証された箇所のみで、バグではない（変更しない）。

## 実施しない判断と根拠
- **状態ロックの範囲縮小（`runtime.rs`）**: コマンドが状態ロックを保持するのは、旧 OAuth タスクを完全に止めてから資格情報を差し替える不変条件のため（途中のトークン更新が古い資格情報を新しい設定へ書く事故を防ぐ）。`view()` が待たされるのは、初回の音声変換など長い処理の間だけで、状態の取得は背景スレッドで行うため画面は止まらない。UI スレッド側のブロッキングは別途解消した（#6）。ロックを細分化すると並行テストでも網羅しにくい競合が増えるため、実機検証なしでは行わない。
- **`set_auto_update` のファイル書込をUIスレッドで実行**: 数 KB の原子的書込のみで、体感できるブロックは確認できていない。
- **再起動時に通常版とプレビュー版のウィンドウを取り違える可能性（`shell.rs`、低）**: 両者は同じタイトルで、`FindWindowW` が別のモードの窓を返し得る。修正には窓の識別子を変える必要があり、見た目（タイトル）に影響するため今回は実施しない。プレビューは開発・撮影用。
- **更新適用直前の編集開始の完全な排除**: 最終確認で窓は狭めたが、UI スレッドで編集を予約する方式が必要。実機で確認できる段階で検討する。

## リポジトリ構成の整理（2026-11-20）
調査: Git 管理 102 ファイルを、コード内の参照・ビルド設定・配布スクリプト・Actions の参照で照合し、Codex（読み取り専用）の独立監査と突き合わせた。

| 区分 | 対象 | 判断 |
| --- | --- | --- |
| 削除（コード） | `src/system/mod.rs` の `report_error`（呼び出しなし。旧 CLI／Tauri 時代の名残）と、それだけが使っていた `MessageBoxW`／`IsTerminal` の参照、ルートの `windows-sys` 機能 `Win32_UI_WindowsAndMessaging` | 呼び出し・使用箇所が無いことを grep とビルドで確認 |
| 追加 | `ARCHITECTURE.md`（現行の GPUI 構成に書き直した。個人情報なし。`.gitignore` の除外を外して管理対象にした）、`.gitignore` に `/scratchpad/`・`/.codex/`・`*.log` | |
| 修正（文書） | `README.md`、`SECURITY.md`、`CHANGELOG.md`、`docs/DEVELOPMENT.md`、`docs/UI_GUIDE.md`（現行の画面に合わせて書き直し）、`docs/AUTO_UPDATE.md`、`docs/MICROSOFT_STORE.md`、`src/lib.rs` のコメント | Tauri／npm／WebView の記述を、現行の GPUI 構成とコードの挙動に合わせた |
| 修正（挙動） | `--debug` のとき SteamVR 連動の自動終了を起動しない（文書どおり） | 実機で未検証 |
| 維持 | 全依存クレート、全 `scripts/*`（呼び出し元あり）、`docs/release-notes/*`（リリース workflow がタグ名で参照）、`docs/images/*`（全 8 枚を UI ガイドが参照）、`desktop/assets/*`（`icon.svg` は画像の元データ）、署名用の `TAURI_SIGNING_PRIVATE_KEY` 名・Tauri CLI の署名コマンド、旧リポジトリ URL・旧保存先の移行、NSIS の更新フラグ・クリーンアップ、Store の identity、両方の lockfile と著作権表示 | 互換性・更新署名・配布に関わるため削除しない |
| 見送り | `Win32_Security` などの `windows-sys` 機能の細分化、Tokio の `full` の縮小 | `CreateMutexW` が `Win32_Security` を要求するなど、ビルド検証なしでは判断できない |
| 見送り | 通常版とプレビュー版のウィンドウ取り違え（`shell.rs`）、開発用「画面サイズの確認」ボタンの削除 | 見た目に影響するため、次の版で扱う |
