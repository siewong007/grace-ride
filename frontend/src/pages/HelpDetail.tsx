import { FormEvent, useEffect, useRef, useState } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { BookOpen, CheckCircle, ExternalLink, HelpCircle, Phone, RefreshCw, Send } from 'lucide-react';
import { post, useApi } from '../api';
import { useAuth } from '../auth';
import { GuideSummary, usePick } from './HelpGuides';
import {
  Button,
  Card,
  CardSkeleton,
  EmptyState,
  ErrorBox,
  Page,
  PageHeader,
  StatusPill,
  useAction,
  useToast,
} from '../components/ui';
import { fmtDay, fmtTime, HELP_STATUS, useFormatT } from '../format';

interface Reply {
  id: number;
  user_id: number;
  name: string;
  body: string;
  staff: boolean;
  created_at: string;
}

interface TicketMember {
  name: string;
  phone: string;
  other_open: number[];
}

export interface TicketDetailData {
  id: number;
  subject: string;
  ride_id: number | null;
  status: string;
  user_id: number;
  replies: Reply[];
  member?: TicketMember;
}

function renderMessageBody(text: string) {
  const guideRegex = /(\/help\/guides\/[a-zA-Z0-9\-_#]+)/g;
  const parts = text.split(guideRegex);
  return parts.map((part, i) => {
    if (part.startsWith('/help/guides/')) {
      return (
        <Link key={i} to={part} className="guide-inline-link">
          {part}
        </Link>
      );
    }
    return part;
  });
}

export default function HelpDetail({
  ticketId: propTicketId,
  isTwoPane = false,
  onStatusChange,
}: {
  ticketId?: number;
  isTwoPane?: boolean;
  onStatusChange?: () => void;
}) {
  const { t } = useTranslation('help');
  const ft = useFormatT();
  const pick = usePick();
  const params = useParams();
  const nav = useNavigate();
  const { me, can } = useAuth();
  const toast = useToast();

  const id = propTicketId ?? Number(params.id);
  const isStaff = can('admin_help', 'view');
  const canUpdate = can('help', 'update') || can('admin_help', 'update');

  const { data: tk, error, reload, loading } = useApi<TicketDetailData>(id ? `/help/${id}` : null, 15000);
  const guidesApi = useApi<GuideSummary[]>(isStaff ? '/help/guides' : null);

  const [body, setBody] = useState('');
  const [optimisticReplies, setOptimisticReplies] = useState<Reply[]>([]);
  const [sendErr, setSendErr] = useState<string | null>(null);
  const [guidePickerOpen, setGuidePickerOpen] = useState(false);
  const { busy, error: actErr, run } = useAction();

  const logRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Auto-scroll to newest message
  useEffect(() => {
    if (typeof logRef.current?.scrollTo === 'function') {
      logRef.current.scrollTo({ top: logRef.current.scrollHeight, behavior: 'smooth' });
    }
  }, [tk?.replies.length, optimisticReplies.length]);

  // Reset optimistic state on reload
  useEffect(() => {
    if (tk?.replies) {
      setOptimisticReplies([]);
    }
  }, [tk?.replies]);

  if (error) {
    return (
      <Page width="narrow">
        <ErrorBox error={error} />
      </Page>
    );
  }

  if (loading && !tk) {
    return (
      <div className="stack" style={{ padding: 'var(--space-4)' }}>
        <CardSkeleton count={2} />
      </div>
    );
  }

  if (!tk) {
    return (
      <div className="stack" style={{ padding: 'var(--space-4)' }}>
        <EmptyState title={t('ticketNotFound', 'Request not found')} />
      </div>
    );
  }

  const allReplies = [...tk.replies, ...optimisticReplies];
  const isResolved = tk.status === 'closed';
  const isAnswered = tk.status === 'answered';
  const isOwner = me?.id === tk.user_id;

  const handleSend = async (e?: FormEvent) => {
    if (e) e.preventDefault();
    const trimmed = body.trim();
    if (!trimmed || busy) return;

    const tempReply: Reply = {
      id: Date.now(),
      user_id: me?.id ?? 0,
      name: me?.name ?? '',
      body: trimmed,
      staff: !!isStaff,
      created_at: new Date().toISOString(),
    };

    setOptimisticReplies((prev) => [...prev, tempReply]);
    setBody('');
    setSendErr(null);

    try {
      await post(`/help/${tk.id}/replies`, { body: trimmed });
      reload();
      onStatusChange?.();
    } catch (x) {
      // Rollback optimistic reply on failure
      setOptimisticReplies((prev) => prev.filter((r) => r.id !== tempReply.id));
      setBody(trimmed);
      setSendErr((x as Error).message);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      handleSend();
    }
  };

  const handleResolve = () => {
    run(async () => {
      await post(`/help/${tk.id}/close`);
      toast(t('resolvedDone'));
      reload();
      onStatusChange?.();
    });
  };

  const handleReopen = () => {
    run(async () => {
      await post(`/help/${tk.id}/reopen`);
      toast(t('reopenedDone', 'Request reopened'));
      reload();
      onStatusChange?.();
    });
  };

  const handleInsertGuide = (guide: GuideSummary) => {
    const title = pick(guide.title_zh, guide.title_en);
    const textToInsert = `\n${title}: /help/guides/${guide.slug}\n`;
    setBody((prev) => prev + textToInsert);
    setGuidePickerOpen(false);
    textareaRef.current?.focus();
  };

  const content = (
    <div className="stack help-detail-container">
      <PageHeader
        title={tk.subject}
        subtitle={
          tk.ride_id ? (
            <Link
              to={isStaff ? `/admin/rides/${tk.ride_id}` : `/rides/${tk.ride_id}`}
              className="row nowrap gap-xs bold small"
            >
              <ExternalLink size={14} />
              <span>{t('viewRide')} #{tk.ride_id}</span>
            </Link>
          ) : undefined
        }
        back={isTwoPane ? undefined : '/help'}
        actions={
          <div className="row nowrap gap-sm align-center">
            <StatusPill map={HELP_STATUS} status={tk.status} />
            {isResolved && (
              <Button
                variant="secondary"
                size="sm"
                onClick={handleReopen}
                disabled={busy}
                icon={<RefreshCw size={14} />}
              >
                {t('reopen', 'Reopen')}
              </Button>
            )}
          </div>
        }
      />

      <div className="help-detail-layout">
        <div className="help-detail-thread stack">
          {/* Turn status indicator */}
          <div className="help-turn-bar">
            {isResolved ? (
              <span className="muted small">{t('statusResolvedDesc', 'Resolved')}</span>
            ) : isAnswered ? (
              <span className="tone-blue bold small">
                {isOwner ? t('turnStaffReplied', 'Help centre replied — waiting for you') : t('turnWaitingMember', 'Replied to member')}
              </span>
            ) : (
              <span className="tone-amber bold small">
                {t('turnWaitingStaff', 'Waiting for help centre')}
              </span>
            )}
          </div>

          <Card className="stack help-thread-card">
            {/* Chat discussion log */}
            <div
              className="chat-log help-chat-log"
              role="log"
              aria-label={t('thread')}
              ref={logRef}
            >
              {allReplies.map((r, i) => {
                const prev = allReplies[i - 1];
                const showDay = !prev || fmtDay(r.created_at, ft) !== fmtDay(prev.created_at, ft);
                const isMine = r.user_id === me?.id;

                return (
                  <div key={r.id} className="stack-xs">
                    {showDay && (
                      <div className="center" style={{ margin: 'var(--space-2) 0' }}>
                        <span className="muted tiny" style={{ background: 'var(--surface-2)', padding: '2px 8px', borderRadius: 'var(--radius-pill)' }}>
                          {fmtDay(r.created_at, ft)}
                        </span>
                      </div>
                    )}
                    <div className={`bubble ${isMine ? 'mine' : ''}`}>
                      <div className="who">
                        {r.name}
                        {r.staff && ` · ${t('staffTag')}`}
                      </div>
                      <div className="pre-wrap">{renderMessageBody(r.body)}</div>
                      <div className="at">{fmtTime(r.created_at, ft)}</div>
                    </div>
                  </div>
                );
              })}
            </div>

            {/* "Did this solve it?" prompt for members when staff replied */}
            {isAnswered && isOwner && !isResolved && (
              <div className="card tight notice info stack-xs">
                <div className="bold small">{t('didThisSolveIt', 'Did this solve it?')}</div>
                <div className="btn-row">
                  <Button
                    variant="primary"
                    size="sm"
                    onClick={handleResolve}
                    disabled={busy}
                    icon={<CheckCircle size={14} />}
                  >
                    {t('yesResolved', 'Yes, resolved')}
                  </Button>
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => textareaRef.current?.focus()}
                    icon={<HelpCircle size={14} />}
                  >
                    {t('stillNeedHelp', 'I still need help')}
                  </Button>
                </div>
              </div>
            )}

            {/* Resolved banner */}
            {isResolved && (
              <div className="row between nowrap notice ok small">
                <span>{t('statusClosed')}</span>
                <Button variant="ghost" size="sm" onClick={handleReopen} disabled={busy}>
                  {t('reopen', 'Reopen')}
                </Button>
              </div>
            )}

            {/* Composer pinned to bottom */}
            {!isResolved && canUpdate && (
              <form onSubmit={handleSend} className="stack-xs help-reply-form">
                {isStaff && guidesApi.data && guidesApi.data.length > 0 && (
                  <div className="row between nowrap">
                    <span className="muted tiny">{t('composerTip', 'Ctrl+Enter to send')}</span>
                    <div style={{ position: 'relative' }}>
                      <Button
                        variant="ghost"
                        size="sm"
                        type="button"
                        onClick={() => setGuidePickerOpen(!guidePickerOpen)}
                        icon={<BookOpen size={14} />}
                      >
                        {t('insertGuide', 'Insert guide')}
                      </Button>
                      {guidePickerOpen && (
                        <div
                          className="card tight stack-xs"
                          style={{
                            position: 'absolute',
                            right: 0,
                            bottom: '100%',
                            marginBottom: 'var(--space-1)',
                            zIndex: 10,
                            minWidth: 220,
                            maxHeight: 180,
                            overflowY: 'auto',
                            boxShadow: 'var(--shadow-md)',
                          }}
                        >
                          <div className="bold tiny muted">{t('chooseGuide', 'Choose a guide:')}</div>
                          {guidesApi.data.map((g) => (
                            <button
                              key={g.id}
                              type="button"
                              className="item"
                              style={{ textAlign: 'left', width: '100%', cursor: 'pointer', padding: '4px 6px' }}
                              onClick={() => handleInsertGuide(g)}
                            >
                              <span className="small bold">{pick(g.title_zh, g.title_en)}</span>
                            </button>
                          ))}
                        </div>
                      )}
                    </div>
                  </div>
                )}

                <textarea
                  ref={textareaRef}
                  className="input help-reply-textarea"
                  value={body}
                  onChange={(e) => setBody(e.target.value)}
                  onKeyDown={handleKeyDown}
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
                  {!isAnswered && !isStaff && (
                    <Button
                      variant="secondary"
                      type="button"
                      disabled={busy}
                      onClick={handleResolve}
                    >
                      {t('resolved')}
                    </Button>
                  )}
                  {isStaff && (
                    <Button
                      variant="secondary"
                      type="button"
                      disabled={busy}
                      onClick={handleResolve}
                    >
                      {t('resolveTicket', 'Resolve')}
                    </Button>
                  )}
                </div>
              </form>
            )}

            <ErrorBox error={sendErr || actErr} />
          </Card>
        </div>

        {/* Member context panel (staff only) */}
        {isStaff && tk.member && (
          <aside className="help-member-context card stack">
            <h3>{t('memberContext', 'Member details')}</h3>
            <div className="stack-xs">
              <div className="bold">{tk.member.name}</div>
              {tk.member.phone && (
                <a className="btn sm secondary row nowrap gap-xs" href={`tel:${tk.member.phone}`}>
                  <Phone size={14} />
                  <span>{tk.member.phone}</span>
                </a>
              )}
            </div>

            {tk.ride_id && (
              <div className="stack-xs">
                <span className="muted tiny">{t('relatedRide', { rideId: tk.ride_id })}</span>
                <Link to={`/admin/rides/${tk.ride_id}`} className="small bold">
                  {t('viewAdminRide', 'View ride #{rideId}', { rideId: tk.ride_id })} →
                </Link>
              </div>
            )}

            {tk.member.other_open && tk.member.other_open.length > 0 && (
              <div className="stack-xs">
                <span className="muted tiny">
                  {t('otherOpenRequests', 'Other open requests ({count})', {
                    count: tk.member.other_open.length,
                  })}
                </span>
                <div className="list">
                  {tk.member.other_open.map((otherId) => (
                    <Link
                      key={otherId}
                      to={isTwoPane ? `/help/${otherId}` : `/help/${otherId}`}
                      onClick={() => {
                        if (isTwoPane) {
                          nav(`/help/${otherId}`);
                        }
                      }}
                      className="item small"
                    >
                      <span>#{otherId}</span>
                    </Link>
                  ))}
                </div>
              </div>
            )}
          </aside>
        )}
      </div>
    </div>
  );

  if (isTwoPane) {
    return content;
  }

  return <Page width="narrow">{content}</Page>;
}
