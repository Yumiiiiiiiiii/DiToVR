# Microsoft Store 登録文

## 短い説明

Discord の通知を XSOverlay で VR 内に表示。必要な会話の選択や、配信時の本文非表示に対応。

## 説明

VR の中でも、Discord の通知を確認できます。

Discord to VR は、Windows の Discord デスクトップアプリから受け取った通知を、同じ PC の XSOverlay に転送する無料の非公式アプリです。DM やメンションを確認したり、配信中は名前・本文・アイコンを隠して新着だけを表示したりできます。

・案内付きの初回設定
・チャンネル・DM 単位の許可リスト（最大32件）
・受信モードの手動切り替えと、自分の Bot による状態の自動判別
・ライト、ダーク、Windows に合わせるテーマ
・トレイ常駐と二重起動の防止
・Microsoft Store による更新

Discord デスクトップアプリ、XSOverlay、SteamVR 環境とインターネット接続が必要です。XSOverlay は別売りです。Microsoft Edge WebView2 Runtime が必要です。

自分で Discord Developer Portal のアプリを作成し、Client ID と Client Secret を設定して承認します。パスワードや Discord アカウントトークンは入力しません。利用できる通知は Discord 側の仕様・権限・設定に依存します。自動判別 Bot は任意です。

会話・認証情報を開発者のサーバーに収集する機能はありません。認証情報は PC 内で DPAPI により暗号化して保存します。Discord、XSOverlay、SteamVR の公式製品ではなく、各提供元の承認・提携を示すものではありません。開発に生成 AI を使用していますが、会話を生成 AI に送信する機能はありません。

掲載画像はダミー情報を使った画面プレビューです。

## キーワード

Discord, VR, XSOverlay, SteamVR, 通知, オーバーレイ, 配信

## 審査担当者向け

Discord to VR is a free, unofficial desktop notification bridge. runFullTrust is needed for Discord desktop local RPC/IPC, local XSOverlay communication, the Windows tray, native file selection and DPAPI-protected credentials. No administrator privileges, drivers, process injection or security setting changes are required. Store builds disable GitHub/NSIS updates and receive updates through Microsoft Store.

Launch from Start. Follow the first-run tutorial. For live receiving, install official Discord desktop, sign into your own test account, create your own Discord Developer Portal application with the redirect URL shown in the tutorial, enter its Client ID and Client Secret, and approve RPC authorization. RPC scope availability depends on Discord. XSOverlay (separately purchased on Steam) and SteamVR are needed for VR delivery. No shared credentials or licensed third-party accounts are provided.

--debug (or -debug) tests Discord receiving without XSOverlay or VR sending. Diagnostics contain counts and fixed outcomes, never credentials or message content. The built-in simulated receive test needs no credentials. --ui-preview shows clearly labelled dummy data without network connections or configuration writes; it is not a live connectivity test. Screenshots use this preview. Microsoft Edge WebView2 Evergreen Runtime is required.
