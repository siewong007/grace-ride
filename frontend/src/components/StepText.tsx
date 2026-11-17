import { Fragment, ReactNode } from 'react';
import { AlertTriangle, Info, Lightbulb } from 'lucide-react';

type Kind = 'note' | 'tip' | 'warning';
const PREFIX: [RegExp, Kind][] = [
  [/^(note|\u6ce8\u610f|\u6ce8)\s*[:\uff1a]\s*/i, 'note'],
  [/^(tip|\u63d0\u793a|\u5c0f\u8d34\u58eb)\s*[:\uff1a]\s*/i, 'tip'],
  [/^(warning|careful|\u8b66\u544a|\u5c0f\u5fc3)\s*[:\uff1a]\s*/i, 'warning'],
];
const ICON: Record<Kind, ReactNode> = {
  note: <Info size={16} aria-hidden="true" />,
  tip: <Lightbulb size={16} aria-hidden="true" />,
  warning: <AlertTriangle size={16} aria-hidden="true" />,
};

/** **bold** becomes bold text. Built as React nodes, so nothing a guide editor types is ever run as HTML. */
export function inline(text: string): ReactNode[] {
  return text.split(/(\*\*[^*]+\*\*)/g).map((part, i) =>
    part.startsWith('**') && part.endsWith('**') && part.length > 4 ? <strong key={i}>{part.slice(2, -2)}</strong> : <Fragment key={i}>{part}</Fragment>,
  );
}

/** Splits a step's text into lines; a line starting "Note:", "Tip:" or "Warning:" (or the Chinese equivalents) becomes a callout. */
export function parseStepText(text: string): { kind: Kind | null; text: string }[] {
  return text.split('\n').filter((l) => l.trim()).map((line) => {
    for (const [re, kind] of PREFIX) if (re.test(line.trim())) return { kind, text: line.trim().replace(re, '') };
    return { kind: null, text: line };
  });
}

export default function StepText({ text, className = '' }: { text: string; className?: string }) {
  return (
    <div className={`step-text ${className}`}>
      {parseStepText(text).map((l, i) =>
        l.kind ? (
          <p key={i} className={`guide-note ${l.kind}`}>{ICON[l.kind]}<span>{inline(l.text)}</span></p>
        ) : (
          <p key={i}>{inline(l.text)}</p>
        ),
      )}
    </div>
  );
}
