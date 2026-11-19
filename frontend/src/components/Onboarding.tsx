// First steps for a new member: the waiting room while an admin reviews a new profile,
// and the "Getting started" checklist on Home once they are approved.
import { ReactNode } from 'react';
import { Link } from 'react-router-dom';
import { CheckCircle2, ChevronRight, Circle, Clock } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { put, useApi } from '../api';
import { useAuth } from '../auth';
import { Button, useAction } from './ui';

export interface Step {
  key: string;
  title: string;
  hint?: string;
  /** Where the member does this step. Omitted for a step that is not theirs to do. */
  to?: string;
  state: 'done' | 'todo' | 'waiting';
}

function StepIcon({ state }: { state: Step['state'] }) {
  if (state === 'done') return <CheckCircle2 size={22} className="onboard-icon done" aria-hidden="true" />;
  if (state === 'waiting') return <Clock size={22} className="onboard-icon waiting" aria-hidden="true" />;
  return <Circle size={22} className="onboard-icon" aria-hidden="true" />;
}

/** A numbered list of steps; a step still to do is a link to the place that does it. */
export function StepList({ steps }: { steps: Step[] }) {
  const { t } = useTranslation('onboarding');
  return (
    <ol className="list onboard-steps">
      {steps.map((s) => {
        const body: ReactNode = (
          <>
            <StepIcon state={s.state} />
            <span className="grow onboard-text">
              <span className={s.state === 'done' ? 'onboard-done-title' : 'bold'}>{s.title}</span>
              {s.hint && s.state !== 'done' && <span className="small muted">{s.hint}</span>}
              <span className="sr-only">
                {s.state === 'done' ? t('stateDone') : s.state === 'waiting' ? t('stateWaiting') : t('stateTodo')}
              </span>
            </span>
            {s.to && s.state === 'todo' && <ChevronRight size={18} className="muted" aria-hidden="true" />}
          </>
        );
        return (
          <li key={s.key} className="item onboard-step">
            {s.to && s.state === 'todo' ? (
              <Link to={s.to} className="onboard-link">
                {body}
              </Link>
            ) : (
              body
            )}
          </li>
        );
      })}
    </ol>
  );
}

/** Shown on the profile page while a new member waits for an admin to approve them. */
export function WaitingRoom() {
  const { t } = useTranslation('onboarding');
  const { me } = useAuth();
  if (!me) return null;
  const steps: Step[] = [
    { key: 'joined', title: t('joined'), state: 'done' },
    {
      key: 'passkey',
      title: t('passkey'),
      hint: t('passkeyHint'),
      to: '/me?section=security',
      state: me.passkey_count > 0 ? 'done' : 'todo',
    },
    {
      key: 'photo',
      title: t('photo'),
      hint: t('photoHint'),
      to: '/me?section=personal',
      state: me.photo_file_id ? 'done' : 'todo',
    },
    { key: 'review', title: t('review'), hint: t('reviewHint'), state: 'waiting' },
  ];
  return (
    <section className="card stack-sm onboard-card" aria-labelledby="waiting-title">
      <div>
        <h2 id="waiting-title">{t('waitingTitle')}</h2>
        <p className="muted small">{t('waitingBody')}</p>
      </div>
      <StepList steps={steps} />
    </section>
  );
}

interface HomeData {
  /** Whether the member has booked a ride (passenger rides Home already loads). */
  hasRide: boolean;
  /** Whether the member is signed up for an event (events Home already loads). */
  hasEvent: boolean;
}

/**
 * The checklist on Home for a newly approved member. It shows only the steps their
 * role can take, and goes away once every step is done or they hide it.
 */
export function GettingStarted({ hasRide, hasEvent }: HomeData) {
  const { t } = useTranslation('onboarding');
  const { me, can, refresh } = useAuth();
  const show = !!me && me.status === 'active' && !me.onboarding_dismissed;
  const places = useApi<unknown[]>(show && can('rides') ? '/saved-places' : null);
  const driver = useApi<{ vehicle?: { plate?: string } | null }>(show && can('drive') ? '/driver/profile' : null);
  const { busy, run } = useAction();
  if (!show || !me) return null;

  const steps: Step[] = [
    {
      key: 'passkey',
      title: t('passkey'),
      hint: t('passkeyHint'),
      to: '/me?section=security',
      state: me.passkey_count > 0 ? 'done' : 'todo',
    },
  ];
  if (can('rides')) {
    steps.push({
      key: 'place',
      title: t('place'),
      hint: t('placeHint'),
      to: '/me?section=places',
      state: Array.isArray(places.data) && places.data.length > 0 ? 'done' : 'todo',
    });
  }
  if (can('rides', 'create')) {
    steps.push({ key: 'ride', title: t('ride'), hint: t('rideHint'), to: '/rides/new', state: hasRide ? 'done' : 'todo' });
  }
  if (can('events')) {
    steps.push({ key: 'event', title: t('event'), hint: t('eventHint'), to: '/events', state: hasEvent ? 'done' : 'todo' });
  }
  if (can('drive')) {
    steps.push({
      key: 'car',
      title: t('car'),
      hint: t('carHint'),
      to: '/drive',
      state: driver.data?.vehicle?.plate ? 'done' : 'todo',
    });
  }
  // Wait for the checks that load, so the card does not flash steps as not done.
  if (places.loading || driver.loading) return null;
  const done = steps.filter((s) => s.state === 'done').length;
  if (done === steps.length) return null;

  const hide = () => run(() => put('/me', { onboarding_dismissed: true }).then(refresh));

  return (
    <section className="card stack-sm onboard-card" aria-labelledby="getting-started-title">
      <div className="row between align-center">
        <h2 id="getting-started-title">{t('startTitle')}</h2>
        <span className="small muted nowrap">{t('progress', { done, total: steps.length })}</span>
      </div>
      <p className="muted small">{t('startBody')}</p>
      <StepList steps={steps} />
      <div className="row between align-center">
        {can('help') ? <Link to="/help" className="small">{t('howItWorks')}</Link> : <span />}
        <Button variant="ghost" size="sm" onClick={hide} loading={busy}>
          {t('hide')}
        </Button>
      </div>
    </section>
  );
}
