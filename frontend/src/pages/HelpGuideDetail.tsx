import { useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Pencil, Play } from 'lucide-react';
import { useApi } from '../api';
import { useAuth } from '../auth';
import { Button, Card, CardSkeleton, EmptyState, ErrorBox, Notice, Page, PageHeader, Pill } from '../components/ui';
import GuideSteps from '../components/GuideSteps';
import { useGuideRunner } from '../components/GuideRunner';
import HelpComposer from '../components/HelpComposer';
import type { GuideDetailData, GuideSummary } from '../types/help';
import { usePick } from '../hooks/usePick';

export default function HelpGuideDetail() {
  const { t } = useTranslation('help');
  const pick = usePick();
  const { slug } = useParams();
  const nav = useNavigate();
  const { can } = useAuth();
  const isEditor = can('admin_help', 'update');
  const { startWalkthrough } = useGuideRunner();

  const { data: g, error, loading } = useApi<GuideDetailData>(`/help/guides/${slug}`);
  // The next guide for the same audience, offered once this one is finished.
  const { data: allGuides } = useApi<GuideSummary[]>('/help/guides');
  const [composerOpen, setComposerOpen] = useState(false);
  const [composerSubject, setComposerSubject] = useState('');

  if (error) {
    return (
      <Page width="narrow">
        <ErrorBox error={error} />
      </Page>
    );
  }

  if (loading || !g) {
    return (
      <Page width="narrow">
        <CardSkeleton count={2} />
      </Page>
    );
  }

  const guideTitle = pick(g.title_zh, g.title_en);
  const guideSummary = pick(g.summary_zh, g.summary_en);

  const list = Array.isArray(allGuides) ? allGuides.filter((x) => x.published && x.audience === g.audience) : [];
  const at = list.findIndex((x) => x.slug === g.slug);
  const next = at >= 0 ? list[at + 1] : undefined;
  const nextGuide = next ? { slug: next.slug, title: pick(next.title_zh, next.title_en) } : null;

  const handleStartWalkthrough = () => {
    startWalkthrough(g);
  };

  const handleContactUs = (prefilledSubject: string) => {
    setComposerSubject(prefilledSubject);
    setComposerOpen(true);
  };

  return (
    <Page width="narrow">
      <div className="stack">
        <PageHeader
          title={guideTitle}
          subtitle={guideSummary}
          back="/help?tab=guides"
          actions={
            <div className="btn-row nowrap align-center">
              <Button
                variant="primary"
                size="sm"
                icon={<Play size={14} />}
                onClick={handleStartWalkthrough}
              >
                {t('startWalkthrough', 'Start walkthrough')}
              </Button>
              {isEditor && (
                <Button
                  variant="secondary"
                  size="sm"
                  to={`/help/guides/${g.slug}/edit`}
                  icon={<Pencil size={14} />}
                >
                  {t('guideEdit')}
                </Button>
              )}
              {!g.published && <Pill tone="warning">{t('guideDraft')}</Pill>}
            </div>
          }
        />

        {!g.published && (
          <Notice tone="warning">
            {t('draftNotice', 'Draft — only editors can see this guide')}
          </Notice>
        )}

        <Card className="stack">
          {!g.steps || g.steps.length === 0 ? (
            <EmptyState title={t('noStepsYet', 'No steps in this guide')} />
          ) : (
            <GuideSteps
              steps={g.steps}
              slug={g.slug}
              guideId={g.id}
              guideTitle={guideTitle}
              onContactUs={handleContactUs}
              onFollow={(idx) => startWalkthrough(g, idx)}
              nextGuide={nextGuide}
            />
          )}
        </Card>

        {composerOpen && (
          <HelpComposer
            onClose={() => setComposerOpen(false)}
            initialSubject={composerSubject}
            onCreated={(newId) => nav(`/help/${newId}`)}
          />
        )}
      </div>
    </Page>
  );
}
