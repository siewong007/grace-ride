import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { BookOpen, ChevronRight } from 'lucide-react';
import { guideStatus } from '../guideProgress';
import type { GuideSummary } from '../types/help';
import { usePick } from '../hooks/usePick';
import { Pill } from './ui';

/** One guide in a list: the same row for members and staff. */
export default function GuideListRow({ guide: g, showSummary = true }: { guide: GuideSummary; showSummary?: boolean }) {
  const { t } = useTranslation('help');
  const pick = usePick();
  const status = guideStatus(g.slug, g.step_count);
  const summary = pick(g.summary_zh, g.summary_en);
  return (
    <Link to={`/help/guides/${g.slug}`} className="help-row">
      <span className="help-row-icon" aria-hidden="true"><BookOpen size={18} /></span>
      <span className="grow min-w-0">
        <span className="block clamp-2 bold">{pick(g.title_zh, g.title_en)}</span>
        {showSummary && summary && <span className="muted small block clamp-2">{summary}</span>}
        <span className="muted small block">
          {status.state === 'started' ? t('guideProgress', { done: status.done, total: status.total }) : t('guideSteps', { count: status.total })}
        </span>
      </span>
      {!g.published && <Pill tone="warning">{t('guideDraft')}</Pill>}
      {status.state === 'finished' && <Pill tone="success">{t('done')}</Pill>}
      <ChevronRight size={18} className="muted" aria-hidden="true" />
    </Link>
  );
}
