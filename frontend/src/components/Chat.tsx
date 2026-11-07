import { FormEvent, useEffect, useRef, useState } from 'react';
import { post, useApi } from '../api';
import { fmtAgo } from '../format';
import { ErrorBox } from './ui';
import { useTranslation } from 'react-i18next';

interface Msg { id: number; user_id: number | null; name: string | null; body: string; created_at: string; mine: boolean }
interface ChatData { open: boolean; members: string[]; messages: Msg[] }

export default function Chat({ rideId }: { rideId: number }) {
  const { t } = useTranslation('rideDetail');
  const { data, error, reload } = useApi<ChatData>(`/rides/${rideId}/messages`, 4000);
  const [text, setText] = useState('');
  const [sendErr, setSendErr] = useState<string | null>(null);
  const logRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    logRef.current?.scrollTo({ top: logRef.current.scrollHeight });
  }, [data?.messages.length]);

  const send = async (e: FormEvent) => {
    e.preventDefault();
    if (!text.trim()) return;
    try {
      await post(`/rides/${rideId}/messages`, { body: text });
      setText('');
      setSendErr(null);
      reload();
    } catch (x) {
      setSendErr((x as Error).message);
    }
  };

  if (error) return <ErrorBox error={error} />;
  if (!data) return null;
  return (
    <div className="card stack-sm">
      <div className="row between">
        <h3>{t('chat')}</h3>
        <span className="muted small">{t('chatMembers', { count: data.members.length })}</span>
      </div>
      <div className="chat">
        <div className="chat-log" ref={logRef} aria-live="polite">
          {data.messages.map((m) =>
            m.user_id === null ? (
              <div key={m.id} className="bubble system">{m.body}</div>
            ) : (
              <div key={m.id} className={`bubble ${m.mine ? 'mine' : ''}`}>
                {!m.mine && <div className="who">{m.name}</div>}
                <div>{m.body}</div>
                <div className="at">{fmtAgo(m.created_at)}</div>
              </div>
            ),
          )}
        </div>
        {data.open ? (
          <form className="chat-input" onSubmit={send}>
            <input className="input" value={text} onChange={(e) => setText(e.target.value)} placeholder={t('chatPlaceholder')} maxLength={1000} aria-label="讯息" />
            <button className="btn primary" type="submit">{t('send')}</button>
          </form>
        ) : (
          <div className="muted small center">{t('chatReadOnly')}</div>
        )}
        <ErrorBox error={sendErr} />
      </div>
    </div>
  );
}
