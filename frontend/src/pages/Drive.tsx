import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useApi } from '../api';
import { useAuth } from '../auth';
import { Ride } from '../components/RideCard';
import {
  EmptyState,
  Page,
  PageHeader,
  Tabs,
} from '../components/ui';
import { DriveSchedule } from '../components/drive/DriveSchedule';
import { DriveRequests } from '../components/drive/DriveRequests';
import { DriveSettings } from '../components/drive/DriveSettings';

export default function Drive() {
  const { t } = useTranslation('drive');
  const { can } = useAuth();
  const [tab, setTab] = useState<'schedule' | 'requests' | 'settings'>('schedule');
  const requests = useApi<Ride[]>(can('drive') ? '/driver/requests?mine_areas=true' : null);

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
        onChange={(v) => setTab(v as 'schedule' | 'requests' | 'settings')}
        items={[
          { value: 'schedule', label: t('schedule') },
          { value: 'requests', label: t('requests'), count: requests.data?.length },
          { value: 'settings', label: t('settings') },
        ]}
      />
      {tab === 'schedule' && <DriveSchedule />}
      {tab === 'requests' && <DriveRequests />}
      {tab === 'settings' && <DriveSettings />}
    </Page>
  );
}
