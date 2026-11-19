import { useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../auth';
import {
  EmptyState,
  Page,
  PageHeader,
  Tabs,
} from '../components/ui';
import { DriveSchedule } from '../components/drive/DriveSchedule';
import { DriveSettings } from '../components/drive/DriveSettings';

export default function Drive() {
  const { t } = useTranslation('drive');
  const { can } = useAuth();
  const [searchParams] = useSearchParams();
  const [tab, setTab] = useState<'schedule' | 'settings'>(searchParams.get('tab') === 'settings' ? 'settings' : 'schedule');

  if (!can('drive')) {
    return (
      <Page>
        <EmptyState title={t('notDriverTitle')} description={t('notDriver')} />
      </Page>
    );
  }

  return (
    <Page>
      <PageHeader title={t('title')} subtitle={t('desc')} />
      <Tabs
        value={tab}
        onChange={(v) => setTab(v as 'schedule' | 'settings')}
        items={[
          { value: 'schedule', label: t('schedule') },
          { value: 'settings', label: t('settings') },
        ]}
      />
      {tab === 'schedule' && <DriveSchedule />}
      {tab === 'settings' && <DriveSettings />}
    </Page>
  );
}
