import { FormEvent, useEffect, useRef, useState } from 'react';
import { post, useApi } from '../api';
import { fmtAgo, useFormatT } from '../format';
import { ErrorBox } from './ui';
import { useTranslation } from 'react-i18next';

interface Msg {
  id: number;
  user_id: number | null;
  name: string | null;
  body: string;
  created_at: string;
  mine: boolean;
}
interface ChatData {
  open: boolean;
  members: string[];
  messages: Msg[];
}

interface PendingMsg {
  body: string;
  failed: boolean;
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
  const logRef = useRef<HTMLDivElement>(null);
  const stickRef = useRef(true);
  const prevCountRef = useRef(0);

  const scrollToBottom = () => {
    const el = logRef.current;
    if (!el) return;
    el.scrollTo?.({ top: el.scrollHeight });
    stickRef.current = true;
    setHasUnreadBelow(false);
  };

  const onLogScroll = () => {
    const el = logRef.current;
    if (!el) return;
    const nearBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 64;
    stickRef.current = nearBottom;
    if (nearBottom) setHasUnreadBelow(false);
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

  const send = (e: FormEvent) => {
    e.preventDefault();
    void submitMessage(text);
  };

  if (error) return <ErrorBox error={error} />;
  if (!data || !Array.isArray(data.messages)) return null;

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
          {data.messages.map((m) =>
            m.user_id === null ? (
              <div key={m.id} className="bubble system">{m.body}</div>
            ) : (
              <div key={m.id} className={`bubble ${m.mine ? 'mine' : ''}`}>
                {!m.mine && <div className="who">{m.name}</div>}
                <div>{m.body}</div>
                <div className="at">{fmtAgo(m.created_at, ft)}</div>
              </div>
            ),
          )}
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
                  t('sendingMsg')
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
