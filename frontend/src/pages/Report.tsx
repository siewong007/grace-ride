import { useState } from 'react';
import { Link } from 'react-router-dom';
import { useApi } from '../api';
import { ErrorBox, Spinner } from '../components/ui';
import { EVENT_KIND, fmtDateTime, rm, useFormatT } from '../format';
import { useTranslation } from 'react-i18next';

interface Badge { key: string; char: string; name: string; rule: string; progress: number; target: number; earned: boolean }
interface MonthReport {
  user: { id: number; name: string }; month: string; rides_driven: number; rides_received: number; events_attended: number;
  by_kind: { kind: string; count: number }[]; events: { id: number; title: string; kind: string; starts_at: string }[];
  costs_paid_cents: number; badges: Badge[];
}

function useTranslationHooks() {
  const { t } = useTranslation('report');
  const ft = useFormatT();
  return { t, ft };
}

export default function Report() {
  const { t, ft } = useTranslationHooks();
  const now = new Date();
  const [month, setMonth] = useState(`${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`);
  const [y, m] = month.split('-').map(Number);
  const { data: r, error } = useApi<MonthReport>(`/reports/monthly?month=${month}`);

  return (
    <div className="stack">
      <div className="page-head">
        <div>
          <h1>{t('title', { month: `${y} 年 ${m} 月` })}</h1>
          <p>{t('desc')}</p>
        </div>
        <input className="input" type="month" value={month} onChange={(e) => setMonth(e.target.value)} style={{ maxWidth: 200 }} aria-label={t('monthLabel')} />
      </div>
      <ErrorBox error={error} />
      {!r ? <Spinner /> : (
        <>
          <div className="grid cols-4">
            <div className="card stat"><span className="num">{r.events_attended}</span><span className="label">{t('attendance')}</span></div>
            <div className="card stat"><span className="num">{r.rides_driven}</span><span className="label">{t('driven')}</span></div>
            <div className="card stat"><span className="num">{r.rides_received}</span><span className="label">{t('received')}</span></div>
            <div className="card stat"><span className="num" style={{ fontSize: '1.5rem' }}>{rm(r.costs_paid_cents)}</span><span className="label">{t('costs')}</span></div>
          </div>
          <div className="split">
            <div className="card stack-sm">
              <h3>{t('events')}</h3>
              {r.by_kind.length ? (
                <p className="bold">{r.by_kind.map((k) => `${ft(EVENT_KIND[k.kind]?.label)} × ${k.count}`).join(' · ')}</p>
              ) : <p className="muted">{t('noEvents', { y, m })}</p>}
              <div className="list">
                {r.events.map((e) => (
                  <Link key={e.id} to={`/events/${e.id}`} className="item">
                    <span aria-hidden>{EVENT_KIND[e.kind]?.emoji}</span>
                    <span className="grow">{e.title}</span>
                    <span className="muted small">{fmtDateTime(e.starts_at, ft)}</span>
                  </Link>
                ))}
              </div>
              <p className="muted tiny">{t('note')}</p>
            </div>
            <div className="card stack">
              <div>
                <h3>{t('badges')}</h3>
                <p className="muted small">{t('badgesDesc')}</p>
              </div>
              {r.badges.length === 0 ? <p className="muted">{t('noBadges')}</p> : r.badges.map((b) => (
                <div key={b.key} className="badge-tile">
                  <div className={`badge-char ${b.earned ? 'earned' : ''}`} aria-hidden>{b.char}</div>
                  <div className="grow stack-sm">
                    <div className="row between"><strong>{b.name}</strong><span className="small muted">{b.earned ? t('earned') : `${Math.min(b.progress, b.target)}/${b.target}`}</span></div>
                    <div className="bar"><div style={{ width: `${Math.min(100, (b.progress / b.target) * 100)}%` }} /></div>
                    <div className="muted tiny">{b.rule}</div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </>
      )}
    </div>
  );
}
