import { useState, useEffect } from 'react';
import QRCode from 'qrcode';
import { Copy, Maximize2, Minimize2, RefreshCw } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { post, useApi } from '../../api';
import {
  Button,
  Modal,
  Spinner,
  useAction,
  useConfirm,
} from '../ui';
import { fmtDateTime, useFormatT } from '../../format';
import type { EventFull } from './types';

export function AttendanceQrModal({
  event,
  onClose,
  onResetCode,
}: {
  event: EventFull;
  onClose: () => void;
  onResetCode: (newCode: string) => void;
}) {
  const { t } = useTranslation('eventDetail');
  const ft = useFormatT();
  const confirm = useConfirm();
  const { busy, run } = useAction();
  const [qrUrl, setQrUrl] = useState<string | null>(null);
  const [fullscreen, setFullscreen] = useState(false);
  const [copied, setCopied] = useState(false);

  const checkinUrl = `${window.location.origin}/events/${event.id}/checkin${
    event.checkin_code ? `?code=${encodeURIComponent(event.checkin_code)}` : ''
  }`;

  useEffect(() => {
    QRCode.toDataURL(checkinUrl, {
      margin: 1,
      width: 280,
      errorCorrectionLevel: 'M',
    })
      .then(setQrUrl)
      .catch(() => setQrUrl(null));
  }, [checkinUrl]);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(checkinUrl);
      setCopied(true);
      setTimeout(() => setCopied(false), 2500);
    } catch {
      // fallback
    }
  };

  const handleReset = async () => {
    const ok = await confirm({
      title: t('qrResetCode'),
      body: t('qrResetConfirm'),
      confirmLabel: t('qrResetCode'),
      tone: 'danger',
    });
    if (ok) {
      run(async () => {
        const res = await post<{ checkin_code: string }>(`/events/${event.id}/checkin-code/reset`);
        onResetCode(res.checkin_code);
      }, t('qrResetDone'));
    }
  };

  return (
    <Modal title={t('qrModalTitle')} onClose={onClose}>
      <div
        className="stack items-center"
        style={
          fullscreen
            ? {
                position: 'fixed',
                inset: 0,
                zIndex: 1000,
                background: 'var(--bg)',
                padding: '2rem',
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
              }
            : undefined
        }
      >
        <div className="text-center stack-sm">
          <h3>{event.title}</h3>
          <p className="muted small">
            {fmtDateTime(event.starts_at, ft)} · {event.location}
          </p>
        </div>

        {qrUrl ? (
          <div
            className="totp-qr"
            style={{
              padding: '1rem',
              borderRadius: '12px',
              border: '1px solid var(--border)',
            }}
          >
            <img src={qrUrl} width={280} height={280} alt={t('qrModalTitle')} />
          </div>
        ) : (
          <Spinner />
        )}

        <div className="text-center stack-sm">
          {event.checkin_code && (
            <p className="small">
              <span className="muted">{t('enterCodePlaceholder')}: </span>
              <strong style={{ letterSpacing: '2px', fontFamily: 'monospace', fontSize: '1.25rem' }}>
                {event.checkin_code}
              </strong>
            </p>
          )}
          <p className="muted tiny max-w-sm">{t('qrModalDesc')}</p>
        </div>

        <div className="btn-row wrap justify-center">
          <Button variant="secondary" size="sm" icon={<Copy size={16} />} onClick={handleCopy}>
            {copied ? t('qrLinkCopied') : t('qrCopyLink')}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            icon={fullscreen ? <Minimize2 size={16} /> : <Maximize2 size={16} />}
            onClick={() => setFullscreen(!fullscreen)}
          >
            {fullscreen ? t('qrExitFullscreen') : t('qrFullscreen')}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            icon={<RefreshCw size={16} />}
            disabled={busy}
            onClick={handleReset}
          >
            {t('qrResetCode')}
          </Button>
        </div>
      </div>
    </Modal>
  );
}

export function WalkIn({ onClose, onPick }: { onClose: () => void; onPick: (uid: number) => void }) {
  const { t } = useTranslation('eventDetail');
  const [q, setQ] = useState('');
  const { data } = useApi<{ id: number; name: string }[]>(
    q.length >= 1 ? `/admin/users?q=${encodeURIComponent(q)}` : null,
  );

  return (
    <Modal title={t('walkInTitle')} onClose={onClose}>
      <input
        className="input"
        placeholder={t('walkInPlaceholder')}
        value={q}
        onChange={(e) => setQ(e.target.value)}
      />
      <div className="list">
        {data?.slice(0, 10).map((u) => (
          <button
            key={u.id}
            type="button"
            className="item btn ghost text-left"
            onClick={() => onPick(u.id)}
          >
            {u.name}
          </button>
        ))}
      </div>
    </Modal>
  );
}

export function MemberPassQrModal({
  event,
  passCode,
  onClose,
}: {
  event: EventFull;
  passCode: string;
  onClose: () => void;
}) {
  const { t } = useTranslation('eventDetail');
  const [qrUrl, setQrUrl] = useState<string | null>(null);
  const payload = `GRPASS:${event.id}:${passCode}`;

  useEffect(() => {
    QRCode.toDataURL(payload, { margin: 1, width: 240, errorCorrectionLevel: 'M' })
      .then(setQrUrl)
      .catch(() => setQrUrl(null));
  }, [payload]);

  return (
    <Modal title={t('myCheckinPass')} onClose={onClose}>
      <div className="stack items-center" style={{ padding: 'var(--space-2) 0' }}>
        {qrUrl && (
          <div className="totp-qr" style={{ padding: 'var(--space-3)', background: 'var(--surface)', borderRadius: 'var(--radius)' }}>
            <img src={qrUrl} width={240} height={240} alt={t('myCheckinPass')} />
          </div>
        )}
        <code className="card flat tight mono">{passCode}</code>
        <p className="muted tiny center max-w-sm">{t('myCheckinPassDesc')}</p>
      </div>
    </Modal>
  );
}
