import { useEffect, useState } from 'react';
import { CarFront } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { post } from '../../api';
import { useAuth } from '../../auth';
import type { EventItem } from '../../pages/Events';
import { Button, ErrorBox, Modal, Spinner, useToast } from '../ui';

export function QuickRsvpModal({
  event,
  initialStep,
  joinChoices,
  loadingChoices = false,
  onCompleteSignup,
  onClose,
}: {
  event: EventItem;
  initialStep: 'group' | 'transport';
  joinChoices?: { id: number; name: string }[];
  loadingChoices?: boolean;
  onCompleteSignup: () => void;
  onClose: () => void;
}) {
  const { t } = useTranslation('events');
  const { can } = useAuth();
  const toast = useToast();
  const [step, setStep] = useState<'group' | 'transport'>(initialStep);
  const [selectedGroup, setSelectedGroup] = useState<number | null>(() =>
    joinChoices && joinChoices.length > 0 ? joinChoices[0].id : null,
  );
  const [groupMissing, setGroupMissing] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (selectedGroup == null && joinChoices && joinChoices.length > 0) {
      setSelectedGroup(joinChoices[0].id);
      setGroupMissing(false);
    }
  }, [joinChoices, selectedGroup]);

  const isFull = event.capacity > 0 && event.going >= event.capacity;
  const canRide = can('rides', 'create');
  const canDrive = can('drive', 'create');

  const handleConfirmGroup = async () => {
    if (selectedGroup == null) {
      setGroupMissing(true);
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const res = await post<{ ok?: boolean; status?: string }>(`/events/${event.id}/signup`, {
        as_group_id: selectedGroup,
      });
      onCompleteSignup();
      if (res?.status === 'waitlist') {
        toast.success(t('joinedWaitlistSuccess'));
        onClose();
      } else if (canRide || canDrive) {
        toast.success(t('signedUpSuccess'));
        setStep('transport');
      } else {
        toast.success(t('signedUpSuccess'));
        onClose();
      }
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setBusy(false);
    }
  };

  const handleGoingOwnWay = async () => {
    setBusy(true);
    setError(null);
    try {
      await post(`/events/${event.id}/transport`, { self_transport: true });
      onCompleteSignup();
      onClose();
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setBusy(false);
    }
  };

  const title =
    step === 'group'
      ? isFull
        ? t('joinWaitlist', { ns: 'eventDetail' })
        : t('signUp', { ns: 'eventDetail' })
      : t('transportPromptTitle', { ns: 'eventDetail' });

  return (
    <Modal title={title} onClose={onClose} size="sm">
      {step === 'group' ? (
        <div className="stack">
          <div className="bold">{event.title}</div>
          {loadingChoices ? (
            <div className="row center gap-sm">
              <Spinner />
              <span className="muted small">{t('rsvpLoading')}</span>
            </div>
          ) : (
            <>
              {joinChoices && joinChoices.length > 0 && (
                <fieldset className="join-as stack-sm">
                  <legend className="bold small">{t('joinAsLegend', { ns: 'eventDetail' })}</legend>
                  <div className="chips">
                    {joinChoices.map((g) => (
                      <label key={g.id} className={`chip ${selectedGroup === g.id ? 'on' : ''}`}>
                        <input
                          type="radio"
                          name="quick-rsvp-group"
                          className="sr-only"
                          checked={selectedGroup === g.id}
                          onChange={() => {
                            setSelectedGroup(g.id);
                            setGroupMissing(false);
                          }}
                        />
                        {g.name}
                      </label>
                    ))}
                  </div>
                  {groupMissing && (
                    <span className="field-error" role="alert">
                      {t('joinAsRequired', { ns: 'eventDetail' })}
                    </span>
                  )}
                </fieldset>
              )}
              <Button
                variant="primary"
                block
                size="lg"
                loading={busy}
                disabled={busy || (joinChoices !== undefined && joinChoices.length === 0)}
                onClick={handleConfirmGroup}
              >
                {isFull ? t('joinWaitlist', { ns: 'eventDetail' }) : t('signUp', { ns: 'eventDetail' })}
              </Button>
            </>
          )}
          <ErrorBox error={error} />
        </div>
      ) : (
        <div className="stack">
          <p className="muted small">{t('transportPromptBody', { ns: 'eventDetail' })}</p>
          <div className="stack-sm">
            {canRide && (
              <Button
                variant="primary"
                block
                icon={<CarFront size={18} />}
                to={`/rides/new?event=${event.id}`}
                onClick={onClose}
              >
                {t('needRideBtn')}
              </Button>
            )}
            {canDrive && (
              <Button variant="secondary" block to={`/events/${event.id}?drive=1`} onClick={onClose}>
                {t('offerDriveBtn')}
              </Button>
            )}
            <Button variant="ghost" block loading={busy} disabled={busy} onClick={handleGoingOwnWay}>
              {t('goingOwnWay', { ns: 'eventDetail' })}
            </Button>
          </div>
          <ErrorBox error={error} />
        </div>
      )}
    </Modal>
  );
}
