import { FormEvent, Fragment, useCallback, useEffect, useRef, useState } from 'react';
import { post, useApi } from '../api';
import { fmtAgo, useFormatT } from '../format';
import { ErrorBox } from './ui';
import { useTranslation } from 'react-i18next';

type MsgStatus = 'sent' | 'delivered' | 'read';

interface Reaction {
  emoji: string;
  count: number;
  mine: boolean;
  names?: string[];
}

interface Msg {
  id: number;
  user_id: number | null;
  name: string | null;
  body: string;
  created_at: string;
  mine: boolean;
  /** Only on the viewer's own messages. */
  status?: MsgStatus;
  reactions?: Reaction[];
}
interface ChatData {
  open: boolean;
  members: string[];
  reactions_allowed?: string[];
  messages: Msg[];
}

interface PendingMsg {
  body: string;
  failed: boolean;
}

const DEFAULT_REACTIONS = ['👍', '❤️', '😂', '😮', '😢', '🙏'];

/** WhatsApp-style receipt: one tick sent, two ticks delivered, two coloured ticks read. */
function Ticks({ status, label }: { status: MsgStatus | 'pending'; label: string }) {
  if (status === 'pending') {
    return (
      <span className="chat-ticks pending" role="img" aria-label={label} title={label}>
        <svg viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.4" aria-hidden="true">
          <circle cx="6" cy="6" r="5" />
          <path d="M6 3v3l2 1.5" strokeLinecap="round" />
        </svg>
      </span>
    );
  }
  const double = status !== 'sent';
  return (
    <span className={`chat-ticks ${status}`} role="img" aria-label={label} title={label}>
      <svg viewBox="0 0 20 12" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        {double ? (
          <>
            <path d="M1.5 6.5l3 3 6.5-7.5" />
            <path d="M8.5 9.5l7-7.5" />
          </>
        ) : (
          <path d="M5 6.5l3 3 6.5-7.5" />
        )}
      </svg>
    </span>
  );
}

export default function Chat({
  rideId,
  viewerRole,
}: {
  rideId: number;
  viewerRole?: 'passenger' | 'driver' | 'staff';
}) {
  const { t } = useTranslation('rideDetail');
  const ft = useFormatT();
  const { data, error, reload } = useApi<ChatData>(`/rides/${rideId}/messages`, 4000);
  const [text, setText] = useState('');
  const [sending, setSending] = useState(false);
  const [pending, setPending] = useState<PendingMsg | null>(null);
  const [sendErr, setSendErr] = useState<string | null>(null);
  const [hasUnreadBelow, setHasUnreadBelow] = useState(false);
  const [pickerFor, setPickerFor] = useState<number | null>(null);
  const [reacting, setReacting] = useState(false);
  const logRef = useRef<HTMLDivElement>(null);
  const stickRef = useRef(true);
  const prevCountRef = useRef(0);
  const readUpToRef = useRef(0);
  // Read receipts count only once the chat log is actually on screen (it often sits
  // below the fold of the ride page). Without IntersectionObserver, assume it is.
  const [inView, setInView] = useState(() => typeof IntersectionObserver === 'undefined');
  const hasData = Boolean(data);

  useEffect(() => {
    const el = logRef.current;
    if (!el || typeof IntersectionObserver === 'undefined') return;
    const io = new IntersectionObserver(
      (entries) => setInView(entries.some((e) => e.isIntersecting)),
      { threshold: 0.3 },
    );
    io.observe(el);
    return () => io.disconnect();
  }, [hasData]);

  const lastId = data?.messages?.length ? data.messages[data.messages.length - 1].id : 0;

  /** Tell the server the viewer has seen everything up to the newest message, while
   *  the chat is on screen and scrolled to the bottom. */
  const markRead = useCallback(() => {
    if (!lastId || lastId <= readUpToRef.current) return;
    if (typeof document !== 'undefined' && document.visibilityState === 'hidden') return;
    if (!inView || !stickRef.current) return;
    readUpToRef.current = lastId;
    post(`/rides/${rideId}/messages/read`, { up_to: lastId }).catch(() => {
      readUpToRef.current = 0;
    });
  }, [inView, lastId, rideId]);

  useEffect(() => {
    markRead();
    const onVisible = () => markRead();
    document.addEventListener('visibilitychange', onVisible);
    return () => document.removeEventListener('visibilitychange', onVisible);
  }, [markRead]);

  const scrollToBottom = () => {
    const el = logRef.current;
    if (!el) return;
    el.scrollTo?.({ top: el.scrollHeight });
    stickRef.current = true;
    setHasUnreadBelow(false);
    markRead();
  };

  const onLogScroll = () => {
    const el = logRef.current;
    if (!el) return;
    const nearBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 64;
    stickRef.current = nearBottom;
    if (nearBottom) {
      setHasUnreadBelow(false);
      markRead();
    }
  };

  useEffect(() => {
    const count = data?.messages?.length ?? 0;
    if (count > prevCountRef.current) {
      if (stickRef.current || prevCountRef.current === 0) {
        logRef.current?.scrollTo?.({ top: logRef.current.scrollHeight });
        setHasUnreadBelow(false);
      } else {
        setHasUnreadBelow(true);
      }
    }
    prevCountRef.current = count;
  }, [data?.messages?.length]);

  useEffect(() => {
    if (pickerFor === null) return;
    const onKey = (e: globalThis.KeyboardEvent) => {
      if (e.key === 'Escape') setPickerFor(null);
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [pickerFor]);

  const submitMessage = async (raw: string) => {
    const body = raw.trim();
    if (!body || sending) return;
    setSending(true);
    setSendErr(null);
    setPending({ body, failed: false });
    stickRef.current = true;
    try {
      await post(`/rides/${rideId}/messages`, { body });
      setText((cur) => (cur.trim() === body ? '' : cur));
      setPending(null);
      await reload();
      scrollToBottom();
    } catch (x) {
      setPending({ body, failed: true });
      setSendErr((x as Error).message);
    } finally {
      setSending(false);
    }
  };

  const react = async (messageId: number, emoji: string) => {
    if (reacting) return;
    setReacting(true);
    setSendErr(null);
    setPickerFor(null);
    try {
      await post(`/rides/${rideId}/messages/${messageId}/reactions`, { emoji });
      await reload();
    } catch (x) {
      setSendErr((x as Error).message);
    } finally {
      setReacting(false);
    }
  };

  const send = (e: FormEvent) => {
    e.preventDefault();
    void submitMessage(text);
  };

  if (error) return <ErrorBox error={error} />;
  if (!data || !Array.isArray(data.messages)) return null;

  const allowed = data.reactions_allowed?.length ? data.reactions_allowed : DEFAULT_REACTIONS;
  const canReact = data.open;
  const statusLabel = (s: MsgStatus) =>
    s === 'read' ? t('msgRead') : s === 'delivered' ? t('msgDelivered') : t('msgSent');

  const togglePicker = (id: number) => setPickerFor((cur) => (cur === id ? null : id));
  const quickReplies =
    viewerRole === 'driver'
      ? [t('quickDriverOnWay'), t('quickDriver5m'), t('quickDriverOutside')]
      : viewerRole === 'passenger'
        ? [t('quickPassengerReady'), t('quickPassenger5m'), t('quickPassengerGate')]
        : [];

  return (
    <div className="card stack-sm">
      <div className="row between">
        <h2>{t('chat')}</h2>
        <span className="muted small">{t('chatMembers', { count: data.members.length })}</span>
      </div>
      <div className="chat">
        <div className="chat-log" ref={logRef} onScroll={onLogScroll} aria-live="polite">
          {data.messages.map((m) => {
            if (m.user_id === null) {
              return (
                <div key={m.id} className="bubble system">
                  {m.body}
                </div>
              );
            }
            const reactions = m.reactions ?? [];
            const myReaction = reactions.find((r) => r.mine)?.emoji;
            return (
              <Fragment key={m.id}>
                <div className={`bubble ${m.mine ? 'mine' : ''}`}>
                  {!m.mine && <div className="who">{m.name}</div>}
                  <div>{m.body}</div>
                  <div className="at">
                    <span>{fmtAgo(m.created_at, ft)}</span>
                    {m.mine && m.status && <Ticks status={m.status} label={statusLabel(m.status)} />}
                    {canReact && (
                      <button
                        type="button"
                        className="chat-react-add"
                        aria-haspopup="true"
                        aria-expanded={pickerFor === m.id}
                        aria-label={t('reactToMessage')}
                        title={t('reactToMessage')}
                        onClick={() => togglePicker(m.id)}
                      >
                        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" aria-hidden="true">
                          <circle cx="8" cy="8" r="6.3" />
                          <path d="M5.6 9.6c.6.8 1.4 1.2 2.4 1.2s1.8-.4 2.4-1.2" />
                          <circle cx="6" cy="6.6" r=".4" fill="currentColor" />
                          <circle cx="10" cy="6.6" r=".4" fill="currentColor" />
                        </svg>
                      </button>
                    )}
                  </div>
                  {reactions.length > 0 && (
                    <div className="chat-reacts" role="group" aria-label={t('reactionsLabel')}>
                      {reactions.map((r) => (
                        <button
                          key={r.emoji}
                          type="button"
                          className={`chat-react-chip ${r.mine ? 'on' : ''}`}
                          aria-pressed={r.mine}
                          disabled={!canReact || reacting}
                          title={r.names?.join(', ')}
                          aria-label={t('reactionChip', {
                            emoji: r.emoji,
                            count: r.count,
                            names: r.names?.join(', ') ?? '',
                          })}
                          onClick={() => void react(m.id, r.emoji)}
                        >
                          <span aria-hidden="true">{r.emoji}</span>
                          {r.count > 1 && <span aria-hidden="true">{r.count}</span>}
                        </button>
                      ))}
                    </div>
                  )}
                </div>
                {pickerFor === m.id && canReact && (
                  <div
                    className={`chat-react-picker ${m.mine ? 'mine' : ''}`}
                    role="group"
                    aria-label={t('pickReaction')}
                  >
                    {allowed.map((emoji) => (
                      <button
                        key={emoji}
                        type="button"
                        aria-pressed={myReaction === emoji}
                        aria-label={t('reactWith', { emoji })}
                        disabled={reacting}
                        onClick={() => void react(m.id, emoji)}
                      >
                        {emoji}
                      </button>
                    ))}
                  </div>
                )}
              </Fragment>
            );
          })}
          {pending && (
            <div className="bubble mine pending-bubble">
              <div>{pending.body}</div>
              <div className="at">
                {pending.failed ? (
                  <button
                    type="button"
                    className="chat-retry-btn"
                    disabled={sending}
                    onClick={() => void submitMessage(pending.body)}
                  >
                    {t('sendFailedRetry')}
                  </button>
                ) : (
                  <>
                    <span>{t('sendingMsg')}</span>
                    <Ticks status="pending" label={t('sendingMsg')} />
                  </>
                )}
              </div>
            </div>
          )}
        </div>
        {hasUnreadBelow && (
          <button type="button" className="btn sm secondary chat-new-pill" onClick={scrollToBottom}>
            {t('newMessagesBelow')}
          </button>
        )}
        {data.open ? (
          <div className="stack-xs">
            {quickReplies.length > 0 && (
              <div className="chips chat-quick-chips" role="group" aria-label={t('quickRepliesLabel')}>
                {quickReplies.map((phrase) => (
                  <button
                    key={phrase}
                    type="button"
                    className="chip"
                    disabled={sending}
                    onClick={() => void submitMessage(phrase)}
                  >
                    {phrase}
                  </button>
                ))}
              </div>
            )}
            <form className="chat-input" onSubmit={send}>
              <input
                className="input"
                value={text}
                onChange={(e) => setText(e.target.value)}
                placeholder={t('chatPlaceholder')}
                maxLength={1000}
                aria-label={t('chatInput')}
              />
              <button className="btn primary" type="submit" disabled={sending || !text.trim()}>
                {sending ? t('sendingMsg') : t('send')}
              </button>
            </form>
          </div>
        ) : (
          <div className="muted small center">{t('chatReadOnly')}</div>
        )}
        <ErrorBox error={sendErr} />
      </div>
    </div>
  );
}
