import { ArrowRight, Bell, Check, Gamepad2, Info, ListFilter, MessageCircle, Volume2 } from "lucide-react";
import type { ReceiveMode, Status } from "./bridge";
import { NotificationModeChoices } from "./notification-mode";
import { Button } from "@animate-ui/react/primitives/buttons/button";

export const receiveChoices: { mode: ReceiveMode; label: string; help: string }[] = [
  { mode: "normal", label: "Discord通知に従う", help: "Discord が通知した会話を転送" },
  { mode: "whitelist", label: "許可リストだけ", help: "登録したチャンネル・DMだけ受信" },
  { mode: "auto", label: "Botで自動判別", help: "取り込み中は許可リストだけ受信" },
];
export const botLabels = {
  disabled: "監視停止中", connecting: "Bot接続中", unknown: "判定待ち", dnd: "取り込み中",
  online: "オンライン", idle: "退席中", offline: "オフライン・判定保留", error: "Token・Presence Intentを確認",
  missing_guild: "指定サーバーにBotがいません", missing_user: "指定サーバーに自分がいません",
  account_mismatch: "RPCのアカウントとユーザーIDが違います",
};
export function discordLabel(status: Status) {
  return status.paused ? "設定中・一時停止" : ["起動待ち", "認証中", "接続済み", "再接続中", "確認が必要"][status.discord] ?? "接続待ち";
}
export function overlayLabel(status: Status) {
  return status.debugEnabled ? "受信テストでは停止" : status.paused ? "設定中・一時停止" : status.overlay ? "接続済み" : "接続待ち";
}
export function ConnectionStrip({ status }: { status: Status }) {
  return <div className="connection-strip" aria-label={status.preview ? "モック接続状態" : "接続状態"}>
    <div><MessageCircle size={18}/><strong>Discord</strong><span className={status.discord === 2 && !status.paused ? "good-text" : ""}>{discordLabel(status)}</span></div>
    <ArrowRight size={14} className="connection-arrow" aria-hidden="true"/>
    <div><Gamepad2 size={18}/><strong>XSOverlay</strong><span className={status.overlay && !status.paused ? "good-text" : ""}>{overlayLabel(status)}</span></div>
  </div>;
}
export function NotificationHome({ status, busy, settings, mode, receiveMode, action, setup }: {
  status: Status; busy: boolean; settings: (section: "sound" | "whitelist" | "discord") => void;
  mode: (soundOnly: boolean) => void; receiveMode: (mode: ReceiveMode) => void;
  action: () => void; setup: () => void;
}) {
  return <div className="notification-page">
    <div className="page-heading"><div><span className="page-kicker">Discord から、あなたの VR へ</span><h1>通知</h1><p>視界に、音に。今の過ごし方に合わせて。</p></div><Bell className="page-symbol" size={30} strokeWidth={1.5}/></div>
    <ConnectionStrip status={status}/>
    {(!status.configured || status.onboardingPending) && <div className="setup-prompt"><div><strong>最初の接続を整えましょう</strong><p>準備から認証まで、順番にご案内します。</p></div><button className="primary" disabled={busy} onClick={setup}>接続を設定<ArrowRight size={16}/></button></div>}
    <section className="home-section" aria-labelledby="delivery-title">
      <div className="home-section-heading"><div><h2 id="delivery-title">通知の届け方</h2><p>切り替えると保存され、次回も同じモードで起動します。</p></div><span className="save-hint"><Check size={13}/>自動保存</span></div>
      <NotificationModeChoices name="home-mode" value={status.soundOnly} onChange={mode} disabled={busy || !status.configured}/>
      <div className="audio-summary"><Volume2 size={16}/><span>{status.soundReady ? "保存した通知音と音量を使います。" : "音を鳴らすには、通知音と音量を設定してください。"}</span><button disabled={busy} onClick={()=>settings("sound")}>音を設定<ArrowRight size={14}/></button></div>
      <p className="inline-note"><Info size={14}/>通知音のみでも XSOverlay への接続が必要です。</p>
    </section>
    <section className="home-section receive-section" aria-labelledby="receive-title">
      <div className="home-section-heading"><div><h2 id="receive-title">受信する通知</h2><p>取り込み中にも、必要な会話を。</p></div><button className="text-button" disabled={busy} onClick={()=>settings("whitelist")}><ListFilter size={15}/>許可リスト</button></div>
      <div className="receive-options">{receiveChoices.map(choice=><button key={choice.mode} className={`receive-option ${status.receiveMode===choice.mode?"selected":""}`} aria-pressed={status.receiveMode===choice.mode} disabled={busy || !status.configured || (choice.mode==="auto" && !status.botConfigured)} onClick={()=>receiveMode(choice.mode)}><span className="radio-check">{status.receiveMode===choice.mode && <Check size={11}/>}</span><span><strong>{choice.label}</strong><small>{choice.help}</small></span></button>)}</div>
      <p className="receive-summary">{status.whitelistOnly ? `許可リスト ${status.whitelistCount} 件だけ受信` : "Discord が通知した会話を受信"}{status.receiveMode==="auto" && ` · ${botLabels[status.botStatus]}`}</p>
      {status.receiveMode==="auto" && status.whitelistOnly && status.botStatus!=="dnd" && <p className="inline-note">判定できるまで許可リストだけ受信します。Bot と自分が同じサーバーにいること、Presence Intent を確認してください。</p>}
      {status.whitelistCount>0 && status.whitelistStatus!==2 && <p className="inline-note">{status.whitelistStatus===3 ? "許可リストの一部を購読できません。ID・閲覧権限・messages.read の承認を確認してください。" : "許可リストの購読待ちです。Discord の承認画面を確認してください。"}</p>}
    </section>
    <div className="test-row"><div><strong>{status.soundOnly ? "通知音を確かめる" : "届き方を確かめる"}</strong><p>{status.preview ? "デモ操作です。実際の通知・音声は送信しません。" : status.debugEnabled ? "VR のテストは -debug を付けずに起動してください。" : status.overlay ? "会話を含まないテスト通知を送ります。" : "XSOverlay に接続するとテストできます。"}</p></div><Button className="primary" hoverScale={1} tapScale={0.98} disabled={busy || status.paused || !status.overlay || status.debugEnabled} onClick={action}><Bell size={16}/>{status.preview?"通知テスト（デモ）":"テスト通知を送る"}</Button></div>
    <p className="workspace-note"><Info size={14}/>{status.tray ? "×・Esc でトレイへ。通知の転送はそのまま続きます。" : "トレイを利用できません。画面を開いたまま使用してください。"}</p>
  </div>;
}
