# Microsoft Store 配布（開発者向け）

Microsoft が署名・ホストする MSIX を使用します。GitHub の NSIS 版と公開済みタグは維持します。Store の署名・審査は Defender の誤検知が絶対に起きないことを保証しません。

## 登録した製品

- Store ID: `9NFDCGL30V3K`
- [Partner Center](https://partner.microsoft.com/dashboard/products/9NFDCGL30V3K/overview)
- [公開後のページ](https://apps.microsoft.com/detail/9NFDCGL30V3K)（審査通過までは未公開）
- [パッケージ識別情報](../store/identity.json)

2026-10-24 に Partner Center の製品 ID 画面で確認した識別情報です。秘密鍵ではありません。Store の発行者表示名は登録された `NakYmi` です。NSIS 版の発行名とは別で、任意の名前や GUID に変更できません。

## ビルド

Windows、Node.js、Rust、C++ ビルド環境、Windows SDK の MakeAppx が必要です。

```powershell
npm.cmd ci
npm.cmd run desktop:build:store
```

この環境のローカル GNU ツールチェーンを使う場合:

```powershell
npm.cmd run desktop:build:store -- -CompilerRoot 'D:/test/discxss/target/security-analysis-tools/compiler'
```

`target/store/<ビルドID>/DiToVR_0.2.1.0_x64.msix` を生成します。3桁バージョンに `.0` を付けます。更新には前回より大きいバージョンが必要です。Tauri、npm、デスクトップ Cargo のバージョンを合わせて更新します。

Store 用 feature を有効にして NSIS を作らず、実行ファイル、ライセンス、アイコン、ビルドに必要な Microsoft の WebView2Loader.dll だけを新しい作業フォルダーへコピーします。設定・秘密鍵は含めません。SDK のマニフェスト検証を省略せず、実行ファイル、DLL、MSIX を Defender で検査します。検査できない場合や検出時は失敗します。

**未署名 MSIX は Store アップロード専用**です。Microsoft が認定後に署名するため、購入した証明書や Tauri 更新署名鍵は不要です。この MSIX を一般利用者に直接配布しないでください。

## GitHub Actions で更新を自動申請する

[Microsoft Store workflow](../.github/workflows/microsoft-store.yml) は以下のタイミングで動作します。

| 操作 | 実行内容 |
| --- | --- |
| `release` ブランチへの push | Store 用 MSIX のビルド・テスト・Defender 検査・Actions 成果物への保存 |
| `vX.Y.Z` タグへの push | 上記に加え、有効化済みの場合に Store への更新申請 |
| Actions の「Run workflow」 | ビルド・検査のみ。手動実行からは申請しません |

ユーザーが選択した運用は **バージョンタグでの自動申請**です。初回の Store 公開と認証連携が済むまでは、タグでもビルドのみ実行します。MSIX は Actions の成果物として保存し、GitHub Releases の添付ファイル構成は変更しません。

### 一度だけ行う設定

1. 下記「申請の進捗と残りの操作」に従い、初回の申請・認定・公開を Partner Center で完了します。自動化は公開済みの申請を複製する方式のため、初回公開前には使用できません。
2. Partner Center のアカウント設定で Microsoft Entra ID テナントを関連付けます。認証済みの Microsoft アカウントとは別に、アプリ用の認証連携が必要です。[公式の前提条件](https://learn.microsoft.com/en-us/windows/uwp/monetize/create-and-manage-submissions-using-windows-store-services#prerequisites) と [GitHub Actions の連携手順](https://learn.microsoft.com/en-us/windows/apps/publish/msstore-dev-cli/github-actions) を参照してください。
3. Entra のアプリ登録でテナント ID、アプリケーションのクライアント ID、クライアントシークレットを用意し、Partner Center のユーザー管理からこの Entra アプリケーションに Manager 権限を割り当てます。シークレットは「シークレット ID」ではなく作成時の**値**を使用します。
4. [リポジトリの Actions Secrets](https://github.com/Yumiiiiiiiiii/DiToVR/settings/secrets/actions) に次の3件を登録します。値をチャット、ソースコード、Actions のログに貼り付けないでください。

| Secret 名 | 内容 |
| --- | --- |
| `AZURE_AD_TENANT_ID` | 関連付けた Entra テナントの ID |
| `AZURE_AD_APPLICATION_CLIENT_ID` | Partner Center に登録したアプリケーションのクライアント ID |
| `AZURE_AD_APPLICATION_SECRET` | そのアプリケーションのクライアントシークレットの値 |

この実装は Microsoft Store Submission API を直接使用するため、CLI 用の `SELLER_ID` は不要です。シークレットには有効期限があるので、期限前に更新します。

5. Store の価格が無料であることを確認します。この製品は無料配布専用として、API が基本価格を `Base` と返す場合も、更新時には明示的な `Free` を指定します。有料価格や有料の市場別価格がある場合は停止します。将来有料化するときは、この自動申請処理の変更が必要です。
6. 初回公開・Store からの動作確認・認証設定が済んだら、[Actions Variables](https://github.com/Yumiiiiiiiiii/DiToVR/settings/variables/actions) に `MICROSOFT_STORE_PUBLISH_ENABLED` を作成し、値を `true` にします。申請を停止するときは `false` に変更します。

### 次回以降の更新

1. Tauri、`package.json`、デスクトップ Cargo のバージョンを、Store の全公開済みパッケージより大きくします。例: `0.2.1` → `0.2.2`。MSIX のバージョンは `0.2.2.0` になります。
2. `docs/release-notes/v0.2.2.md` に、前回からの変更点だけを記載します。Store の「このバージョンの最新情報」にも同じ内容を使います。Store の上限に合わせて空白除去後 1,500 文字以内にしてください。
3. コードと変更点を反映した `v0.2.2` タグを push します。タグとプロジェクトのバージョンが一致しなければ失敗します。
4. Actions が Store 版テスト、MSIX ビルド、SDK 検証、Defender 検査を行い、検査レポートの SHA-256 と実際の MSIX・内包マニフェストを確認します。その後、公開済みの掲載文・画像・審査メモを引き継いで x64 パッケージを置き換え、認定へ送信します。他のアーキテクチャのパッケージは保持します。
5. Actions のサマリーで申請 ID と受付状態を確認します。**受付成功は公開完了ではありません**。Microsoft の処理・認定が通ると自動公開され、その後の利用者への更新配信は Store が管理します。

既存の下書き・審査中の申請がある場合、バージョンの再利用・巻き戻し、識別情報の不一致、検査失敗時には自動申請を停止します。下書きを自動削除しません。アップロードや commit の途中で失敗・タイムアウトした場合は、ログの申請 ID を使って Partner Center の状態を先に確認してください。同じタグの再実行で未完了の申請を上書きすることはありません。

API が作成した申請を Partner Center の画面で編集すると、その申請を API から commit できなくなる場合があります。自動処理中の手動編集は避け、失敗時は状態を確認したうえで Partner Center で対応してください。認証情報、アクセストークン、アップロード用 SAS URL は成果物に保存しません。

## 動作と検証

- Store feature または Windows パッケージ ID がある場合、GitHub 更新の確認・取得・NSIS 起動・更新設定の書き込みを拒否します。パッケージ ID 確認の予期しないエラー時も GitHub 更新を無効にします。
- 更新画面には Microsoft Store の「ライブラリ」を案内します。自動更新は Store 側で管理します。
- `runFullTrust` は Discord のローカル RPC、XSOverlay、トレイ、ファイル選択、DPAPI に必要です。管理者権限は要求しません。
- Windows 10 build 19041 以降、x64 向け。Microsoft Edge WebView2 Evergreen Runtime が必要です。未導入の場合は Microsoft 公式 Runtime を先に導入します。
- 設定先は `%APPDATA%\DiToVR` です。MSIX の AppData 仮想化によりパッケージのユーザーデータ領域に保存される場合があります。GitHub 版との設定共有・移行は保証せず、初回案内から設定します。アンインストールは Windows のアプリ管理から行います。

[登録文と審査手順](../store/LISTING.ja.md)、[プライバシーポリシー](PRIVACY.md) を用意しています。掲載用の4画像は `target/store-upload/images/` にある、ダミー情報のプレビューです。無料・PC 向けで申請します。プライバシーポリシーは Partner Center の本文入力で登録済みのため、別途公開 URL を用意する必要はありません。

Store 経由のインストール、二重起動、RPC、XSOverlay、更新、アンインストールは実機確認が必要です。自動化の追加だけで Actions や新規バージョンタグを実行・作成することはありません。

2026-10-24 のローカル確認では、本番ビルド、SDK MakeAppx の検証、通常版のデスクトップテスト9件、Store feature 付きテスト10件、配布スクリプトのテスト9件が成功しました。Defender 定義 `1.461.179.0` で本体・Microsoft のローダー DLL・MSIX の検出はありませんでした。Store 経由のインストールと実際の Store 更新は未検証です。

アップロード用のコピーと画像は `target/store-upload/` にあります。これはビルド成果物なので Git の管理対象外です。未署名 MSIX の SHA-256 は `3fe9ee41dfb5224585428f16cdd0154349b0518a2c88e90a3cc972e5796045b5` です。

自動申請スクリプトは Microsoft API を模したテストで、申請状態・掲載情報の保護、バージョン判定、アップロード前後の失敗、秘密情報のログ抑制を検証します。実際の Store API への接続と更新申請は、初回公開・Secrets 設定が済むまで未検証です。

## 申請の進捗と残りの操作

2026-10-24 に日本語の説明文、短い説明、検索キーワード7件、プライバシーポリシー本文、サポート先、審査担当者向けのテスト手順を保存しました。価格は 0 円、全市場・一般公開・Store 内で検索可能な設定です。年齢区分アンケートは、通知表示用のユーティリティとして回答し、利用者本人の成人確認と IARC 使用条件への同意を得て保存しました。評価 ID は申請前のため「保留中」です。

ブラウザーのファイル選択操作がタイムアウトし、Windows のネイティブ操作接続も利用できなかったため、**パッケージと掲載画像はまだアップロードされていません**。認定への送信・公開も未実施です。次の操作が残っています。

1. [パッケージ](https://partner.microsoft.com/ja-jp/dashboard/products/9NFDCGL30V3K/submissions/1152921505702221008/packages) のファイル選択で `D:\test\discxss\target\store-upload\DiToVR_0.2.1.0_x64.msix` を指定します。検証結果と制限付き機能 `runFullTrust` の理由欄を確認し、デバイスは Windows 10/11 Desktop を対象として保存します。
2. [日本語の Store 登録情報](https://partner.microsoft.com/ja-jp/dashboard/products/9NFDCGL30V3K/submissions/1152921505702221008/listings?languageid=17&languagecode=ja-jp) のデスクトップ用スクリーンショットに、`D:\test\discxss\target\store-upload\images` の `01-home.png`、`02-guide.png`、`03-whitelist.png`、`04-updates.png` を指定して保存します。画像以外の検査資料は添付しません。
3. 概要画面の未完了・エラーを解消し、「送信して認定を受ける」を実行します。追加の規約への法的同意が表示された場合は、その時点で利用者本人に確認します。申請後のステータスで受け付けられたことを確認します。

認定と公開が済んだ後、Store から実際にインストールし、起動、Discord 受信、XSOverlay 送信、トレイからの復帰、二重起動の防止、更新、アンインストールを確認してください。

## 公式資料

- [Tauri を MSIX にする](https://learn.microsoft.com/en-us/windows/apps/dev-tools/winapp-cli/guides/tauri)
- [配布方法と署名](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/choose-distribution-path)
- [MSIX の AppData](https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-behind-the-scenes)
- [GitHub Actions から Store を更新する](https://learn.microsoft.com/en-us/windows/apps/publish/msstore-dev-cli/github-actions)
- [Microsoft Store Submission API の設定](https://learn.microsoft.com/en-us/windows/uwp/monetize/create-and-manage-submissions-using-windows-store-services)
- [申請の作成・更新・送信](https://learn.microsoft.com/en-us/windows/uwp/monetize/manage-app-submissions)
