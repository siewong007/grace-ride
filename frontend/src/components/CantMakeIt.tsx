import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { post } from '../api';
import { Button, ErrorBox, Field, Modal, useAction } from './ui';

/** Longest reason the server keeps (backend `UNABLE_REASON_MAX`). */
export const CANT_MAKE_IT_MAX = 300;

/**
 * "Can't make it today": confirms the passenger is skipping this pickup, with an optional
 * reason that the driver and transport team see on the ride. Shared by Home and the ride page.
 */
export default function CantMakeItModal({
  rideId,
  riderName,
  onClose,
  onDone,
}: {
  rideId: number;
  /** Set when the ride was booked for someone else, so the wording names them. */
  riderName?: string;
  onClose: () => void;
  onDone: () => void;
}) {
  const { t } = useTranslation('rideDetail');
  const { busy, error, run } = useAction();
  const [reason, setReason] = useState('');

  const submit = async () => {
    const notes = reason.trim();
    const ok = await run(
      () => post(`/rides/${rideId}/readiness`, { status: 'unable_to_join', ...(notes ? { notes } : {}) }),
      t('unableDone'),
    );
    if (ok) {
      onClose();
      onDone();
    }
  };

  return (
    <Modal title={riderName ? t('unableTitleProxy', { name: riderName }) : t('confirmUnableTitle')} description={t('confirmUnableBody')} onClose={onClose}>
      <div className="stack">
        <Field label={t('unableReasonLabel')} hint={t('unableReasonHint')}>
          <textarea
            className="input"
            value={reason}
            maxLength={CANT_MAKE_IT_MAX}
            onChange={(e) => setReason(e.target.value)}
            placeholder={t('unableReasonPlaceholder')}
            rows={3}
          />
        </Field>
        <ErrorBox error={error} />
        <div className="btn-row">
          <Button variant="danger" disabled={busy} onClick={submit}>
            {t('confirmUnableBtn')}
          </Button>
          <Button variant="ghost" disabled={busy} onClick={onClose}>
            {t('unableKeep')}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
