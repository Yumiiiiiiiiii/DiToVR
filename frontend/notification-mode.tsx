import { MessageCircle, Volume2 } from "lucide-react";

export function NotificationModeChoices({ value, onChange, disabled, name, className = "" }: {
  value: boolean;
  onChange: (soundOnly: boolean) => void;
  disabled?: boolean;
  name: string;
  className?: string;
}) {
  return <div className={`mode-options ${className}`} role="radiogroup" aria-label="通知モード">
    {[false, true].map(soundOnly => <label key={String(soundOnly)} className={`mode-option ${value === soundOnly ? "selected" : ""} ${disabled ? "disabled" : ""}`}>
      <span className="mode-option-icon" aria-hidden="true">{soundOnly ? <Volume2 size={22} /> : <MessageCircle size={22} />}</span>
      <span><strong>{soundOnly ? "通知音のみ" : "通常表示"}</strong><small>{soundOnly ? "音だけを鳴らし、VR に通知を表示しない" : "名前・本文・アイコンを VR に表示"}</small></span>
      <input className="mode-radio" type="radio" name={name} value={soundOnly ? "sound-only" : "normal"} checked={value === soundOnly} disabled={disabled} onChange={() => onChange(soundOnly)} />
    </label>)}
  </div>;
}
