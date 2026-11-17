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

/** **bold**, *italic*, and `code` become formatted inline nodes. Built as React nodes, so nothing a guide editor types is ever run as HTML. */
export function inline(text: string): ReactNode[] {
  return text.split(/(\*\*[^*]+\*\*|`[^`]+`|\*[^*\n]+\*)/g).map((part, i) => {
    if (part.startsWith('**') && part.endsWith('**') && part.length > 4) {
      return <strong key={i}>{part.slice(2, -2)}</strong>;
    }
    if (part.startsWith('`') && part.endsWith('`') && part.length > 2) {
      return <code key={i}>{part.slice(1, -1)}</code>;
    }
    if (part.startsWith('*') && part.endsWith('*') && part.length > 2) {
      return <em key={i}>{part.slice(1, -1)}</em>;
    }
    return <Fragment key={i}>{part}</Fragment>;
  });
}

/** Splits a step's text into lines; a line starting "Note:", "Tip:" or "Warning:" (or the Chinese equivalents) becomes a callout. */
export function parseStepText(text: string): { kind: Kind | null; text: string }[] {
  return text.split('\n').filter((l) => l.trim()).map((line) => {
    for (const [re, kind] of PREFIX) if (re.test(line.trim())) return { kind, text: line.trim().replace(re, '') };
    return { kind: null, text: line };
  });
}

export default function StepText({ text, className = '' }: { text: string; className?: string }) {
  const lines = parseStepText(text);
  const blocks: ReactNode[] = [];
  let bullets: { key: number; content: string }[] = [];
  const flushBullets = () => {
    if (!bullets.length) return;
    blocks.push(
      <ul key={`ul-${bullets[0].key}`} className="step-bullets">
        {bullets.map((b) => (
          <li key={b.key}>{inline(b.content)}</li>
        ))}
      </ul>,
    );
    bullets = [];
  };

  lines.forEach((l, i) => {
    const trimmed = l.text.trimStart();
    if (!l.kind && (trimmed.startsWith('- ') || trimmed.startsWith('* '))) {
      bullets.push({ key: i, content: trimmed.slice(2).trim() });
      return;
    }
    flushBullets();
    if (l.kind) {
      blocks.push(
        <p key={i} className={`guide-note ${l.kind}`}>
          {ICON[l.kind]}
          <span>{inline(l.text)}</span>
        </p>,
      );
    } else {
      blocks.push(<p key={i}>{inline(l.text)}</p>);
    }
  });
  flushBullets();

  return <div className={`step-text ${className}`}>{blocks}</div>;
}
