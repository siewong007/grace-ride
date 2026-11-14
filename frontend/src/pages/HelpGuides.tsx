import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { BookOpen, Pencil, Plus, Trash2 } from 'lucide-react';
import { del, useApi } from '../api';
import { useAuth } from '../auth';
import {
  Button,
  Card,
  EmptyState,
  ErrorBox,
  PageHeader,
  Pill,
  Spinner,
  useAction,
} from '../components/ui';

export interface GuideStep {
  title_en: string;
  title_zh: string;
  body_en: string;
  body_zh: string;
  link: string;
}

export interface GuideSummary {
  id: number;
  slug: string;
  title_en: string;
  title_zh: string;
  summary_en: string;
  summary_zh: string;
  published: boolean;
  step_count: number;
}

export interface GuideDetailData extends Omit<GuideSummary, 'step_count'> {
  steps: GuideStep[];
}

/** Pick the active language, falling back to the other when empty. */
export function usePick() {
  const { i18n } = useTranslation();
  return (zh: string, en: string) => (i18n.language.startsWith('zh') ? zh || en : en || zh);
}

export default function HelpGuides() {
  const { t } = useTranslation('help');
  const pick = usePick();
  const { can } = useAuth();
  const isEditor = can('admin_help', 'update');
  const { data, error, loading, reload } = useApi<GuideSummary[]>(
    `/help/guides${isEditor ? '?include_drafts=true' : ''}`,
  );
  const { busy, error: actErr, run } = useAction();

  const remove = (g: GuideSummary) => {
    if (!window.confirm(t('guideDeleteConfirm'))) return;
    run(async () => {
      await del(`/help/guides/${g.id}`);
      reload();
    });
  };

  return (
    <div className="stack">
      <PageHeader
        title={t('guideTitle')}
        subtitle={t('guideDesc')}
        actions={
          isEditor ? (
            <Button variant="secondary" icon={<Plus size={16} aria-hidden="true" />}>
              {t('guideNew')}
            </Button>
          ) : undefined
        }
      />
      <ErrorBox error={error} />
      {loading ? (
        <Spinner />
      ) : !data?.length ? (
        <EmptyState title={t('guideNone')} />
      ) : (
        <Card className="stack">
          <div className="list">
            {data.map((g) => (
              <div key={g.id} className="item">
                <Link to={`/help/guides/${g.slug}`} className="grow min-w-0">
                  <div className="bold ellipsis">
                    <BookOpen size={14} aria-hidden="true" /> {pick(g.title_zh, g.title_en)}
                  </div>
                  <div className="muted tiny">
                    {pick(g.summary_zh, g.summary_en)} · {t('guideSteps', { count: g.step_count })}
                  </div>
                </Link>
                <Pill tone={g.published ? 'success' : 'warning'}>
                  {g.published ? t('guidePublishedLabel') : t('guideDraft')}
                </Pill>
                {isEditor && (
                  <span className="btn-row">
                    <button className="btn sm secondary" type="button" aria-label={t('guideEdit')}>
                      <Pencil size={14} aria-hidden="true" />
                    </button>
                    <button
                      className="btn sm secondary"
                      type="button"
                      aria-label={t('guideDelete')}
                      disabled={busy}
                      onClick={() => remove(g)}
                    >
                      <Trash2 size={14} aria-hidden="true" />
                    </button>
                  </span>
                )}
              </div>
            ))}
          </div>
          <ErrorBox error={actErr} />
        </Card>
      )}
    </div>
  );
}
