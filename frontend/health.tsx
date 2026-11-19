import { Activity, ArrowRight, Check, Info, ShieldCheck } from "lucide-react";
import type { Status } from "./bridge";
import { botLabels, discordLabel, overlayLabel } from "./notification-home";

export function HealthPanel({status, onSetup, onDebug, busy}: {status: Status; onSetup:()=>void; onDebug:()=>void; busy: boolean}) {
  const checks = [
    {title:"Discord", value:discordLabel(status), help:status.paused ? "設定を保存するか閉じると、受信を再開します。" : status.discord===2 ? "デスクトップアプリから通知を受信する準備ができています。" : status.discord===4 ? "Client ID・Secret・Redirect URI と承認状態を確認してください。" : status.discord===1 ? "Discord に表示される承認画面を確認してください。" : "Discord デスクトップアプリを起動し、接続状態を確認してください。", ok:status.discord===2&&!status.paused},
    {title:"XSOverlay", value:overlayLabel(status), help:status.debugEnabled ? "受信テスト専用で起動中です。VR へは送信しません。" : status.overlay&&!status.paused ? "通知画面からテスト通知を送って、届き方を確認できます。" : "SteamVR と XSOverlay を起動し、通知用 WebSocket を有効にしてください。", ok:status.overlay&&!status.paused},
    {title:"通知音", value:status.soundReady?"設定済み":"無音・未設定", help:"通知音のみモードには、通知音と1%以上の音量が必要です。音は XSOverlay 側で再生します。", ok:status.soundReady},
    {title:"常駐", value:status.tray?"トレイを利用できます":"トレイを利用できません", help:status.tray?"×・Esc で画面を閉じても転送を続けます。「アプリを終了」で停止します。":"画面を開いたまま使用してください。", ok:status.tray},
  ];
  return <div className="health-page"><div className="page-heading"><div><span className="page-kicker">接続と常駐の状態</span><h1>診断</h1><p>通知が届かないときは、ここから確認できます。</p></div><Activity size={30} className="page-symbol"/></div>
    {status.preview&&<p className="inline-note"><Info size={16}/>以下は画面確認用のモック状態です。実接続の診断結果ではありません。</p>}
    <div className="health-checks">{checks.map(check=><section key={check.title}><div className={`health-check-icon ${check.ok?"good-text":""}`}>{check.ok?<Check size={19}/>:<Info size={19}/>}</div><div><h2>{check.title}</h2><p>{check.help}</p></div><strong className={check.ok?"good-text":""}>{check.value}</strong></section>)}</div>
    <div className="health-detail"><h2>受信の判定</h2><dl><div><dt>受信モード</dt><dd>{{normal:"Discord通知に従う",whitelist:"許可リストだけ",auto:"Botで自動判別"}[status.receiveMode]}</dd></div><div><dt>許可リスト</dt><dd>{status.whitelistCount} 件</dd></div><div><dt>Bot</dt><dd>{botLabels[status.botStatus]}</dd></div></dl></div>
    <div className="health-actions"><button className="secondary" disabled={busy} onClick={onSetup}>接続設定を確認<ArrowRight size={15}/></button>{status.debugEnabled&&<button className="secondary" disabled={busy} onClick={onDebug}>受信テストを開く<ArrowRight size={15}/></button>}</div>
    <p className="inline-note"><ShieldCheck size={17}/>本文・名前・ID・認証情報をこの診断画面に表示しません。</p>
    {!status.debugEnabled&&<p className="inline-note">詳しい受信履歴は、アプリを終了して -debug を付けて起動すると確認できます。受信テスト中は VR に送信しません。</p>}
  </div>;
}
