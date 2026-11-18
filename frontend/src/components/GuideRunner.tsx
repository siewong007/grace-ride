import { createContext, ReactNode, useContext, useEffect, useState } from 'react';
import { useLocation, useNavigate, Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AlertTriangle, ChevronDown, ChevronUp, ExternalLink, HelpCircle, Info, Lightbulb, X } from 'lucide-react';
import { markDone } from '../guideProgress';
import StepText, { inline } from './StepText';
import type { GuideDetailData, GuideStep, GuideSummary } from '../types/help';
import { usePick } from '../hooks/usePick';
import { Button, useToast } from './ui';
import { useApi } from '../api';

const STORAGE_KEY = 'graceride_walkthrough';

/** The path part of an in-app link, without ?query or #hash. */
const linkPath = (link: string) => link.split(/[?#]/)[0];
/** True when the link points at the page the person is already on (a link with a ?query must match it too). */
const isHere = (link: string, pathname: string, search: string) => {
  const q = link.includes('?') ? `?${link.split('?')[1].split('#')[0]}` : '';
  return linkPath(link) === pathname && (!q || q === search);
};

interface WalkthroughState {
  guide: GuideDetailData;
  stepIndex: number;
}

interface GuideRunnerContextType {
  active: WalkthroughState | null;
  startWalkthrough: (guide: GuideDetailData, startAtStep?: number) => void;
  stopWalkthrough: () => void;
  nextStep: () => void;
  prevStep: () => void;
  matchingGuideForPage: { guide: GuideSummary; stepIndex: number } | null;
}

const GuideRunnerCtx = createContext<GuideRunnerContextType>({
  active: null,
  startWalkthrough: () => {},
  stopWalkthrough: () => {},
  nextStep: () => {},
  prevStep: () => {},
  matchingGuideForPage: null,
});

export const useGuideRunner = () => useContext(GuideRunnerCtx);

function WalkthroughCard({ active, minimised, onMinimise, onPrev, onNext, onStop, hereLink }: {
  active: WalkthroughState;
  minimised: boolean;
  onMinimise: (v: boolean) => void;
  onPrev: () => void;
  onNext: () => void;
  onStop: () => void;
  hereLink: (link: string) => boolean;
}) {
  const { t } = useTranslation('help');
  const pick = usePick();
  const { guide, stepIndex } = active;
  const step = guide.steps[stepIndex];
  const total = guide.steps.length;
  const last = stepIndex === total - 1;
  const stepTitle = pick(step?.title_zh ?? '', step?.title_en ?? '');
  const stepBody = pick(step?.body_zh ?? '', step?.body_en ?? '');
  const stepNote = pick(step?.note_zh ?? '', step?.note_en ?? '');
  const noteClass =
    step?.note_kind === 'warning' || step?.note_kind === 'important'
      ? 'warning'
      : step?.note_kind === 'note'
        ? 'note'
        : 'tip';
  const counter = t('stepOfTotal', { n: stepIndex + 1, total });

  return (
    <aside className={`guide-card ${minimised ? 'min' : ''}`} role="region" aria-label={t('walkthroughBar', 'Guide walkthrough')}>
      <div className="guide-card-head">
        <div className="guide-card-kicker">
          <span className="guide-card-step">{counter}</span>
          {!minimised && <span className="muted ellipsis"> · {pick(guide.title_zh, guide.title_en)}</span>}
          {minimised && <span className="ellipsis"> · {stepTitle}</span>}
        </div>
        <button type="button" className="icon-btn sm ghost" onClick={() => onMinimise(!minimised)} aria-expanded={!minimised}
          aria-label={minimised ? t('expandStep', 'Expand') : t('collapseStep', 'Collapse')}>
          {minimised ? <ChevronUp size={18} aria-hidden="true" /> : <ChevronDown size={18} aria-hidden="true" />}
        </button>
        <button type="button" className="icon-btn sm ghost" onClick={onStop} aria-label={t('closeWalkthrough', 'Exit walkthrough')}>
          <X size={18} aria-hidden="true" />
        </button>
      </div>

      {!minimised && (
        <>
          <div className="guide-card-body">
            <h2 className="guide-card-title">{stepTitle || t('stepNumber', { n: stepIndex + 1 })}</h2>
            {stepBody && <StepText text={stepBody} />}
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
            {step?.image_file_id && (
              <a
                href={`/api/files/${step.image_file_id}`}
                target="_blank"
                rel="noreferrer"
                className="guide-step-img-link"
              >
                <img
                  src={`/api/files/${step.image_file_id}`}
                  alt={stepTitle || t('stepNumber', { n: stepIndex + 1 })}
                  className="guide-step-img"
                  loading="lazy"
                />
              </a>
            )}
            {!step?.link && <p className="small">{t('stepHere')}</p>}
            {step?.link && !hereLink(step.link) && (
              <Link to={step.link} className="guide-step-link">
                <ExternalLink size={14} aria-hidden="true" />
                <span>{t('openStepLink', 'Open this step page')}</span>
              </Link>
            )}
            <Link to={`/help/guides/${guide.slug}`} className="small guide-card-all">{t('seeAllSteps')}</Link>
          </div>
          <div className="guide-card-actions">
            <button type="button" className="btn sm secondary" onClick={onPrev} disabled={stepIndex === 0}>
              {t('back')}
            </button>
            <button type="button" className="btn sm primary" onClick={onNext}>
              {last ? t('finish', 'Finish') : t('next', 'Next')}
            </button>
          </div>
        </>
      )}
    </aside>
  );
}

export function GuideRunnerProvider({ children }: { children: ReactNode }) {
  const { t } = useTranslation('help');
  const nav = useNavigate();
  const loc = useLocation();
  const toast = useToast();

  const [active, setActive] = useState<WalkthroughState | null>(() => {
    try {
      const raw = sessionStorage.getItem(STORAGE_KEY);
      return raw ? JSON.parse(raw) : null;
    } catch {
      return null;
    }
  });

  const [minimised, setMinimised] = useState(false);

  // Sync to sessionStorage
  useEffect(() => {
    if (!active) {
      sessionStorage.removeItem(STORAGE_KEY);
    } else {
      try {
        sessionStorage.setItem(STORAGE_KEY, JSON.stringify(active));
      } catch {
        // ignore
      }
    }
  }, [active]);

  // Tell the stylesheet a card is showing, so the page leaves room under it and toasts sit above it.
  useEffect(() => {
    if (active) document.body.dataset.walkthrough = minimised ? 'min' : 'open';
    else delete document.body.dataset.walkthrough;
    return () => {
      delete document.body.dataset.walkthrough;
    };
  }, [active, minimised]);

  // Find if any published guide has a step pointing to the current page
  const guidesApi = useApi<(GuideSummary & { steps?: GuideStep[] })[]>('/help/guides?full=true');
  const allGuides = guidesApi.data ?? [];

  const matchingGuideForPage = (() => {
    const currentPath = loc.pathname;
    if (currentPath === '/' || currentPath.startsWith('/help')) return null;

    for (const g of allGuides) {
      if (!g.published || !g.steps) continue;
      const stepIdx = g.steps.findIndex((s) => s.link && linkPath(s.link) === currentPath);
      if (stepIdx !== -1) {
        return { guide: g, stepIndex: stepIdx };
      }
    }
    return null;
  })();

  const startWalkthrough = (guide: GuideDetailData, startAtStep = 0) => {
    const safeIdx = Math.max(0, Math.min(startAtStep, guide.steps.length - 1));
    setActive({ guide, stepIndex: safeIdx });
    setMinimised(false);

    // If starting step has a link and we're not on it, navigate
    const step = guide.steps[safeIdx];
    if (step?.link && !isHere(step.link, loc.pathname, loc.search)) {
      nav(step.link);
    }
  };

  const stopWalkthrough = () => {
    setActive(null);
  };

  const nextStep = () => {
    if (!active) return;
    const { guide, stepIndex } = active;
    markDone(guide.slug, guide.steps, stepIndex);

    if (stepIndex < guide.steps.length - 1) {
      const nextIdx = stepIndex + 1;
      setActive({ guide, stepIndex: nextIdx });
      const nextStepObj = guide.steps[nextIdx];
      if (nextStepObj?.link && !isHere(nextStepObj.link, loc.pathname, loc.search)) {
        nav(nextStepObj.link);
      }
    } else {
      // Completed all steps
      toast(t('walkthroughDone', 'Walkthrough complete!'));
      stopWalkthrough();
      nav(`/help/guides/${guide.slug}`);
    }
  };

  const prevStep = () => {
    if (!active || active.stepIndex <= 0) return;
    const prevIdx = active.stepIndex - 1;
    setActive({ guide: active.guide, stepIndex: prevIdx });
    const prevStepObj = active.guide.steps[prevIdx];
    if (prevStepObj?.link && !isHere(prevStepObj.link, loc.pathname, loc.search)) {
      nav(prevStepObj.link);
    }
  };

  return (
    <GuideRunnerCtx.Provider
      value={{
        active,
        startWalkthrough,
        stopWalkthrough,
        nextStep,
        prevStep,
        matchingGuideForPage,
      }}
    >
      {children}

      {active && <WalkthroughCard
        active={active}
        minimised={minimised}
        onMinimise={setMinimised}
        onPrev={prevStep}
        onNext={nextStep}
        onStop={stopWalkthrough}
        hereLink={(link) => isHere(link, loc.pathname, loc.search)}
      />}
    </GuideRunnerCtx.Provider>
  );
}

/** Small button for PageHeader when a guide links to the current page */
export function PageHeaderGuideButton() {
  const { t } = useTranslation('help');
  const { active, matchingGuideForPage, startWalkthrough } = useGuideRunner();
  const pick = usePick();
  const { data: fullGuide } = useApi<GuideDetailData>(
    matchingGuideForPage ? `/help/guides/${matchingGuideForPage.guide.slug}` : null,
  );

  if (active || !matchingGuideForPage || !fullGuide) return null;

  return (
    <Button
      variant="ghost"
      size="sm"
      icon={<HelpCircle size={14} />}
      aria-label={t('pageGuideNamed', { title: pick(fullGuide.title_zh, fullGuide.title_en) })}
      onClick={() => startWalkthrough(fullGuide, matchingGuideForPage.stepIndex)}
    >
      {t('pageGuideBtn', 'Guide')}
    </Button>
  );
}
