import { KeyboardEvent as ReactKeyboardEvent, useId, useRef, useState } from 'react';
import { Check } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import {
  GROKBOT_COLOR_PALETTES,
  GROKBOT_COLORS,
  GROKBOT_FACES,
  GROKBOT_MOVES,
  GROKBOT_SHAPES,
  type GrokBotColor,
  type GrokBotFace,
  type GrokBotMove,
  type GrokBotShape,
  type GrokBotSpec,
} from '../grokBotStudio';
import GrokBotStudioSvg from './GrokBotSvg';

export interface GrokBotControlsProps {
  spec: GrokBotSpec;
  onSpecChange: (next: GrokBotSpec) => void;
  move: GrokBotMove;
  onMoveChange: (next: GrokBotMove) => void;
  compact?: boolean;
}

function stepIndex(current: number, total: number, key: string, cols: number): number | null {
  switch (key) {
    case 'ArrowRight':
      return (current + 1) % total;
    case 'ArrowLeft':
      return (current - 1 + total) % total;
    case 'ArrowDown':
      return (current + cols) % total;
    case 'ArrowUp':
      return (current - cols + total) % total;
    case 'Home':
      return 0;
    case 'End':
      return total - 1;
    default:
      return null;
  }
}

export default function GrokBotControls({
  spec,
  onSpecChange,
  move,
  onMoveChange,
  compact = false,
}: GrokBotControlsProps) {
  const { t } = useTranslation('studio');
  const uid = useId();
  const [tab, setTab] = useState<'look' | 'move'>('look');
  const shapeRefs = useRef<(HTMLButtonElement | null)[]>([]);
  const faceRefs = useRef<(HTMLButtonElement | null)[]>([]);
  const colorRefs = useRef<(HTMLButtonElement | null)[]>([]);

  const handleRadioKey = <T extends string>(
    e: ReactKeyboardEvent<HTMLButtonElement>,
    items: readonly T[],
    currentVal: T,
    cols: number,
    refs: (HTMLButtonElement | null)[],
    apply: (next: T) => void,
  ) => {
    const idx = items.indexOf(currentVal);
    if (idx < 0) return;
    const nextIdx = stepIndex(idx, items.length, e.key, cols);
    if (nextIdx === null) return;
    e.preventDefault();
    apply(items[nextIdx]);
    refs[nextIdx]?.focus();
  };

  return (
    <div className={`grok-studio-controls${compact ? ' compact' : ''}`}>
      <div className="grok-studio-tabs" role="tablist" aria-label={t('title')}>
        <button
          id={`${uid}-tab-look`}
          type="button"
          role="tab"
          aria-selected={tab === 'look'}
          aria-controls={`${uid}-panel-look`}
          className={`grok-studio-tab${tab === 'look' ? ' on' : ''}`}
          onClick={() => setTab('look')}
        >
          {t('lookTab')}
        </button>
        <button
          id={`${uid}-tab-move`}
          type="button"
          role="tab"
          aria-selected={tab === 'move'}
          aria-controls={`${uid}-panel-move`}
          className={`grok-studio-tab${tab === 'move' ? ' on' : ''}`}
          onClick={() => setTab('move')}
        >
          {t('moveTab')}
        </button>
      </div>

      {tab === 'look' ? (
        <div id={`${uid}-panel-look`} role="tabpanel" aria-labelledby={`${uid}-tab-look`} className="grok-studio-panel stack">
          <section className="stack-xs">
            <div className="grok-studio-group-head">
              <span className="grok-studio-group-label">{t('shapeLabel')}</span>
              <span className="grok-studio-group-val">{t(`shapes.${spec.shape}`)}</span>
            </div>
            <div
              className="grok-studio-tile-grid cols-4"
              role="radiogroup"
              aria-label={t('shapeLabel')}
            >
              {GROKBOT_SHAPES.map((s: GrokBotShape, idx: number) => {
                const selected = spec.shape === s;
                return (
                  <button
                    key={s}
                    ref={(el) => {
                      shapeRefs.current[idx] = el;
                    }}
                    type="button"
                    role="radio"
                    aria-checked={selected}
                    tabIndex={selected ? 0 : -1}
                    className={`grok-studio-tile${selected ? ' on' : ''}`}
                    onClick={() => onSpecChange({ ...spec, shape: s })}
                    onKeyDown={(e) =>
                      handleRadioKey(e, GROKBOT_SHAPES, spec.shape, 4, shapeRefs.current, (next) =>
                        onSpecChange({ ...spec, shape: next }),
                      )
                    }
                  >
                    {selected && (
                      <span className="grok-studio-check" aria-hidden="true">
                        <Check size={11} strokeWidth={3} />
                      </span>
                    )}
                    <GrokBotStudioSvg
                      shape={s}
                      face={spec.face}
                      color={spec.color}
                      variant="tile"
                      className="grok-tile-svg"
                    />
                    <span className="grok-studio-tile-label">{t(`shapes.${s}`)}</span>
                  </button>
                );
              })}
            </div>
          </section>

          <section className="stack-xs">
            <div className="grok-studio-group-head">
              <span className="grok-studio-group-label">{t('faceLabel')}</span>
              <span className="grok-studio-group-val">{t(`faces.${spec.face}`)}</span>
            </div>
            <div
              className="grok-studio-tile-grid cols-4"
              role="radiogroup"
              aria-label={t('faceLabel')}
            >
              {GROKBOT_FACES.map((f: GrokBotFace, idx: number) => {
                const selected = spec.face === f;
                return (
                  <button
                    key={f}
                    ref={(el) => {
                      faceRefs.current[idx] = el;
                    }}
                    type="button"
                    role="radio"
                    aria-checked={selected}
                    tabIndex={selected ? 0 : -1}
                    className={`grok-studio-tile${selected ? ' on' : ''}`}
                    onClick={() => onSpecChange({ ...spec, face: f })}
                    onKeyDown={(e) =>
                      handleRadioKey(e, GROKBOT_FACES, spec.face, 4, faceRefs.current, (next) =>
                        onSpecChange({ ...spec, face: next }),
                      )
                    }
                  >
                    {selected && (
                      <span className="grok-studio-check" aria-hidden="true">
                        <Check size={11} strokeWidth={3} />
                      </span>
                    )}
                    <GrokBotStudioSvg
                      shape={spec.shape}
                      face={f}
                      color={spec.color}
                      variant="tile"
                      className="grok-tile-svg"
                    />
                    <span className="grok-studio-tile-label">{t(`faces.${f}`)}</span>
                  </button>
                );
              })}
            </div>
          </section>

          <section className="stack-xs">
            <div className="grok-studio-group-head">
              <span className="grok-studio-group-label">{t('colorLabel')}</span>
              <span className="grok-studio-group-val">{t(`colors.${spec.color}`)}</span>
            </div>
            <div
              className="grok-studio-color-grid"
              role="radiogroup"
              aria-label={t('colorLabel')}
            >
              {GROKBOT_COLORS.map((c: GrokBotColor, idx: number) => {
                const selected = spec.color === c;
                const p = GROKBOT_COLOR_PALETTES[c];
                const isLightSwatch = c === 'cream' || c === 'amber' || c === 'grey';
                return (
                  <button
                    key={c}
                    ref={(el) => {
                      colorRefs.current[idx] = el;
                    }}
                    type="button"
                    role="radio"
                    aria-checked={selected}
                    aria-label={t(`colors.${c}`)}
                    title={t(`colors.${c}`)}
                    tabIndex={selected ? 0 : -1}
                    className={`grok-studio-swatch${selected ? ' on' : ''}${isLightSwatch ? ' light-ink' : ''}`}
                    style={{
                      background: `radial-gradient(circle at 32% 28%, ${p.stops[0]} 0%, ${p.stops[1]} 32%, ${p.stops[2]} 72%, ${p.stops[3]} 100%)`,
                    }}
                    onClick={() => onSpecChange({ ...spec, color: c })}
                    onKeyDown={(e) =>
                      handleRadioKey(e, GROKBOT_COLORS, spec.color, 6, colorRefs.current, (next) =>
                        onSpecChange({ ...spec, color: next }),
                      )
                    }
                  >
                    {selected && <Check size={15} strokeWidth={3} aria-hidden="true" />}
                  </button>
                );
              })}
            </div>
          </section>
        </div>
      ) : (
        <div id={`${uid}-panel-move`} role="tabpanel" aria-labelledby={`${uid}-tab-move`} className="grok-studio-panel stack">
          <section className="stack-xs">
            <div className="grok-studio-group-head">
              <span className="grok-studio-group-label">{t('animationsLabel')}</span>
              <span className="grok-studio-group-val">{t(`moves.${move}`)}</span>
            </div>
            <div className="grok-studio-tile-grid cols-4" role="group" aria-label={t('animationsLabel')}>
              {GROKBOT_MOVES.map((m: GrokBotMove) => {
                const active = move === m;
                return (
                  <button
                    key={m}
                    type="button"
                    aria-pressed={active}
                    className={`grok-studio-tile${active ? ' on' : ''}`}
                    onClick={() => onMoveChange(active && m !== 'idle' ? 'idle' : m)}
                  >
                    {active && (
                      <span className="grok-studio-check" aria-hidden="true">
                        <Check size={11} strokeWidth={3} />
                      </span>
                    )}
                    <GrokBotStudioSvg
                      shape={spec.shape}
                      face={spec.face}
                      color={spec.color}
                      move={m}
                      variant="tile"
                      staticMovePreview
                      className="grok-tile-svg"
                    />
                    <span className="grok-studio-tile-label">{t(`moves.${m}`)}</span>
                  </button>
                );
              })}
            </div>
          </section>
        </div>
      )}
    </div>
  );
}
