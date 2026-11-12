import { useEffect, useRef, useState } from 'react';
import { MapPin, Navigation } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { post } from '../api';
import { Button, ErrorBox, Field, Modal, useAction } from './ui';

export function requestPosition(): Promise<GeolocationPosition> {
  return new Promise((resolve, reject) => {
    if (!('geolocation' in navigator)) {
      reject(new Error('unsupported'));
      return;
    }
    navigator.geolocation.getCurrentPosition(resolve, reject, {
      enableHighAccuracy: true,
      timeout: 15000,
      maximumAge: 10000,
    });
  });
}

export const sendDriverPosition = (pos: GeolocationPosition) =>
  post('/driver/location', {
    lat: pos.coords.latitude,
    lng: pos.coords.longitude,
    heading: pos.coords.heading,
  }).catch(() => {});

/**
 * "Start trip" for drivers: explains that location is shared automatically, asks the
 * browser for permission, then starts the ride and sends the first position at once.
 */
export function StartTripButton({
  ride,
  onStarted,
  className = 'btn primary',
  label,
  block = false,
}: {
  ride: { id: number };
  onStarted: () => void;
  className?: string;
  label: string;
  block?: boolean;
}) {
  const { t } = useTranslation('rideDetail');
  const { busy, error, run } = useAction();
  const [open, setOpen] = useState(false);
  const [locErr, setLocErr] = useState<'denied' | 'unsupported' | null>(null);

  const start = async (pos: GeolocationPosition | null) => {
    const ok = await run(async () => {
      await post(`/rides/${ride.id}/start`);
      if (pos) await sendDriverPosition(pos);
    }, pos ? t('respondedStarted') : t('startedNoLocation'));
    if (ok) {
      setOpen(false);
      onStarted();
    }
  };

  const allow = async () => {
    setLocErr(null);
    try {
      await start(await requestPosition());
    } catch (e) {
      setLocErr((e as Error).message === 'unsupported' ? 'unsupported' : 'denied');
    }
  };

  return (
    <>
      <Button
        variant="primary"
        block={block}
        className={className}
        disabled={busy}
        onClick={() => {
          setLocErr(null);
          setOpen(true);
        }}
      >
        {label}
      </Button>
      {open && (
        <Modal title={t('startPromptTitle')} onClose={() => setOpen(false)}>
          <p>{t('startPromptBody')}</p>
          <ul className="small stack-sm">
            <li>{t('startPromptWho')}</li>
            <li>{t('startPromptWhen')}</li>
            <li>{t('startPromptPassenger')}</li>
          </ul>
          {locErr && (
            <div className="notice">
              {locErr === 'unsupported' ? t('sharingUnsupported') : t('sharingDenied')}
            </div>
          )}
          <ErrorBox error={error} />
          <Button
            variant="primary"
            block
            disabled={busy}
            onClick={allow}
            icon={<MapPin size={18} />}
          >
            {locErr ? t('tryAgain') : t('allowAndStart')}
          </Button>
          {locErr && (
            <Button
              variant="secondary"
              block
              disabled={busy}
              onClick={() => start(null)}
            >
              {t('startWithoutLocation')}
            </Button>
          )}
        </Modal>
      )}
    </>
  );
}

export function ShareLocation() {
  const { t } = useTranslation('rideDetail');
  const [state, setState] = useState<'idle' | 'on' | 'denied' | 'unsupported'>('idle');
  const last = useRef(0);
  const watchRef = useRef<number | null>(null);

  const startWatching = () => {
    if (!('geolocation' in navigator)) {
      setState('unsupported');
      return;
    }
    setState('idle');
    try {
      if (watchRef.current !== null) {
        navigator.geolocation.clearWatch(watchRef.current);
      }
      watchRef.current = navigator.geolocation.watchPosition(
        (pos) => {
          setState('on');
          if (Date.now() - last.current < 15000) return;
          last.current = Date.now();
          sendDriverPosition(pos);
        },
        () => setState('denied'),
        { enableHighAccuracy: true, maximumAge: 10000 },
      );
    } catch {
      setState('denied');
    }
  };

  useEffect(() => {
    startWatching();
    return () => {
      if (watchRef.current !== null) {
        navigator.geolocation.clearWatch(watchRef.current);
      }
    };
  }, []);

  return (
    <div className={`notice ${state === 'on' ? 'ok' : ''}`}>
      <div className="row between grow">
        <span>
          {state === 'on' && t('sharingOn')}
          {state === 'idle' && t('sharingIdle')}
          {state === 'denied' && t('sharingDenied')}
          {state === 'unsupported' && t('sharingUnsupported')}
        </span>
        {state === 'denied' && (
          <Button variant="secondary" size="sm" onClick={startWatching}>
            {t('tryAgain')}
          </Button>
        )}
      </div>
    </div>
  );
}

export function NavButtonGroup({
  address,
  label,
}: {
  address: string;
  label?: string;
}) {
  const { t } = useTranslation('rideDetail');
  return (
    <div className="nav-buttons-group">
      <span className="nav-group-label muted small bold">
        {label ?? `${t('navigateToPickup')}:`}
      </span>
      <div className="nav-group-buttons">
        <a
          className="btn sm secondary"
          target="_blank"
          rel="noreferrer"
          href={`https://www.google.com/maps/dir/?api=1&destination=${encodeURIComponent(address)}`}
        >
          <Navigation size={14} aria-hidden="true" />
          <span>{t('navigateGoogleMaps')}</span>
        </a>
        <a
          className="btn sm secondary"
          target="_blank"
          rel="noreferrer"
          href={`https://www.waze.com/ul?q=${encodeURIComponent(address)}`}
        >
          <Navigation size={14} aria-hidden="true" />
          <span>{t('navigateWaze')}</span>
        </a>
      </div>
    </div>
  );
}

export function ReasonModal({
  title,
  label,
  confirm,
  initial = '',
  error,
  onClose,
  onSubmit,
}: {
  title: string;
  label: string;
  confirm: string;
  initial?: string;
  error?: string | null;
  onClose: () => void;
  onSubmit: (reason: string) => Promise<boolean>;
}) {
  const [reason, setReason] = useState(initial);
  return (
    <Modal title={title} onClose={onClose}>
      <Field label={label}>
        <textarea
          className="input"
          value={reason}
          onChange={(e) => setReason(e.target.value)}
          rows={3}
        />
      </Field>
      <ErrorBox error={error} />
      <Button
        variant="primary"
        block
        onClick={async () => {
          if (await onSubmit(reason)) onClose();
        }}
      >
        {confirm}
      </Button>
    </Modal>
  );
}
