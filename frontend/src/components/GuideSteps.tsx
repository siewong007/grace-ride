import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Check, CheckCircle2, ExternalLink, ThumbsDown, ThumbsUp } from 'lucide-react';
import { GuideStep } from '../pages/HelpGuides';
import { Button, useAction, useToast } from './ui';
import { post } from '../api';

interface GuideStepsProps {
  steps: GuideStep[];
  slug?: string;
  guideId?: number;
  guideTitle?: string;
  lang?: 'en' | 'zh';
  readOnly?: boolean;
  onContactUs?: (prefilled: string) => void;
}

export default function GuideSteps({
  steps,
  slug,
  guideId,
  guideTitle = '',
  lang,
  readOnly = false,
  onContactUs,
}: GuideStepsProps) {
  const { t, i18n } = useTranslation('help');
  const activeLang = lang || (i18n.language.startsWith('zh') ? 'zh' : 'en');
  const pick = (zh: string, en: string) => (activeLang === 'zh' ? zh || en : en || zh);
  const toast = useToast();
  const { run, busy } = useAction();

  // Completed steps state
  const storageKey = slug ? `graceride_guide_${slug}_done` : null;
  const [completed, setCompleted] = useState<number[]>(() => {
    if (!storageKey) return [];
    try {
      const raw = localStorage.getItem(storageKey);
      return raw ? JSON.parse(raw) : [];
    } catch {
      return [];
    }
  });

  // Feedback state
  const [feedbackGiven, setFeedbackGiven] = useState<'yes' | 'no' | null>(null);

  // Sync completed steps with localStorage
  const toggleStep = (idx: number) => {
    if (readOnly) return;
    setCompleted((prev) => {
      const next = prev.includes(idx) ? prev.filter((i) => i !== idx) : [...prev, idx];
      if (storageKey) {
        try {
          localStorage.setItem(storageKey, JSON.stringify(next));
        } catch {
          // ignore
        }
      }
      return next;
    });
  };

  const markStepDone = (idx: number) => {
    if (readOnly) return;
    setCompleted((prev) => {
      if (prev.includes(idx)) return prev;
      const next = [...prev, idx];
      if (storageKey) {
        try {
          localStorage.setItem(storageKey, JSON.stringify(next));
        } catch {
          // ignore
        }
      }
      return next;
    });
  };

  // Scroll to hash on mount
  useEffect(() => {
    const hash = window.location.hash;
    if (hash && hash.startsWith('#step-')) {
      const el = document.querySelector(hash);
      el?.scrollIntoView({ behavior: 'smooth' });
    }
  }, []);

  const total = steps.length;
  const doneCount = completed.length;
  const progressPct = total > 0 ? Math.round((doneCount / total) * 100) : 0;

  const handleFeedback = (helpful: boolean) => {
    if (!guideId) {
      setFeedbackGiven(helpful ? 'yes' : 'no');
      return;
    }
    run(async () => {
      await post(`/help/guides/${guideId}/feedback`, { helpful });
      setFeedbackGiven(helpful ? 'yes' : 'no');
      toast(helpful ? t('feedbackThanks', 'Thank you for your feedback!') : t('feedbackSaved', 'Feedback received'));
    });
  };

  return (
    <div className="stack guide-steps-container">
      {/* Overall Progress Bar */}
      {total > 0 && (
        <div className="stack-xs" style={{ marginBottom: 'var(--space-3)' }}>
          <div className="row between nowrap tiny muted">
            <span>{t('guideProgress', { done: doneCount, total })}</span>
            <span>{progressPct}%</span>
          </div>
          <div className="guide-progress-bar">
            <div className="guide-progress-fill" style={{ width: `${progressPct}%` }} />
          </div>
        </div>
      )}

      {/* Stepper list */}
      <div className="guide-stepper" role="list">
        {steps.map((s, idx) => {
          const isDone = completed.includes(idx);
          const isLast = idx === total - 1;
          const stepTitle = pick(s.title_zh, s.title_en);
          const stepBody = pick(s.body_zh, s.body_en);

          return (
            <div
              key={idx}
              id={`step-${idx}`}
              className="guide-step-item"
              role="listitem"
            >
              {!isLast && <div className="guide-step-line" aria-hidden="true" />}
              <button
                type="button"
                className={`guide-step-indicator ${isDone ? 'done' : ''}`}
                onClick={() => toggleStep(idx)}
                aria-label={isDone ? t('markStepIncomplete', 'Mark step incomplete') : t('markStepComplete', 'Mark step complete')}
                disabled={readOnly}
              >
                {isDone ? <Check size={16} /> : idx + 1}
              </button>
              <div className="guide-step-body stack-xs">
                <div className="row between nowrap">
                  <h4 className="bold" style={{ margin: 0 }}>
                    {stepTitle || `${t('stepNumber', { n: idx + 1 })}`}
                  </h4>
                  {isDone && (
                    <span className="tiny tone-green row nowrap gap-xs">
                      <CheckCircle2 size={14} />
                      <span>{t('done', 'Done')}</span>
                    </span>
                  )}
                </div>

                {stepBody && (
                  <p className="muted pre-wrap" style={{ margin: 'var(--space-1) 0' }}>
                    {stepBody}
                  </p>
                )}

                {s.link && (
                  <div style={{ marginTop: 'var(--space-2)' }}>
                    <Link
                      to={s.link}
                      className="btn sm secondary row nowrap gap-xs"
                      onClick={() => markStepDone(idx)}
                    >
                      <ExternalLink size={14} />
                      <span>{t('guideOpen', 'Open this page')}</span>
                    </Link>
                  </div>
                )}
              </div>
            </div>
          );
        })}
      </div>

      {/* "Did this help?" Section at the end of the guide */}
      {!readOnly && (
        <div className="card tight center stack-xs" style={{ marginTop: 'var(--space-4)', background: 'var(--surface-2)' }}>
          {feedbackGiven === null ? (
            <>
              <p className="bold small">{t('didThisHelp', 'Did this help?')}</p>
              <div className="btn-row center">
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => handleFeedback(true)}
                  disabled={busy}
                  icon={<ThumbsUp size={14} />}
                >
                  {t('yes', 'Yes')}
                </Button>
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => handleFeedback(false)}
                  disabled={busy}
                  icon={<ThumbsDown size={14} />}
                >
                  {t('no', 'No')}
                </Button>
              </div>
            </>
          ) : feedbackGiven === 'yes' ? (
            <p className="tone-green bold small">{t('feedbackThanks', 'Thank you for your feedback!')}</p>
          ) : (
            <div className="stack-xs center">
              <p className="muted small">{t('sorryDidntHelp', 'Sorry this guide did not help.')}</p>
              {onContactUs && (
                <Button
                  variant="primary"
                  size="sm"
                  onClick={() => onContactUs(guideTitle ? `${guideTitle} (guide inquiry)` : '')}
                >
                  {t('contactUs')}
                </Button>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
