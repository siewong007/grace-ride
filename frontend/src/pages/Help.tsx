import { FormEvent, useState } from 'react';
import { Link, useNavigate, useParams, useSearchParams } from 'react-router-dom';
import { post, useApi } from '../api';
import { useAuth } from '../auth';
import { Empty, ErrorBox, Field, Pill, Spinner, StatusPill, useAction } from '../components/ui';
import { fmtAgo, HELP_STATUS, useFormatT } from '../format';
import { useTranslation } from 'react-i18next';

export default function Help() {
  const { t } = useTranslation('help');
  const ft = useFormatT();
  const { isStaff } = useAuth();
  const [params] = useSearchParams();
  const rideId = params.get('ride');
  const [all, setAll] = useState(isStaff);
  const { data, error, loading } = useApi<any[]>(`/help${all ? '?all=true' : ''}`);
  const [f, setF] = useState({ subject: rideId ? t('relatedRide', { rideId }) : '', body: '' });
  const nav = useNavigate();
  const { busy, error: actErr, run } = useAction();

  const submit = (e: FormEvent) => {
    e.preventDefault();
    run(async () => {
      const r = await post<{ id: number }>('/help', { ...f, ride_id: rideId ? Number(rideId) : undefined });
      nav(`/help/${r.id}`);
    }, t('submitted'));
  };

  return (
    <div className="stack">
      <div className="page-head">
        <div><h1>{t('title')}</h1><p>{t('desc')}</p></div>
      </div>
      <div className="split">
        <form className="card stack" onSubmit={submit}>
          <h3>{t('ask')}</h3>
          {rideId && <Pill tone="blue">{t('relatedRide', { rideId })}</Pill>}
          <Field label={t('subject')}>
            <input className="input" required value={f.subject} onChange={(e) => setF({ ...f, subject: e.target.value })} />
          </Field>
          <Field label={t('body')}>
            <textarea className="input" required value={f.body} onChange={(e) => setF({ ...f, body: e.target.value })} placeholder={t('bodyPlaceholder')} />
          </Field>
          <ErrorBox error={actErr} />
          <button className="btn primary" disabled={busy}>{t('submit')}</button>
          <p className="muted small">{t('urgent')}</p>
        </form>
        <div className="stack">
          {isStaff && (
            <div className="tabs">
              <button className={all ? 'on' : ''} onClick={() => setAll(true)}>{t('allHelp')}</button>
              <button className={!all ? 'on' : ''} onClick={() => setAll(false)}>{t('mine')}</button>
            </div>
          )}
          <ErrorBox error={error} />
          {loading ? <Spinner /> : !data?.length ? <Empty>{t('none')}</Empty> : (
            <div className="card"><div className="list">
              {data.map((tk: any) => (
                <Link key={tk.id} to={`/help/${tk.id}`} className="item">
                  <div className="grow">
                    <div className="bold">{tk.subject}</div>
                    <div className="muted tiny">{all && `${tk.user_name} · `}{fmtAgo(tk.updated_at, ft)}</div>
                  </div>
                  <StatusPill map={HELP_STATUS} status={tk.status} />
                </Link>
              ))}
            </div></div>
          )}
        </div>
      </div>
    </div>
  );
}

export function HelpDetail() {
  const { t } = useTranslation('help');
  const ft = useFormatT();
  const { id } = useParams();
  const { data: tk, error, reload } = useApi<any>(`/help/${id}`, 15000);
  const [body, setBody] = useState('');
  const { busy, error: actErr, run } = useAction();
  if (error) return <ErrorBox error={error} />;
  if (!tk) return <Spinner />;
  return (
    <div className="stack" style={{ maxWidth: 820 }}>
      <div className="page-head">
        <div><h1>{tk.subject}</h1>{tk.ride_id && <p><Link to={`/rides/${tk.ride_id}`}>{t('viewRide')}</Link></p>}</div>
        <StatusPill map={HELP_STATUS} status={tk.status} />
      </div>
      <div className="card stack">
        {tk.replies.map((r: any) => (
          <div key={r.id} className={`bubble ${r.staff ? '' : 'mine'}`} style={{ maxWidth: '100%' }}>
            <div className="who">{r.name}{r.staff && ` · ${t('staffTag')}`}</div>
            <div style={{ whiteSpace: 'pre-wrap' }}>{r.body}</div>
            <div className="at">{fmtAgo(r.created_at, ft)}</div>
          </div>
        ))}
      </div>
      {tk.status !== 'closed' && (
        <form className="card stack" onSubmit={(e) => { e.preventDefault(); run(() => post(`/help/${id}/replies`, { body }).then(() => { setBody(''); reload(); }), t('resolved')); }}>
          <Field label={t('reply')}>
            <textarea className="input" required value={body} onChange={(e) => setBody(e.target.value)} />
          </Field>
          <ErrorBox error={actErr} />
          <div className="btn-row">
            <button className="btn primary" disabled={busy}>{t('send')}</button>
            <button type="button" className="btn" disabled={busy} onClick={() => run(() => post(`/help/${id}/close`).then(reload), t('resolvedDone'))}>{t('resolved')}</button>
          </div>
        </form>
      )}
    </div>
  );
}
