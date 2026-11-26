import { useState, type FormEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { Camera, CameraOff, QrCode } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { post } from '../../api';
import {
  Button,
  ErrorBox,
  Modal,
  useAction,
  useToast,
} from '../ui';
import type { EventFull } from './types';
import { QrCameraScanner } from './QrCameraScanner';
import { extractCheckinDetails } from './qrUtils';

export function EventScannerModal({
  event,
  onClose,
  onSuccess,
}: {
  event: EventFull;
  onClose: () => void;
  onSuccess: () => void;
}) {
  const { t } = useTranslation('eventDetail');
  const [code, setCode] = useState('');
  const { busy, error, run } = useAction();
  const [cameraActive, setCameraActive] = useState(true);

  const handleCheckin = (codeToUse: string) => {
    const details = extractCheckinDetails(codeToUse);
    const clean = details.code;
    if (!clean) return;
    run(async () => {
      await post(`/events/${event.id}/checkin`, { code: clean });
      onSuccess();
    }, t('checkinSuccess'));
  };

  const handleManualSubmit = (e: FormEvent) => {
    e.preventDefault();
    if (code.trim()) handleCheckin(code);
  };

  return (
    <Modal title={t('scanQrModalTitle')} onClose={onClose}>
      <div className="stack">
        <p className="muted small">{t('scanQrModalDesc')}</p>

        {cameraActive && (
          <QrCameraScanner
            active={cameraActive}
            onScan={(val) => {
              setCameraActive(false);
              handleCheckin(val);
            }}
          />
        )}

        <div className="row center">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setCameraActive((v) => !v)}
            icon={cameraActive ? <CameraOff size={16} /> : <Camera size={16} />}
          >
            {cameraActive ? t('stopCamera') : t('startCamera')}
          </Button>
        </div>

        <div
          style={{
            width: '100%',
            display: 'flex',
            alignItems: 'center',
            margin: '0.5rem 0',
            color: 'var(--muted)',
            fontSize: 'var(--fs-xs, 0.75rem)',
          }}
        >
          <div style={{ flex: 1, height: '1px', background: 'var(--border)' }} />
          <span style={{ padding: '0 0.5rem' }}>{t('cannotScanPrompt')}</span>
          <div style={{ flex: 1, height: '1px', background: 'var(--border)' }} />
        </div>

        <form className="stack-sm" onSubmit={handleManualSubmit}>
          <input
            className="input"
            type="text"
            placeholder={t('enterCodePlaceholder')}
            value={code}
            onChange={(e) => setCode(e.target.value)}
            disabled={busy}
          />
          <Button variant="primary" block type="submit" loading={busy} disabled={!code.trim()}>
            {t('submitCheckin')}
          </Button>
        </form>

        <p className="muted tiny">{t('participateNoRide')}</p>
        <ErrorBox error={error} />
      </div>
    </Modal>
  );
}

export function ScanMemberPassModal({
  eventId,
  onClose,
  onSuccess,
}: {
  eventId: number;
  onClose: () => void;
  onSuccess: () => void;
}) {
  const { t } = useTranslation('eventDetail');
  const toast = useToast();
  const [rawCode, setRawCode] = useState('');
  const [scanError, setScanError] = useState<string | null>(null);
  const { busy, error, run } = useAction();
  const [cameraOn, setCameraOn] = useState(false);

  const submitCode = async (input: string): Promise<boolean> => {
    const details = extractCheckinDetails(input);
    const code = details.code;
    if (details.eventId && String(details.eventId) !== String(eventId)) {
      setScanError(t('events.INVALID_PASS_CODE', { ns: 'errors' }));
      return false;
    }
    if (!code) return false;
    setScanError(null);
    return run(async () => {
      const res = await post<{ ok: boolean; user: { id: number; name: string } }>(
        `/events/${eventId}/checkin/scan-member`,
        { pass_code: code },
      );
      setCameraOn(false);
      toast.success(t('memberCheckedIn', { name: res.user?.name ?? '' }));
      onSuccess();
      onClose();
    });
  };

  return (
    <Modal title={t('scanMemberPass')} onClose={onClose}>
      <div className="stack">
        <p className="muted small">{t('scanMemberPassDesc')}</p>

        {cameraOn && (
          <QrCameraScanner
            active={cameraOn}
            onScan={(val) => {
              setCameraOn(false);
              void submitCode(val);
            }}
            hint={t('scanMemberPassDesc')}
          />
        )}

        <div className="btn-row">
          <Button
            variant="secondary"
            size="sm"
            onClick={() => setCameraOn((v) => !v)}
            icon={cameraOn ? <CameraOff size={16} /> : <QrCode size={16} />}
          >
            {cameraOn ? t('stopCamera') : t('scanQrCheckin')}
          </Button>
        </div>

        <div
          style={{
            width: '100%',
            display: 'flex',
            alignItems: 'center',
            margin: '0.5rem 0',
            color: 'var(--muted)',
            fontSize: 'var(--fs-xs, 0.75rem)',
          }}
        >
          <div style={{ flex: 1, height: '1px', background: 'var(--border)' }} />
          <span style={{ padding: '0 0.5rem' }}>{t('cannotScanPassPrompt')}</span>
          <div style={{ flex: 1, height: '1px', background: 'var(--border)' }} />
        </div>

        <form
          className="row nowrap gap-sm"
          onSubmit={(e) => {
            e.preventDefault();
            if (rawCode.trim()) void submitCode(rawCode);
          }}
        >
          <input
            className="input grow mono"
            value={rawCode}
            onChange={(e) => setRawCode(e.target.value)}
            placeholder={t('memberPassPlaceholder')}
            disabled={busy}
          />
          <Button variant="primary" type="submit" loading={busy} disabled={!rawCode.trim()}>
            {t('submitCheckin')}
          </Button>
        </form>

        <ErrorBox error={scanError || error} />
      </div>
    </Modal>
  );
}

export function UniversalCheckinScannerModal({
  onClose,
}: {
  onClose: () => void;
}) {
  const { t } = useTranslation('eventDetail');
  const navigate = useNavigate();
  const [manualCode, setManualCode] = useState('');
  const [cameraActive, setCameraActive] = useState(true);
  const { busy, error, run } = useAction();

  const handleScan = (scanned: string) => {
    setCameraActive(false);
    const details = extractCheckinDetails(scanned);
    if (details.eventId) {
      onClose();
      navigate(
        `/events/${details.eventId}/checkin${
          details.code ? `?code=${encodeURIComponent(details.code)}` : ''
        }`,
      );
      return;
    }
    if (details.code) {
      submitByCode(details.code);
      return;
    }
  };

  const submitByCode = (input: string) => {
    const details = extractCheckinDetails(input);
    if (details.eventId) {
      onClose();
      navigate(
        `/events/${details.eventId}/checkin${
          details.code ? `?code=${encodeURIComponent(details.code)}` : ''
        }`,
      );
      return;
    }
    const clean = details.code || input.trim();
    if (!clean) return;

    run(async () => {
      const res = await post<{ ok: boolean; event_id: number; title: string }>('/events/checkin', {
        code: clean,
      });
      onClose();
      navigate(`/events/${res.event_id}/checkin`);
    }, t('checkinSuccess'));
  };

  const handleManualSubmit = (e: FormEvent) => {
    e.preventDefault();
    submitByCode(manualCode);
  };

  return (
    <Modal title={t('universalScanTitle')} onClose={onClose}>
      <div className="stack">
        <p className="muted small">{t('universalScanDesc')}</p>

        {cameraActive && (
          <QrCameraScanner
            active={cameraActive}
            onScan={handleScan}
          />
        )}

        <div className="row center">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setCameraActive((v) => !v)}
            icon={cameraActive ? <CameraOff size={16} /> : <Camera size={16} />}
          >
            {cameraActive ? t('stopCamera') : t('startCamera')}
          </Button>
        </div>

        <div
          style={{
            width: '100%',
            display: 'flex',
            alignItems: 'center',
            margin: '0.5rem 0',
            color: 'var(--muted)',
            fontSize: 'var(--fs-xs, 0.75rem)',
          }}
        >
          <div style={{ flex: 1, height: '1px', background: 'var(--border)' }} />
          <span style={{ padding: '0 0.5rem' }}>{t('cannotScanPrompt')}</span>
          <div style={{ flex: 1, height: '1px', background: 'var(--border)' }} />
        </div>

        <form className="stack-sm" onSubmit={handleManualSubmit}>
          <input
            className="input"
            type="text"
            placeholder={t('enterCodeOrUrlPlaceholder')}
            value={manualCode}
            onChange={(e) => setManualCode(e.target.value)}
            disabled={busy}
          />
          <Button variant="primary" block type="submit" loading={busy} disabled={!manualCode.trim()}>
            {t('submitCheckin')}
          </Button>
        </form>

        <ErrorBox error={error} />
      </div>
    </Modal>
  );
}
