# リファクタリング計画（refactor/internal-quality）

方針: 機能追加・UI変更なし。設定形式、DPAPI、OAuth保存、更新署名検証、Store/通常版の分離、単一起動、トレイ、`--debug`、`--ui-preview` は維持する。変更は作業単位ごとに回帰テストを付け、Core と desktop を別々に検証する。

`ARCHITECTURE.md` は Tauri 時代の記述のままで現行コードと一致しない。最終段階で現行構成に合わせて更新する。

基準状態（変更前, 2026-11-20）: fmt・clippy(-D warnings)・Core 50 テスト・desktop 16 テスト・Node 33 テストはすべて成功。

## 作業単位（担当ファイルは重ならない）

| # | 内容 | ファイル | 担当 | 状態 |
| --- | --- | --- | --- | --- |
| 1a | 更新コールバックが非同期タスク上で `block_on` に入り panic する不具合の修正、`busy` の RAII 化 | `desktop/src/update.rs` | Claude | 完了 |
| 1b | 更新前に設定移行してから環境設定を読む、更新応答のサイズ上限 | `desktop/src/update.rs`, `update_view.rs` | Claude | 完了 |
| 2 | 重複除外を空通知の判定後に記録、ID 正規化の共通化（`data.clone()` 削減は見送り） | `src/discord/notifier.rs` | Claude（Codex は書込権限がなく未実施） | 完了 |
| 3 | PONG 書込を受信期限に含める | `src/discord/ipc.rs` | Claude（同上） | 完了 |
| 4 | コマンド処理中の状態ロック範囲を縮小、診断完了の sleep 依存を除去 | `desktop/src/runtime.rs` | 見送り | 並行性の設計変更で、実機検証なしでは危険 |
| 5 | 送信側が全て閉じたとき VR ワーカーが終了する（デスクトップは明示的に停止しており現状は問題なし） | `src/vr/mod.rs` | 見送り | 今回は対象外 |
| 6 | UI スレッドのブロッキング処理の移動、テーマボタン等の重複描画の統合 | `desktop/src/main.rs`, `service.rs` | 見送り | UI 変更リスク。次回に残す |
| 7 | 設定の復号項目リストの共通化 | `src/config/mod.rs` | 見送り | 保存・復号の経路で互換性リスクが高い |
| 8 | Gateway のフレーム分類の抽出 | `src/discord/presence.rs` | 低優先 | 未着手 |

## 見送り
- 速度改善の名目だけの変更、大規模な依存追加、trait の新設。
- `unwrap()` は長さ・範囲が保証された箇所のみで、バグではない（変更しない）。
