import { createContext, ReactNode, useContext, useEffect, useState } from 'react';
import { useLocation, useNavigate, Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { ChevronDown, ChevronUp, ExternalLink, HelpCircle, X } from 'lucide-react';
import { GuideDetailData, GuideStep, GuideSummary, usePick } from '../pages/HelpGuides';
import { Button, useToast } from './ui';
import { useApi } from '../api';

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

export function GuideRunnerProvider({ children }: { children: ReactNode }) {
  const { t } = useTranslation('help');
  const pick = usePick();
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

  const [expanded, setExpanded] = useState(false);

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

  // Find if any published guide has a step pointing to the current page
  const guidesApi = useApi<(GuideSummary & { steps?: GuideStep[] })[]>('/help/guides?full=true');
  const allGuides = guidesApi.data ?? [];

  const matchingGuideForPage = (() => {
    const currentPath = loc.pathname;
    if (currentPath === '/' || currentPath.startsWith('/help')) return null;

    for (const g of allGuides) {
      if (!g.published || !g.steps) continue;
      const stepIdx = g.steps.findIndex((s) => s.link && s.link.split('#')[0] === currentPath);
      if (stepIdx !== -1) {
        return { guide: g, stepIndex: stepIdx };
      }
    }
    return null;
  })();

  const startWalkthrough = (guide: GuideDetailData, startAtStep = 0) => {
    const safeIdx = Math.max(0, Math.min(startAtStep, guide.steps.length - 1));
    setActive({ guide, stepIndex: safeIdx });
    setExpanded(false);

    // If starting step has a link and we're not on it, navigate
    const step = guide.steps[safeIdx];
    if (step?.link && loc.pathname !== step.link) {
      nav(step.link);
    }
  };

  const stopWalkthrough = () => {
    setActive(null);
  };

  const markStepDoneInStorage = (slug: string, stepIdx: number) => {
    const key = `graceride_guide_${slug}_done`;
    try {
      const raw = localStorage.getItem(key);
      const arr: number[] = raw ? JSON.parse(raw) : [];
      if (!arr.includes(stepIdx)) {
        arr.push(stepIdx);
        localStorage.setItem(key, JSON.stringify(arr));
      }
    } catch {
      // ignore
    }
  };

  const nextStep = () => {
    if (!active) return;
    const { guide, stepIndex } = active;
    markStepDoneInStorage(guide.slug, stepIndex);

    if (stepIndex < guide.steps.length - 1) {
      const nextIdx = stepIndex + 1;
      setActive({ guide, stepIndex: nextIdx });
      const nextStepObj = guide.steps[nextIdx];
      if (nextStepObj?.link && loc.pathname !== nextStepObj.link) {
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
    if (prevStepObj?.link && loc.pathname !== prevStepObj.link) {
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
      <div className={`guide-runner-container ${active ? 'has-active-walkthrough' : ''}`}>
        {children}
      </div>

      {active && (
        <aside
          className="guide-runner-bar stack-xs"
          role="region"
          aria-label={t('walkthroughBar', 'Guide walkthrough')}
        >
          <div className="row between nowrap">
            <div className="row nowrap gap-xs align-center min-w-0 grow">
              <span className="bold small ellipsis">
                {pick(active.guide.title_zh, active.guide.title_en)}
              </span>
              <span className="muted tiny nowrap">
                · {t('stepNumber', { n: active.stepIndex + 1 })} / {active.guide.steps.length} ·
              </span>
              <span className="small bold ellipsis">
                {pick(
                  active.guide.steps[active.stepIndex]?.title_zh ?? '',
                  active.guide.steps[active.stepIndex]?.title_en ?? '',
                )}
              </span>
            </div>

            <div className="btn-row nowrap align-center">
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setExpanded(!expanded)}
                aria-label={expanded ? t('collapseStep', 'Collapse') : t('expandStep', 'Expand')}
              >
                {expanded ? <ChevronDown size={16} /> : <ChevronUp size={16} />}
              </Button>
              <Button
                variant="secondary"
                size="sm"
                onClick={prevStep}
                disabled={active.stepIndex === 0}
              >
                {t('back', 'Back')}
              </Button>
              <Button variant="primary" size="sm" onClick={nextStep}>
                {active.stepIndex === active.guide.steps.length - 1
                  ? t('finish', 'Finish')
                  : t('next', 'Next')}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={stopWalkthrough}
                aria-label={t('closeWalkthrough', 'Exit walkthrough')}
              >
                <X size={16} />
              </Button>
            </div>
          </div>

          {expanded && (
            <div
              className="card tight stack-xs"
              style={{
                marginTop: 'var(--space-2)',
                background: 'var(--surface-2)',
                border: '1px solid var(--border)',
              }}
            >
              <p className="muted small pre-wrap">
                {pick(
                  active.guide.steps[active.stepIndex]?.body_zh ?? '',
                  active.guide.steps[active.stepIndex]?.body_en ?? '',
                )}
              </p>
              {active.guide.steps[active.stepIndex]?.link && (
                <div style={{ marginTop: 'var(--space-1)' }}>
                  <Link
                    to={active.guide.steps[active.stepIndex].link}
                    className="small row nowrap gap-xs bold"
                  >
                    <ExternalLink size={14} />
                    <span>{t('openStepLink', 'Open this step page')}</span>
                  </Link>
                </div>
              )}
            </div>
          )}
        </aside>
      )}
    </GuideRunnerCtx.Provider>
  );
}

/** Small button for PageHeader when a guide links to the current page */
export function PageHeaderGuideButton() {
  const { t } = useTranslation('help');
  const { matchingGuideForPage, startWalkthrough } = useGuideRunner();
  const { data: fullGuide } = useApi<GuideDetailData>(
    matchingGuideForPage ? `/help/guides/${matchingGuideForPage.guide.slug}` : null,
  );

  if (!matchingGuideForPage || !fullGuide) return null;

  return (
    <Button
      variant="ghost"
      size="sm"
      icon={<HelpCircle size={14} />}
      onClick={() => startWalkthrough(fullGuide, matchingGuideForPage.stepIndex)}
    >
      {t('pageGuideBtn', 'Guide')}
    </Button>
  );
}
