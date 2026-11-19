import { MessageCircle, Volume2 } from "lucide-react";
import { Highlight, HighlightItem } from "@animate-ui/react/primitives/effects/highlight";
import { spring, transition, useShouldReduceMotion, useTempoTransition } from "@animate-ui/react/utils/motion";
import { motion } from "motion/react";

export function NotificationModeChoices({ value, onChange, disabled, name, className = "" }: {
  value: boolean;
  onChange: (soundOnly: boolean) => void;
  disabled?: boolean;
  name: string;
  className?: string;
}) {
  const check = useTempoTransition(spring.checkPop);
  const reduce = useShouldReduceMotion();
  return <div className={`mode-options ${className}`} role="radiogroup" aria-label="通知モード">
    <Highlight value={String(value)} controlledItems click={false} transition={reduce?transition.instant:spring.selection} exitDelay={0} className="mode-selection-surface">
    {[false, true].map(soundOnly => <HighlightItem as="label" value={String(soundOnly)} key={String(soundOnly)} className={`mode-option ${value === soundOnly ? "selected" : ""} ${disabled ? "disabled" : ""}`}>
      <span className="mode-content">
      <span className="mode-option-icon" aria-hidden="true">{soundOnly ? <Volume2 size={22} /> : <MessageCircle size={22} />}</span>
      <span className="mode-copy"><strong>{soundOnly ? "通知音のみ" : "通常表示"}</strong><small>{soundOnly ? "音だけを鳴らし、VR に通知を表示しない" : "名前・本文・アイコンを VR に表示"}</small></span>
      <input className="mode-radio" type="radio" name={name} value={soundOnly ? "sound-only" : "normal"} checked={value === soundOnly} disabled={disabled} onChange={() => onChange(soundOnly)} />
      <span className="mode-choice-dot" aria-hidden="true"><motion.span className="mode-choice-fill" initial={false} animate={{scale:value===soundOnly?1:0}} transition={reduce?transition.instant:check}/></span>
      </span>
    </HighlightItem>)}
    </Highlight>
  </div>;
}
