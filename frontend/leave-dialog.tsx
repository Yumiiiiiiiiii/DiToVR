import { useEffect, useRef } from "react";

export function LeaveDialog({ onStay, onLeave }: { onStay:()=>void; onLeave:()=>void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(()=>{
    const previous = document.activeElement as HTMLElement | null;
    dialog.current?.showModal();
    return ()=>{ dialog.current?.close(); previous?.focus(); };
  },[]);
  return <dialog ref={dialog} className="leave-dialog" aria-labelledby="leave-title" aria-describedby="leave-description" onCancel={event=>{event.preventDefault();onStay();}}>
    <h2 id="leave-title">設定を閉じますか？</h2><p id="leave-description">未保存の入力は破棄され、通知の転送を再開します。保存する場合は、設定に戻ってください。</p><div><button className="secondary" autoFocus onClick={onStay}>設定に戻る</button><button className="primary" onClick={onLeave}>破棄して移動</button></div>
  </dialog>;
}
