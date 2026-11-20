# 検証記録

環境: Windows 11、`CARGO_TARGET_DIR` は `%LOCALAPPDATA%\DiToVRBuildCache\refactor`（実アプリのビルドとは別）。

## 変更前（基準, 2026-11-20）
fmt（Core・desktop）差分なし / Core 50 テスト成功 / desktop 16 テスト成功 / clippy(-D warnings) 両方警告なし / Node 33 テスト成功。

## 変更ごとの回帰テスト
| 変更 | 追加したテスト | 根拠 |
| --- | --- | --- |
| 更新コールバックの `block_on` panic 修正 | `host_callbacks_run_from_an_updater_task_without_nesting_runtimes`、`busy_flag_is_released_even_when_the_flow_unwinds` | 修正前は同じ呼び出しが「Cannot start a runtime from within a runtime」で panic することを再現テストで確認した |
| 更新応答の上限 | `bodies_are_bounded_by_declared_length_and_by_streamed_size`（ちょうど上限・申告超過・ストリーム超過） | ローカルの TCP サーバーで検証 |
| 更新設定の移行順序 | 既存の移行テスト（`preferences_migrate_once_and_never_overwrite`）は維持。順序の入れ替えは起動時の経路のため、単体テストは未追加 | 実機の起動経路では未検証 |
| 重複 ID の記録位置 | `an_empty_event_does_not_consume_the_id_of_a_later_populated_one`、`seen_ids_normalize_numbers_and_evict_the_oldest` | 既存の重複・自己投稿・Bot 系テストは全て成功 |
| PONG 書込の予算 | `pong_write_budget_stays_inside_the_receive_deadline` | 実パイプで書込が詰まる状況のテストは未追加 |
| VR ワーカーの終了 | `worker_exits_when_every_external_sender_is_dropped`（接続中・再接続待ち） | 修正前は自分用の送信側が受信側の終了を妨げていた |
| 認証情報の復号共通化 | `every_protected_secret_is_decoded_by_both_load_paths`（4項目を DPAPI で暗号化して両経路で復号） | 既存の移行・破損データのテストも維持 |
| Gateway の分類抽出 | `gateway_hello_and_close_codes_are_classified` | 既存の ACK 欠落・RESUME・プレゼンス更新テストが成功 |
| 更新中止時に停止中の受信を再開しない | `stopped_services_report_not_running`（状態の取得のみ。更新フロー全体は通信を伴うため自動テストなし） | 実機で未検証 |
| 診断の sleep 依存の除去 | 既存 `public_view_and_simulation...` が `tests > 0` を決定的に検証 | |
| UI スレッドのブロッキング解消（初期表示の順序、初回案内の背景実行） | 自動テストなし（GPUI の画面構造のため）。実機で確認が必要 | |

## 現在の結果
- Core: `cargo test --locked` 56 テスト成功、clippy(-D warnings) 警告なし、fmt 差分なし。
- desktop: 標準版・`--features microsoft-store` とも 20 テスト成功、clippy(-D warnings) 警告なし、fmt 差分なし。
- Node.js: `node --test scripts/*.test.mjs` 33 テスト成功。
- リリースビルド（標準版・Store 版）: `cargo build --release --locked` が成功。

## Codex の関与
- 独立分析（読み取り専用）2 回、`review` 2 回（いずれも作業ツリーの差分）、`adversarial-review` 1 回。指摘は反映済み。
- Codex は `--write` を付けても環境の方針で書込・コマンド実行が拒否されたため、実装・テスト・ビルドは Claude が行った。Codex のレビューは静的解析のみ。

## 未検証
- 実機での更新適用（旧 Tauri → 新 NSIS、署名付きインストーラーの起動）、更新中止時の受信状態の復帰、Discord / XSOverlay の実受信、トレイ復帰、`--debug` の実接続診断、起動直後の画面の応答性。
