import { FormEvent, useState } from 'react';
import { Link, useNavigate, useParams, useSearchParams } from 'react-router-dom';
import { Phone, Send } from 'lucide-react';
import { post, useApi } from '../api';
import { useAuth } from '../auth';
import {
  Button,
  Card,
  EmptyState,
  ErrorBox,
  Field,
  Page,
  PageHeader,
  Pill,
  Spinner,
  StatusPill,
  Tabs,
  useAction,
} from '../components/ui';
import { fmtAgo, HELP_STATUS, useFormatT } from '../format';
import { useTranslation } from 'react-i18next';
import HelpGuides from './HelpGuides';

interface TicketSummary {
  id: number;
  subject: string;
  user_name: string;
  status: string;
  updated_at: string;
}

interface Reply {
  id: number;
  user_id: number;
  name: string;
  body: string;
  staff: boolean;
  created_at: string;
}

interface TicketDetailData {
  id: number;
  subject: string;
  ride_id: number | null;
  status: string;
  user_id: number;
  user_name: string;
  replies: Reply[];
}

export default function Help() {
  const { t } = useTranslation('help');
  const ft = useFormatT();
  const { can } = useAuth();
  // Help-desk staff see every request; members see their own.
  const isStaff = can('admin_help');
  // The admin guide is for anyone who can see a staff page, not just help desk.
  const [params, setParams] = useSearchParams();
  const [view, setView] = useState<'requests' | 'guide'>(() => params.get('tab') === 'guides' ? 'guide' : 'requests');
  const rideId = params.get('ride');
  const [all, setAll] = useState(isStaff);
  const { data, error, loading } = useApi<TicketSummary[]>(`/help${all ? '?all=true' : ''}`);
  const [f, setF] = useState({ subject: rideId ? t('relatedRide', { rideId }) : '', body: '' });
  const nav = useNavigate();
  const { busy, error: actErr, run } = useAction();

  const submit = (e: FormEvent) => {
    e.preventDefault();
    run(async () => {
      const r = await post<{ id: number }>('/help', {
        ...f,
        ride_id: rideId ? Number(rideId) : undefined,
      });
      nav(`/help/${r.id}`);
    }, t('submitted'));
  };

  return (
    <Page width="wide">
      <div className="stack">
        <PageHeader title={t('title')} subtitle={t('desc')} />

        <Tabs
          value={view}
          onChange={(v) => {
            const next = v as 'requests' | 'guide';
            setView(next);
            setParams(next === 'guide' ? { tab: 'guides' } : {});
          }}
          items={[
            { value: 'requests', label: t('requestsTab') },
            { value: 'guide', label: t('guideTab') },
          ]}
        />

        {view === 'guide' ? (
          <HelpGuides />
        ) : (
          <div className="split">
          {can('help', 'create') && (
            <form className="card stack" onSubmit={submit}>
              <h3>{t('ask')}</h3>
              {rideId && <Pill tone="info">{t('relatedRide', { rideId })}</Pill>}
              <Field label={t('subject')}>
                <input
                  className="input"
                  required
                  value={f.subject}
                  onChange={(e) => setF({ ...f, subject: e.target.value })}
                />
              </Field>
              <Field label={t('body')}>
                <textarea
                  className="input"
                  required
                  value={f.body}
                  onChange={(e) => setF({ ...f, body: e.target.value })}
                  placeholder={t('bodyPlaceholder')}
                  rows={4}
                />
              </Field>
              <ErrorBox error={actErr} />
              <Button variant="primary" block type="submit" loading={busy}>
                {t('submit')}
              </Button>
              <div className="stack-xs">
                <p className="muted small">{t('urgent')}</p>
                <a className="btn sm secondary" href="tel:+60379573888">
                  <Phone size={14} aria-hidden="true" />
                  <span>{t('callTransport')}</span>
                </a>
              </div>
            </form>
          )}

          <div className="stack">
            {isStaff && (
              <Tabs
                value={all ? 'all' : 'mine'}
                onChange={(v) => setAll(v === 'all')}
                items={[
                  { value: 'all', label: t('allHelp') },
                  { value: 'mine', label: t('mine') },
                ]}
              />
            )}
            <ErrorBox error={error} />
            {loading ? (
              <Spinner />
            ) : !data?.length ? (
              <EmptyState title={t('none')} />
            ) : (
              <Card className="stack">
                <div className="list">
                  {data.map((tk) => (
                    <Link key={tk.id} to={`/help/${tk.id}`} className="item">
                      <div className="grow min-w-0">
                        <div className="bold ellipsis">{tk.subject}</div>
                        <div className="muted tiny">
                          {all && `${tk.user_name} · `}
                          {fmtAgo(tk.updated_at, ft)}
                        </div>
                      </div>
                      <StatusPill map={HELP_STATUS} status={tk.status} />
                    </Link>
                  ))}
                </div>
              </Card>
            )}
          </div>
        </div>
        )}
      </div>
    </Page>
  );
}

export function HelpDetail() {
  const { t } = useTranslation('help');
  const ft = useFormatT();
  const { id } = useParams();
  const { can } = useAuth();
  const { data: tk, error, reload } = useApi<TicketDetailData>(`/help/${id}`, 15000);
  const [body, setBody] = useState('');
  const { busy, error: actErr, run } = useAction();
  const canUpdate = can('help', 'update') || can('admin_help', 'update');

  if (error) return <ErrorBox error={error} />;
  if (!tk) return <Spinner />;

  const sendReply = (e: FormEvent) => {
    e.preventDefault();
    if (!body.trim()) return;
    run(async () => {
      await post(`/help/${tk.id}/replies`, { body });
      setBody('');
      reload();
    });
  };

  const closeTicket = () => {
    run(async () => {
      await post(`/help/${tk.id}/close`);
      reload();
    }, t('resolvedDone'));
  };

  return (
    <Page width="narrow">
      <div className="stack">
        <PageHeader
          title={tk.subject}
          subtitle={
            tk.ride_id ? (
              <Link to={`/rides/${tk.ride_id}`}>{t('viewRide')}</Link>
            ) : undefined
          }
          back="/help"
          actions={<StatusPill map={HELP_STATUS} status={tk.status} />}
        />

        <Card className="stack">
          {/* Chat-style thread */}
          <div className="chat-log" role="log" aria-label={t('thread')}>
            {tk.replies.map((r) => (
              <div
                key={r.id}
                className={`bubble ${r.staff ? '' : 'mine'}`}
              >
                <div className="who">
                  {r.name}
                  {r.staff && ` · ${t('staffTag')}`}
                </div>
                <div className="pre-wrap">{r.body}</div>
                <div className="at">{fmtAgo(r.created_at, ft)}</div>
              </div>
            ))}
          </div>

          {tk.status !== 'closed' ? (
            canUpdate ? (
              <form onSubmit={sendReply} className="stack-sm">
                <textarea
                  className="input"
                  value={body}
                  onChange={(e) => setBody(e.target.value)}
                  placeholder={t('reply')}
                  rows={3}
                />
                <div className="btn-row">
                  <Button
                    variant="primary"
                    type="submit"
                    disabled={busy || !body.trim()}
                    loading={busy}
                    icon={<Send size={16} />}
                  >
                    {t('send')}
                  </Button>
                  <Button
                    variant="secondary"
                    type="button"
                    disabled={busy}
                    onClick={closeTicket}
                  >
                    {t('resolved')}
                  </Button>
                </div>
              </form>
            ) : null
          ) : (
            <div className="notice ok small">{t('statusClosed')}</div>
          )}

          <ErrorBox error={actErr} />
        </Card>
      </div>
    </Page>
  );
}
