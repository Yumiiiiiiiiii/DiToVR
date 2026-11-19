import React, { useCallback, useEffect, useRef, useState } from "react";
import { AnimateUIProvider } from "@animate-ui/react/provider";
import { Button } from "@animate-ui/react/primitives/buttons/button";
import { Fade } from "@animate-ui/react/primitives/effects/fade";
import { Highlight } from "@animate-ui/react/primitives/effects/highlight";
import { ReducedMotionConfig, spring, transition, useShouldReduceMotion, useTempoTransition } from "@animate-ui/react/utils/motion";
import { startVisiblePolling } from "./visible-poll";
import { ConnectionStrip } from "./notification-home";
import { NotificationWorkspace } from "./notification-workspace";
import { HealthPanel } from "./health";
import { LeaveDialog } from "./leave-dialog";
import { createRoot } from "react-dom/client";
import { listen } from "@tauri-apps/api/event";
import {
  Activity,
  Download,
  Bell,
  Check,
  CircleHelp,
  ExternalLink,
  FolderOpen,
  Info,
  LockKeyhole,
  ListFilter,
  Plus,
  MessageCircle,
  Monitor,
  Power,
  Radio,
  Settings2,
  Shield,
  ShieldCheck,
  Volume2,
  X,
} from "lucide-react";
import {
  api,
  channelIdFromInput,
  desktop,
  errorMessage,
  type Settings,
  type SettingsInput,
  type Status,
  type ReceiveMode,
  type ThemePreference,
} from "./bridge";
import "./styles.css";
import "./workspace.css";
import { Onboarding } from "./onboarding";
import { NotificationModeChoices } from "./notification-mode";
import { DiagnosticsPanel } from "./diagnostics";
import { UpdatesPanel, UpdateNotice, useUpdateStatus } from "./updates";

type Page = "home" | "settings" | "tutorial" | "debug" | "updates" | "health";
type Tab = "display" | "sound" | "discord" | "whitelist";
type Form = Settings & {
  clientSecret: string;
  botToken: string;
  clearBotToken: boolean;
};
const emptyStatus: Status = {
  currentVersion: "",
  storeManaged: true,
  theme: "system",
  discord: 0,
  overlay: false,
  soundOnly: false,
  soundReady: false,
  notificationSound: "",
  volumePercent: 0,
  configured: false,
  paused: false,
  tray: false,
  problem: null,
  preview: !desktop,
  startSettings: false,
  receiveMode: "normal",
  whitelistOnly: false,
  whitelistCount: 0,
  whitelistStatus: 0,
  botStatus: "disabled",
  botConfigured: false,
  onboardingPending: false,
  debugEnabled: false,
};
const tabs: { id: Tab; label: string; icon: typeof Monitor }[] = [
  { id: "display", label: "表示・テーマ", icon: Monitor },
  { id: "sound", label: "通知音・動作", icon: Volume2 },
  { id: "discord", label: "Discord接続", icon: MessageCircle },
  { id: "whitelist", label: "許可リスト", icon: ListFilter },
];

function App() {
  const reveal = useTempoTransition(transition.reveal);
  const selection = useShouldReduceMotion() ? transition.instant : spring.selection;
  const [page, setPage] = useState<Page>("home");
  const [tab, setTab] = useState<Tab>("display");
  const [status, setStatus] = useState<Status>(emptyStatus);
  const {status:updateStatus,error:updateError}=useUpdateStatus(!status.storeManaged);
  const [form, setForm] = useState<Form | null>(null);
  const draftRef = useRef<Form | null>(null);
  draftRef.current = form;
  const [pendingPage, setPendingPage] = useState<Page | null>(null);
  const [statusError, setStatusError] = useState(false);
  const [lastStatusAt, setLastStatusAt] = useState<number | null>(null);
  const theme = form?.theme ?? status.theme;
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    if (desktop) void api("preview_theme", { theme }).catch(error =>
      setToast({ message: errorMessage(error), error: true }));
  }, [theme]);
  const [tutorialSettings, setTutorialSettings] = useState<Settings | null>(null);
  const [tutorialResume, setTutorialResume] = useState(false);
  const [busy, setBusy] = useState(false);
  const [channelInput, setChannelInput] = useState("");
  const [channelError, setChannelError] = useState<string | null>(null);
  const [toast, setToast] = useState<{
    message: string;
    error: boolean;
  } | null>(null);
  const [formError, setFormError] = useState<{
    field?: string;
    message: string;
  } | null>(null);
  const formRef = useRef<HTMLFormElement>(null);
  const menuRef = useRef<HTMLDetailsElement>(null);
  useEffect(() => {
    document.querySelector(".main-content")?.scrollTo({top: 0});
  }, [page, tab]);
  const ready = status.discord === 2 && (status.debugEnabled || status.overlay) && !status.paused;
  const refresh = useCallback(async () => {
    try {
      setStatus(await api<Status>("get_status"));
      setStatusError(false);
      setLastStatusAt(Date.now());
    } catch (e) {
      setStatusError(true);
    }
  }, []);
  const notify = (message: string, error = false) =>
    setToast({ message, error });
  const settings = useCallback(async (section: Tab = "display") => {
    if (draftRef.current) { setTab(section); setPage("settings"); return true; }
    setBusy(true);
    setFormError(null);
    setChannelInput("");
    setChannelError(null);
    try {
      const values = await api<Settings>("get_settings");
      setTutorialSettings(null);
      setStatus((previous) => ({ ...previous, paused: true, overlay: false }));
      setForm({
        ...values,
        clientSecret: "",
        botToken: "",
        clearBotToken: false,
      });
      setTab(section);
      setPage("settings");
      return true;
    } catch (e) {
      setToast({ message: errorMessage(e), error: true });
      return false;
    } finally {
      setBusy(false);
    }
  }, []);
  async function editAudio() {
    if (await settings("sound")) setPage("home");
  }
  const tutorial = useCallback(async (resumeConnection = false) => {
    setBusy(true);
    try {
      const values = await api<Settings>("get_settings");
      if (resumeConnection) await api("cancel_settings");
      setForm(null);
      setTutorialSettings(values);
      setTutorialResume(resumeConnection);
      setStatus(previous => ({ ...previous, paused: !resumeConnection, overlay: false }));
      setPage("tutorial");
    } catch (e) { setToast({ message: errorMessage(e), error: true }); }
    finally { setBusy(false); }
  }, []);
  async function openDebug() {
    setBusy(true);
    try {
      if (form || page === "tutorial") await api("cancel_settings");
      setForm(null); setTutorialSettings(null); setPage("debug");
    } catch(e) {notify(errorMessage(e), true);}
    finally {setBusy(false);await refresh();}
  }
  async function openUpdates() {
    setBusy(true);
    try {
      if(form||page==="tutorial")await api("cancel_settings");
      setForm(null);setTutorialSettings(null);setPage("updates");
    }catch(e){notify(errorMessage(e),true);}
    finally{setBusy(false);await refresh();}
  }
  async function navigate(next: Page) {
    if (next === page) return;
    if (form || page === "tutorial") { setPendingPage(next); return; }
    await go(next);
  }
  async function go(next: Page) {
    setPendingPage(null);
    if (next === "tutorial") { await tutorial(); return; }
    if (next === "updates") { await openUpdates(); return; }
    if (next === "debug") { await openDebug(); return; }
    setBusy(true);
    try {
      if (form || page === "tutorial") await api("cancel_settings");
      setForm(null); setTutorialSettings(null); setPage(next);
    } catch (error) { notify(errorMessage(error), true); }
    finally { setBusy(false); await refresh(); }
  }
  useEffect(() => {
    let active = true;
    let first = true;
    const stop = startVisiblePolling(async () => {
      const value = await api<Status>("get_status");
      if (!active) return;
      setStatus(value); setStatusError(false); setLastStatusAt(Date.now());
      if (first) {
        first = false;
        if (value.startSettings) void settings(value.configured ? "display" : "discord");
        else if (value.debugEnabled) setPage("debug");
        else if (value.onboardingPending) void tutorial(value.configured);
        else if (!value.configured) void settings("discord");
      }
    }, {onError: () => { if (active) setStatusError(true); }});
    const subscriptions = desktop ? [
      listen("navigate-settings", () => void settings()),
      listen("navigate-home", () => {
        setPendingPage(null); setForm(null); setTutorialSettings(null); setPage("home"); void refresh();
      }),
      listen<string>("operation-error", event => notify(event.payload, true)),
      listen<{Ok?: string; Err?: string}>("operation-result", event => notify(event.payload.Ok ?? event.payload.Err ?? "", Boolean(event.payload.Err))),
    ] : [];
    return () => { active = false; stop(); subscriptions.forEach(p => void p.then(unlisten => unlisten())); };
  }, [refresh, settings, tutorial]);
  useEffect(() => {
    const closeMenu = (event: PointerEvent) => {
      if (event.target instanceof Node && !menuRef.current?.contains(event.target)) menuRef.current?.removeAttribute("open");
    };
    document.addEventListener("pointerdown", closeMenu);
    return () => document.removeEventListener("pointerdown", closeMenu);
  }, []);
  useEffect(() => {
    const hide = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented || pendingPage) return;
      if (menuRef.current?.open) {
        event.preventDefault();
        menuRef.current.removeAttribute("open");
        menuRef.current.querySelector("summary")?.focus();
        return;
      }
      if (busy || !desktop) return;
      event.preventDefault();
      void api("hide_to_tray").catch((error) =>
        setToast({ message: errorMessage(error), error: true }),
      );
    };
    window.addEventListener("keydown", hide);
    return () => window.removeEventListener("keydown", hide);
  }, [busy, pendingPage]);
  useEffect(() => {
    if (!toast || toast.error) return;
    const timer = setTimeout(() => setToast(null), 5000);
    return () => clearTimeout(timer);
  }, [toast]);
  async function mode(soundOnly: boolean) {
    if (soundOnly && !status.soundReady) {
      if (await settings("sound")) {
        setForm(previous => previous ? { ...previous, soundOnly: true } : previous);
        setPage("home");
      }
      return;
    }
    setBusy(true);

    try {
      await api("set_sound_only", { enabled: soundOnly });
      notify("通知モードを保存しました。次回もこのモードで起動します。");
    } catch (e) {
      notify(errorMessage(e), true);
    } finally {
      setBusy(false);
      await refresh();
    }
  }
  async function receiveMode(mode: ReceiveMode) {
    setBusy(true);
    try {
      await api("set_receive_mode", { mode });
      notify("受信モードを切り替えました。");
    } catch (error) {
      notify(errorMessage(error), true);
    } finally {
      setBusy(false);
      await refresh();
    }
  }
  async function cancel() {
    setBusy(true);
    try {
      await api("cancel_settings");
      setForm(null);
      setTutorialSettings(null);
      setPage("home");
      setFormError(null);
    } catch (e) {
      notify(errorMessage(e), true);
    } finally {
      setBusy(false);
      await refresh();
    }
  }
  async function save(event: React.FormEvent) {
    event.preventDefault();
    if (!form) return;
    setBusy(true);
    setFormError(null);
    const {
      secretSet: _secretSet,
      botTokenSet: _botTokenSet,
      clientSecret,
      botToken,
      ...values
    } = form;
    const input: SettingsInput = {
      ...values,
      clientSecret: clientSecret.trim() || null,
      botToken: botToken.trim() || null,
    };
    try {
      const saved = await api<Settings>("save_settings", { input });
      setStatus(previous => ({ ...previous, theme: saved.theme }));
      setForm(null);
      setPage("home");
      notify("設定を保存しました。接続を再開します。");
    } catch (error) {
      const field =
        error && typeof error === "object" && "field" in error
          ? String(error.field ?? "")
          : "";
      setFormError({ field, message: errorMessage(error) });
      if (page === "home" && field && !["volumePercent", "notificationSound", "soundOnly"].includes(field)) setPage("settings");
      if (["clientId", "clientSecret", "redirectUri"].includes(field))
        setTab("discord");
      else if (
        ["whitelistChannelIds", "botToken", "botGuildId", "botUserId"].includes(
          field,
        )
      )
        setTab("whitelist");
      else if (["volumePercent", "notificationSound", "soundOnly"].includes(field))
        setTab("sound");
      else if (field) setTab("display");
      setTimeout(() => {
        const input = field === "soundOnly" && page === "home"
          ? document.querySelector<HTMLInputElement>('input[name="notification-mode"]:checked')
          : formRef.current?.querySelector<HTMLInputElement>(`[name="${field}"]`);
        input?.closest("details")?.setAttribute("open", "");
        input?.focus();
      }, 0);
    } finally {
      setBusy(false);
      await refresh();
    }
  }
  async function saveTutorial(input: SettingsInput) {
    setBusy(true);
    try { return await api<Settings>("save_settings", { input }); }
    finally { setBusy(false); await refresh(); }
  }
  async function finishTutorial(extras = false) {
    setBusy(true);
    try {
      await api("complete_onboarding");
      await api("cancel_settings");
      setTutorialSettings(null);
      setPage("home");
      if (extras) await settings("whitelist");
    } finally { setBusy(false); await refresh(); }
  }
  function change<K extends keyof Form>(key: K, value: Form[K]) {
    setForm((previous) =>
      previous ? { ...previous, [key]: value } : previous,
    );
  }
  async function action(command: string, success?: string) {
    setBusy(true);
    try {
      await api(command);
      if (success) notify(success);
    } catch (e) {
      notify(errorMessage(e), true);
    } finally {
      setBusy(false);
    }
  }
  const fieldError = (name: string) => formError?.field === name;
  const numberField = (
    name: "notificationTimeout" | "maxTitleLength" | "maxContentLength",
    label: string,
    help: string,
    min: number,
    max: number,
    unit: string,
  ) =>
    form && (
      <div className="setting-row">
        <div>
          <label htmlFor={name}>{label}</label>
          <p>{help}</p>
        </div>
        <div className="number-field">
          <input
            id={name}
            name={name}
            type="number"
            min={min}
            max={max}
            step={name === "notificationTimeout" ? "0.5" : "1"}
            required
            value={form[name]}
            aria-invalid={fieldError(name)}
            onChange={(e) => change(name, Number(e.target.value))}
          />
          <span>{unit}</span>
        </div>
      </div>
    );

  return (
    <div className="app-shell workspace-layout">
      <header className="workspace-header">
        <div className="workspace-brand"><span className="workspace-brand-mark">VR</span><strong>DiToVR</strong><small>{status.currentVersion ? "v"+status.currentVersion : ""}</small></div>
        <nav className="workspace-navigation" aria-label="メイン">
          <Highlight value={page==="settings"?(tab==="discord"||tab==="whitelist"?"connection":"behavior"):page==="home"||page==="health"?page:null} click={false} transition={selection} exitDelay={0} className="workspace-nav-indicator">
            <Button data-value="home" hoverScale={1} tapScale={0.98} aria-current={page==="home"?"page":undefined} className={page==="home"?"active":""} disabled={busy} onClick={()=>void navigate("home")}><Bell size={17}/>通知</Button>
            <Button data-value="connection" hoverScale={1} tapScale={0.98} aria-current={page==="settings"&&(tab==="discord"||tab==="whitelist")?"page":undefined} className={page==="settings"&&(tab==="discord"||tab==="whitelist")?"active":""} disabled={busy||page==="tutorial"} onClick={()=>void settings("discord")}><MessageCircle size={17}/>接続</Button>
            <Button data-value="behavior" hoverScale={1} tapScale={0.98} aria-current={page==="settings"&&(tab==="sound"||tab==="display")?"page":undefined} className={page==="settings"&&(tab==="sound"||tab==="display")?"active":""} disabled={busy||page==="tutorial"} onClick={()=>void settings("sound")}><Settings2 size={17}/>動作設定</Button>
            <Button data-value="health" hoverScale={1} tapScale={0.98} aria-current={page==="health"?"page":undefined} className={page==="health"?"active":""} disabled={busy} onClick={()=>void navigate("health")}><Activity size={17}/>診断</Button>
          </Highlight>
        </nav>
        <div className="workspace-tools">
          <button className="workspace-theme" aria-label="テーマと表示の設定" title="テーマと表示の設定" disabled={busy||page==="tutorial"} onClick={()=>void settings("display")}><Monitor size={18}/></button>
          <details ref={menuRef} className="workspace-menu" onClick={event=>{if(event.target instanceof Element&&event.target.closest("button"))menuRef.current?.removeAttribute("open");}}><summary aria-label="その他の操作" title="その他の操作"><CircleHelp size={18}/></summary><div>
            <button disabled={busy} onClick={()=>void navigate("tutorial")}><CircleHelp size={16}/>セットアップ</button>
            <button disabled={busy} onClick={()=>void navigate("updates")}><Download size={16}/>アプリの更新</button>
            {status.debugEnabled&&<button disabled={busy} onClick={()=>void navigate("debug")}><Radio size={16}/>受信テスト</button>}
            <button disabled={busy} onClick={()=>void action("exit_app")}><Power size={16}/>アプリを終了</button>
            <p><LockKeyhole size={12}/>認証情報はこのPCで保護</p>
          </div></details>
        </div>
      </header>
      <div className="workspace">
        <header className="topbar">
          <ConnectionStrip status={status}/>
          <span className={ready&&!statusError?"connection-pill connected":"connection-pill"}><i/>{statusError?"状態を取得できません":status.paused?"設定中 · 転送を一時停止":ready?status.debugEnabled?"Discord受信テスト中":"接続済み":"接続待ち"}</span>
        </header>
        <main className="main-content">
          {updateError && <div className="error-banner" role="alert"><Info size={17}/>更新状態を取得できませんでした。画面を表示中は自動で再試行します。</div>}
          {!status.storeManaged&&page!=="updates"&&<UpdateNotice status={updateStatus} onOpen={()=>void navigate("updates")}/>}
          {status.preview && (
            <div className="preview-banner">
              <Info size={15} />
              画面プレビュー · 接続状態はモックです。実接続・通知送信・設定保存は行いません。
            </div>
          )}
          {statusError && <div className="error-banner" role="alert"><Info size={17}/><span>状態の取得に失敗しました。{lastStatusAt ? `最終取得: ${new Date(lastStatusAt).toLocaleTimeString("ja-JP")}` : "接続状態は未確認です。"} 画面を表示中は自動で再試行します。</span><Button hoverScale={1} tapScale={0.98} className="secondary" onClick={()=>void refresh()}>再試行</Button></div>}
          {status.problem && (
            <div className="error-banner" role="alert">
              <CircleHelp size={18} />
              {status.problem}
            </div>
          )}
          <Fade key={page} className="workspace-page" initialOpacity={0.75} transition={reveal}>
          {page === "updates" ? status.storeManaged ? <div className="store-updates"><div className="page-heading"><div><h1>アプリの更新</h1><p>Microsoft Store が更新を管理します。</p></div></div><div className="health-detail"><Download size={28}/><h2>Microsoft Store 版</h2><p>現在のバージョン: v{status.currentVersion || "読み込み中"}</p><p>Microsoft Store のライブラリで更新を確認してください。</p></div></div> : <UpdatesPanel status={updateStatus}/> : page === "health" ? <HealthPanel status={status} busy={busy} onSetup={()=>void settings("discord")} onDebug={()=>void navigate("debug")}/> : page === "debug" && status.debugEnabled ? (
            <DiagnosticsPanel status={status} onSetup={()=>void (status.configured?settings("discord"):tutorial())}/>
          ) : page === "tutorial" && tutorialSettings ? (
            <Onboarding initial={tutorialSettings} status={status} busy={busy} resumeConnection={tutorialResume} onSave={saveTutorial} onEdit={() => api<Settings>("get_settings")} onFinish={finishTutorial} onLater={cancel} />
          ) : page === "home" ? (
            <NotificationWorkspace status={status} busy={busy} draft={form} formRef={formRef} error={formError?.message ?? null}
              settings={section=>void settings(section)} mode={value=>form?change("soundOnly",value):void mode(value)} receiveMode={value=>void receiveMode(value)}
              setup={()=>void tutorial(status.configured)} edit={()=>void editAudio()} change={(key,value)=>key==="volumePercent"?change(key,Number(value)):change(key,String(value))}
              browse={()=>{void api<string|null>("browse_sound").then(path=>{if(path)change("notificationSound",path);}).catch(error=>notify(errorMessage(error),true));}}
              save={event=>void save(event)} cancel={()=>void cancel()}
              test={()=>void action("send_test", status.preview ? "デモの通知テストを実行しました。実際の通知・音声は送信していません。" : "送信待ちに追加しました。XSOverlay 側で届き方を確認してください。")}/>
          ) : (
            <>
              <div className="page-heading settings-heading">
                <div>
                  <span className="page-kicker">あなたに合う、受け取り方へ</span>
                  <h1>{status.configured ? tabs.find(item=>item.id===tab)?.label : "はじめの接続設定"}</h1>
                  <p>設定中は転送を一時停止します。保存すると再開します。</p>
                </div>
                <span className="heading-icon">
                  <Settings2 size={27} strokeWidth={1.5} />
                </span>
              </div>
              <form
                ref={formRef}
                onSubmit={save}
                noValidate
                className="settings-form"
              >
                <nav className="settings-tabs" aria-label="設定の項目"><Highlight value={tab} click={false} transition={selection} exitDelay={0} className="settings-tab-indicator">{tabs.map(item=><Button key={item.id} data-value={item.id} type="button" hoverScale={1} tapScale={0.98} aria-current={tab===item.id?"page":undefined} className={tab===item.id?"active":""} disabled={busy} onClick={()=>setTab(item.id)}><item.icon size={16}/>{item.label}</Button>)}</Highlight></nav>
                {!form ? (
                  <div className="loading-card">設定を読み込んでいます…</div>
                ) : (
                  <Fade key={tab} initialOpacity={0.75} transition={reveal}>
                    <section
                      className="settings-card"
                      role="region"
                      id="panel-whitelist"
                      aria-label="許可リスト"
                      hidden={tab !== "whitelist"}
                    >
                      <div className="section-heading">
                        <div>
                          <h2>受信を許可するチャンネル・DM</h2>
                          <p>必要な会話だけを登録します。最大32件です。</p>
                        </div>
                        <ListFilter size={21} />
                      </div>
                      <div className="whitelist-content">
                        <div className="security-note">
                          <Info size={18} />
                          <div>
                            <p>
                              登録した会話のメッセージを直接受信します。初回は
                              Discord でメッセージの読み取り権限の追加承認が必要です。
                              「Discord通知に従う」では、通常の通知だけを転送します。
                            </p>
                            <p>
                              Discord
                              側でミュートした会話も、許可リストだけ受信する間は表示対象になります。必要な会話だけを登録してください。
                            </p>
                          </div>
                        </div>
                        <div className="security-note">
                          <Info size={18} />
                          <div>
                            <p>
                              Discord
                              の「詳細設定」で開発者モードを有効にし、チャンネルの右クリックから
                              ID をコピーします。
                            </p>
                            <p>
                              DM
                              はメッセージのリンクをコピーして貼り付けることもできます。リンクからチャンネル
                              ID だけを取り出し、外部へアクセスしません。
                            </p>
                          </div>
                        </div>
                        <label htmlFor="whitelistChannelIds">
                          チャンネル ID またはメッセージリンク
                        </label>
                        <div className="whitelist-add">
                          <input
                            id="whitelistChannelIds"
                            name="whitelistChannelIds"
                            placeholder="例: 123456789012345678"
                            autoComplete="off"
                            value={channelInput}
                            aria-invalid={
                              Boolean(channelError) ||
                              fieldError("whitelistChannelIds")
                            }
                            onChange={(event) => {
                              setChannelInput(event.target.value);
                              setChannelError(null);
                            }}
                            onKeyDown={(event) => {
                              if (event.key === "Enter") {
                                event.preventDefault();
                                document.getElementById("add-channel")?.click();
                              }
                            }}
                          />
                          <Button hoverScale={1} tapScale={0.98}
                            id="add-channel"
                            type="button"
                            className="secondary"
                            disabled={
                              busy || form.whitelistChannelIds.length >= 32
                            }
                            onClick={() => {
                              const id = channelIdFromInput(channelInput);
                              if (!id) {
                                setChannelError(
                                  "数字の ID または Discord のメッセージリンクを入力してください。",
                                );
                                document
                                  .getElementById("whitelistChannelIds")
                                  ?.focus();
                                return;
                              }
                              if (form.whitelistChannelIds.includes(id)) {
                                setChannelError(
                                  "このチャンネルは登録済みです。",
                                );
                                return;
                              }
                              change("whitelistChannelIds", [
                                ...form.whitelistChannelIds,
                                id,
                              ]);
                              setChannelInput("");
                              setChannelError(null);
                            }}
                          >
                            <Plus size={15} />
                            追加
                          </Button>
                        </div>
                        {channelError && (
                          <p className="whitelist-error" role="alert">
                            {channelError}
                          </p>
                        )}
                        <div className="whitelist-count">
                          登録済み {form.whitelistChannelIds.length} / 32
                        </div>
                        {form.whitelistChannelIds.length === 0 ? (
                          <div className="whitelist-empty">
                            まだ登録されていません。
                          </div>
                        ) : (
                          <ul className="whitelist-list">
                            {form.whitelistChannelIds.map((id) => (
                              <li key={id}>
                                <MessageCircle size={16} />
                                <code>{id}</code>
                                <Button hoverScale={1} tapScale={0.98}
                                  type="button"
                                  className="secondary"
                                  aria-label={`${id} を削除`}
                                  onClick={() =>
                                    change(
                                      "whitelistChannelIds",
                                      form.whitelistChannelIds.filter(
                                        (entry) => entry !== id,
                                      ),
                                    )
                                  }
                                >
                                  <X size={14} />
                                  削除
                                </Button>
                              </li>
                            ))}
                          </ul>
                        )}
                      </div>
                    </section>
                    <details
                      className="settings-card bot-settings"
                      open={form.receiveMode === "auto" || formError?.field?.startsWith("bot") ? true : undefined}
                      id="panel-bot"
                      aria-labelledby="bot-heading"
                      hidden={tab !== "whitelist"}
                    >
                      <summary>Botで自動判別を使う（任意）</summary>
                      <div className="section-heading">
                        <div>
                          <h2 id="bot-heading">Bot による取り込み中の自動判別</h2>
                          <p>
                            Botは状態監視専用です。会話はDiscordデスクトップから受信します。
                          </p>
                        </div>
                        <ShieldCheck size={21} />
                      </div>
                      <div className="whitelist-content">
                        <ol className="bot-guide">
                          <li>
                            Developer Portal の Bot ページで{" "}
                            <strong>Presence Intent</strong>{" "}
                            を有効にします。Message Content Intent は不要です。
                          </li>
                          <li>
                            OAuth2 の URL Generator で <strong>bot</strong>{" "}
                            を選び、権限は <strong>0</strong>{" "}
                            のまま、自分と同じサーバーへ追加します。自分専用の小さいサーバーでも使えます。
                          </li>
                          <li>
                            Bot Token、サーバー ID、自分のユーザー ID
                            を下に入力します。ID
                            は開発者モードで右クリックしてコピーできます。
                          </li>
                        </ol>
                        <div className="credential-fields">
                          <div>
                            <label htmlFor="botGuildId">
                              共通サーバーの ID
                            </label>
                            <input
                              id="botGuildId"
                              name="botGuildId"
                              inputMode="numeric"
                              maxLength={20}
                              autoComplete="off"
                              value={form.botGuildId}
                              aria-invalid={fieldError("botGuildId")}
                              onChange={(e) =>
                                change("botGuildId", e.target.value)
                              }
                            />
                            <small>
                              Bot と自分がどちらも参加しているサーバー
                            </small>
                          </div>
                          <div>
                            <label htmlFor="botUserId">自分のユーザー ID</label>
                            <input
                              id="botUserId"
                              name="botUserId"
                              inputMode="numeric"
                              maxLength={20}
                              autoComplete="off"
                              value={form.botUserId}
                              aria-invalid={fieldError("botUserId")}
                              onChange={(e) =>
                                change("botUserId", e.target.value)
                              }
                            />
                            <small>
                              Discord でログインしている自分のアカウント
                            </small>
                          </div>
                          <div className="full-width">
                            <label htmlFor="botToken">
                              Bot Token{" "}
                              {form.botTokenSet && !form.clearBotToken && (
                                <span className="configured-tag">
                                  <LockKeyhole size={12} />
                                  設定済み
                                </span>
                              )}
                            </label>
                            <input
                              id="botToken"
                              name="botToken"
                              type="password"
                              autoComplete="new-password"
                              spellCheck={false}
                              maxLength={512}
                              value={form.botToken}
                              placeholder={
                                form.botTokenSet && !form.clearBotToken
                                  ? "変更する場合だけ入力してください"
                                  : "Bot ページで発行した Token"
                              }
                              aria-invalid={fieldError("botToken")}
                              onChange={(e) =>
                                change("botToken", e.target.value)
                              }
                            />
                            <small>
                              空欄なら保存済みTokenを維持します。暗号化し、画面には読み戻しません。
                            </small>
                          </div>
                        </div>
                        <div className="bot-forget">
                          <Button hoverScale={1} tapScale={0.98}
                            type="button"
                            className="secondary"
                            disabled={!form.botTokenSet || form.clearBotToken}
                            onClick={() => {
                              change("clearBotToken", true);
                              change("botToken", "");
                              change("receiveMode", "normal");
                            }}
                          >
                            保存済み Bot Token を削除
                          </Button>
                          {form.clearBotToken && (
                            <span>
                              保存すると Token を削除し、通常受信に戻ります。
                            </span>
                          )}
                        </div>
                        <p className="inline-note">保存後、通知画面の「受信する通知」でBotによる自動判別を選べます。</p>
                        <div className="security-note">
                          <Shield size={18} />
                          <div>
                            <p>
                              自動判別できないときは許可リストだけ受信します。オフライン・Invisible
                              は Bot
                              から区別できないため、制限を維持します。ホームからいつでも手動で上書きできます。
                            </p>
                          </div>
                        </div>
                      </div>
                    </details>
                    <section
                      className="settings-card"
                      role="region"
                      id="panel-display"
                      aria-label="表示・テーマ"
                      hidden={tab !== "display"}
                    >
                      <div className="section-heading">
                        <div>
                          <h2>画面と通知の見え方</h2>
                          <p>アプリのテーマと VR 内の表示を調整します。</p>
                        </div>
                        <Monitor size={21} />
                      </div>
                      <div className="setting-row">
                        <div>
                          <label htmlFor="theme">アプリのテーマ</label>
                          <p id="theme-help">システムは Windows のライト・ダーク設定に従います。変更は「保存」で確定します。</p>
                        </div>
                        <select id="theme" name="theme" className="theme-select"
                          aria-describedby="theme-help" value={form.theme}
                          onChange={event => change("theme", event.target.value as ThemePreference)}>
                          <option value="system">システム（初期設定）</option>
                          <option value="light">ライト</option>
                          <option value="dark">ダーク</option>
                        </select>
                      </div>
                      {numberField(
                        "notificationTimeout",
                        "表示時間",
                        "通知を表示する長さ。1 ～ 30 秒で調整できます。",
                        1,
                        30,
                        "秒",
                      )}
                      {numberField(
                        "maxTitleLength",
                        "タイトルの文字数",
                        "長いタイトルは末尾を省略します。10 ～ 200 文字。",
                        10,
                        200,
                        "文字",
                      )}
                      {numberField(
                        "maxContentLength",
                        "本文の文字数",
                        "メッセージを読みやすい長さに。10 ～ 2000 文字。",
                        10,
                        2000,
                        "文字",
                      )}
                    </section>
                    <section
                      className="settings-card"
                      role="region"
                      id="panel-sound"
                      aria-label="通知音・動作"
                      hidden={tab !== "sound"}
                    >
                      <div className="section-heading">
                        <div>
                          <h2>通知音・動作</h2>
                          <p>音と、VR を終了したときの動作を調整します。</p>
                        </div>
                        <Volume2 size={21} />
                      </div>
                      <div className="sound-section">
                        <label>通知モード</label>
                        <NotificationModeChoices name="soundOnly" value={form.soundOnly} onChange={value => change("soundOnly", value)} disabled={busy} className="mode-options-inline" />
                        <p className="sound-help">{form.soundOnly ? "音だけでお知らせします。下で通知音を選び、音量を1%以上にしてください。" : "通知を VR に表示します。通知音は無音にもできます。"}</p>
                      </div>
                      <div className="sound-section">
                        <label>通知音</label>
                        <div className="sound-options">
                          {[
                            {
                              value: "",
                              title: "無音",
                              text: "静かに通知を受け取る",
                            },
                            {
                              value: "default",
                              title: "標準の通知音",
                              text: "XSOverlay の通知音",
                            },
                            {
                              value: "file",
                              title: "音声ファイル",
                              text: "好きな音を選択",
                            },
                          ].map((option) => {
                            const selected =
                              option.value === "file"
                                ? !["", "default"].includes(
                                    form.notificationSound,
                                  )
                                : form.notificationSound === option.value;
                            return (
                              <Button hoverScale={1} tapScale={0.98}
                                key={option.value}
                                type="button"
                                className={selected ? "selected" : ""}
                                aria-pressed={selected}
                                onClick={() =>
                                  change(
                                    "notificationSound",
                                    option.value === "file"
                                      ? "file"
                                      : option.value,
                                  )
                                }
                              >
                                <span className="sound-choice-mark">
                                  {selected ? (
                                    <Check size={13} />
                                  ) : (
                                    <Volume2 size={14} />
                                  )}
                                </span>
                                <strong>{option.title}</strong>
                                <small>{option.text}</small>
                              </Button>
                            );
                          })}
                        </div>
                        <p className="sound-help">MP3・WAV・Ogg Vorbis に対応。16 MiB・120秒以下。MP3・WAV は自動変換し、元ファイルは変更しません。</p>
                        {!["", "default"].includes(form.notificationSound) && (
                          <div className="file-field">
                            <input
                              name="notificationSound"
                              aria-label="通知音のファイル"
                              aria-invalid={fieldError("notificationSound")}
                              readOnly
                              value={
                                form.notificationSound === "file"
                                  ? ""
                                  : form.notificationSound
                              }
                              placeholder="音声ファイルを選んでください"
                            />
                            <Button hoverScale={1} tapScale={0.98}
                              type="button"
                              className="secondary"
                              onClick={async () => {
                                try {
                                  const path = await api<string | null>(
                                    "browse_sound",
                                  );
                                  if (path) change("notificationSound", path);
                                } catch (e) {
                                  notify(errorMessage(e), true);
                                }
                              }}
                            >
                              <FolderOpen size={15} />
                              参照
                            </Button>
                          </div>
                        )}
                      </div>
                      <div className="setting-row">
                        <div>
                          <label htmlFor="volumePercent">音量</label>
                          <p>
                            {form.notificationSound
                              ? "通知音を鳴らすには 1% 以上にしてください。"
                              : "現在は無音が選択されています。"}
                          </p>
                        </div>
                        <div className="volume-control">
                          <input
                            type="range"
                            id="volumePercent"
                            name="volumePercent"
                            min="0"
                            max="100"
                            value={form.volumePercent}
                            disabled={!form.notificationSound}
                            aria-invalid={fieldError("volumePercent")}
                            aria-valuetext={`${form.volumePercent}%`}
                            onChange={(e) =>
                              change("volumePercent", Number(e.target.value))
                            }
                          />
                          <output>{form.volumePercent}%</output>
                        </div>
                      </div>
                      <div className="setting-row">
                        <div>
                          <label htmlFor="autoExitWithVr">
                            VR と一緒に終了
                          </label>
                          <p>
                            一度検知した VR
                            関連アプリがすべて終了すると停止します。
                          </p>
                        </div>
                        <Button hoverScale={1} tapScale={0.98}
                          type="button"
                          id="autoExitWithVr"
                          role="switch"
                          aria-checked={form.autoExitWithVr}
                          className="switch-hit-area"
                          onClick={() =>
                            change("autoExitWithVr", !form.autoExitWithVr)
                          }
                        >
                          <span className={`switch ${form.autoExitWithVr ? "on" : ""}`} aria-hidden="true"><span /></span>
                        </Button>
                      </div>
                    </section>
                    <section
                      className="settings-card connection-settings"
                      role="region"
                      id="panel-discord"
                      aria-label="Discord接続"
                      hidden={tab !== "discord"}
                    >
                      <div className="section-heading">
                        <div>
                          <h2>自分の Discord アプリを接続</h2>
                          <p>
                            利用する本人のアカウントで作成したアプリを使います。
                          </p>
                        </div>
                        <MessageCircle size={21} />
                      </div>
                      <details className="setup-guide" open={!status.configured || undefined}>
                        <summary>Discord アプリの作り方</summary>
                        <div>
                          <span className="step">1</span>
                          <p>
                            Developer Portal で <strong>New Application</strong>{" "}
                            を作成
                          </p>
                        </div>
                        <div>
                          <span className="step">2</span>
                          <p>
                            <strong>OAuth2</strong> で Client ID と Client
                            Secret を確認
                          </p>
                        </div>
                        <div>
                          <span className="step">3</span>
                          <p>
                            <strong>Redirects</strong> に下の URI を追加し、Save
                            Changes
                          </p>
                        </div>
                        <Button hoverScale={1} tapScale={0.98}
                          type="button"
                          className="portal-button"
                          onClick={() => void action("open_portal")}
                        >
                          Developer Portal を開く
                          <ExternalLink size={13} />
                        </Button>
                      </details>
                      <div className="credential-fields">
                        <div>
                          <label htmlFor="clientId">Client ID</label>
                          <input
                            id="clientId"
                            name="clientId"
                            inputMode="numeric"
                            autoComplete="off"
                            value={form.clientId}
                            placeholder="例：123456789012345678"
                            aria-invalid={fieldError("clientId")}
                            onChange={(e) => change("clientId", e.target.value)}
                          />
                          <small>OAuth2 に表示されている数字の ID</small>
                        </div>
                        <div>
                          <label htmlFor="clientSecret">
                            Client Secret
                            {form.secretSet && (
                              <span className="saved-secret">
                                <LockKeyhole size={11} />
                                設定済み
                              </span>
                            )}
                          </label>
                          <input
                            id="clientSecret"
                            name="clientSecret"
                            type="password"
                            autoComplete="new-password"
                            value={form.clientSecret}
                            placeholder={
                              form.secretSet
                                ? "変更する場合だけ入力してください"
                                : "自分のアプリの Secret を入力"
                            }
                            aria-invalid={fieldError("clientSecret")}
                            onChange={(e) =>
                              change("clientSecret", e.target.value)
                            }
                          />
                          <small>
                            {form.secretSet
                              ? "空欄で保存すると、現在の Secret を引き継ぎます。"
                              : "Bot Token やアカウントのトークンは入力しません。"}
                          </small>
                        </div>
                        <div className="redirect-field">
                          <label htmlFor="redirectUri">Redirect URI</label>
                          <input
                            id="redirectUri"
                            name="redirectUri"
                            autoComplete="off"
                            value={form.redirectUri}
                            aria-invalid={fieldError("redirectUri")}
                            onChange={(e) =>
                              change("redirectUri", e.target.value)
                            }
                          />
                          <small>
                            Portal の Redirects に登録した URL
                            と一致させてください。
                          </small>
                        </div>
                      </div>
                      <div className="security-note">
                        <ShieldCheck size={17} />
                        <p>
                          Secret は Windows ユーザーに紐づけて暗号化保存します。
                          <br />
                          設定ファイルは他の人と共有しないでください。
                        </p>
                      </div>
                    </section>
                    {formError && (
                      <div className="error-banner" role="alert">
                        <CircleHelp size={18} />
                        {formError.message}
                      </div>
                    )}
                    <div className="settings-footer">
                      <span>
                        <Info size={14} />
                        設定中は通知の転送を一時停止しています。
                      </span>
                      <div>
                        <Button hoverScale={1} tapScale={0.98}
                          type="button"
                          className="secondary"
                          onClick={() => void cancel()}
                          disabled={busy}
                        >
                          キャンセル
                        </Button>
                        <Button hoverScale={1} tapScale={0.98}
                          type="submit"
                          className="primary"
                          disabled={busy}
                        >
                          <Check size={16} />
                          {busy ? "保存中…" : "保存して戻る"}
                        </Button>
                      </div>
                    </div>
                  </Fade>
                )}
              </form>
            </>
          )}
          </Fade>
          <footer className="page-footer">
            <span>DiToVR</span>
            <span>{status.storeManaged ? "Microsoft Store 版" : "GitHub 版"}</span>
          </footer>
        </main>
      </div>
      {pendingPage && <LeaveDialog onStay={()=>setPendingPage(null)} onLeave={()=>void go(pendingPage)}/>}
      <div className="toast-region" aria-live="polite" aria-atomic="true">
      {toast && (
        <div
          className={`toast ${toast.error ? "error" : ""}`}
          role={toast.error ? "alert" : "status"}
        >
          {toast.error ? <CircleHelp size={18} /> : <Check size={18} />}
          <p>{toast.message}</p>
          <Button hoverScale={1} tapScale={0.98} aria-label="通知を閉じる" onClick={() => setToast(null)}>
            <X size={16} />
          </Button>
        </div>
      )}
      </div>
    </div>
  );
}
createRoot(document.getElementById("root")!).render(<AnimateUIProvider tempo={0.85}><ReducedMotionConfig><App /></ReducedMotionConfig></AnimateUIProvider>);
