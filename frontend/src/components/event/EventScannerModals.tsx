import { useState, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { QrCode } from 'lucide-react';
import { post } from '../../api';
import {
  Button,
  ErrorBox,
  Modal,
  Notice,
  useAction,
  useToast,
} from '../ui';
import type { EventFull } from './types';

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
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const [cameraActive, setCameraActive] = useState(false);

  const extractCode = (input: string) => {
    try {
      if (input.includes('code=')) {
        const u = new URL(input, window.location.origin);
        return u.searchParams.get('code') || input;
      }
    } catch {
      // not a full url
    }
    return input.trim();
  };

  const handleCheckin = (codeToUse: string) => {
    const clean = extractCode(codeToUse);
    run(async () => {
      await post(`/events/${event.id}/checkin`, { code: clean });
      onSuccess();
    }, t('checkinSuccess'));
  };
  const checkinRef = useRef(handleCheckin);
  checkinRef.current = handleCheckin;

  useEffect(() => {
    let cancelled = false;
    let stream: MediaStream | null = null;
    let timer: number | null = null;

    if (navigator.mediaDevices && navigator.mediaDevices.getUserMedia) {
      navigator.mediaDevices
        .getUserMedia({ video: { facingMode: 'environment' } })
        .then((s) => {
          if (cancelled) {
            s.getTracks().forEach((track) => track.stop());
            return;
          }
          stream = s;
          if (videoRef.current) {
            videoRef.current.srcObject = s;
            videoRef.current.play().catch(() => {});
            setCameraActive(true);
          }
          if ('BarcodeDetector' in window) {
            // @ts-expect-error BarcodeDetector browser standard
            const detector = new window.BarcodeDetector({ formats: ['qr_code'] });
            const scanLoop = async () => {
              if (cancelled) return;
              if (
                videoRef.current &&
                videoRef.current.readyState === videoRef.current.HAVE_ENOUGH_DATA
              ) {
                try {
                  const barcodes = await detector.detect(videoRef.current);
                  if (barcodes.length > 0) {
                    const raw = barcodes[0].rawValue;
                    checkinRef.current(raw);
                    return;
                  }
                } catch {
                  // ignore
                }
              }
              if (!cancelled) timer = window.setTimeout(scanLoop, 250);
            };
            timer = window.setTimeout(scanLoop, 500);
          }
        })
        .catch(() => {
          if (!cancelled) setCameraActive(false);
        });
    }

    return () => {
      cancelled = true;
      if (timer) clearTimeout(timer);
      if (stream) {
        stream.getTracks().forEach((track) => track.stop());
      }
    };
  }, []);

  return (
    <Modal title={t('scanQrModalTitle')} onClose={onClose}>
      <div className="stack">
        <p className="muted small">{t('scanQrModalDesc')}</p>

        {cameraActive && (
          <div
            style={{
              position: 'relative',
              width: '100%',
              maxWidth: '320px',
              margin: '0 auto',
              overflow: 'hidden',
              borderRadius: '12px',
              background: 'var(--surface-2)',
            }}
          >
            <video
              ref={videoRef}
              style={{ width: '100%', height: '240px', objectFit: 'cover' }}
              muted
              playsInline
            />
          </div>
        )}

        <form
          className="stack-sm"
          onSubmit={(e) => {
            e.preventDefault();
            if (code.trim()) handleCheckin(code);
          }}
        >
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
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const [cameraOn, setCameraOn] = useState(false);
  const hasBarcodeDetector = typeof window !== 'undefined' && 'BarcodeDetector' in window;

  const stopCamera = () => {
    if (streamRef.current) {
      streamRef.current.getTracks().forEach((tr) => tr.stop());
      streamRef.current = null;
    }
    setCameraOn(false);
  };

  useEffect(() => () => stopCamera(), []);

  const submitCode = async (input: string): Promise<boolean> => {
    const trimmed = input.trim();
    const parts = trimmed.split(':');
    if (parts[0] === 'GRPASS') {
      if (parts.length !== 3 || parts[1] !== String(eventId) || !parts[2].trim()) {
        setScanError(t('events.INVALID_PASS_CODE', { ns: 'errors' }));
        return false;
      }
    }
    const code = parts.length === 3 && parts[0] === 'GRPASS' ? parts[2].trim() : trimmed;
    if (!code) return false;
    setScanError(null);
    return run(async () => {
      const res = await post<{ ok: boolean; user: { id: number; name: string } }>(
        `/events/${eventId}/checkin/scan-member`,
        { pass_code: code },
      );
      stopCamera();
      toast.success(t('memberCheckedIn', { name: res.user?.name ?? '' }));
      onSuccess();
      onClose();
    });
  };
  const submitCodeRef = useRef(submitCode);
  submitCodeRef.current = submitCode;

  useEffect(() => {
    if (!cameraOn) return;
    let cancelled = false;
    let raf = 0;
    let retryTimer = 0;
    const start = async () => {
      try {
        const stream = await navigator.mediaDevices.getUserMedia({ video: { facingMode: 'environment' } });
        if (cancelled) {
          stream.getTracks().forEach((tr) => tr.stop());
          return;
        }
        streamRef.current = stream;
        if (videoRef.current) {
          videoRef.current.srcObject = stream;
          await videoRef.current.play();
        }
        if (cancelled) {
          stream.getTracks().forEach((tr) => tr.stop());
          streamRef.current = null;
          return;
        }
        const Detector = (
          window as Window & {
            BarcodeDetector?: new (opts?: { formats?: string[] }) => {
              detect(source: HTMLVideoElement): Promise<{ rawValue?: string }[]>;
            };
          }
        ).BarcodeDetector;
        if (Detector) {
          const detector = new Detector({ formats: ['qr_code'] });
          const scan = async () => {
            if (cancelled || !videoRef.current) return;
            try {
              const codes = await detector.detect(videoRef.current);
              if (codes.length > 0 && codes[0].rawValue) {
                const ok = await submitCodeRef.current(codes[0].rawValue);
                if (ok || cancelled) return;
                retryTimer = window.setTimeout(() => {
                  if (!cancelled) raf = requestAnimationFrame(scan);
                }, 1500);
                return;
              }
            } catch {
              // skip frame
            }
            if (!cancelled) raf = requestAnimationFrame(scan);
          };
          raf = requestAnimationFrame(scan);
        }
      } catch {
        if (!cancelled) setCameraOn(false);
      }
    };
    void start();
    return () => {
      cancelled = true;
      cancelAnimationFrame(raf);
      if (retryTimer) window.clearTimeout(retryTimer);
      stopCamera();
    };
  }, [cameraOn]);

  return (
    <Modal title={t('scanMemberPass')} onClose={onClose}>
      <div className="stack">
        <p className="muted small">{t('scanMemberPassDesc')}</p>
        {cameraOn && (
          <div style={{ borderRadius: 'var(--radius)', overflow: 'hidden', background: 'var(--surface-2)' }}>
            <video ref={videoRef} playsInline muted style={{ width: '100%', maxHeight: 240, display: 'block', objectFit: 'cover' }} />
          </div>
        )}
        {cameraOn && !hasBarcodeDetector && (
          <Notice tone="info">{t('scanMemberPassDesc')}</Notice>
        )}
        <div className="btn-row">
          <Button variant="secondary" size="sm" onClick={() => setCameraOn((v) => !v)} icon={<QrCode size={16} />}>
            {t('scanQrCheckin')}
          </Button>
        </div>
        <div className="row nowrap gap-sm">
          <input
            className="input grow mono"
            value={rawCode}
            onChange={(e) => setRawCode(e.target.value)}
            placeholder={t('memberPassPlaceholder')}
            onKeyDown={(e) => e.key === 'Enter' && void submitCode(rawCode)}
          />
          <Button variant="primary" disabled={busy || !rawCode.trim()} onClick={() => void submitCode(rawCode)}>
            {t('submitCheckin')}
          </Button>
        </div>
        <ErrorBox error={scanError || error} />
      </div>
    </Modal>
  );
}
