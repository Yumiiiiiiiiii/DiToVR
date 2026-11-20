# 開発ガイド

Microsoft Store 用 MSIX の作成・申請は [Store 配布ガイド](MICROSOFT_STORE.md) を参照してください。

ソースからの起動・検証・署名付きビルド・更新配信を行う開発者と配布者向けの手順です。アプリを使う方は [README](../README.md) と [操作ガイド](UI_GUIDE.md) を参照してください。

## 開発環境と検証

画面と処理は Rust のネイティブアプリ（GPUI）で、通知転送・認証・暗号化は共通 Core（ルートの `ditovr-core`）が担当します。ビルドには Rust stable の MSVC ツールチェーンと、Visual Studio Build Tools の「C++ によるデスクトップ開発」・Windows SDK が必要です。配布スクリプトのテストに Node.js 22 を使います。VS Code だけでは MSVC の `link.exe` は導入されません。

| 場所 | 内容 |
| --- | --- |
| `src/`、ルートの `Cargo.toml` | 共通 Core（Discord IPC、受信判定、XSOverlay、音声、設定、診断） |
| `desktop/` | 製品本体（GPUI）。独立した Cargo manifest / lockfile。`installer/` に NSIS、`tests/fixtures/` に更新署名の検証データ |
| `scripts/`、`store/`、`.github/workflows/` | 配布・Store 申請・CI |

通常版と Microsoft Store 版は同じ `DiToVR.exe` です。`microsoft-store` feature で Store 版になり、独自の更新処理を持ちません。

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File desktop/build.ps1 -Release        # 通常版
powershell -NoProfile -ExecutionPolicy Bypass -File desktop/build.ps1 -Release -Store # Store 版
```

出力は `%LOCALAPPDATA%/DiToVRBuildCache/desktop-standard|desktop-store/release/DiToVR.exe` です。`-Check`、`-Clippy`、`-Test` も使えます。ダミー表示の確認は `DiToVR.exe --ui-preview`（実設定・外部接続なし）、実受信の診断は `--debug` です。

```powershell
cargo fmt --check
cargo fmt --manifest-path desktop/Cargo.toml --check
cargo test --locked
cargo test --manifest-path desktop/Cargo.toml --locked
cargo test --manifest-path desktop/Cargo.toml --locked --features microsoft-store
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --manifest-path desktop/Cargo.toml --locked --all-targets -- -D warnings
cargo clippy --manifest-path desktop/Cargo.toml --locked --all-targets --features microsoft-store -- -D warnings
node --test scripts/*.test.mjs
```

アンインストール時の設定削除処理は、NSIS（`makensis`）を使って隔離した場所で検証できます。

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-installer-hooks.ps1
```

この検証はワークスペースの `target` 内に作ったダミー設定だけを使います。実際のアプリ設定には触れません。

署名付き配布ビルドには更新用の秘密鍵が必要です。[ビルド・公開手順](#ローカルで署名付きビルドを作る) を参照してください。

[設定のサンプル](../config.example.json) は参照用です。GUI からの保存後は、受信中だった場合に自動で接続を再開し、設定ファイルを直接編集した場合は再起動が必要です。画面だけを試すには `DiToVR.exe --ui-preview` を使います。プレビューはダミー情報で動き、Discord・XSOverlay への接続や設定の読み書きは行いません。

`src/` は通知転送・設定保存などの共通ライブラリ、`desktop/` が画面とアプリの起動（GPUI）です。v0.3.x までの Tauri／React 構成（`src-tauri/`、`frontend/`、npm）は v0.4.0 で廃止しました。必要なときは v0.3.1 以前の Git 履歴とタグを参照してください。内部構造は [ARCHITECTURE.md](../ARCHITECTURE.md) にまとめています。

## 配布者の初回設定

安定版タグの Actions は、両方の Rust lockfile（ルートと `desktop/`）を監査し、Defender の定義を更新して、本体と setup を検査してから公開します。既知の脆弱性・検出・定義更新失敗・検査不能の場合は公開を止めます。Defender の検査結果は `ditovr-security` の成果物に保存します。これは Windows のコード署名とは別の確認で、すべての PC で検出されないことを保証しません。

Windows の発行元情報は `desktop/installer/DiToVR.nsi` の `MANUFACTURER` で `DiToVR` に設定しています。アプリ一覧の発行元とインストーラーの会社名へ反映します。この値と Windows の Authenticode 署名は別のものです。UAC・SmartScreen の認証済み発行元にはコード署名証明書が必要で、更新用の署名鍵ではその表示を変更できません。名称変更時は NSIS フックが旧名のインストールを `/UPDATE` で削除し、設定を保持して DiToVR に置き換えます。

名称変更後の保存先は `%APPDATA%\DiToVR` です。旧 `%APPDATA%\discord-to-vr`、旧 Tauri 保存先、実行ファイル隣の設定の順に引き継ぎます。旧 Tauri の identifier（`io.github.yumiiiiiiiiii.discord-to-vr`）は旧設定の移行元として読み取るだけで、書き込みません。二重起動防止の名前と Store package identity は、配布済みの値を維持します。公開済みファイル名・過去の検出記録は当時の名称を残しています。旧インストールの置き換えでは、既存のスタートメニュー・デスクトップのショートカットだけを新しい起動先へ引き継ぎます。置き換えテストは `target` 内のダミーだけを使い、実際のレジストリやインストールには触れません。

この環境では更新用鍵を生成し、公開鍵をアプリへ組み込みました。秘密鍵はリポジトリ外の次の場所に保存しています。

```text
%LOCALAPPDATA%\DiscordToVR-release-signing\updater.key
```

この鍵を安全な場所にバックアップしてください。公開済みアプリへの更新には同じ鍵が必要です。上書き生成やリポジトリへの追加、配布ファイルへの同梱、チャット・Issue への貼り付けはしないでください。

ローカルビルドは `%LOCALAPPDATA%\DiToVR-release-signing\updater.key` を優先し、なければ上記の既存鍵を使います。名称変更だけを理由に鍵を生成し直す必要はありません。

1. GitHub リポジトリの Settings → Secrets and variables → Actions で、`TAURI_SIGNING_PRIVATE_KEY` に秘密鍵ファイルの**内容**を登録します。ファイルのパスではありません。この環境の鍵はパスワードなしで生成したため、`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` は未設定で構いません。
2. アプリのバージョンは `desktop/Cargo.toml` の `version` だけを更新します（`scripts/release-manifest.mjs` がタグ・インストーラー名・署名と照合します）。ルートの Cargo は共通ライブラリなので変更不要です。
3. `docs/release-notes/v<バージョン>.md` に前回からの変更点だけを書き、検証後、同じバージョンの安定版タグ（例えば `v0.2.1`）を公開します。タグの Actions は署名付き NSIS インストーラーと更新情報を検証・生成します。GitHub Release と更新フィードの公開は、Microsoft Store での正式公開を確認した後に `release-after-store.yml` がタグ指定で `release.yml` を起動して行います。次回以降は実行用の ZIP 版と単独の `.sig` を GitHub Release に添付せず、ソースコードは GitHub 標準の「Source code (zip)」「Source code (tar.gz)」から取得します。リリース文にはその Markdown の内容を使用します。`release` ブランチだけのビルドは Actions の成果物を作り、公開更新を変更しません。

公開版の更新配信には上の設定が必要です。別のリポジトリへ移す場合は配布先の固定 URL と検証スクリプト、鍵を自分のものへ変更してください。

## 更新情報の配信先を移す

次の安定版を移行版として、Releases には setup インストーラー・ソースコード・`latest.json` を用意します。その次からは setup・ソースコードだけにし、更新情報は同じリポジトリの `updates` 専用ブランチで配信します。移行版は新しい取得先を優先し、旧 URL も予備の取得先として持ちます。GitHub 標準のソースコード ZIP / tar.gz は実行用 ZIP ではありません。

タグの Actions が、公開済みのインストーラーを確認した後で `updates` ブランチに `latest.json` と `migration.json` を書き込みます。初回は更新情報だけの独立したブランチを作成します。`migration.json` の移行版バージョンを保持し、その版の再実行時だけ Releases にも `latest.json` を添付します。その後の版では自動で添付を省くため、バージョン番号の決め打ちや手動の設定変更は不要です。

配信には Actions 標準の `GITHUB_TOKEN` と `contents: write` を使い、追加の署名鍵や外部サーバーは不要です。古い版への配信やビルド中に更新情報が変わった場合は停止し、ブランチの強制更新は行いません。配信に失敗した場合は同じタグのワークフローを再実行してください。`updates` ブランチを手動で初期化したり、`migration.json` を削除したりしないでください。

0.2.0 / 0.2.1 が移行版を飛ばして残っている場合、移行完了後は一度だけ最新の setup を手動で実行する必要があります。公開済みの 0.2.1 の添付ファイルとタグは変更しません。

## ローカルで署名付きビルドを作る

通常版のインストーラーは次のコマンドで作ります。`-Sign` を付けると、Tauri CLI の署名コマンド（`signer sign --app-version`、版を署名に結び付ける）で、既存の更新鍵（`TAURI_SIGNING_PRIVATE_KEY`、未設定なら `%LOCALAPPDATA%\DiToVR-release-signing\updater.key`）で署名します。鍵は再生成しません。

```powershell
./scripts/build-installer.ps1 -Sign
node scripts/release-manifest.mjs
```

`target/installer/DiToVR_<版>_x64-setup.exe` と `.sig`、続いて `latest.json` を作ります。署名にはバージョンが結び付き、アプリは通知された版と署名済みの版が一致しない更新を拒否します。

`latest.json` はアプリバージョン・タグ・インストーラー名・署名のバージョンが一致する場合だけ生成します。署名はこの更新情報に含まれるため、`.sig` の個別添付は不要です。ビルド時の署名生成は引き続き必要で、Actions の成果物には検証用の `.sig` と更新情報を保存します。ローカルビルドは生成までで、配信先への書き込みは行いません。公開はタグの Actions から行ってください。秘密鍵を紛失した場合、既存の公開鍵を持つアプリへ新しい鍵で自動更新することはできません。
