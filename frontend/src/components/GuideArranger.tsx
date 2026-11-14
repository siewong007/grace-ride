import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ArrowDown, ArrowUp } from 'lucide-react';
import { post } from '../api';
import { GuideSummary, usePick } from '../pages/HelpGuides';
import { Button, ErrorBox, Pill, useAction } from './ui';

/**
 * Editor-only list for putting guides in the order members and staff see them.
 * Saves the whole order at once; cancelling leaves the server untouched.
 */
export default function GuideArranger({
  guides,
  onDone,
}: {
  guides: GuideSummary[];
  onDone: (saved: boolean) => void;
}) {
  const { t } = useTranslation('help');
  const pick = usePick();
  const { busy, error, run } = useAction();
  const [order, setOrder] = useState(guides);

  const move = (idx: number, delta: -1 | 1) =>
    setOrder((prev) => {
      const next = [...prev];
      const [g] = next.splice(idx, 1);
      next.splice(idx + delta, 0, g);
      return next;
    });

  const save = () =>
    run(async () => {
      await post('/help/guides/reorder', { ids: order.map((g) => g.id) });
      onDone(true);
    }, t('arrangeSaved'));

  return (
    <div className="stack">
      <p className="muted small">{t('arrangeHint')}</p>
      <ol className="list guide-arrange-list" aria-label={t('arrangeTitle')}>
        {order.map((g, idx) => {
          const title = pick(g.title_zh, g.title_en);
          return (
            <li key={g.id} className="item row nowrap">
              <span className="muted guide-arrange-pos" aria-hidden="true">{idx + 1}</span>
              <span className="grow min-w-0 ellipsis">{title}</span>
              {!g.published && <Pill tone="warning">{t('guideDraft')}</Pill>}
              <Pill tone="neutral">{g.audience === 'member' ? t('audienceMembers') : t('audienceStaff')}</Pill>
              <Button
                variant="ghost"
                size="sm"
                disabled={idx === 0}
                onClick={() => move(idx, -1)}
                aria-label={t('arrangeUp', { title })}
              >
                <ArrowUp size={16} />
              </Button>
              <Button
                variant="ghost"
                size="sm"
                disabled={idx === order.length - 1}
                onClick={() => move(idx, 1)}
                aria-label={t('arrangeDown', { title })}
              >
                <ArrowDown size={16} />
              </Button>
            </li>
          );
        })}
      </ol>
      <ErrorBox error={error} />
      <div className="row nowrap gap-sm">
        <Button variant="primary" onClick={save} disabled={busy}>
          {t('arrangeSave')}
        </Button>
        <Button variant="secondary" onClick={() => onDone(false)} disabled={busy}>
          {t('arrangeCancel')}
        </Button>
      </div>
    </div>
  );
}
