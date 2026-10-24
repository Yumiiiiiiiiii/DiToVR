# Windows Defender の検出調査

2026-10-24 に公開済み 0.2.1 の検出を調査しました。SmartScreen の「不明な発行元」と、Microsoft Defender が脅威として隔離する検出は別です。今回確認したのは後者です。

**最新の確認結果:** 19:32（日本時間）に保護定義を `1.461.179.0` へ更新したところ、公開済み 0.2.1 の setup・本体と、今回ビルドした修正版の setup・本体はすべて検出なしでした。本体の Microsoft 再審査申請も受け付けられましたが、アナリストの最終判断はまだ保留中です。今回の申請によって検出が解除されたと確認できたわけではありません。

## 初回に確認したこと

| 対象 | Defender の検出名 | SHA-256 |
| --- | --- | --- |
| `Discord.to.VR_0.2.1_x64-setup.exe` | `Trojan:Win32/Bearfoos.B!ml` | `fae1fb8ddcf811fc05fa25fdad860ae459858d73549a3a7669604c48bce7985d` |
| setup に含まれる `discord_To_VR.exe` | `Trojan:Win32/Bearfoos.A!ml` | `4e12d1a05afbe76e97f08de30884e0a0689f4bdef79b8222c5eeae82fe93eae3` |

- setup のハッシュは GitHub Release が返す公開 asset の SHA-256 と一致しました。インストール済み本体も、setup を実行せずに展開した本体と一致しました。
- setup と本体は Windows の Authenticode では `NotSigned` です。Tauri の自動更新用署名は別の仕組みで、Windows 用のコード署名にはなりません。
- Windows の検出・検疫イベント（1116 / 1117）を確認しました。定義 `1.461.176.0`、エンジン `1.1.26090.9` の診断スキャンでも両方の検出を再現しました。
- setup は NSIS 3 Unicode / LZMA のアーカイブです。UPX による実行ファイル圧縮は確認していません。
- ZIP に含まれる本体のハッシュは `cac504217e60d479eb277791cf71661ed72be07e597e2a8aee40f130fa0c9db4` です。インストール版との差は `.rdata` の3バイトだけで、Tauri の配布形式識別子 `__TAURI_BUNDLE_TYPE_VAR_UNK` → `__TAURI_BUNDLE_TYPE_VAR_NSS` に対応します。`.text` を含む他のセクションは一致しました。この差は改ざんの根拠にはなりません。

確認したコードでは、Discord の名前付きパイプ、公式 OAuth / Bot Gateway / CDN、XSOverlay のローカル WebSocket、固定の GitHub 更新先を使用しています。DPAPI の復号対象は利用者が本アプリに保存した認証情報です。SteamVR の終了監視にはプロセス名の列挙を使います。外部プロセスへのコード注入や Windows の保護設定を無効化する処理は見つかりませんでした。

この調査だけで誤検知や安全性を断定することはできません。Defender が判定に使った特徴は公開ログには出ません。未署名、Rust の最適化、インストーラー圧縮などのどれが決め手かは未特定です。署名を付けるだけで脅威検出が必ず消えるとも限りません。

## 修正版のローカル検査結果

依存関係の修正後、既存のローカル GNU ビルド手順でアプリと NSIS setup をビルドしました。2026-10-24 19:23〜19:25（日本時間）、エンジン `1.1.26090.9`・定義 `1.461.176.0` で、次のファイルはいずれも検出なし（終了コード 0）でした。

| 対象 | SHA-256 |
| --- | --- |
| ビルドした本体 | `7729e6d4d0ad72388e5700501ef0f042d2d5ee2cacf9ca18ab34278e8f41e8e1` |
| 修正版 setup | `c003b5d3278b85b903a220a9ce55e53ee549b6807cbe8efdef59f11f412faee6` |
| 修正版 setup から実行せずに展開した本体 | `3bfc283db9d11d7011db8bb922d70236eec7a4975cbbb534363629e210da3d0c` |

ビルドした本体と setup 内の本体の差は、Tauri の配布形式識別子 `UNK` → `NSS` の3バイトのみでした。すべて Windows の Authenticode では `NotSigned` です。更新用署名は既存の鍵で生成し、署名不正やバージョン再利用を拒否する既存テストも通りました。コア 42 件・デスクトップ 8 件・更新配布関連 7 件のテスト、ビルド、アンインストール設定の3ケース、workflow の actionlint を確認しています。

比較のため同じ定義で公開済み旧版も再検査したところ、setup は引き続き `Bearfoos.B!ml` で検出されました。一方、旧版本体はハッシュが変わっていないにもかかわらず、19:25 の追試では検出なしに変わりました。判定が変わった理由は未確認です。このため修正版の検出なしという結果だけで、特定のコード変更が検出を解消したとは断定しません。

Windows の保護設定や除外リストは変更していません。ローカル検証版は `target/tauri/release/bundle/nsis/Discord to VR_0.2.1_x64-setup.exe` にあります。まだバージョンを上げておらず、公開済み 0.2.1 の差し替え・新規公開・GitHub Actions の実行は行っていません。次の正式版も、実際に公開するビルドを改めて検査する必要があります。

その後、`Update-MpSignature` で通常の保護定義更新を行い、19:32 に定義 `1.461.179.0`（エンジンは同じ）で再検査しました。公開済み旧版の setup・本体、修正版の setup・setup 内の本体の4ファイルがすべて検出なしになりました。ハッシュはいずれも変わっていません。ローカルの診断結果は `target/release-security-latest-definitions.json` に保存しています。現時点では旧版 setup の追加申請は行っていません。再発する場合は、その時点の定義・検出名・対象ハッシュを添えて個別に申請します。

## 配布前の対策

安定版タグのワークフローに、公開前の Defender 定義更新とスキャンを追加しました。定義更新の失敗・検出・スキャン失敗の場合は公開を止めます。検査はビルドした本体と setup の両方に行い、定義バージョン・ハッシュ・署名状態・終了コードを Actions の成果物に保存します。

`scripts/check-release-security.ps1` は診断用のカスタムスキャンを使います。検査対象のファイルを修復・隔離せず、除外設定を無視してアーカイブ内も検査します。通常の保護設定や除外リストは変更しません。Defender を利用できない環境でも、検査済みとして公開を進めません。この検査はその時点の定義と環境での結果であり、すべての PC での判定を保証するものではありません。

本体の Microsoft 再審査申請は受け付けられており、最終判断を待ちます。次の配布物でも確認が必要です。Windows 用コード署名の導入は発行元の検証に有効ですが、別途証明書または署名サービスが必要です。既存の Tauri 更新用秘密鍵で代用したり、自己署名証明書で一般利用者の信頼を得たりはできません。

## 依存関係の安全性改善

2026-10-24 の RustSec データベースを使用し、公式の `cargo-audit 0.22.2` で両方の lockfile を検査しました。監査ツール自体は公式配布 ZIP の SHA-256 を確認してから実行しています。監査で見つかった問題を修正するため、次の依存関係を更新しました。

| 依存関係 | 修正前（コア / デスクトップ） | 修正後 |
| --- | --- | --- |
| `rustls` | 0.23.42 / 0.23.45 | 0.23.46 |
| `rustls-webpki` | 0.103.13 / 0.103.15 | 0.103.16 |
| `webpki-roots` | 1.0.8 / 1.0.9 | 1.0.11 |
| `webpki-root-certs` | デスクトップ 1.0.9 | 1.0.11 |
| `tokio` | 1.52.3 / 1.53.2 | 1.54.1 |
| `quinn-proto` | コア 0.11.16 | 0.11.19 |
| `chacha20` | コア 0.10.1（yank 済み） | 0.10.2 |

このアプリの更新先は署名付き NSIS setup に限定しているため、updater の不要な ZIP 展開機能も無効にし、`zip 4.6.1` を依存関係から除外しました。更新ファイルの署名検証、TLS、システムプロキシ対応は維持しています。

修正後の脆弱性件数は、コアの lockfile が 11 → 0、デスクトップが 10 → 0 です。これは機能が無効なもの・別 OS 向けのもの・重複するアドバイザリも含む lockfile 全体の件数で、Windows 実行ファイルに同数の悪用可能な問題があったという意味ではありません。npm の監査も開発依存関係を含めて 0 件でした。Linux 向けの `glib` の安全性警告と `proc-macro-error` / `zvariant` 系の保守警告は残っていますが、Windows 向け依存関係ツリーに含まれないことを確認しました。これらを監査の除外リストで隠す変更は行っていません。

安定版タグの Actions にも npm と両方の Rust lockfile の監査を追加し、既知の脆弱性が報告された場合は公開を止めます。依存関係の改善と Defender の誤検知判定は別の問題であり、今回の検出原因がこれらだったとは確認できていません。

## Microsoft への申請用メモ

申請先: <https://www.microsoft.com/en-us/wdsi/filesubmission>

ソフトウェア開発者として、setup と setup 内の本体をそれぞれ「誤って検出されたファイル」の再審査対象にしてください。

2026-10-24 に送信許可を得て、Microsoft のフォームへ旧版本体の検出名・定義バージョン・説明を入力しました。ブラウザー連携のファイル URL アクセス権限がないため、利用者がファイル選択と画像認証・最終送信を手動で行い、19:28:34 に申請が受け付けられたことを確認しました。申請番号と結果ページはローカルの `target/microsoft-submission-status.json` に記録しています。認証情報や利用者の設定ファイルは提出していません。

受付画面では「提出済み」「最終決定: 保留中」です。自動検査欄はクラウド・クライアントとも「マルウェアは検出されませんでした」、クライアント定義は `1.461.179.0` と表示されています。この自動検査表示と、アナリストが誤検知と認定することは区別してください。

```text
Product: Discord to VR 0.2.1
Source: https://github.com/Yumiiiiiiiiii/discord_To_VR
Release: https://github.com/Yumiiiiiiiiii/discord_To_VR/releases/tag/v0.2.1

Please review a suspected false-positive detection of our Windows application.
The NSIS installer is detected as Trojan:Win32/Bearfoos.B!ml.
The executable inside that installer is detected as Trojan:Win32/Bearfoos.A!ml.
Installer SHA-256: fae1fb8ddcf811fc05fa25fdad860ae459858d73549a3a7669604c48bce7985d
Executable SHA-256: 4e12d1a05afbe76e97f08de30884e0a0689f4bdef79b8222c5eeae82fe93eae3

The application uses Tauri 2, Rust, React and Microsoft Edge WebView2.
It forwards Discord notifications through the Discord desktop RPC interface to
XSOverlay on localhost. Optional Discord Bot access only monitors presence.
It stores the user's app credentials with Windows DPAPI, monitors SteamVR
process names to exit with VR, and supports signed updates from our GitHub repo.
The files are currently not Authenticode-signed.

Please analyze the files and advise whether these detections are false positives
or whether specific unsafe behavior needs to be corrected.
```

認証情報や利用者の `config.json` を提出物に含めないでください。新しいコード署名用の秘密鍵や、既存の更新用秘密鍵も提出しません。

## 参考資料

- [Microsoft Defender コマンドラインの仕様](https://learn.microsoft.com/en-us/defender-endpoint/command-line-arguments-microsoft-defender-antivirus)
- [Microsoft の誤検知・見逃しへの対応](https://learn.microsoft.com/en-us/defender-endpoint/defender-endpoint-false-positives-negatives)
- [Tauri の Windows コード署名](https://v2.tauri.app/distribute/sign/windows/)
- [Tauri 側の類似報告](https://github.com/tauri-apps/tauri/issues/2486) — 類似例であり、本アプリの検出が誤検知である証明ではありません。

Computer Use は初期化後の `list_apps` が `native pipe unavailable (os error 2)` で失敗し、再試行でも同じ結果でした。Windows セキュリティ画面は操作せず、読み取り専用の Defender 履歴・イベントログ、公開ハッシュ、静的なファイル解析と診断スキャンを使用しました。インストーラーは実行していません。
