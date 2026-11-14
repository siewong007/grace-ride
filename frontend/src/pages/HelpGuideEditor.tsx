import { FormEvent, useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import {
  ArrowDown,
  ArrowUp,
  Copy,
  Eye,
  Play,
  Plus,
  Trash2,
} from 'lucide-react';
import { del, post, put, useApi } from '../api';
import GuideSteps from '../components/GuideSteps';
import { useGuideRunner } from '../components/GuideRunner';
import { GuideDetailData, GuideStep } from './HelpGuides';
import {
  Button,
  Card,
  CardSkeleton,
  ErrorBox,
  Field,
  Modal,
  Page,
  PageHeader,
  useAction,
  useConfirm,
  useToast,
} from '../components/ui';

const APP_PAGES = [
  { label: 'None', value: '' },
  { label: 'Home', value: '/' },
  { label: 'Rides (My rides)', value: '/rides' },
  { label: 'Request a ride', value: '/rides/new' },
  { label: 'Drive schedule', value: '/drive' },
  { label: 'Events', value: '/events' },
  { label: 'Cost splits', value: '/settlements' },
  { label: 'Reimbursements', value: '/reimbursements' },
  { label: 'My Report', value: '/report' },
  { label: 'Profile / Account Security', value: '/me' },
  { label: 'Admin Dashboard', value: '/admin' },
  { label: 'Admin Ride Management', value: '/admin/rides' },
  { label: 'Admin Live Map', value: '/admin/live' },
  { label: 'Admin Announcements', value: '/admin/announcements' },
  { label: 'Admin Reimbursement Review', value: '/admin/reimbursements' },
  { label: 'Admin Member Reports', value: '/admin/reports' },
  { label: 'Admin Members & Invites', value: '/admin/members' },
  { label: 'Admin Roles & Permissions', value: '/admin/roles' },
  { label: 'Admin Settings (Groups & Areas)', value: '/admin/settings' },
  { label: 'Admin Audit Log', value: '/admin/audit' },
];

const emptyStep = (): GuideStep => ({
  title_en: '',
  title_zh: '',
  body_en: '',
  body_zh: '',
  link: '',
});

function slugify(text: string): string {
  return text
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/(^-|-$)/g, '');
}

export default function HelpGuideEditor() {
  const { t } = useTranslation('help');
  const { t: tc } = useTranslation();
  const { slug } = useParams();
  const nav = useNavigate();
  const toast = useToast();
  const confirm = useConfirm();
  const { startWalkthrough } = useGuideRunner();
  const { busy, error, run } = useAction();

  const isNew = !slug;
  const { data: initialGuide, loading } = useApi<GuideDetailData>(
    slug ? `/help/guides/${slug}` : null,
  );

  const [formLang, setFormLang] = useState<'en' | 'zh'>('en');
  const [previewLang, setPreviewLang] = useState<'en' | 'zh'>('en');

  const [f, setF] = useState({
    slug: '',
    title_en: '',
    title_zh: '',
    summary_en: '',
    summary_zh: '',
    published: false,
    audience: 'member' as 'member' | 'staff',
    steps: [emptyStep()],
    sort_order: 0,
  });

  const [autoSlug, setAutoSlug] = useState(isNew);
  const [isDirty, setIsDirty] = useState(false);
  const [checklistOpen, setChecklistOpen] = useState(false);
  // Steps whose link is being typed by hand ("Custom path…" chosen in the picker).
  const [customLinks, setCustomLinks] = useState<Set<number>>(new Set());
  const setCustom = (idx: number, on: boolean) =>
    setCustomLinks((prev) => {
      const next = new Set(prev);
      if (on) next.add(idx);
      else next.delete(idx);
      return next;
    });

  // Populate from fetched guide
  useEffect(() => {
    if (initialGuide) {
      setF({
        slug: initialGuide.slug,
        title_en: initialGuide.title_en,
        title_zh: initialGuide.title_zh,
        summary_en: initialGuide.summary_en,
        summary_zh: initialGuide.summary_zh,
        published: initialGuide.published,
        audience: initialGuide.audience,
        steps: initialGuide.steps?.length ? initialGuide.steps : [emptyStep()],
        sort_order: (initialGuide as { sort_order?: number }).sort_order ?? 0,
      });
      setAutoSlug(false);
    }
  }, [initialGuide]);

  // Unsaved changes warning
  useEffect(() => {
    const handleBeforeUnload = (e: BeforeUnloadEvent) => {
      if (isDirty) {
        e.preventDefault();
        e.returnValue = '';
      }
    };
    window.addEventListener('beforeunload', handleBeforeUnload);
    return () => window.removeEventListener('beforeunload', handleBeforeUnload);
  }, [isDirty]);

  if (!isNew && loading && !initialGuide) {
    return (
      <Page width="wide">
        <CardSkeleton count={3} />
      </Page>
    );
  }

  const handleTitleEnChange = (val: string) => {
    setIsDirty(true);
    setF((prev) => {
      const nextSlug = autoSlug ? slugify(val) : prev.slug;
      return { ...prev, title_en: val, slug: nextSlug };
    });
  };

  const setStep = (idx: number, patch: Partial<GuideStep>) => {
    setIsDirty(true);
    setF((prev) => ({
      ...prev,
      steps: prev.steps.map((s, j) => (j === idx ? { ...s, ...patch } : s)),
    }));
  };

  const moveStep = (idx: number, delta: -1 | 1) => {
    setCustomLinks(new Set());
    setIsDirty(true);
    setF((prev) => {
      const steps = [...prev.steps];
      const [item] = steps.splice(idx, 1);
      steps.splice(idx + delta, 0, item);
      return { ...prev, steps };
    });
  };

  const removeStep = (idx: number) => {
    setCustomLinks(new Set());
    setIsDirty(true);
    setF((prev) => ({
      ...prev,
      steps: prev.steps.filter((_, j) => j !== idx),
    }));
    toast(t('stepRemoved', 'Step removed'), 'info');
  };

  const addStep = () => {
    setIsDirty(true);
    setF((prev) => ({ ...prev, steps: [...prev.steps, emptyStep()] }));
  };

  // Checklist validation
  const missingEn = !f.title_en.trim() || !f.summary_en.trim() || f.steps.some((s) => !s.title_en.trim());
  const missingZh = !f.title_zh.trim() || !f.summary_zh.trim() || f.steps.some((s) => !s.title_zh.trim());
  const gaps: string[] = [];
  if (!f.title_en.trim()) gaps.push('English title is missing');
  if (!f.title_zh.trim()) gaps.push('Chinese title is missing');
  if (!f.summary_en.trim()) gaps.push('English summary is missing');
  if (!f.summary_zh.trim()) gaps.push('Chinese summary is missing');
  f.steps.forEach((s, i) => {
    if (!s.title_en.trim()) gaps.push(`Step ${i + 1} English title is missing`);
    if (!s.title_zh.trim()) gaps.push(`Step ${i + 1} Chinese title is missing`);
  });

  const saveGuide = async (shouldPublish?: boolean) => {
    const payload = {
      ...f,
      published: shouldPublish !== undefined ? shouldPublish : f.published,
    };

    run(async () => {
      if (initialGuide) {
        await put(`/help/guides/${initialGuide.id}`, payload);
      } else {
        await post('/help/guides', payload);
      }
      setIsDirty(false);
      toast(t('guideSaved'));
      nav(`/help/guides/${payload.slug}`);
    });
  };

  const handlePublishClick = () => {
    if (gaps.length > 0) {
      setChecklistOpen(true);
    } else {
      saveGuide(true);
    }
  };

  const handleDelete = async () => {
    if (!initialGuide) return;
    const ok = await confirm({
      title: t('guideDelete'),
      body: t('guideDeleteConfirm'),
      confirmLabel: t('guideDelete'),
      tone: 'danger',
    });
    if (!ok) return;

    run(async () => {
      await del(`/help/guides/${initialGuide.id}`);
      toast(t('guideDeleted', 'Guide deleted'));
      nav('/help?tab=guides');
    });
  };

  const handleDuplicate = () => {
    const duplicatedSlug = `${f.slug}-copy`;
    setF((prev) => ({
      ...prev,
      slug: duplicatedSlug,
      title_en: `${prev.title_en} (Copy)`,
      title_zh: `${prev.title_zh} (Copy)`,
      published: false,
    }));
    setIsDirty(true);
    toast(t('guideDuplicated', 'Duplicated as new draft'));
  };

  const testWalkthrough = () => {
    startWalkthrough({
      id: initialGuide?.id ?? 0,
      slug: f.slug,
      title_en: f.title_en,
      title_zh: f.title_zh,
      summary_en: f.summary_en,
      summary_zh: f.summary_zh,
      published: f.published,
      audience: f.audience,
      steps: f.steps,
    });
  };

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    saveGuide();
  };

  return (
    <Page width="wide">
      <form onSubmit={handleSubmit} className="stack">
        <PageHeader
          title={isNew ? t('guideNew') : t('guideEditPage')}
          back="/help?tab=guides"
          actions={
            <div className="btn-row nowrap align-center">
              <Button
                variant="ghost"
                size="sm"
                type="button"
                onClick={testWalkthrough}
                icon={<Play size={14} />}
              >
                {t('testWalkthrough', 'Test walkthrough')}
              </Button>
              {!isNew && (
                <Button
                  variant="ghost"
                  size="sm"
                  type="button"
                  onClick={handleDuplicate}
                  icon={<Copy size={14} />}
                >
                  {t('duplicate', 'Duplicate')}
                </Button>
              )}
              {!isNew && (
                <Button
                  variant="danger"
                  size="sm"
                  type="button"
                  onClick={handleDelete}
                  icon={<Trash2 size={14} />}
                >
                  {t('guideDelete')}
                </Button>
              )}
            </div>
          }
        />

        {/* Stage Save Bar */}
        <div className="card tight row between nowrap align-center" style={{ background: 'var(--surface-2)' }}>
          <div className="row nowrap gap-sm align-center">
            <span className="bold small">
              {f.published ? t('statusPublished', 'Stage: Published') : t('statusDraft', 'Stage: Draft')}
            </span>
            <label className="check" style={{ margin: 0 }}>
              <input
                type="checkbox"
                checked={f.published}
                onChange={(e) => {
                  setIsDirty(true);
                  setF({ ...f, published: e.target.checked });
                }}
              />
              <span>{t('guidePublishedLabel')}</span>
            </label>
          </div>
          <div className="btn-row nowrap">
            <Button variant="secondary" type="submit" loading={busy}>
              {t('saveDraft', 'Save draft')}
            </Button>
            {!f.published && (
              <Button
                variant="primary"
                type="button"
                onClick={handlePublishClick}
                disabled={busy}
              >
                {t('publish', 'Publish…')}
              </Button>
            )}
          </div>
        </div>

        <ErrorBox error={error} />

        {/* 2-column layout on wide screens: form left, live preview right */}
        <div className="grid cols-2" style={{ alignItems: 'start' }}>
          {/* Left Column: Form */}
          <div className="stack">
            {/* Basics */}
            <Card className="stack">
              <h3>{t('guideBasics', 'Basics')}</h3>

              <Field label={t('guideSlug')}>
                <input
                  className="input"
                  required
                  value={f.slug}
                  onChange={(e) => {
                    setIsDirty(true);
                    setAutoSlug(false);
                    setF({ ...f, slug: slugify(e.target.value) });
                  }}
                />
              </Field>

              <div className="field">
                <span id="guide-audience-label">{t('guideAudience')}</span>
                <div className="segmented-control" role="group" aria-labelledby="guide-audience-label">
                  <button
                    type="button"
                    className={`segmented-item ${f.audience === 'member' ? 'active' : ''}`}
                    aria-pressed={f.audience === 'member'}
                    onClick={() => {
                      setIsDirty(true);
                      setF({ ...f, audience: 'member' });
                    }}
                  >
                    {t('audienceMembers')}
                  </button>
                  <button
                    type="button"
                    className={`segmented-item ${f.audience === 'staff' ? 'active' : ''}`}
                    aria-pressed={f.audience === 'staff'}
                    onClick={() => {
                      setIsDirty(true);
                      setF({ ...f, audience: 'staff' });
                    }}
                  >
                    {t('audienceStaff')}
                  </button>
                </div>
              </div>

              {/* Language Switch for Content Fields */}
              <div className="stack-xs" style={{ marginTop: 'var(--space-2)' }}>
                <div className="row between nowrap">
                  <span className="bold small">{t('contentLanguage', 'Content language')}</span>
                  <div className="segmented-control">
                    <button
                      type="button"
                      className={`segmented-item ${formLang === 'en' ? 'active' : ''}`}
                      onClick={() => setFormLang('en')}
                    >
                      <span>English</span>
                      {missingEn && <span className="dot" style={{ background: 'var(--danger)' }} />}
                    </button>
                    <button
                      type="button"
                      className={`segmented-item ${formLang === 'zh' ? 'active' : ''}`}
                      onClick={() => setFormLang('zh')}
                    >
                      <span>{t('langZh')}</span>
                      {missingZh && <span className="dot" style={{ background: 'var(--danger)' }} />}
                    </button>
                  </div>
                </div>

                {formLang === 'en' ? (
                  <>
                    <Field label={t('guideTitleEn')}>
                      <input
                        className="input"
                        required
                        value={f.title_en}
                        onChange={(e) => handleTitleEnChange(e.target.value)}
                        placeholder="e.g. Plan transport for an event"
                      />
                    </Field>
                    <Field label={t('guideSummaryEn')}>
                      <textarea
                        className="input"
                        rows={2}
                        value={f.summary_en}
                        onChange={(e) => {
                          setIsDirty(true);
                          setF({ ...f, summary_en: e.target.value });
                        }}
                        placeholder="Brief summary shown on guide card"
                      />
                    </Field>
                  </>
                ) : (
                  <>
                    <Field label={t('guideTitleZh')}>
                      <input
                        className="input"
                        value={f.title_zh}
                        onChange={(e) => {
                          setIsDirty(true);
                          setF({ ...f, title_zh: e.target.value });
                        }}
                        placeholder={t('titleZhPlaceholder')}
                      />
                    </Field>
                    <Field label={t('guideSummaryZh')}>
                      <textarea
                        className="input"
                        rows={2}
                        value={f.summary_zh}
                        onChange={(e) => {
                          setIsDirty(true);
                          setF({ ...f, summary_zh: e.target.value });
                        }}
                        placeholder={t('summaryZhPlaceholder')}
                      />
                    </Field>
                  </>
                )}
              </div>
            </Card>

            {/* Steps */}
            <Card className="stack">
              <div className="row between nowrap">
                <h3>{t('guideStepsTitle', 'Workflow Steps')} ({f.steps.length})</h3>
                <Button variant="secondary" size="sm" type="button" onClick={addStep} icon={<Plus size={14} />}>
                  {t('guideAddStep')}
                </Button>
              </div>

              <div className="stack">
                {f.steps.map((s, idx) => (
                  <div key={idx} className="card tight stack-xs" style={{ background: 'var(--surface-2)' }}>
                    <div className="row between nowrap">
                      <span className="bold small ellipsis">
                        {idx + 1}. {(formLang === 'en' ? s.title_en : s.title_zh) || t('untitledStep')}
                      </span>
                      <div className="row nowrap gap-xs">
                        <Button
                          variant="ghost"
                          size="sm"
                          type="button"
                          disabled={idx === 0}
                          onClick={() => moveStep(idx, -1)}
                          aria-label={t('guideMoveUp')}
                        >
                          <ArrowUp size={14} />
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          type="button"
                          disabled={idx === f.steps.length - 1}
                          onClick={() => moveStep(idx, 1)}
                          aria-label={t('guideMoveDown')}
                        >
                          <ArrowDown size={14} />
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          type="button"
                          disabled={f.steps.length === 1}
                          onClick={() => removeStep(idx)}
                          aria-label={t('guideRemoveStep')}
                        >
                          <Trash2 size={14} />
                        </Button>
                      </div>
                    </div>

                    {formLang === 'en' ? (
                      <>
                        <Field label={t('guideStepTitleEn')}>
                          <input
                            className="input"
                            value={s.title_en}
                            onChange={(e) => setStep(idx, { title_en: e.target.value })}
                            placeholder="Step action, e.g. Open the planning board"
                          />
                        </Field>
                        <Field label={t('guideStepBodyEn')}>
                          <textarea
                            className="input"
                            rows={2}
                            value={s.body_en}
                            onChange={(e) => setStep(idx, { body_en: e.target.value })}
                            placeholder="Detailed instructions for this step"
                          />
                        </Field>
                      </>
                    ) : (
                      <>
                        <Field label={t('guideStepTitleZh')}>
                          <input
                            className="input"
                            value={s.title_zh}
                            onChange={(e) => setStep(idx, { title_zh: e.target.value })}
                            placeholder={t('stepTitleZhPlaceholder')}
                          />
                        </Field>
                        <Field label={t('guideStepBodyZh')}>
                          <textarea
                            className="input"
                            rows={2}
                            value={s.body_zh}
                            onChange={(e) => setStep(idx, { body_zh: e.target.value })}
                            placeholder={t('stepBodyZhPlaceholder')}
                          />
                        </Field>
                      </>
                    )}

                    {/* Link Picker */}
                    <Field label={t('guideStepLink')}>
                      <div className="stack-xs">
                        <select
                          className="input"
                          value={
                            !customLinks.has(idx) && APP_PAGES.some((p) => p.value === s.link)
                              ? s.link
                              : 'custom'
                          }
                          onChange={(e) => {
                            const val = e.target.value;
                            setCustom(idx, val === 'custom');
                            if (val !== 'custom') setStep(idx, { link: val });
                          }}
                        >
                          {APP_PAGES.map((p) => (
                            <option key={p.value} value={p.value}>
                              {p.label} {p.value ? `(${p.value})` : ''}
                            </option>
                          ))}
                          <option value="custom">{t('customPath', 'Custom path…')}</option>
                        </select>

                        {(customLinks.has(idx) || !APP_PAGES.some((p) => p.value === s.link)) && (
                          <input
                            className="input"
                            aria-label={t('customPath', 'Custom path…')}
                            value={s.link}
                            placeholder="/admin/rides"
                            onChange={(e) => setStep(idx, { link: e.target.value })}
                          />
                        )}
                      </div>
                    </Field>
                  </div>
                ))}
              </div>
            </Card>
          </div>

          {/* Right Column: Live Preview */}
          <div className="stack" style={{ position: 'sticky', top: 20 }}>
            <Card className="stack">
              <div className="row between nowrap">
                <div className="row nowrap gap-xs align-center">
                  <Eye size={18} className="muted" />
                  <h3 style={{ margin: 0 }}>{t('livePreview', 'Live Preview')}</h3>
                </div>
                <div className="segmented-control">
                  <button
                    type="button"
                    className={`segmented-item ${previewLang === 'en' ? 'active' : ''}`}
                    onClick={() => setPreviewLang('en')}
                  >
                    English
                  </button>
                  <button
                    type="button"
                    className={`segmented-item ${previewLang === 'zh' ? 'active' : ''}`}
                    onClick={() => setPreviewLang('zh')}
                  >
                    {t('langZh')}
                  </button>
                </div>
              </div>

              <div className="stack-xs" style={{ borderBottom: '1px solid var(--border)', paddingBottom: 'var(--space-3)' }}>
                <h2>{previewLang === 'en' ? f.title_en || 'Guide Title' : f.title_zh || t('previewTitleZh')}</h2>
                <p className="muted small">
                  {previewLang === 'en' ? f.summary_en || 'Guide summary will appear here.' : f.summary_zh || t('previewSummaryZh')}
                </p>
                <div className="row nowrap gap-xs">
                  <span className="pill tone-blue tiny">{f.audience === 'member' ? t('audienceMembers') : t('audienceStaff')}</span>
                  {!f.published && <span className="pill tone-warning tiny">{t('guideDraft')}</span>}
                </div>
              </div>

              <GuideSteps
                steps={f.steps}
                lang={previewLang}
                readOnly
              />
            </Card>
          </div>
        </div>

        {/* Publish Checklist Modal */}
        {checklistOpen && (
          <Modal
            title={t('publishChecklist', 'Publish Checklist')}
            onClose={() => setChecklistOpen(false)}
            size="md"
          >
            <div className="stack">
              <p className="muted">
                {gaps.length === 0
                  ? t('checklistAllComplete', 'All checklist items complete!')
                  : t('checklistHasGaps', 'This guide has missing fields. We recommend completing both languages before publishing:')}
              </p>

              {gaps.length > 0 && (
                <div className="card tight notice warning stack-xs">
                  {gaps.map((gap, i) => (
                    <div key={i} className="small">
                      • {gap}
                    </div>
                  ))}
                </div>
              )}

              <div className="btn-row">
                <Button
                  variant="primary"
                  onClick={() => {
                    setChecklistOpen(false);
                    saveGuide(true);
                  }}
                >
                  {gaps.length > 0 ? t('publishAnyway', 'Publish anyway') : t('publishNow', 'Publish now')}
                </Button>
                <Button variant="secondary" onClick={() => setChecklistOpen(false)}>
                  {tc('cancel')}
                </Button>
              </div>
            </div>
          </Modal>
        )}
      </form>
    </Page>
  );
}
