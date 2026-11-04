# DiToVR 開発引継ぎ

最終調査日: **2026-11-04 22:38 JST**。公開ソースの基準は `v0.3.0` → `7d553a15bb33b32bf414d64c2ab02c744d24bd4b`。初回調査時の基準は `c544cb3a8dff3e850b1e58aaf55aae63c0be8ab8`。

**同日22:38 JSTの更新:** ユーザーの公開指示に基づき **v0.3.0をGitHub・BOOTHで公開済み**。署名付き更新フィードも0.3.0へ更新済み。Storeの0.3.0.0は申請受付済み（PreProcessing）で公開完了は未確認。音声修正・MP3/WAV対応・通知音のみ・配信用表示削除を含む。CIと配布物の照合は成功。XSOverlayの実音声出力は引き続き未検証。末尾「v0.3.0公開結果」が最新の記録。

この文書は開発再開用の現在地です。利用方法は [README](../README.md) と [操作ガイド](UI_GUIDE.md)、通常の開発手順は [開発ガイド](DEVELOPMENT.md) を参照してください。開発時の継続ルールは [AGENTS.md](../AGENTS.md) にあります。

## 調査の範囲と根拠

- ソース、Git 履歴、既存文書、ローカルのテスト・ビルド結果を確認した。
- 初回はGitHub API、BOOTH公開ページ、Partner Centerの製品概要・パッケージ画面を読み取り確認した。後続のユーザー指示に基づく公開・更新申請は末尾に記録。認証設定や秘密鍵は変更していない。
- このセッションで提示された過去の会話と引継ぎ情報は参照できるが、過去の Codex セッション全会話を直接閲覧したわけではない。記憶上の配布状況は現在の画面・API で再確認した。
- 実ユーザーの設定ファイルや秘密鍵の内容は調査していない。以下の「実装済み」はコード上の存在を表し、実サービスとの動作確認の完了とは区別する。
- 初回調査（05:57～06:03）の変更は引継ぎ文書・開発ルール・案内リンクのみ。後続の実装・検証・コミット・公開は末尾に記録する。初回時点の未実施事項と最新の完了事項を区別する。

## 現在の状態

DiToVR は Discord デスクトップアプリの通知を RPC/IPC で受信し、XSOverlay のローカル WebSocket に転送する Windows アプリ。利用者が自分の Discord アプリと Client ID を用意する方式を維持している。Bot は任意のプレゼンス判定用で、会話の取得経路ではない。

| 対象 | 2026-11-04 の確認結果 |
| --- | --- |
| 現行ソース | デスクトップ版 **0.3.0**。npm、Tauri、デスクトップ Cargo と各 lockfile は整合。共通ライブラリ `ditovr-core` は独立して **0.1.0** |
| GitHub | 最新公開 [Releaseはv0.3.0](https://github.com/Yumiiiiiiiiii/DiToVR/releases/tag/v0.3.0)。添付は `DiToVR_0.3.0_x64-setup.exe` のみ。Source codeはGitHub標準のzip / tar.gz |
| 更新フィード | `updates` の `latest.json` は **0.3.0**。`migration.json` の `bridgeVersion` は互換性維持のため **0.2.2** のまま。実インストーラーの署名とバージョン結合を検証済み |
| Microsoft Store | 製品 **9NFDCGL30V3K**。0.2.21.0は公開済み。0.3.0.0は申請 **1152921505702314667** の受付済み（22:34:48 JSTにPreProcessing）。最新の公開完了・端末更新は未確認 |
| BOOTH | 商品 [8621228](https://yuuumiii.booth.pm/items/8621228)。無料・応援用とも `DiToVR_0.3.0_x64-setup.zip`、ファイルID10078568。説明文も更新。無料版の公開ダウンロードを取得し、GitHubと同じインストーラーであることをSHA-256で照合済み |

**v0.3.0は通常版・Store版共通のリリース。** Store版のアプリ内更新削除とStore管理を維持し、通常版は署名付き自動更新を維持する。従来のStore専用0.2.21とは配布範囲が異なる。

次回は通常版・Store版の最新公開済み／申請中パッケージより大きい番号にする。今回の0.3.0は従来の0.2.21より大きい。ユーザーの明示指示なく次のタグ・Release・Store申請を作成しない。

## 初回調査時のGitスナップショット（履歴）

確認日時: **2026-11-04 05:57～06:03 JST**。作業ディレクトリ: `D:\test\discxss`。

| 項目 | 結果 |
| --- | --- |
| 現在のブランチ | `develop`、追跡先 `origin/develop`、ahead / behind ともに 0 |
| HEAD | `c544cb3a8dff3e850b1e58aaf55aae63c0be8ab8` — `docs: record Store-only v0.2.21 submission acceptance [skip ci]` |
| release | ローカル・リモートとも `80e4a90ac59ec1f8d185e6202c453fab0fd4677f` |
| ブランチ間 | `develop` と `release` は履歴が異なるが、確認時のファイルツリー差分は空 |
| origin | `https://github.com/Yumiiiiiiiiii/DiToVR.git`、リモート既定ブランチは `release` |
| リモート確認 | `git ls-remote --heads origin` で develop / release が追跡参照と一致。`updates` は `5a522157208a3825bffab5c0776d3e4508e4e8b1` |
| Store タグ | `store-v0.2.21` → `eee43626ce554eaa788f0f6f8967bb5ba92f21a6` |
| 通常タグ | `v0.2.2` → `2157841e1eebac598bdf5e79fd69e8d478b6c535` |
| 調査開始時の作業ツリー | index・未コミット変更・非 ignored の未追跡ファイルはいずれもなし。作業途中のソース差分はなかった |

この文書等の作成後は未コミットのドキュメント変更がある。`target/`・`node_modules/`・`dist/` 等の ignored 成果物はクリーンという判定には含まれない。追跡済みの配布 exe / dll / zip / msix、秘密鍵、実設定 `config.json` は見つからなかった。

この端末では Git の所有者チェックで拒否されるため、調査コマンドに `git -c safe.directory=D:/test/discxss ...` を使った。グローバル設定は変更していない。

### 直近までの開発経緯

| コミット | 内容 |
| --- | --- |
| `93011d9` | Tauri GUI、受信制限、安全性改善、自動更新 |
| `a921ef4` | v0.2.1: テーマ選択、既存ウィンドウへの復帰、サーバー・チャンネル名 |
| `4d38104` / `069623b` | 不要な旧 GUI 等の除去、Release 添付簡素化、更新フィード移行 |
| `e8d8b6c` / `2414d5c` | Store ビルド・タグ申請、DiToVR への名称変更 |
| `9579fbd` / `fa36af5` | BOOTH 実インストーラー配布の記録、v0.2.2 の準備 |
| `39c21fa` | Store テスト前のフロントエンドビルド、既存タグからの再申請 |
| `eee4362` / `c544cb3` | Store 専用 v0.2.21 のアプリ内更新削除と申請受付記録 |

## 技術構成と主要ソース

ルート Cargo と `src-tauri/Cargo.toml` は別 manifest・別 lockfile で、Cargo workspace ではない。Rust の通知基盤、Tauri のデスクトップ制御、React の画面を分けている。

| 場所 | 責務 |
| --- | --- |
| `src/lib.rs`、`src/discord/` | 共通ライブラリ、Discord IPC・OAuth・通知処理・受信制御・Bot |
| `src/discord/ipc.rs` | Windows 名前付きパイプ、フレーム・タイムアウト・PING/PONG |
| `src/discord/notifier.rs`、`context.rs` | 認証、購読、重複除外、通知整形、サーバー・チャンネル情報のキャッシュ |
| `src/discord/receive.rs`、`presence.rs` | 通常／許可リスト／自動判別のポリシー、Bot Gateway のプレゼンス追跡 |
| `src/vr/mod.rs` | XSOverlay送信、画像取得、キュー、送信直前の受信条件・通知音のみモードの再確認 |
| `src/audio.rs` | MP3/WAVのデコード・Ogg Vorbis変換・キャッシュ・音声検証 |
| `src/config/mod.rs`、`storage.rs` | 設定定義・入力検証・互換形式・DPAPI・原子的保存 |
| `src/diagnostics.rs`、`src/system/mod.rs` | メモリ内の受信診断、DPI、SteamVR 終了検知 |
| `src-tauri/src/main.rs`、`app.rs` | 起動、Tauri commands / DTO、サービス寿命、トレイ、二重起動防止 |
| `src-tauri/src/paths.rs`、`distribution.rs` | 旧保存先の移行、Store 管理の判定 |
| `src-tauri/src/updater.rs`、`updater_tests.rs` | 通常版の署名付き自動更新、更新設定、検証テスト |
| `frontend/main.tsx`、`bridge.ts` | アプリシェル・ホーム・設定、型付き IPC、ブラウザ用ダミープレビュー |
| `frontend/onboarding.tsx`、`diagnostics.tsx`、`updates.tsx` | 初回案内、受信診断、通常版の更新画面 |
| `frontend/theme.css` 等 | 既存のテーマ・コンポーネントスタイル |
| `src-tauri/installer-hooks.nsh` | 旧名インストールとショートカットの引継ぎ、明示時のみの設定削除 |
| `scripts/`、`store/`、`.github/workflows/` | 検証・通常配布・MSIX・Store 自動申請、予約済み identity、CI |

通知の主経路は、Discord デスクトップ IPC → `notifier` → `ReceiveControl` → 有界キュー → `vr` → XSOverlay。Bot Gateway → `ReceiveControl` は、自動受信モードの判定にだけ使う。画面は Tauri commands を呼び、保存済みの秘密情報自体は受け取らない。

## 実装済みの機能と重要な挙動

| 機能 | 現在の実装 |
| --- | --- |
| Discord 接続・認証 | `discord-ipc-0..9` を探索。AUTHORIZE → Discord 公式 token endpoint → AUTHENTICATE。access / refresh token を保存し、失効と一時障害を区別して再認証・再接続する。アカウントトークン抽出は行わない |
| 通知受信 | `NOTIFICATION_CREATE` と許可リスト先の `MESSAGE_CREATE`。自分の送信、空の通知、対象外、重複を除外。添付等は代替文を使う。サーバー通知は可能ならサーバー名・チャンネル名を付加 |
| 権限 | 通常は `rpc` + `rpc.notifications.read`。許可リストが空でない場合は `messages.read` も要求する。現在の受信モードだけで追加 scope の有無が決まるわけではない。旧トークンの権限は自動撤回しない |
| フィルター・許可リスト | 通常／許可リストだけ／Bot 自動判別。チャンネル・DM の ID を最大32件、重複なしで登録。GUI はメッセージリンクからも ID を取り出す。通常イベントと直接購読イベントの重複を抑制 |
| Bot 自動判別 | `GUILDS` + `GUILD_PRESENCES` のみ。指定したサーバーと自分のユーザーの状態を追う。RPC 利用者との一致を確認し、online / idle のとき通常受信、それ以外・不明・切断時は許可リストに制限。リスト空なら会話を通さない |
| XSOverlay | `ws://127.0.0.1:42070/?client=DiToVR` に送信、再接続あり。通知キュー64件・期限15秒。切断中・期限超過は破棄し、復帰後に古い通知を再送しない |
| 配信用表示 | 名前・本文・画像を匿名化し、送信待ちの通知にも現在の状態を再適用。メタ情報・画像取得も抑制する。送信開始済み・送信済み通知は撤回できない |
| 設定・機密保護 | `%APPDATA%\DiToVR\config.json`。Client Secret、OAuth access / refresh、Bot Token を Windows ユーザーに紐づく DPAPI で暗号化。未知の JSON 項目を保持し、不正 JSON を上書きしない。認証設定変更時はトークンを消去。隣接一時ファイルから原子的置換 |
| GUI・初回案内 | Tauri 2 + React / TypeScript / Vite。接続状態、設定4タブ、5段階チュートリアル、通知テスト、システム／ライト／ダークのテーマ選択。保存済み Secret / Token は WebView に返さず、設定済みフラグで表示 |
| トレイ・背景動作 | × / Esc はトレイがある場合に収納。再起動操作は単一インスタンス機構で起動済み画面へ復帰。2回目の起動フラグは再適用しない。SteamVR が一度動作した後の終了を監視し、通常起動時はアプリを終了する |
| 診断・プレビュー | `-debug` / `--debug` は実際の Discord 受信を診断し、XSOverlay・画像取得・VR 終了連動を使わない。実受信とダミーテストを区別。履歴は本文・名前・ID・秘密情報を含まずメモリ最大100件。`--ui-preview` は接続・実設定読み書きなし |
| 自動更新 | 通常インストーラー版は署名・バージョン署名・固定 HTTPS 配布 URL を検証し、自動ダウンロード・適用・再起動。起動15秒後、以降12時間間隔。設定入力中は適用待ち。インストール先照合に失敗する展開版は確認のみ。デバッグ・診断・プレビューでは更新通信なし |
| Store ビルド | `microsoft-store` feature で独自 updater モジュール・commands・plugin・起動・設定移行を除外。UI も更新メニュー・通知・ポーリングを省く。現在のバージョンは表示。通常 feature でもパッケージ identity を検知して Store 管理扱いにする |
| 旧設定からの移行 | 旧 `%APPDATA%\discord-to-vr` → Tauri identifier の旧保存先 → exe 隣の旧設定の順。新保存先があれば優先し、旧ファイルを残す。読込・復号失敗時は無理に上書きしない。旧名の更新設定も通常版で引継ぐ |

IPC の受信上限は16 MiB、部分書込はタイムアウト時に切断する。RPC 待機イベントは最大128件、重複 ID は最大1000件で、接続復帰では保持するがサービスを作り直すと消える。サーバー・チャンネル情報は有界・期限付きキャッシュで、取得失敗に待ち時間がある。画像は許可した Discord HTTPS CDN のみ、リダイレクトなし、1秒・256 KiB・キャッシュ64件に制限する。

**診断の「送信完了」は WebSocket 書込成功であり、VR 上に描画された応答ではない。** また設定画面を開く間は接続サービスを停止し、保存／キャンセルで再開する。その間の通知欠落は現在の設計上発生し得る。

セキュリティの詳細は [SECURITY](../SECURITY.md)、RPC 配布上の前提は [設計レビュー](RPC_DISTRIBUTION_REVIEW.md) を参照。自分の Client ID で動作することと、全利用者に同じ RPC scope が承認されることは別であり、この調査では Discord の最新承認条件を再調査していない。

## 配布と CI の継続上の注意

| ワークフロー | 起動と役割 |
| --- | --- |
| `check.yml` | push / PR の Windows チェック。整形、フロントエンド、Node テスト、Core・通常デスクトップのテスト / Clippy / デバッグビルド |
| `release.yml` | `release` ブランチ push はビルド成果物のみ。`v*` タグで依存監査・テスト・署名・Defender 検査・GitHub Release・更新フィード配信 |
| `microsoft-store.yml` | `release` ブランチ push はビルドのみ。`v*` / `store-v*` タグは公開有効変数と認証設定に従い Store 申請。手動 build、読み取り専用 check-connection、既存タグの retry-submission に対応 |

- `store-v*` は通常 Release workflow を起動しない。共通の `v*` は両方の配布処理を起動し得るため、タグ作成を単なる記録として行わない。
- `MICROSOFT_STORE_PUBLISH_ENABLED=true` が申請条件。現在、タグ時の `--validate-release` 自体も true を要求するため、変数を無効にしたタグ実行は単なる build-only ではなく検証で失敗する。ブランチ push / 手動 build と区別する。
- 更新フィードの橋渡し版はすでに **0.2.2**。後続の通常 Release は setup と標準ソースのみ、`latest.json` は `updates` ブランチへ。橋渡し版の再実行だけ Release にも残す。`migration.json` や公開済みタグを安易に作り直さない。
- Release 文は `docs/release-notes/` の該当バージョンに、前回からの変更点だけを書く。Store 専用タグの文書は `store-vX.Y.Z.md`。
- Tauri identifier **`io.github.yumiiiiiiiiii.discord-to-vr`**、Store identity **`NakYmi.DiscordtoVR`** と `store/identity.json` の Publisher を維持する。表示名の変更と識別情報の変更を混同しない。
- 既存の updater 署名鍵との互換性を維持する。署名鍵を再生成・表示・同梱しない。Updater の署名、Authenticode、Store による署名は別の仕組み。
- Store は検証済みの x64 MSIX を申請し、認定過程で Microsoft が署名する。ローカル生成の未署名アップロード用 MSIX を、そのまま一般配布用インストーラーとみなさない。
- Store 申請は公開済み掲載情報・画像等を引継ぎ、x64 パッケージを置換し、他アーキテクチャを保持する。未処理申請・下書きがある場合は停止する。API 受付成功と認定・公開・利用者端末での更新完了は別々に確認する。

手順の詳細は [自動更新](AUTO_UPDATE.md)、[Store 配布](MICROSOFT_STORE.md)、[BOOTH 配布](BOOTH.md) を参照する。ただし次節の情報差分に注意する。

## 未解決項目と確認された情報差分

### 確認済みの不一致・不足

| 場所 | 状況と次の対応候補 |
| --- | --- |
| `docs/MICROSOFT_STORE.md` | 0.2.21 は申請受付・公開未確認までの記録。今回 Partner Center で公開済みを確認した。これは過去の記録の更新待ちで、申請し直す理由にはならない |
| BOOTH 公開説明・`docs/BOOTH.md` | 実際の無料ダウンロード名は 0.2.2 だが、説明に 0.2.1 の ZIP・旧 exe 名・旧 Release URL が残る。外部ページ変更は今回行っていない |
| `README.md` | 対象を 0.2.2 と記載しつつ、Store 版の更新メニュー削除後の説明もある。通常版 0.2.2 / Store 0.2.21 の対象範囲を明示する余地がある |
| `docs/RPC_DISTRIBUTION_REVIEW.md`・`IMPROVEMENTS.md` | `GET_CHANNEL` / `GET_GUILD` を使わない・詳細を追加取得しないという説明が古い。現在の `notifier.rs` は通知文脈を取得する。`SECURITY.md` の説明とコードを優先する |
| `docs/AUTO_UPDATE.md`・`docs/DEVELOPMENT.md`・`IMPROVEMENTS.md` | 更新フィード移行を「次の配布」で実施する説明が残る。現在は bridgeVersion 0.2.2 として実施済み |
| 過去のテスト記録 | `CHANGELOG.md` 等の 40 / 8 件などは当時の結果。今回の結果は次節に記載。過去の未検証記録や利用者の成功報告を今回の E2E 成功として扱わない |
| ローカル GNU ビルド環境 | `build-local.ps1` の既定コンパイラー保存先は `%TEMP%\discord-vr-portable-compiler`。今回その場所には必要ファイルが揃わず、既定のままではビルドできない。利用可能なものは `target/security-analysis-tools/compiler` にあり、必要なら `-CompilerRoot` を指定する |

GitHub の open Issues / PR は初回調査の API 読取時点でいずれも 0 件。ただし、未登録の不具合がないことは意味しない。初回調査では確定した実行時バグは再現していない。後続調査では通知音パスと無音固定のテスト通知の問題を確認した（末尾参照）。

### コード上で気になった点（未修正・影響未実測）

- `src-tauri/src/app.rs` の `AppState::new` は `--privacy` / `--normal` の保存結果を `let _ = ...` で捨てる。現在の起動中の表示モードには反映するが、保存失敗を知らせず次回起動に引き継げない可能性がある。通常の画面操作による切替ではエラーを返す実装との違いがある。
- `frontend/main.tsx`、`updates.tsx`、`diagnostics.tsx` の状態取得は1秒間隔。トレイ収納による visibility を条件に止める処理は見当たらない。不要な呼出や CPU 使用量の有無は実測していない。
- `notifier.rs` と `app.rs`、`frontend/main.tsx` が大きく、認証・サービス寿命・設定画面の変更は影響範囲が広い。分割は改善候補だが、今回の目的ではリファクタリングしない。
- 通常 feature のバイナリーを MSIX に入れた場合、実行時判定で UpdateManager の登録を省く一方、更新 commands のコンパイル登録自体は残る。通常 UI は呼ばない。正式 Store スクリプトは `microsoft-store` feature で commands も除外するため、現在の正式配布経路の不具合と断定しない。
- 通知の期限切れ・キューあふれ・切断・設定中の停止は、欠落を許容する現在の設計。重複除外もサービス再生成をまたいだ永続化はない。改善する場合は遅延通知の再送や個人情報保持とのトレードオフを先に確認する。

### 実装途中・今後の候補

未コミットの作業途中コードや、新機能の未完成を示す確定的な差分は見つからなかった。[改善候補](../IMPROVEMENTS.md) のグローバルショートカット、一時ミュート、連続通知のまとめ表示、秘密情報を含まない診断エクスポート、ログアウト、通知音の試聴、通常版の Authenticode 署名は、完了機能と区別する。今回これらの実装を開始していない。

## ビルド・テスト結果

2026-11-04、上記 HEAD のソースで実行。フロントエンドを先にビルドし、Rust は **Windows GNU / LLVM-MinGW** を使用した。Node は **26.12.1**、npm は **12.2.1**。CI の Node 22 / MSVC と完全に同じ環境ではない。

| 検証 | 今回の結果 |
| --- | --- |
| TypeScript + Vite (`npm.cmd run build`) | 成功 |
| 配布・更新・Store の Node テスト5ファイル | **30件成功** |
| Core (`cargo test --locked`) | **42件成功** |
| 通常 desktop (`custom-protocol`) | **9件成功** |
| Store desktop (`custom-protocol,microsoft-store`) | **4件成功** |
| rustfmt（両 manifest） | 成功 |
| Clippy（Core・通常 desktop・Store desktop、all-targets / 警告エラー扱い） | すべて成功 |
| 通常 desktop・Store desktop のデバッグビルド | 両方成功 |
| 署名付き release build・MSIX 包装・Defender・依存関係監査 | 今回は再実行していない。既存 Store workflow [41705416566](https://github.com/Yumiiiiiiiiii/DiToVR/actions/runs/41705416566) と Windows checks [41705416561](https://github.com/Yumiiiiiiiiii/DiToVR/actions/runs/41705416561) の成功を読み取り確認 |
| Discord → 実 XSOverlay、Bot / DND、Store からの更新、実インストール・アンインストール | 今回は未実施。ビルド・模擬 IPC / WebSocket テスト・Store 公開確認から実機成功を推定しない |

標準環境は Node 22 LTS、Rust stable MSVC、Visual Studio Build Tools の C++ / Windows SDK、WebView2。今回この端末の既定 PATH では `link.exe` が利用できず、GNU に切り替えた。実設定を読む通常アプリの起動は行っていない。

### 再検証用コマンド

Windows のビルド環境を整えたプロジェクトルートで実行する。新しい環境ではまず `npm.cmd ci`。既存の lockfile を更新しない。失敗した場合はその時点で止め、後続の成功で上書きしない。

```powershell
npm.cmd run build
node --test scripts/release-manifest.test.mjs scripts/publish-update-feed.test.mjs scripts/store-manifest.test.mjs scripts/publish-store.test.mjs scripts/check-store-connection.test.mjs
cargo fmt --check
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --locked
cargo test --manifest-path src-tauri/Cargo.toml --locked --features custom-protocol
cargo test --manifest-path src-tauri/Cargo.toml --locked --features custom-protocol,microsoft-store
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets --features custom-protocol -- -D warnings
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets --features custom-protocol,microsoft-store -- -D warnings
cargo build --manifest-path src-tauri/Cargo.toml --locked --features custom-protocol
cargo build --manifest-path src-tauri/Cargo.toml --locked --features custom-protocol,microsoft-store
```

この端末で使った GNU 環境の設定例（その PowerShell プロセス限り）。パス・ファイル存在は実行前に確認する。

```powershell
$diRepo = (Get-Location).Path
$diCompiler = Join-Path $diRepo 'target/security-analysis-tools/compiler'
$diCompilerBin = Join-Path $diCompiler 'llvm-mingw-20261006-msvcrt-x86_64/bin'
$env:RUSTUP_TOOLCHAIN = 'stable-x86_64-pc-windows-gnu'
$env:PATH = "$diCompilerBin;$env:PATH"
$env:CARGO_BUILD_TARGET = $null
$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = Join-Path $diCompilerBin 'x86_64-w64-mingw32-clang.exe'
$env:CC_x86_64_pc_windows_gnu = $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER
$env:RUSTFLAGS = $null
$env:CARGO_ENCODED_RUSTFLAGS = '-L' + [char]31 + 'native=' + (Join-Path $diCompiler 'gcc-runtime')
$env:CARGO_TARGET_DIR = Join-Path $diRepo 'target/tauri'
```

署名付き配布ビルドは [開発ガイド](DEVELOPMENT.md)、Store ビルドは [Store ガイド](MICROSOFT_STORE.md) の専用手順を使う。上のテスト環境設定だけでは配布用署名や MSIX を作成したことにはならない。

## 次回セッションで最初に確認すること

1. [AGENTS.md](../AGENTS.md) と本書を読み、今回の依頼範囲を確認する。新機能の内容や公開許可を過去の一般的な要望から推測しない。
2. `git status`、現在ブランチ・HEAD・未追跡ファイル・`git diff` を再確認する。今回作った文書も未コミットなので、他の作業と一緒に破棄しない。
3. 変更対象のコード・テストと関連ガイドを読み直す。機能の存在、模擬テスト成功、実機成功、公開済みを別々に記録する。
4. 配布作業を依頼された場合だけ、GitHub のタグ・Release・`updates`、Partner Center の公開版と処理中申請、BOOTH の実ファイルと説明を再確認する。再申請・タグ再作成を先に行わない。
5. 次のバージョンを決める場合は、Store 0.2.21 より増加すること、npm / Tauri / desktop Cargo / lockfile / release notes の整合を確認する。
6. 必要最小限の変更後に関連するチェックを行い、実機検証の未実施項目を明示する。実ユーザーの認証情報や設定には無断で触れない。

次の優先候補は、配布版ごとの説明の整合と、Discord 通常／DND・許可リスト・Bot 不明時・配信用切替・XSOverlay 再接続・Store 更新の実機確認。修正や外部公開は具体的な依頼が出てから行う。

更新時は、確認日・基準コミット・実際に実行した検証と限界を追記し、古くなった「現在の状態」を置き換える。秘密情報、個人の会話、アカウントのメールアドレス等は保存しない。

## 通知音修正と画面変更の保留（13:15時点の記録）

2026-11-04 **13:15 JST** 確認の追加開発。ブランチ `develop`、基準 HEAD は引き続き `c544cb3a8dff3e850b1e58aaf55aae63c0be8ab8`。作業開始時の `docs/DEVELOPMENT.md` の未コミット差分、および未追跡の `AGENTS.md`・本書を保持した。`git ls-remote --heads origin develop release updates` は午前の記録と一致。配布先の公開状態は今回再照会していない。

ユーザーの依頼は、通知音が鳴らない問題の修正、配信用表示の削除、可能なら通知音のみモードの追加。後二者の画面変更については、Better Design MCP が利用できないため問い合わせ、ユーザーが **「Better Design MCPを接続してから画面を変更する」** を選択した。画面・トレイ・チュートリアルの変更はまだ行っておらず、配信用表示も現時点では残る。

### 音声処理の修正

- `src/vr/mod.rs`: 独自音声の絶対パスを `file:///` URL に変換して XSOverlay に送る。日本語・空白・`#`・`%` をエスケープする。設定ファイルには元のパスを維持する。標準音名と無音は従来どおり。
- `src-tauri/src/app.rs`: 「テスト通知」で保存した通知音・音量・表示時間を使う。従来は常に `""` / `0.0` / `5.0` を送っていた。選んだ音の音量が0%の場合やファイルを読めない場合は理由を返す。「無音」の表示テストは維持。
- 保存時に Ogg Vorbis（`.ogg`）以外の形式や読めないファイルを拒否し、変換・選び直しを案内する。旧設定に未対応形式が残る場合は、音を無効にして問題をログに知らせ、通知本文まで破棄しない。既存の設定・認証情報は変更していない。
- `docs/UI_GUIDE.md`: Ogg Vorbis・音量・テスト通知の操作説明を追加。

根拠は [公式 WebSocket 通知API](https://xsoverlay.vercel.app/Developer/API/websockets/apicommands#sendnotification) と、この PC の Steam 配下 `XSOverlay_Beta` の読み取り調査。`_Shared/api.js` の `AudioClipObject` / `PlayAudio`、通知UIの `PlayNotificationAudio`、`Assembly-CSharp.dll` の `AudioManager.PlaySound` を照合した。このビルドは `UnityWebRequestMultimedia.GetAudioClip(path, AudioType.OGGVORBIS)` を使い、渡されたパスに `file://` を補わない。ファイルURL化が必要と判断した。インストール済み XSOverlay のコードは変更していない。

### 追加開発の検証結果

前節の GNU / LLVM-MinGW 環境で実行。フロントエンドを先にビルドし、desktop を検証した。アプリのバージョンは0.2.21のまま。commit・push・配布・ユーザーの実設定の変更はしていない。

| 検証 | 結果 |
| --- | --- |
| `npm.cmd run build` | 成功（画面ソースは未変更） |
| Core `cargo test --locked` | **44件成功**。追加した模擬WebSocketテストで日本語・空白・記号入りパスのURL往復、送信音量・表示時間を確認。標準音・無音・音量0%・旧未対応形式の通知維持も確認 |
| 通常desktop `custom-protocol` | **10件成功**。テスト通知の0%・未対応形式・削除済みファイルのエラー、標準音・無音のキュー投入を確認 |
| Store desktop `custom-protocol,microsoft-store` | **5件成功** |
| rustfmt（両manifest）、`git diff --check` | 成功 |
| Clippy（Core・通常desktop・Store desktop、all-targets / `-D warnings`） | すべて成功 |
| 通常desktop・Store desktop のデバッグビルド | 両方成功 |
| 実際の XSOverlay 音声再生・Discordからの実通知・VR表示 | **未検証**。模擬WebSocketへの送信成功は音が鳴った証明ではない |

新規の製品依存関係・lockfile変更はない。読み取り調査の補助ツールは ignored の `target/audio-analysis/` 内のみ。

### 続行待ちの作業（13:15時点、後述で更新）

1. Better Design MCP の接続復旧。設定には `https://better-design.com/api/mcp` が存在するが、セッションのツール一覧に公開されていない。プラグイン検索でも該当なし。設定済み接続先への初期化は Cloudflare HTTP 403 / Error 1010 / `browser_signature_banned` / `retryable: false` だったため再試行・回避をしていない。
2. 必須の原則取得・レビューを実施できるようになってから、ホーム・設定・トレイ・チュートリアル・起動引数・関連文書から配信用表示を削除する。旧設定項目の扱いを明確にし、認証情報・未知の設定は保持する。
3. 「通常表示／通知音のみ」の選択を追加する。インストール済み XSOverlay の `PlayAudio` には `audioPath`・`volume`・`spatial` 等の引数が存在するため実装可能と判断したが、まだ実装も再生確認もしていない。キュー・受信条件・期限・診断を既存経路と揃える。
4. 音声ファイル選択ダイアログのフィルターはまだ `*.wav;*.ogg;*.mp3` のまま。画面変更再開時に `.ogg` に揃え、選択時の音量0%への案内と試聴の導線も確認する。保存時の未対応形式の拒否は今回実装済み。
5. 修正済みバイナリーで実際の XSOverlay 再生を確認する。Ogg Vorbisの独自音・標準音・無音・日本語パス・音量を確認し、UI変更後に通知音のみと通常表示も確認する。実機での音声出力は現時点で未検証。

## MP3/WAV対応と通知音のみの内部実装

2026-11-04 **13:44 JST** 確認。ブランチ `develop`、基準 HEAD `c544cb3a8dff3e850b1e58aaf55aae63c0be8ab8` は変わらない。以下は未コミットの最新コードとローカル検証の記録であり、13:15時点のOgg限定・依存追加なし・通知音のみ未実装という記述を更新する。公開状態は再照会・変更していない。

### 実装済みの内部処理

- `src/audio.rs`: MP3・WAVをSymphoniaで読み、vorbis_rsでXSOverlay対応のOgg Vorbisへ変換する。FFmpegの追加インストールは不要。保存設定と元ファイルは元のまま、実行時の設定コピーにだけ変換先を渡す。Oggは実際にVorbisとしてデコードできるか検証する。
- 変換結果は設定フォルダー内の `sound-cache/ditovr-audio-v1-<SHA256>.ogg` に原子的に保存。内容が変わると再変換し、破損キャッシュも再生成する。生成名に一致するファイルのみ最大8件に整理し、利用者の別ファイルには触れない。入力16 MiB・120秒・変換15秒の制限。MP3/WAVは8～192 kHz・モノラルまたはステレオに対応する。
- desktopの保存・起動・テスト通知で、変換を `spawn_blocking` に移して使う。読み込み失敗は理由を返すか状態に残し、通常表示の通知本文まで捨てない。実ユーザーの設定は今回書き換えていない。
- `NOTIFICATION_SOUND_ONLY`（既定false）、設定DTO・status・`set_sound_only` commandを追加。通知音未選択・音量0では有効にできない。設定保存はDPAPI・未知項目・OAuthトークンを維持する。
- `src/vr/mod.rs`: 通知音のみでは `PlayAudio` の `audioPath`・`volume`・`spatial` だけを送る。本文・名前・アイコンは送らず、接続時の表示通知やアイコン取得も抑制する。送信待ち通知にも送信直前のモードを適用し、受信条件・期限・有界キュー・診断を維持する。
- 通知音のみでの標準音は、実行中 `XSOverlay.exe` の配置から同製品の音声ファイルを探して使う。インストール済み製品のファイルは変更しない。実機での標準音の探索・再生は未検証。
- 受信診断では変換・ファイル読み取り・音声再生を行わず、MP3/WAVの元パスを含めて設定形式と受信条件を検証する。診断のValidatedはデコード・送信・再生の成功を意味しない。
- 共通Cargo manifestと両lockfileに音声用依存を追加。一括依存更新はしていない。`THIRD_PARTY_AUDIO_NOTICES.txt` にライセンスとSymphoniaのソース取得先を記載し、通常Tauri bundleとStore梱包の明示リストへ追加した。テスト用MP3は自作の正弦波で、個人の音声は含まない。

### 最新の検証結果

| 検証 | 結果 |
| --- | --- |
| `npm.cmd run build` | 成功。`frontend/` の画面ソースは未変更 |
| Core `cargo test --locked` | **50件成功**。実MP3/WAVの変換・デコード・原本保持・破損キャッシュ復旧、PlayAudioの模擬WebSocket送信、待機中モード切替、期限・受信条件、設定保全を確認 |
| 通常desktop / Store desktop | **10件 / 5件成功** |
| 配布・更新・StoreのNodeテスト5ファイル | **31件成功**。音声ライセンス同梱と余分な設定ファイルを梱包しないことを追加確認 |
| rustfmt（両manifest）、`git diff --check` | 成功 |
| Clippy（Core・通常desktop・Store desktop、all-targets / `-D warnings`） | 成功 |
| 通常desktop・Store desktopのデバッグビルド | 両方成功。配布用署名・MSIX・公開は行っていない |
| XSOverlayの実音声出力・Discord実通知・VR表示 | **未検証** |

起動済みの旧テスト版が `target/tauri/debug/WebView2Loader.dll` を使用しており、同じ出力先でのdesktop再ビルドはos error 32で失敗した。設定画面の未保存項目を保持するため終了していない。別出力先のGNUリンクは初回に `-lgcc` / `-lgcc_eh` の探索で失敗し、`LIBRARY_PATH` に既存の `gcc-runtime` を追加して解消。その後Dドライブの空き容量不足でStoreリンクが失敗したため、最終検証先をCドライブへ移した。一時ファイル削除を含むコマンドは実行ポリシーで拒否され、削除は実行されていない。D側の生成済み検証キャッシュは残る。

最終検証は既述のGNU環境に以下を追加・変更して実行した（そのPowerShellプロセス限り）。

```powershell
$env:LIBRARY_PATH = Join-Path $diCompiler 'gcc-runtime'
$env:CARGO_TARGET_DIR = Join-Path $env:TEMP 'ditovr-audio-check-20261104'
$env:CARGO_PROFILE_DEV_DEBUG = '0'
$env:CARGO_INCREMENTAL = '0'
```

更新版の通常バイナリーは `C:\Users\Ymii\AppData\Local\Temp\ditovr-audio-check-20261104\debug\DiToVR.exe`。同じフォルダーにWebView2Loaderと音声ライセンス文書がある。**更新版は未起動**。現在起動中なのは以前の `D:\test\discxss\target\tauri\debug\DiToVR.exe` で、今回のMP3変換・通知音のみ内部処理は含まない。二重起動は既存ウィンドウへ転送されるため、利用者が編集中の設定を保存・終了してから更新版を起動する。

### 再開待ちの作業

1. Better Design MCPの最新の接続確認は `OAuth tokens cannot be refreshed; authorization required`。過去のCloudflareエラーよりこの認証エラーを優先する。ユーザーに `codex.cmd mcp login better-design`、必要なら `codex.cmd mcp logout better-design` 後の再loginを案内済み。`.cmd` を使うのは既定の `codex.ps1` が実行ポリシーで停止するためで、ポリシーは変更しない。認証はユーザー自身がブラウザーで行う。
2. ユーザー指定の「Better Design MCPを接続してから画面を変更する」を維持。MCPの必須原則・レビューは未取得。**配信用表示はまだ画面・トレイ・起動引数・コードに残り、通知音のみの選択UIもまだない。** 内部実装完了と利用者向け機能の完成を区別する。
3. 接続復旧後、既存テーマで原則取得→配信用表示削除と通常／通知音のみの選択→MCPレビュー・描画DOMのspacing確認を行う。旧 `PRIVACY_MODE` と未知の設定・認証情報を保全する。音声選択フィルターはMP3/WAV/Ogg対応のままでよい。`frontend/bridge.ts`・ホーム・設定・初回案内・トレイ・起動引数・関連文書を揃える。
4. 利用者の設定編集を保持したうえで更新版を起動し、XSOverlayでMP3・WAV・Ogg・標準音・音量・日本語パス、通常表示と通知音のみの実動作を確認する。現状の自動テストを実音声出力の成功として扱わない。

## 画面変更の完了と更新版の起動

2026-11-04 **22:03 JST** 確認。ブランチ `develop`、基準 HEAD `c544cb3a8dff3e850b1e58aaf55aae63c0be8ab8`、アプリ0.2.21のまま。以下は未コミットの最新コード・ローカル検証に基づき、13:44時点の「UI未実装・認証待ち・更新版未起動」を更新する。配布先の公開状態は今回再照会・変更していない。

### 完了した変更

- Better Design MCPの再認証が成功。既存テーマを維持し、`find-design-system`、`get-ui-principle`、`get-ux-principle`、既存CSSの抽出、`get-review-rules`、3画面の `check-comprehension` と描画DOMの `inspect-spacing` を実施した。新しいデザインシステムの導入や公開デプロイは行っていない。
- ホーム・設定・初回案内・トレイを「通常表示／通知音のみ」に統一した。共通のネイティブラジオボタンを `frontend/notification-mode.tsx` に追加。音が未設定の場合、ホームから音設定へ進み、保存・キャンセルで確定する。音だけの場合、VRに通知を表示しないこととXSOverlayへの接続が必要なことを明記した。
- 配信用表示の分岐・DTO・Tauri command・起動引数を削除。`--sound-only` / `--normal` が現在の指定。旧 `PRIVACY_MODE` は未知のJSON項目として保存時も保持し、実行時には無視する。認証情報・その他の未知項目は従来どおり保全する。
- 音設定にMP3・WAV・Ogg Vorbis対応と制限を表示。初回案内で音だけを選ぶと、未設定の場合は標準音・30%を用意する。既存ユーザーの音を勝手に変更しない。
- 通知音のみへの切替は音声検証・保存の成功後に画面とトレイへ反映する。保存失敗時の起動モード指定も問題として残す。受信条件・キュー・期限・診断は維持した。
- README・操作ガイド・セキュリティ・プライバシー文書とBOOTH／Storeのローカル原稿を更新。未公開の変更と従来版スクリーンショットであることを明記した。外部掲載・commit・push・公開・Store提出は行っていない。

### 検証とレビュー

| 検証 | 結果 |
| --- | --- |
| フロントエンド `npm.cmd run build` | 最終ソースで成功 |
| Core / 通常desktop / Store desktop | **50件 / 10件 / 5件成功** |
| 配布・更新・StoreのNodeテスト5ファイル | **31件成功** |
| Clippy（Core・通常desktop・Store desktop、all-targets / `-D warnings`） | すべて成功 |
| rustfmt（両manifest）、`git diff --check` | 成功 |
| 通常desktop・Store desktopのデバッグビルド | 最終UIで両方成功。最後に通常版をビルド |
| ブラウザープレビュー | 未設定時の音設定への遷移、無音／0%での保存エラー、ダミーMP3選択・27%で保存、キーボードのモード切替、キャンセルで既存値維持、初回案内、ライト／ダークを確認。コンソールの警告・エラーなし |
| Better Design文言レビュー | ホーム・音設定・初回モード選択の3画面とも指摘0 |
| 描画DOMのspacing | 390・820・1440・1728幅を計測。製品の最小幅は820。変更部分を820・1440・1728で再計測し、重大指摘はホーム0・音設定0・初回選択0。音量スライダーと自動終了スイッチの操作範囲、狭い幅でのモード選択の折返しを修正 |
| 実Windowsアプリ | 更新版を起動。Discord接続済み、新モード表示、既存受信設定の引継ぎを確認。XSOverlayは未接続 |
| XSOverlayでの実音声出力・Discord実通知・VR表示 | **未検証**。起動・Discord接続や模擬WebSocketテストだけでは再生成功を意味しない |

spacingの全体走査には、既存サイドバー末尾の自動余白・2行のモード表示とアイコンの位置・既存デスクトップ専用レイアウトの390幅への指摘が含まれた。ソースと描画を照合し、意図した配置・対象外の幅として分離した。新しいラジオの余白は自動marginに依存しない配置へ修正した。残るmoderateには既存の間隔、ボタン高、クリック可能なlabel全体を考慮しないラジオ単体の寸法がある。全画面に自動レビュー合格点が付いたという記録ではない。

### 起動中の更新版と残確認

- 更新版: `C:\Users\Ymii\AppData\Local\Temp\ditovr-audio-check-20261104\debug\DiToVR.exe`。同じフォルダーにWebView2Loaderと音声ライセンス文書がある。確認時PID **7036**。現在はホームで待機している。
- ユーザーからStore版の終了完了の返答を受けて起動した。Computer Useの `launch_app` は明示exeパスでもStore版を選んだため、その起動直後のホームから終了し、PowerShell `Start-Process -FilePath` で上記exeを直接起動した。実行中プロセスの絶対パスと新しい画面を照合済み。Store版のファイルは変更していない。
- 利用者の実設定を手動変更していない。XSOverlay・SteamVRは今回起動していない。アプリの設定で音を選び1%以上にして保存し、XSOverlay接続後に「テスト通知を送る」で試せる。
- 次の確認は、実XSOverlayでのMP3・WAV・Ogg・標準音、日本語パス、通常表示／通知音のみ、音量、実Discord受信。署名済み配布物・MSIX・インストーラー・Store更新の検証も今回の対象外。
- 検証用Viteサーバーとダミープレビューのブラウザータブは終了済み。未コミット変更と未追跡ファイルは保持している。前節のDドライブの検証キャッシュも削除していない。

## v0.3.0 公開準備

2026-11-04 **22:23 JST** 確認。作業ブランチ `develop`、準備開始時HEAD `c544cb3a8dff3e850b1e58aaf55aae63c0be8ab8`。ユーザーの「vは0.3にして公開して」に基づき、通常版・Store版を **0.3.0**（MSIXは0.3.0.0）に統一して公開する。Coreの独立した0.1.0は維持する。既存差分と引継ぎ・開発ルールも保持する。

- `origin` と公開APIを再確認。GitHub最新はv0.2.2、`release` は80e4a90、`develop` はc544cb3で、両ブランチのツリーは同一だった。v0.3.0タグは存在しない。
- [読み取り専用Store確認](https://github.com/Yumiiiiiiiiii/DiToVR/actions/runs/41836844831)が成功。公開済み申請は `1152921505702308182`、x64は0.2.21.0。認証・既存掲載・無料価格・申請待ちなしを確認した。0.3.0.0は既存より大きい。
- BOOTHの公開配布は0.2.2、説明文は0.2.1を記載していた。掲載原稿を0.3.0へ修正。公開ファイル・説明の差し替え結果は別途記録する。
- バージョン整合検証、frontend build、配布Nodeテスト31件、Core50件、通常desktop10件、Store desktop5件は0.3.0のソースで成功。npm本番依存と両Cargo.lockの監査は既知の脆弱性によるエラーなし。cargo-auditは既存の保守終了・unsound・yanked警告6件を報告したが監査は成功。依存の一括更新はしていない。
- 起動中テスト版のDLLロックを避け、C側 `ditovr-release-check-0.3.0` に検証出力をコピーしdesktopテストを実行した。ユーザーの実設定・起動中アプリは変更していない。
- 変更点は `docs/release-notes/v0.3.0.md` とCHANGELOGに記載。READMEの未公開表記を更新した。旧画面画像であることは明記を維持した。
- 署名済みインストーラー・MSIXのMSVCビルド、Defender検査、GitHub公開、更新フィード更新、Store申請の完了はこの時点では未確認。タグのActions結果と実際の公開・受付を照合して追記する。XSOverlay実再生・Store端末更新・実Discord通知の転送は未検証。

## v0.3.0公開結果

2026-11-04 **22:38 JST** の公開確認。ユーザーの明示指示に基づき実装差分・既存の開発文書をコミットし、既存のrelease履歴をマージした。初回調査以降の「未コミット」「未公開」の記録は当時の履歴で、最新状態は本節と冒頭の表を優先する。

- 実装コミット: `73a5b5f`。公開タグ **v0.3.0** のコミット: **`7d553a15bb33b32bf414d64c2ab02c744d24bd4b`**。元の差分を破棄・stash・resetしていない。公開結果の文書コミットを含めてdevelopとreleaseを同期する。
- [GitHub Release](https://github.com/Yumiiiiiiiiii/DiToVR/releases/tag/v0.3.0)は22:34:27 JSTに公開済み。添付は `DiToVR_0.3.0_x64-setup.exe` のみで、更新用latest.jsonを新Releaseへ再添付していない。旧タグ・過去Release・予約済みidentity・既存署名鍵は変更していない。
- [通常版CI](https://github.com/Yumiiiiiiiiii/DiToVR/actions/runs/41837556896)、[Windowsチェック](https://github.com/Yumiiiiiiiiii/DiToVR/actions/runs/41837557199)、[Store CI](https://github.com/Yumiiiiiiiiii/DiToVR/actions/runs/41837557341)はすべて成功。同一コミットの重複したdevelopチェックのみ実行中にキャンセルした。通常版の依存監査・テスト・MSVC配布ビルド・インストーラーフック/名称移行検証・Defender・公開・更新フィード更新、Storeのテスト・MSVCビルド・MakeAppx・Defender・申請が成功した。
- `updates` のHEADは `93117b2`、latest.jsonは0.3.0。移行元bridgeVersionは0.2.2のまま。公開インストーラーを取得し、アプリに組み込まれた公開鍵でEd25519署名・trusted comment・0.3.0へのバージョン結合を検証した。旧リポジトリURLを使う更新URLの互換性も維持している。実際のアプリ内更新インストールは未検証。
- [BOOTH](https://yuuumiii.booth.pm/items/8621228)は22:37 JSTに無料版・応援用を同一の0.3.0 ZIPへ差し替え、説明文を更新して公開保存した。ファイルIDは10078568。無料0円・応援200円、既存画像・タグ等は維持した。公開ページからZIPを再取得し、アップロードしたZIPとの完全一致・内包インストーラーのGitHubとのSHA-256一致を確認した。旧0.2.2のファイルは保存したまま関連付けを外した。
- Storeは `DiToVR_0.3.0.0_x64.msix` の申請 **1152921505702314667** が22:34:48 JSTに **PreProcessing** として受付済み。公開完了とは区別する。既存の掲載文・スクリーンショット・価格・審査メモは自動で引き継ぎ、今回の変更点はrelease notesへ反映した。`store/LISTING.ja.md`の機能説明原稿が外部本文へ自動反映されたという意味ではない。処理中の申請を重複作成しない。

### 配布物の照合

| 対象 | SHA-256 / 検査 |
| --- | --- |
| 通常版setup | `b1fa728437a0779ec6e01c062f2bd15ae940c648c092398fb50a9a3e261a0d24`。GitHub asset digest・Defenderレポート・実ダウンロード・BOOTH内包exeが一致 |
| BOOTH ZIP | `3027a1f0d6d4dda52e7ee3ed274aeac55a37ff5ee49c9b06fbad8934952e0046`。内包はsetup 1件のみ |
| Store MSIX | `7227b0852a3b529b64abd184ba30d1df88fac41d4169b1d7df109f5962900f52`。取得した成果物を検査レポート・実manifest・同梱音声ライセンスと照合 |
| Defender | 通常版: 定義1.463.122.0、本体・setupの検出なし。Store: 定義1.463.121.0、本体・MSIXの検出なし。Windows AuthenticodeはNotSignedで、更新用署名とは別 |

ローカルの成果物・受付ログ・検査レポート・検証JSONは `%TEMP%\ditovr-release-v0.3.0\`。BOOTHアップロード用ZIPはignoredの `target/release-v0.3.0/`、公開再ダウンロードはignoredの `.playwright-mcp/DiToVR-0-3-0-x64-setup.zip` にもある。秘密情報や実ユーザー設定は成果物・Gitへ追加していない。Dドライブの空き容量は少なく、今後もビルド出力をC側へ分ける。

### 残る実機・外部確認

1. Store申請の認定・公開状態とStore経由の端末更新。申請の受付成功を公開成功と扱わない。
2. XSOverlayでのMP3/WAV/Ogg・標準音・日本語パス・音量・通知音のみの実音声出力、実Discord受信のVR転送。今回のコードテスト・配布検査では音声を実際に聴けたことにならない。
3. 実際の通常版インストール・アプリ内自動更新・Storeとの切替。今回利用者の起動中アプリや実設定は変更していない。
4. README・操作ガイド・外部掲載画像の撮り直しと、Storeの既存掲載本文の新機能説明への反映は未実施。画像が従来版であることはローカルREADME/ガイドに明記した。機能追加・公開済みファイルの差し替えと区別する。
