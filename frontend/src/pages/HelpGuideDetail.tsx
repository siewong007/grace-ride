import { useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useApi } from '../api';
import {
  Card,
  ErrorBox,
  Page,
  PageHeader,
  Pill,
  Spinner,
} from '../components/ui';
import { GuideDetailData, usePick } from './HelpGuides';

export default function HelpGuideDetail() {
  const { t } = useTranslation('help');
  const pick = usePick();
  const { slug } = useParams();
  const { data: g, error } = useApi<GuideDetailData>(`/help/guides/${slug}`);

  if (error)
    return (
      <Page width="narrow">
        <ErrorBox error={error} />
      </Page>
    );
  if (!g) return <Spinner />;

  return (
    <Page width="narrow">
      <div className="stack">
        <PageHeader
          title={pick(g.title_zh, g.title_en)}
          subtitle={pick(g.summary_zh, g.summary_en)}
          back="/help"
          actions={!g.published ? <Pill tone="warning">{t('guideDraft')}</Pill> : undefined}
        />
        <Card className="stack">
          {g.steps.map((s, i) => (
            <div key={i} className="stack-xs">
              <div className="bold">
                {i + 1}. {pick(s.title_zh, s.title_en)}
              </div>
              {(pick(s.body_zh, s.body_en) || s.link) && (
                <div className="stack-xs">
                  <div className="muted pre-wrap">{pick(s.body_zh, s.body_en)}</div>
                  {s.link && (
                    <a className="btn sm secondary" href={s.link}>
                      {t('guideOpen')}
                    </a>
                  )}
                </div>
              )}
            </div>
          ))}
        </Card>
      </div>
    </Page>
  );
}
