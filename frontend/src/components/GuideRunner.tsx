import { createContext, ReactNode, useContext, useEffect, useRef, useState } from 'react';
import { useLocation, useNavigate, Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AlertTriangle, ChevronDown, ExternalLink, HelpCircle, Info, Lightbulb, X } from 'lucide-react';
import { markDone } from '../guideProgress';
import StepText, { inline } from './StepText';
import type { GuideDetailData, GuideStep, GuideSummary } from '../types/help';
import { usePick } from '../hooks/usePick';
import { Button, useToast } from './ui';
import { useApi } from '../api';
import { useAuth } from '../auth';
import { guideForPage, isHere, isPatternLink, navigableLink } from '../guideLinks';

const STORAGE_KEY = 'graceride_walkthrough';

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

function WalkthroughCard({ active, minimised, onMinimise, onPrev, onNext, onStop, hereLink, pathname }: {
  active: WalkthroughState;
  minimised: boolean;
  onMinimise: (v: boolean) => void;
  onPrev: () => void;
  onNext: () => void;
  onStop: () => void;
  hereLink: (link: string) => boolean;
  pathname: string;
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
  // An item-page step points at the list the item is picked from, unless the person is already in it.
  const target = step?.link ? navigableLink(step.link) : null;
  const listLink =
    target && !(isPatternLink(step!.link) && (pathname === target || pathname.startsWith(`${target}/`))) ? target : null;

  const bodyWrapRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const el = bodyWrapRef.current;
    if (el) el.inert = minimised;
  }, [minimised]);

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
          <ChevronDown
            size={18}
            className={`guide-card-chevron${minimised ? ' is-min' : ''}`}
            aria-hidden="true"
          />
        </button>
        <button type="button" className="icon-btn sm ghost" onClick={onStop} aria-label={t('closeWalkthrough', 'Exit walkthrough')}>
          <X size={18} aria-hidden="true" />
        </button>
      </div>

      <div
        ref={bodyWrapRef}
        className={`collapsible-grid${minimised ? '' : ' is-open'}`}
        aria-hidden={minimised || undefined}
      >
        <div className="collapsible-grid-inner">
          <div key={stepIndex} className="guide-card-body guide-step-enter">
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
            {step?.link && !hereLink(step.link) && isPatternLink(step.link) && (
              <p className="small">{t('stepOpenItem')}</p>
            )}
            {step?.link && !hereLink(step.link) && listLink && (
              <Link to={listLink} className="guide-step-link">
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
        </div>
      </div>
    </aside>
  );
}

export function GuideRunnerProvider({ children }: { children: ReactNode }) {
  const { t } = useTranslation('help');
  const { me } = useAuth();
  const meId = me?.id ?? null;
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
  const prevUserRef = useRef<number | null>(meId);

  useEffect(() => {
    if (prevUserRef.current !== null && meId !== prevUserRef.current) {
      setActive(null);
      try {
        sessionStorage.removeItem(STORAGE_KEY);
      } catch {
        // ignore
      }
    }
    prevUserRef.current = meId;
  }, [meId]);

  // Sync to sessionStorage
  useEffect(() => {
    try {
      if (!active) {
        sessionStorage.removeItem(STORAGE_KEY);
      } else {
        sessionStorage.setItem(STORAGE_KEY, JSON.stringify(active));
      }
    } catch {
      // ignore
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

  // The guide whose steps cover the current page (see guideForPage for which one wins).
  const guidesApi = useApi<(GuideSummary & { steps?: GuideStep[] })[]>('/help/guides?full=true');
  const matchingGuideForPage = guideForPage(guidesApi.data ?? [], loc.pathname);

  /** Go to a step's page. Item-page patterns are left to the person, except when a walkthrough starts. */
  const goTo = (link: string | undefined, starting: boolean) => {
    if (!link || isHere(link, loc.pathname, loc.search)) return;
    if (isPatternLink(link) && !starting) return;
    const target = navigableLink(link);
    if (target && !isHere(target, loc.pathname, loc.search)) nav(target);
  };

  const startWalkthrough = (guide: GuideDetailData, startAtStep = 0) => {
    const safeIdx = Math.max(0, Math.min(startAtStep, guide.steps.length - 1));
    setActive({ guide, stepIndex: safeIdx });
    setMinimised(false);

    // If the starting step has a page and we're not on it, go there.
    goTo(guide.steps[safeIdx]?.link, true);
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
      goTo(guide.steps[nextIdx]?.link, false);
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
    goTo(active.guide.steps[prevIdx]?.link, false);
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
        pathname={loc.pathname}
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
      className="page-header-guide-btn"
      icon={<HelpCircle size={14} />}
      aria-label={t('pageGuideNamed', { title: pick(fullGuide.title_zh, fullGuide.title_en) })}
      onClick={() => startWalkthrough(fullGuide, matchingGuideForPage.stepIndex)}
    >
      <span className="page-header-guide-label">{t('pageGuideBtn', 'Guide')}</span>
    </Button>
  );
}
