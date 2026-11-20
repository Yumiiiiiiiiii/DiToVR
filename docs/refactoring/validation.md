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

## 現在の結果
- desktop: `cargo test --locked` と `--features microsoft-store` の両方で 19 テスト成功、clippy(-D warnings) 警告なし（標準・Store）、fmt 差分なし。
- Core: `cargo test --locked` 53 テスト成功、clippy(-D warnings) 警告なし、fmt 差分なし。

## 未検証
- 実機での更新適用（旧 Tauri → 新 NSIS、署名付きインストーラーの起動）、Discord / XSOverlay の実受信、トレイ復帰、`--debug` の実接続診断。
- Codex の `review` は読み取り専用のため、ビルド・テストは Claude 側で実行した。
