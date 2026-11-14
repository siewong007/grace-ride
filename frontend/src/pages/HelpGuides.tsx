import { FormEvent, useState } from 'react';
import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { ArrowDown, ArrowUp, BookOpen, Pencil, Plus, Trash2 } from 'lucide-react';
import { del, post, put, useApi } from '../api';
import { useAuth } from '../auth';
import {
  Button,
  Card,
  EmptyState,
  ErrorBox,
  Field,
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

const emptyStep = (): GuideStep => ({ title_en: '', title_zh: '', body_en: '', body_zh: '', link: '' });

/** Create/edit form. Pass a guide to edit, or null to create. */
function GuideForm({ guide, onDone }: { guide: GuideDetailData | null; onDone: () => void }) {
  const { t } = useTranslation('help');
  const { busy, error, run } = useAction();
  const [f, setF] = useState({
    slug: guide?.slug ?? '',
    title_en: guide?.title_en ?? '',
    title_zh: guide?.title_zh ?? '',
    summary_en: guide?.summary_en ?? '',
    summary_zh: guide?.summary_zh ?? '',
    published: guide?.published ?? true,
    steps: guide?.steps ?? [emptyStep()],
  });

  const setStep = (i: number, patch: Partial<GuideStep>) =>
    setF((p) => ({ ...p, steps: p.steps.map((s, j) => (j === i ? { ...s, ...patch } : s)) }));
  const move = (i: number, delta: -1 | 1) =>
    setF((p) => {
      const steps = [...p.steps];
      const [s] = steps.splice(i, 1);
      steps.splice(i + delta, 0, s);
      return { ...p, steps };
    });

  const submit = (e: FormEvent) => {
    e.preventDefault();
    run(async () => {
      if (guide) await put(`/help/guides/${guide.id}`, f);
      else await post('/help/guides', f);
      onDone();
    }, t('guideSaved'));
  };

  return (
    <form className="card stack" onSubmit={submit}>
      <h3>{guide ? t('guideEdit') : t('guideNew')}</h3>
      <Field label={t('guideSlug')}>
        <input className="input" required value={f.slug} onChange={(e) => setF({ ...f, slug: e.target.value })} />
      </Field>
      <div className="grid cols-2">
        <Field label={t('guideTitleEn')}>
          <input className="input" value={f.title_en} onChange={(e) => setF({ ...f, title_en: e.target.value })} />
        </Field>
        <Field label={t('guideTitleZh')}>
          <input className="input" value={f.title_zh} onChange={(e) => setF({ ...f, title_zh: e.target.value })} />
        </Field>
      </div>
      <div className="grid cols-2">
        <Field label={t('guideSummaryEn')}>
          <input className="input" value={f.summary_en} onChange={(e) => setF({ ...f, summary_en: e.target.value })} />
        </Field>
        <Field label={t('guideSummaryZh')}>
          <input className="input" value={f.summary_zh} onChange={(e) => setF({ ...f, summary_zh: e.target.value })} />
        </Field>
      </div>
      <label className="check">
        <input
          type="checkbox"
          checked={f.published}
          onChange={(e) => setF({ ...f, published: e.target.checked })}
        />
        {t('guidePublishedLabel')}
      </label>

      {f.steps.map((s, i) => (
        <div key={i} className="card stack-xs">
          <div className="btn-row">
            <span className="bold">{i + 1}</span>
            <button type="button" className="btn sm secondary" disabled={i === 0} onClick={() => move(i, -1)} aria-label={t('guideMoveUp')}>
              <ArrowUp size={14} aria-hidden="true" />
            </button>
            <button type="button" className="btn sm secondary" disabled={i === f.steps.length - 1} onClick={() => move(i, 1)} aria-label={t('guideMoveDown')}>
              <ArrowDown size={14} aria-hidden="true" />
            </button>
            <button type="button" className="btn sm secondary" disabled={f.steps.length === 1} onClick={() => setF((p) => ({ ...p, steps: p.steps.filter((_, j) => j !== i) }))}>
              {t('guideRemoveStep')}
            </button>
          </div>
          <div className="grid cols-2">
            <Field label={t('guideStepTitleEn')}>
              <input className="input" value={s.title_en} onChange={(e) => setStep(i, { title_en: e.target.value })} />
            </Field>
            <Field label={t('guideStepTitleZh')}>
              <input className="input" value={s.title_zh} onChange={(e) => setStep(i, { title_zh: e.target.value })} />
            </Field>
          </div>
          <div className="grid cols-2">
            <Field label={t('guideStepBodyEn')}>
              <textarea className="input" rows={2} value={s.body_en} onChange={(e) => setStep(i, { body_en: e.target.value })} />
            </Field>
            <Field label={t('guideStepBodyZh')}>
              <textarea className="input" rows={2} value={s.body_zh} onChange={(e) => setStep(i, { body_zh: e.target.value })} />
            </Field>
          </div>
          <Field label={t('guideStepLink')}>
            <input className="input" value={s.link} placeholder="/admin/plan" onChange={(e) => setStep(i, { link: e.target.value })} />
          </Field>
        </div>
      ))}

      <Button variant="secondary" type="button" disabled={f.steps.length >= 30} onClick={() => setF((p) => ({ ...p, steps: [...p.steps, emptyStep()] }))}>
        {t('guideAddStep')}
      </Button>
      <ErrorBox error={error} />
      <div className="btn-row">
        <Button variant="primary" type="submit" loading={busy}>{t('guideSave')}</Button>
        <Button variant="secondary" type="button" onClick={onDone}>
          {t('guideCancel')}
        </Button>
      </div>
    </form>
  );
}

export default function HelpGuides() {
  const { t } = useTranslation('help');
  const pick = usePick();
  const { can } = useAuth();
  const isEditor = can('admin_help', 'update');
  const { data, error, loading, reload } = useApi<GuideSummary[]>(
    `/help/guides${isEditor ? '?include_drafts=true' : ''}`,
  );
  // Editing loads the full guide (steps); creating passes null.
  const [editSlug, setEditSlug] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const { data: editing } = useApi<GuideDetailData>(editSlug ? `/help/guides/${editSlug}` : null);
  const { busy, error: actErr, run } = useAction();

  const remove = (g: GuideSummary) => {
    if (!window.confirm(t('guideDeleteConfirm'))) return;
    run(async () => {
      await del(`/help/guides/${g.id}`);
      reload();
    });
  };

  if (creating || editSlug)
    return (
      <div className="stack">
        {/* Wait for the guide to load before showing the form. */}
        {editSlug && !editing ? <Spinner /> : (
          <GuideForm
            key={editSlug ?? 'new'}
            guide={editSlug ? editing : null}
            onDone={() => {
              setCreating(false);
              setEditSlug(null);
              reload();
            }}
          />
        )}
      </div>
    );

  return (
    <div className="stack">
      <PageHeader
        title={t('guideTitle')}
        subtitle={t('guideDesc')}
        actions={
          isEditor ? (
            <Button variant="secondary" icon={<Plus size={16} aria-hidden="true" />} onClick={() => setCreating(true)}>
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
                    <button className="btn sm secondary" type="button" aria-label={t('guideEdit')} onClick={() => setEditSlug(g.slug)}>
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
