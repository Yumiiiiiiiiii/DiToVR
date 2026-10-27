# Microsoft Store 配布（開発者向け）

Microsoft が署名・ホストする MSIX を使用します。GitHub の NSIS 版と公開済みタグは維持します。Store の署名・審査は Defender の誤検知が絶対に起きないことを保証しません。

## 登録した製品

- Store ID: `9NFDCGL30V3K`
- [Partner Center](https://partner.microsoft.com/dashboard/products/9NFDCGL30V3K/overview)
- [公開中の Store ページ](https://apps.microsoft.com/detail/9NFDCGL30V3K)
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
| `vX.Y.Z` タグへの push | 有効化済みの場合、接続確認に成功してからビルド・検査・Store 更新申請 |
| 「Run workflow」→ `build` | ビルド・検査のみ。手動実行からは申請しません |
| 「Run workflow」→ `check-connection` | 認証と公開済み製品を読み取り専用で確認。ビルドも更新申請もしません |

ユーザーが選択した運用は **バージョンタグでの自動申請**です。初回公開は完了しています。認証連携後に自動申請を有効化します。MSIX は Actions の成果物として保存し、GitHub Releases の添付ファイル構成は変更しません。

### 一度だけ行う設定

1. 初回公開は完了しています。自動化は公開済みの申請を複製する方式です。新しい手動の下書き・審査中の申請がないことを確認します。
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
6. Actions の **Microsoft Store → Run workflow → task: `check-connection`** を実行します。認証、製品 ID、発行者、公開済みバージョン、無料価格、日本語の掲載情報を読み取り専用で確認し、申請の作成・アップロード・送信は行いません。このチェックは読取アクセスの確認です。更新申請の権限と新パッケージの妥当性は実際のタグ更新時に検証します。
7. 接続確認と Partner Center の権限設定が済んだら、[Actions Variables](https://github.com/Yumiiiiiiiiii/DiToVR/settings/variables/actions) に `MICROSOFT_STORE_PUBLISH_ENABLED` を作成し、値を `true` にします。申請を停止するときは `false` に変更します。有効化済みタグでは長いビルドの前に同じ接続確認を実行し、不備があれば停止します。

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

[登録文と審査手順](../store/LISTING.ja.md)、[プライバシーポリシー](PRIVACY.md) を用意しています。掲載用の4画像は `target/store-upload/DiToVR-0.2.1/images/` にある、ダミー情報のプレビューです。無料・PC 向けで公開しています。プライバシーポリシーは Partner Center の本文入力で登録済みです。

Store 経由のインストール、二重起動、RPC、XSOverlay、更新、アンインストールは実機確認が必要です。連携設定時には読み取り専用の接続確認を使用し、バージョンを変更せずに更新申請や新規バージョンタグを作ることはありません。

2026-10-24 のローカル確認では、本番ビルド、SDK MakeAppx の検証、通常版のデスクトップテスト9件、Store feature 付きテスト10件、配布スクリプトのテスト9件が成功しました。Defender 定義 `1.461.179.0` で本体・Microsoft のローダー DLL・MSIX の検出はありませんでした。Store 経由のインストールと実際の Store 更新は未検証です。

申請済みパッケージと画像は `target/store-upload/DiToVR-0.2.1/` にあります。これはビルド成果物なので Git の管理対象外です。申請した未署名 MSIX の SHA-256 は `0f16b972f6823a926ae9ef68389736e9aba11c48857037082486e41d1e4996ef` です。

自動申請スクリプトは Microsoft API を模したテストで、申請状態・掲載情報の保護、バージョン判定、アップロード前後の失敗、秘密情報のログ抑制を検証します。接続確認にも、読み取りだけを行うこと、公開済み情報に不備があると停止することのテストを用意しています。実際の Store API への接続と更新申請は、Secrets 設定後にそれぞれ確認します。

## 公開状況と残りの設定

2026-10-27 に Partner Center の製品概要で **「Microsoft Store で取り扱い中」** と「現在 Store に公開されている製品」の表示を確認しました。初回申請 ID は `1152921505702221008`、パッケージは `DiToVR_0.2.1.0_x64.msix` です。パッケージと日本語掲載画像4枚は登録済みです。初回申請をやり直す必要はありません。

同日に Entra アプリケーション「DiToVR GitHub Actions」を登録し、Store 用 Secrets 3件を GitHub に保存しました。クライアントシークレットの有効期限は **2027-10-27** です。期限前に更新し、新しい値を GitHub Secrets へ保存してください。

Partner Center へのテナント関連付けとアプリケーションへの Manager 権限付与は未完了です。職場アカウントの作成・初回サインインは完了しましたが、関連付け操作は権限エラーになりました。既存のグローバル管理者である外部 Microsoft アカウントは、「サインイン オプション → 組織にサインイン」で既存のテナントドメインを指定すると、Entra のユーザーとしてサインインできることを確認しました。新しい職場アカウントへの管理者権限追加は行っていません。

現在は Partner Center の所有者アカウントへ戻す際の、スマートフォンによる本人確認待ちです。所有者で関連付け操作を開始した後に既存の Entra 管理者で認証する経路を引き続き確認します。自動申請はまだ有効化していません。関連付けと Manager 権限の設定後に `check-connection` を実行し、接続を確認してから有効化します。

[初回の読み取り専用接続確認](https://github.com/Yumiiiiiiiiii/DiToVR/actions/runs/40468355438)では、認証トークンを取得した後の Store 製品情報取得が HTTP 400 で停止しました。Store の下書き・公開情報は変更しておらず、MSIX のビルド・更新申請も実行していません。関連付けと権限設定が済んだら同じ確認を再実行します。

Store からのインストール、起動、Discord 受信、XSOverlay 送信、トレイからの復帰、二重起動の防止、更新、アンインストールの実機確認は、公開状態の確認と別の作業です。

## 公式資料

- [Tauri を MSIX にする](https://learn.microsoft.com/en-us/windows/apps/dev-tools/winapp-cli/guides/tauri)
- [配布方法と署名](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/choose-distribution-path)
- [MSIX の AppData](https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-behind-the-scenes)
- [GitHub Actions から Store を更新する](https://learn.microsoft.com/en-us/windows/apps/publish/msstore-dev-cli/github-actions)
- [Microsoft Store Submission API の設定](https://learn.microsoft.com/en-us/windows/uwp/monetize/create-and-manage-submissions-using-windows-store-services)
- [申請の作成・更新・送信](https://learn.microsoft.com/en-us/windows/uwp/monetize/manage-app-submissions)
