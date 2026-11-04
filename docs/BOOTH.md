# BOOTH 商品ページ

掲載先: https://yuuumiii.booth.pm/items/8621228

## 商品名

【無料】DiToVR｜Discordの通知をVRへ【XSOverlay連携】

## 商品紹介文

VRの中でも、Discordの連絡を確認。

DiToVR は、Discord デスクトップアプリの通知を XSOverlay に届ける、無料の Windows アプリです。ヘッドセットを外さずに、DM やメンションの通知を確認できます。

■ ダウンロード・インストール（無料）

このページの「無料ダウンロード」から Windows 用インストーラーを入手できます。
DiToVR_0.3.0_x64-setup.zip を展開し、中の DiToVR_0.3.0_x64-setup.exe を実行してください。

BOOTH 版は GitHub で公開済みの v0.3.0 と同じインストーラーです。無料版・応援用ともに同じファイルです。

Microsoft Store からも無料でインストールできます。Store 版の更新は Microsoft Store が管理します。
https://apps.microsoft.com/detail/9NFDCGL30V3K

■ 3つのできること

1. Discord の通知を VR 内に表示
DM やメンションを確認できます。サーバーからの通知には、サーバー名とチャンネル名も表示します。

2. 必要な会話だけを受信
チャンネル・DM 単位で「許可リスト」に登録できます。「取り込み中」でも必要な連絡だけ受け取りたいときは、「許可リストだけ」に切り替えて使えます。最大32件まで登録でき、登録先の閲覧・読み取り権限が必要です。このモードでは、ミュートした会話も受信します。

3. 表示するか、通知音だけで受け取るか選択
「通知音のみ」では、VR に通知を表示せず、音だけでお知らせします。XSOverlay が必要です。MP3・WAV・Ogg Vorbis の通知音を選べます。切り替える前に送信・表示された通知は消せません。

受信モードは手動で切り替えられます。Discord のステータスに合わせた自動切り替えを使う場合のみ、自分の Bot を追加設定します。ライト・ダーク・Windows に合わせるテーマや、トレイ常駐にも対応しています。

■ はじめ方

1. BOOTH の ZIP を展開して setup.exe を実行するか、Microsoft Store から DiToVR をインストールします。
2. Discord・SteamVR・XSOverlay を起動し、XSOverlay の通知用 WebSocket を有効にします。
3. DiToVR の初回チュートリアルに沿って接続し、テスト通知を確認します。

初回に、自分の Discord アカウントで Developer Portal のアプリを作成し、Client ID・Client Secret を設定する必要があります。作成手順はアプリ内で案内します。Discord のパスワードやアカウントトークンは入力しません。手動切り替えは Bot なしで利用できます。

■ 必要な環境

・Windows 10（build 19041 以降）/ 11 の64bit版
・Discord デスクトップアプリ
・SteamVR
・XSOverlay（別売り）
・Microsoft Edge WebView2 Runtime
・インターネット接続

Quest などのヘッドセット単体では動作しません。Discord のブラウザー版には対応していません。通常の通知受信は Discord 側の通知設定・ステータス・権限に依存します。

■ 無料版と応援用について

機能に違いはありません。アプリは無料で利用できます。応援用は開発への支援をしたい方向けで、追加機能や個別対応をお約束するものではありません。

■ 認証情報と生成 AI について

認証情報は、この Windows ユーザーにひも付けて暗号化保存します。会話や認証情報を開発者のサーバーへ収集する機能はありません。

コードの作成・修正、UI 設計、説明文・紹介画像の構成には生成 AI を使用しています。会話を生成 AI に送信する機能はありません。掲載画像はダミー情報を使ったアプリ画面のプレビューです。

本ソフトは非公式アプリです。Discord・XSOverlay・SteamVR の各提供元による公式製品や提携製品ではありません。

■ 詳しい使い方・ソースコード・お問い合わせ

操作ガイド
https://github.com/Yumiiiiiiiiii/DiToVR/blob/release/docs/UI_GUIDE.md

GitHub
https://github.com/Yumiiiiiiiiii/DiToVR

不具合報告・機能の要望
https://github.com/Yumiiiiiiiiii/DiToVR/issues

報告の際は、Client Secret・トークン・実際の会話内容を載せないでください。

## タグ

ツール / Windows / VR / ソフトウェア / Discord / SteamVR / XSOverlay / 通知

## 配布内容

無料: 「無料版（Windows用インストーラー）」

応援用: 「応援用（無料版と同じインストーラー）」、価格は既存の200円を維持。

2026-11-04 **22:38 JST**、無料版・応援用の両方を `DiToVR_0.3.0_x64-setup.zip` に差し替え、上記の商品紹介文を公開しました。無料は0円、応援用は200円を維持しています。旧版ファイルは提供対象から外し、保管済みファイル自体は削除していません。Store 申請用 MSIX は添付していません。

公開ページから無料版ZIPを再ダウンロードし、内容が `DiToVR_0.3.0_x64-setup.exe` 1件のみで、GitHub Releaseと同じインストーラーであることを確認しました。応援用も同じBOOTHファイルID `10078568` に関連付けています。

- インストーラー SHA-256: `b1fa728437a0779ec6e01c062f2bd15ae940c648c092398fb50a9a3e261a0d24`
- ZIP SHA-256: `3027a1f0d6d4dda52e7ee3ed274aeac55a37ff5ee49c9b06fbad8934952e0046`
- [通常版のCI](https://github.com/Yumiiiiiiiiii/DiToVR/actions/runs/41837556896)で依存監査、回帰テスト、更新用署名付きビルド、インストーラー検証、Defender検査が成功。Defender定義は `1.463.122.0`、本体・インストーラーの検出なし。公開したインストーラーの更新用署名と0.3.0へのバージョン結合を再検証済みです。Windows Authenticodeの署名とは別です。

紹介画像は表紙、通知の設定、初回設定の順。元の画面プレビューと書き出し用ファイルはローカルの `target/booth-update/` に保管します。
