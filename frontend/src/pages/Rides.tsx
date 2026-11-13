import { useMemo, useState } from 'react';
import { CalendarPlus } from 'lucide-react';
import { useApi } from '../api';
import RideCard, { Ride } from '../components/RideCard';
import {
  Button,
  EmptyState,
  ErrorBox,
  ListSkeleton,
  Page,
  PageHeader,
  Tabs,
} from '../components/ui';
import { fmtDay, localDate, todayStr, useFormatT } from '../format';
import { useTranslation } from 'react-i18next';

interface DateGroup {
  dateKey: string;
  label: string;
  rides: Ride[];
}

function groupRidesByDate(
  rides: Ride[],
  ft: (key: string, opts?: Record<string, unknown>) => string,
  t: (key: string) => string,
): DateGroup[] {
  const map = new Map<string, Ride[]>();
  const today = todayStr();
  const tmr = new Date();
  tmr.setDate(tmr.getDate() + 1);
  const tmrStr = localDate(tmr);

  for (const ride of rides) {
    const day = ride.pickup_at.slice(0, 10);
    if (!map.has(day)) {
      map.set(day, []);
    }
    map.get(day)!.push(ride);
  }

  const groups: DateGroup[] = [];
  for (const [day, dayRides] of map.entries()) {
    let label = fmtDay(`${day}T00:00:00`, ft);
    if (day === today) {
      label = t('dateToday');
    } else if (day === tmrStr) {
      label = t('dateTomorrow');
    }
    groups.push({ dateKey: day, label, rides: dayRides });
  }

  return groups;
}

export default function Rides() {
  const { t } = useTranslation('rides');
  const ft = useFormatT();
  const [scope, setScope] = useState<'upcoming' | 'past'>('upcoming');
  const { data, error, loading } = useApi<Ride[]>(`/rides?scope=${scope}`, 30000);

  const dateGroups = useMemo(() => {
    if (!data || data.length === 0) return [];
    return groupRidesByDate(data, ft, t);
  }, [data, ft, t]);

  return (
    <Page width="wide">
      <div className="stack">
        <PageHeader
          title={t('title')}
          subtitle={t('desc')}
          actions={
            <Button variant="primary" to="/rides/new" icon={<CalendarPlus size={20} />}>
              {t('signupRide')}
            </Button>
          }
        />

        <Tabs
          value={scope}
          onChange={(v) => setScope(v as 'upcoming' | 'past')}
          items={[
            { value: 'upcoming', label: t('upcoming') },
            { value: 'past', label: t('past') },
          ]}
        />

        <ErrorBox error={error} />

        {loading ? (
          <ListSkeleton count={4} />
        ) : dateGroups.length > 0 ? (
          <div className="rides-grouped-list stack">
            {dateGroups.map((group) => (
              <div key={group.dateKey} className="rides-date-section">
                <div className="sticky-date-header">{group.label}</div>
                <div className="grid cols-2">
                  {group.rides.map((r) => (
                    <RideCard key={r.id} ride={r} />
                  ))}
                </div>
              </div>
            ))}
          </div>
        ) : (
          <EmptyState
            icon={<CalendarPlus size={36} />}
            title={scope === 'upcoming' ? t('noUpcoming') : t('noPast')}
            action={
              scope === 'upcoming' ? (
                <Button variant="primary" to="/rides/new" icon={<CalendarPlus size={18} />}>
                  {t('signupRide')}
                </Button>
              ) : undefined
            }
          />
        )}
      </div>
    </Page>
  );
}
