import { ArrowRight, Gamepad2, MessageCircle } from "lucide-react";
import type { ReceiveMode, Status } from "./bridge";

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
