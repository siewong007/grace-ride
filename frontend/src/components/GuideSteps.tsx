import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AlertTriangle, Check, CheckCircle2, ChevronRight, Info, Lightbulb, ThumbsDown, ThumbsUp } from 'lucide-react';
import { GuideStep } from '../pages/HelpGuides';
import { Button, useAction, useToast } from './ui';
import { post } from '../api';
import StepText, { inline } from './StepText';
import { readDone, readFeedback, writeDone, writeFeedback } from '../guideProgress';

/** The app page a step links to, named as the menu names it ("My rides", "Events"). */
const PAGE_LABELS: [string, string][] = [
  ['/rides', 'myRides'], ['/events', 'events'], ['/costs', 'myCosts'], ['/reimbursements', 'reimbursements'],
  ['/report', 'myReport'], ['/help', 'help'], ['/drive', 'drive'], ['/settlements', 'settlements'], ['/me', 'profile'],
  ['/admin/live', 'liveMap'], ['/admin/rides', 'adminRides'], ['/admin/reimbursements', 'reimbReview'],
  ['/admin/members', 'members'], ['/admin/settings', 'settings'], ['/admin/audit', 'audit'], ['/admin', 'dashboard'],
];
const pageKey = (link: string) => PAGE_LABELS.find(([p]) => link === p || link.startsWith(`${p}/`) || link.startsWith(`${p}?`))?.[1];

interface GuideStepsProps {
  steps: GuideStep[];
  slug?: string;
  guideId?: number;
  guideTitle?: string;
  lang?: 'en' | 'zh';
  readOnly?: boolean;
  onContactUs?: (prefilled: string) => void;
  /** Called when someone follows a step's link, so the guide can travel with them as a walkthrough. */
  onFollow?: (stepIndex: number) => void;
  /** The guide to offer after this one is finished. */
  nextGuide?: { slug: string; title: string } | null;
}

export default function GuideSteps({
  steps,
  slug,
  guideId,
  guideTitle = '',
  lang,
  readOnly = false,
  onContactUs,
  onFollow,
  nextGuide,
}: GuideStepsProps) {
  const { t, i18n } = useTranslation('help');
  const { t: tl } = useTranslation('layout');
  const activeLang = lang || (i18n.language.startsWith('zh') ? 'zh' : 'en');
  const pick = (zh: string, en: string) => (activeLang === 'zh' ? zh || en : en || zh);
  const toast = useToast();
  const { run, busy } = useAction();
  const total = steps.length;

  const [completed, setCompleted] = useState<number[]>(() => (slug ? readDone(slug, steps) : []));
  const [feedbackGiven, setFeedbackGiven] = useState<'yes' | 'no' | null>(() => (slug ? readFeedback(slug) : null));

  const update = (next: number[]) => {
    setCompleted(next);
    if (slug) writeDone(slug, steps, next);
  };
  const toggleStep = (idx: number) => {
    if (readOnly) return;
    update(completed.includes(idx) ? completed.filter((i) => i !== idx) : [...completed, idx]);
  };

  // Scroll to #step-N on arrival (search results link straight to a step).
  useEffect(() => {
    const hash = window.location.hash;
    if (hash && hash.startsWith('#step-')) document.querySelector(hash)?.scrollIntoView({ behavior: 'smooth' });
  }, []);

  const doneCount = completed.length;
  const current = steps.findIndex((_, i) => !completed.includes(i));
  const finished = total > 0 && current === -1;
  const progressPct = total > 0 ? Math.round((doneCount / total) * 100) : 0;
  const progressLabel = t('stepsDone', { done: doneCount, total });

  const handleFeedback = (helpful: boolean) => {
    const answer = helpful ? 'yes' : 'no';
    const remember = () => {
      setFeedbackGiven(answer);
      if (slug) writeFeedback(slug, answer);
    };
    if (!guideId) return remember();
    run(async () => {
      await post(`/help/guides/${guideId}/feedback`, { helpful });
      remember();
      toast(helpful ? t('feedbackThanks', 'Thank you for your feedback!') : t('feedbackSaved', 'Feedback received'));
    });
  };

  return (
    <div className="stack guide-steps-container">
      {total > 0 && (
        <div className="stack-xs">
          <span className="small muted">{progressLabel}</span>
          <div
            className="guide-progress-bar"
            role="progressbar"
            aria-label={progressLabel}
            aria-valuemin={0}
            aria-valuemax={total}
            aria-valuenow={doneCount}
          >
            <div className="guide-progress-fill" style={{ width: `${progressPct}%` }} />
          </div>
        </div>
      )}

      {finished && !readOnly && (
        <div className="guide-finish stack-xs" role="status">
          <CheckCircle2 size={32} aria-hidden="true" />
          <p className="bold">{t('guideFinished')}</p>
          <div className="btn-row">
            {nextGuide && (
              <Link to={`/help/guides/${nextGuide.slug}`} className="btn sm primary">
                {t('nextGuide', { title: nextGuide.title })}
              </Link>
            )}
            <button type="button" className="btn sm secondary" onClick={() => update([])}>
              {t('startAgain')}
            </button>
          </div>
        </div>
      )}

      <ol className="guide-stepper" style={{ listStyle: 'none', margin: 0 }}>
        {steps.map((s, idx) => {
          const isDone = completed.includes(idx);
          const isCurrent = idx === current && !readOnly;
          const isLast = idx === total - 1;
          const stepTitle = pick(s.title_zh, s.title_en);
          const stepBody = pick(s.body_zh, s.body_en);
          const stepNote = pick(s.note_zh ?? '', s.note_en ?? '');
          const noteClass = s.note_kind === 'warning' || s.note_kind === 'important' ? 'warning' : s.note_kind === 'note' ? 'note' : 'tip';
          const page = s.link ? pageKey(s.link) : undefined;

          return (
            <li
              key={s.id || idx}
              id={`step-${idx}`}
              className={`guide-step-item ${isDone ? 'done' : ''} ${isCurrent ? 'current' : ''}`}
              aria-current={isCurrent ? 'step' : undefined}
            >
              {!isLast && <div className="guide-step-line" aria-hidden="true" />}
              <button
                type="button"
                className={`guide-step-indicator ${isDone ? 'done' : ''}`}
                onClick={() => toggleStep(idx)}
                aria-pressed={isDone}
                aria-label={isDone ? t('unmarkStepN', { n: idx + 1 }) : t('markStepN', { n: idx + 1 })}
                disabled={readOnly}
              >
                {isDone ? <Check size={16} aria-hidden="true" /> : idx + 1}
              </button>
              <div className="guide-step-body stack-xs">
                {isCurrent && doneCount > 0 && <span className="guide-step-kicker">{t('upNext')}</span>}
                <h4 className="bold guide-step-title">{stepTitle || t('stepNumber', { n: idx + 1 })}</h4>
                {stepBody && <StepText text={stepBody} className="guide-step-text" />}
                {stepNote && (
                  <p className={`guide-note ${noteClass}`}>
                    {noteClass === 'warning' ? (
                      <AlertTriangle size={16} aria-hidden="true" />
                    ) : noteClass === 'note' ? (
                      <Info size={16} aria-hidden="true" />
                    ) : (
                      <Lightbulb size={16} aria-hidden="true" />
                    )}
                    <span>{inline(stepNote)}</span>
                  </p>
                )}
                {s.image_file_id && (
                  <a
                    href={`/api/files/${s.image_file_id}`}
                    target="_blank"
                    rel="noreferrer"
                    className="guide-step-img-link"
                  >
                    <img
                      src={`/api/files/${s.image_file_id}`}
                      alt={stepTitle || t('stepNumber', { n: idx + 1 })}
                      className="guide-step-img"
                      loading="lazy"
                    />
                  </a>
                )}
                {s.link && (
                  <div style={{ marginTop: 'var(--space-2)' }}>
                    <Link
                      to={s.link}
                      className={`guide-step-link ${isCurrent ? 'primary' : ''}`}
                      onClick={(e) => {
                        if (onFollow && !readOnly) {
                          e.preventDefault();
                          onFollow(idx);
                        }
                      }}
                    >
                      <span>{page ? t('goToPage', { page: tl(page) }) : t('guideOpen', 'Open this page')}</span>
                      <ChevronRight size={16} aria-hidden="true" />
                    </Link>
                  </div>
                )}
              </div>
            </li>
          );
        })}
      </ol>

      {!readOnly && (
        <div className="guide-feedback">
          {feedbackGiven === null ? (
            <>
              <span className="small bold">{t('didThisHelp', 'Did this guide help?')}</span>
              <div className="btn-row">
                <Button variant="secondary" size="sm" onClick={() => handleFeedback(true)} disabled={busy} icon={<ThumbsUp size={14} />}>
                  {t('yes', 'Yes')}
                </Button>
                <Button variant="secondary" size="sm" onClick={() => handleFeedback(false)} disabled={busy} icon={<ThumbsDown size={14} />}>
                  {t('no', 'No')}
                </Button>
              </div>
            </>
          ) : feedbackGiven === 'yes' ? (
            <span className="small bold" style={{ color: 'var(--success)' }}>{t('feedbackThanks', 'Thank you for your feedback!')}</span>
          ) : (
            <>
              <span className="muted small">{t('sorryDidntHelp', 'Sorry this guide did not help.')}</span>
              {onContactUs && (
                <Button variant="secondary" size="sm" onClick={() => onContactUs(guideTitle ? `${guideTitle} (guide inquiry)` : '')}>
                  {t('contactUs')}
                </Button>
              )}
            </>
          )}
        </div>
      )}
    </div>
  );
}
