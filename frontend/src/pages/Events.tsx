import { useState } from 'react';
import { Link } from 'react-router-dom';
import { Plus } from 'lucide-react';
import { useApi } from '../api';
import { useAuth } from '../auth';
import { Empty, ErrorBox, Pill, Spinner, Tabs } from '../components/ui';
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
    <div className="stack">
      <div className="page-head">
        <div>
          <h1>{t('title')}</h1>
          <p>{t('desc')}</p>
        </div>
        {has('leader') && <Link className="btn primary" to="/events/new"><Plus size={20} />{t('create')}</Link>}
      </div>
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
      {loading ? <Spinner /> : !data?.length ? <Empty>{t('none')}</Empty> : (
        <div className="grid cols-3">
          {data.map((e) => (
            <Link key={e.id} to={`/events/${e.id}`} className="card link stack-sm">
              <div className="row nowrap">
                <span className="event-emoji" aria-hidden>{EVENT_KIND[e.kind]?.emoji}</span>
                <div className="grow">
                  <div className="bold">{e.title}</div>
                  <div className="muted small">{ft(EVENT_KIND[e.kind]?.label ?? e.kind)} · {e.groups}</div>
                </div>
              </div>
              <div>{fmtDateTime(e.starts_at, ft)}</div>
              <div className="muted small">{e.location}</div>
              <div className="row">
                {e.status !== 'published' && <Pill tone="gray">{e.status === 'cancelled' ? t('cancelled') : t('finished')}</Pill>}
                {e.my_signup?.status === 'going' && <Pill tone="green">{t('going')}</Pill>}
                {e.my_signup?.status === 'waitlist' && <Pill tone="amber">{t('waitlist')}</Pill>}
                {e.my_signup?.need_ride && <Pill tone="blue">{t('needRide')}</Pill>}
                {e.capacity > 0 ? <Pill>{e.going}/{e.capacity} {t('peopleUnit')}</Pill> : <Pill>{e.going} {t('peopleAttend')}</Pill>}
              </div>
            </Link>
          ))}
        </div>
      )}
    </div>
  );
}
