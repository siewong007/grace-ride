import { useState } from 'react';
import { Link } from 'react-router-dom';
import { Plus } from 'lucide-react';
import { useApi } from '../api';
import { useAuth } from '../auth';
import {
  Button,
  CardSkeleton,
  EmptyState,
  ErrorBox,
  Page,
  PageHeader,
  Pill,
  Tabs,
} from '../components/ui';
import { EVENT_KIND, fmtDateTime, useFormatT } from '../format';
import { useTranslation } from 'react-i18next';

export interface EventItem {
  id: number;
  title: string;
  kind: string;
  description: string;
  starts_at: string;
  location: string;
  capacity: number;
  cost_note: string;
  signup_deadline: string | null;
  status: string;
  created_by: { id: number; name: string };
  going: number;
  waitlist: number;
  spots_left: number | null;
  my_signup: { status: string; need_ride: boolean; offer_drive: boolean; attended: number | null } | null;
  groups: string;
  can_manage: boolean;
}

export default function Events() {
  const { t } = useTranslation('events');
  const ft = useFormatT();
  const { has } = useAuth();
  const [scope, setScope] = useState<'upcoming' | 'past' | 'managing'>('upcoming');
  const { data, error, loading } = useApi<EventItem[]>(`/events?scope=${scope}`);

  return (
    <Page width="wide">
      <div className="stack">
        <PageHeader
          title={t('title')}
          subtitle={t('desc')}
          actions={
            has('leader') ? (
              <Button variant="primary" to="/events/new" icon={<Plus size={20} />}>
                {t('create')}
              </Button>
            ) : undefined
          }
        />

        <Tabs
          value={scope}
          onChange={(v) => setScope(v as 'upcoming' | 'past' | 'managing')}
          items={[
            { value: 'upcoming', label: t('upcoming') },
            { value: 'past', label: t('past') },
            ...(has('leader') ? [{ value: 'managing', label: t('managing') }] : []),
          ]}
        />

        <ErrorBox error={error} />

        {loading ? (
          <CardSkeleton count={3} />
        ) : !data?.length ? (
          <EmptyState
            title={t('none')}
            action={
              has('leader') ? (
                <Button variant="primary" to="/events/new" icon={<Plus size={18} />}>
                  {t('create')}
                </Button>
              ) : undefined
            }
          />
        ) : (
          <div className="grid cols-3">
            {data.map((e) => {
              const d = new Date(e.starts_at);
              const monthName = d.toLocaleDateString(undefined, { month: 'short' });
              const dayNum = d.getDate();
              const percent = e.capacity > 0 ? Math.min(100, Math.round((e.going / e.capacity) * 100)) : 0;

              return (
                <Link key={e.id} to={`/events/${e.id}`} className="card link event-card">
                  <div className="event-card-head">
                    <div className="event-date-badge" aria-hidden="true">
                      <span className="event-date-month">{monthName}</span>
                      <span className="event-date-day">{dayNum}</span>
                    </div>
                    <div className="grow min-w-0">
                      <div className="bold ellipsis">{e.title}</div>
                      <div className="muted small ellipsis">
                        {ft(EVENT_KIND[e.kind]?.label ?? e.kind)} · {e.groups}
                      </div>
                    </div>
                  </div>

                  <div className="small">{fmtDateTime(e.starts_at, ft)}</div>
                  <div className="muted small ellipsis">{e.location}</div>

                  {e.capacity > 0 ? (
                    <div className="event-seats-meter">
                      <div className="row between small muted">
                        <span>{e.going}/{e.capacity} {t('peopleUnit')}</span>
                        {e.spots_left != null && (
                          <span>{e.spots_left > 0 ? t('spotsLeft', { n: e.spots_left }) : t('full')}</span>
                        )}
                      </div>
                      <div className="bar">
                        <div style={{ width: `${percent}%` }} />
                      </div>
                    </div>
                  ) : (
                    <div className="small muted">
                      {e.going} {t('peopleAttend')}
                    </div>
                  )}

                  <div className="row wrap gap-xs">
                    {e.status !== 'published' && (
                      <Pill tone="neutral">{e.status === 'cancelled' ? t('cancelled') : t('finished')}</Pill>
                    )}
                    {e.my_signup?.status === 'going' && <Pill tone="success">{t('signedUp')}</Pill>}
                    {e.my_signup?.status === 'waitlist' && <Pill tone="warning">{t('waitlist')}</Pill>}
                    {e.my_signup?.need_ride && <Pill tone="info">{t('needRide')}</Pill>}
                  </div>
                </Link>
              );
            })}
          </div>
        )}
      </div>
    </Page>
  );
}
