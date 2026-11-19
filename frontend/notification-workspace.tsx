import type { FormEvent, RefObject } from "react";
import { ArrowRight, Bell, Check, FileAudio2, FolderOpen, Headphones, Info, ListFilter, Play, ShieldCheck, Volume2 } from "lucide-react";
import { Button } from "@animate-ui/react/primitives/buttons/button";
import type { ReceiveMode, Settings, Status } from "./bridge";
import { botLabels, receiveChoices } from "./notification-home";
import { NotificationModeChoices } from "./notification-mode";

type AudioDraft = Pick<Settings, "notificationSound" | "volumePercent" | "soundOnly">;
export function NotificationWorkspace({ status, busy, draft, formRef, error, settings, mode, receiveMode, setup, edit, change, browse, save, cancel, test }: {
  status: Status; busy: boolean; draft: AudioDraft | null;
  formRef: RefObject<HTMLFormElement | null>; error: string | null;
  settings: (section: "sound" | "whitelist" | "discord") => void;
  mode: (value: boolean) => void; receiveMode: (value: ReceiveMode) => void;
  setup: () => void; edit: () => void; browse: () => void;
  change: (key: "notificationSound" | "volumePercent", value: string | number) => void;
  save: (event: FormEvent) => void; cancel: () => void; test: () => void;
}) {
  const sound = draft?.notificationSound ?? status.notificationSound;
  const volume = draft?.volumePercent ?? status.volumePercent;
  const source = sound === "" ? "" : sound === "default" ? "default" : "file";
  const filename = source === "file" ? sound.split(/[\\/]/).pop() : null;
  const locked = busy || !draft;
  return <div className="notification-workspace">
    <div className="page-heading"><div><span className="page-kicker">自分に合う受け取り方に</span><h1>通知の設定</h1><p>VRでの過ごし方に合わせて、届け方を選びましょう。</p></div></div>
    {(!status.configured || status.onboardingPending) && <div className="setup-prompt"><div><strong>最初の接続を整えましょう</strong><p>Discordの認証から順番にご案内します。</p></div><button className="primary" disabled={busy || !!draft} onClick={setup}>接続を設定<ArrowRight size={16}/></button></div>}
    <div className="notification-columns">
      <section className="delivery-panel" aria-labelledby="delivery-title">
        <h2 id="delivery-title">通知の届け方</h2>
        <NotificationModeChoices name="notification-mode" value={draft?.soundOnly ?? status.soundOnly} onChange={mode} disabled={busy || !status.configured}/>
        <div className="workspace-receive">
          <label htmlFor="notification-receive">受信する通知</label>
          <select id="notification-receive" value={status.receiveMode} disabled={busy || !!draft || !status.configured} onChange={e=>receiveMode(e.target.value as ReceiveMode)}>
            {receiveChoices.map(choice=><option key={choice.mode} value={choice.mode} disabled={choice.mode==="auto"&&!status.botConfigured}>{choice.label}</option>)}
          </select>
          <p>{receiveChoices.find(choice=>choice.mode===status.receiveMode)?.help}</p>
          <button className="text-button" disabled={busy} onClick={()=>settings("whitelist")}><ListFilter size={14}/>許可リスト・自動判別を設定<ArrowRight size={14}/></button>
          <p className="receive-summary">{status.whitelistOnly?`許可リスト ${status.whitelistCount} 件だけ受信`:"Discordが通知した会話を受信"}{status.receiveMode==="auto"&&` · ${botLabels[status.botStatus]}`}</p>
          {status.receiveMode==="auto"&&status.whitelistOnly&&status.botStatus!=="dnd"&&<p className="inline-note">判定できるまで許可リストだけ受信します。Botと自分が同じサーバーにいること、Presence Intentを確認してください。</p>}
          {status.whitelistCount>0&&status.whitelistStatus!==2&&<p className="inline-note">{status.whitelistStatus===3?"許可リストの一部を購読できません。ID・閲覧権限・messages.readの承認を確認してください。":"許可リストの購読待ちです。Discordの承認画面を確認してください。"}</p>}
        </div>
        <p className="inline-note"><ShieldCheck size={15}/>受信条件の変更は自動保存されます。</p>
      </section>
      <form ref={formRef} className="notification-audio" aria-labelledby="audio-title" onSubmit={save} noValidate>
        <div className="audio-heading"><h2 id="audio-title">通知音とテスト</h2>{!draft&&<button type="button" className="text-button" disabled={busy} onClick={edit}>音を変更</button>}</div>
        <div className="audio-field-heading"><label htmlFor="workspace-sound">通知音</label><span>MP3 / WAV / Ogg</span></div>
        <div className="audio-file-summary"><FileAudio2 size={24}/><div><strong>{source==="default"?"標準の通知音":source===""?"無音":filename==="file"?"ファイルを選択してください":filename}</strong><span>{source==="default"?"XSOverlayの通知音":source===""?"音を鳴らさずに通知を表示": "選択した音声ファイル"}</span></div></div>
        <select id="workspace-sound" name="notificationSound" aria-label="通知音" value={source} disabled={locked} onChange={e=>change("notificationSound",e.target.value)}>
          <option value="">無音</option><option value="default">標準の通知音を使う</option><option value="file">音声ファイルを使う</option>
        </select>
        {source==="file"&&draft&&<div className="audio-file-picker"><button type="button" className="secondary" disabled={busy} onClick={browse}><FolderOpen size={15}/>ファイルを選ぶ</button><span>16 MiB・120秒以下</span></div>}
        <div className="workspace-volume"><div className="audio-field-heading"><label htmlFor="workspace-volume">音量</label><output>{Math.round(volume)}<small>%</small></output></div><div><Volume2 size={17}/><input id="workspace-volume" name="volumePercent" aria-label="通知音の音量" type="range" min="0" max="100" step="1" value={volume} disabled={locked||!sound} aria-valuetext={`${Math.round(volume)}%`} onChange={e=>change("volumePercent",Number(e.target.value))}/></div></div>
        {draft ? <>
          <p className="inline-note"><Info size={14}/>編集中は転送を一時停止しています。保存すると再開します。</p>
          {error&&<p className="error-banner" role="alert">{error}</p>}
          <div className="audio-save-actions"><button className="secondary" type="button" disabled={busy} onClick={cancel}>キャンセル</button><Button className="primary" type="submit" disabled={busy} hoverScale={1} tapScale={0.98}><Check size={15}/>保存</Button></div>
        </> : <>
          <Button className="primary notification-test" type="button" disabled={busy||status.paused||!status.overlay||status.debugEnabled} hoverScale={1} tapScale={0.98} onClick={test}><Play size={16}/>{status.preview?"通知テスト（デモ）":"テスト通知を送る"}<ArrowRight size={15}/></Button>
          <p className="audio-test-help">{status.preview?"画面確認用です。実際の通知・音声は送信しません。":status.debugEnabled?"受信テスト中はVRへ送信しません。":status.overlay?"保存した設定でXSOverlayへテスト通知を送ります。":"テストにはXSOverlayへの接続が必要です。"}</p>
          <p className="audio-test-help">{status.soundOnly? <><Headphones size={13}/>通知音のみでもXSOverlayへの接続が必要です。</>:<><Bell size={13}/>無音を選ぶとメッセージだけを表示します。</>}</p>
        </>}
      </form>
    </div>
    <p className="workspace-note"><Info size={14}/>{status.tray?"×・Escでトレイへ。通知の転送はそのまま続きます。":"トレイを利用できません。画面を開いたまま使用してください。"}</p>
  </div>;
}
