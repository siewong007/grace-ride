// Answer first, then ask: a help topic opens with the quickest fix and the buttons to do
// it, and only then the message form (ride already picked, optional photo).
import { FormEvent, ReactNode, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import {
  BookOpen, CalendarX, Clock, MapPin, MessageCircle, Phone, UserCheck, UserPlus, Users, Wallet,
} from 'lucide-react';
import { post } from '../api';
import { Ride } from './RideCard';
import PhotoAttach from './PhotoAttach';
import { Button, ErrorBox, Field, Modal, useAction, useToast } from './ui';
import { fmtDay, fmtTime, useFormatT } from '../format';

export type TopicKey =
  | 'driverLate' | 'changePickup' | 'cancelRide' | 'driverOffer' | 'visitor' | 'events' | 'payment' | 'other' | 'ride';

/** Topics in the order members meet them. `guide`: the member guide that explains it. */
export const TOPICS: { key: Exclude<TopicKey, 'ride'>; icon: ReactNode; guide?: string; rideTopic?: boolean }[] = [
  { key: 'driverOffer', icon: <UserCheck size={18} />, guide: 'answer-a-driver-offer', rideTopic: true },
  { key: 'driverLate', icon: <Clock size={18} />, guide: 'track-your-ride', rideTopic: true },
  { key: 'changePickup', icon: <MapPin size={18} />, rideTopic: true },
  { key: 'cancelRide', icon: <CalendarX size={18} />, guide: 'cant-make-it', rideTopic: true },
  { key: 'visitor', icon: <UserPlus size={18} />, guide: 'bring-a-visitor' },
  { key: 'events', icon: <Users size={18} />, guide: 'join-as-a-group' },
  { key: 'payment', icon: <Wallet size={18} />, guide: 'pay-your-share' },
  { key: 'other', icon: <MessageCircle size={18} /> },
];

const ENDED = ['completed', 'cancelled', 'no_show', 'alt_transport'];
const APPROVED = ['confirmed', 'in_progress'];

/** The ride a topic is most likely about: the one asked for, today's, the next one. */
function rideFor(topic: TopicKey, rides: Ride[], rideId: number | null): Ride | undefined {
  const live = rides.filter((r) => !ENDED.includes(r.status));
  if (rideId != null) return rides.find((r) => r.id === rideId);
  if (topic === 'driverOffer') return live.find((r) => r.status === 'pending_passenger');
  return live[0];
}

export default function HelpTopicSheet({
  topic,
  rides,
  rideId = null,
  guideSlugs,
  onClose,
  onSent,
}: {
  topic: TopicKey;
  rides: Ride[];
  rideId?: number | null;
  /** Published member guides, so a topic links only to a guide that exists. */
  guideSlugs: Set<string>;
  onClose: () => void;
  onSent?: (ticketId: number) => void;
}) {
  const { t } = useTranslation('help');
  const ft = useFormatT();
  const nav = useNavigate();
  const toast = useToast();
  const { busy, error, run } = useAction();
  const def = TOPICS.find((x) => x.key === topic);
  const ride = rideFor(topic, rides, rideId);
  const upcoming = rides.filter((r) => !ENDED.includes(r.status)).slice(0, 3);
  const choices = ride && !upcoming.some((r) => r.id === ride.id) ? [ride, ...upcoming] : upcoming;
  const [linked, setLinked] = useState<number | null>(def?.rideTopic || topic === 'ride' ? ride?.id ?? null : null);
  const [body, setBody] = useState('');
  const [photo, setPhoto] = useState<number | null>(null);
  const [missing, setMissing] = useState(false);
  // "Something else" and a ride question go straight to the message.
  const answerFirst = topic !== 'other' && topic !== 'ride';
  const [asking, setAsking] = useState(!answerFirst);

  const title = topic === 'ride' && ride
    ? t('hub.rideTitle', { when: `${fmtDay(ride.pickup_at, ft)} · ${fmtTime(ride.pickup_at, ft)}` })
    : t(`hub.topic.${topic}`);
  const rideLink = ride ? `/rides/${ride.id}` : '/rides';
  const driverPhone = ride && APPROVED.includes(ride.status) ? ride.driver?.phone : null;
  const guide = def?.guide && guideSlugs.has(def.guide) ? def.guide : null;

  const actions: { label: string; to?: string; href?: string; icon?: ReactNode; primary?: boolean }[] = [];
  if (topic === 'driverLate' && driverPhone) {
    actions.push({ label: t('hub.callDriver', { name: ride!.driver!.name }), href: `tel:${driverPhone}`, icon: <Phone size={16} />, primary: true });
  }
  if (def?.rideTopic) actions.push({ label: ride ? t('hub.openRide') : t('hub.myRides'), to: rideLink, primary: !actions.length });
  if (topic === 'visitor' || topic === 'events') actions.push({ label: t('hub.openEvents'), to: '/events', primary: true });
  if (topic === 'payment') actions.push({ label: t('hub.openCosts'), to: '/costs', primary: true });

  const send = (e: FormEvent) => {
    e.preventDefault();
    const text = body.trim();
    if (!text) {
      setMissing(true);
      return;
    }
    // A ride question or "something else" is titled from the message itself.
    const subject = answerFirst ? t(`hub.topic.${topic}`) : topic === 'ride' ? title : text.split('\n')[0].slice(0, 80);
    run(async () => {
      const res = await post<{ id: number }>('/help', {
        subject,
        body: text,
        ...(linked != null ? { ride_id: linked } : {}),
        ...(photo != null ? { file_id: photo } : {}),
      });
      toast(t('submitted'));
      onClose();
      if (onSent) onSent(res.id);
      else nav(`/help/${res.id}`);
    });
  };

  return (
    <Modal title={title} onClose={onClose}>
      <div className="stack help-topic">
        {answerFirst && (
          <div className="help-answer stack-sm">
            <span className="bold">{t('hub.tryFirst')}</span>
            <p className="muted">{t(`hub.answer.${topic}`)}</p>
            {(actions.length > 0 || guide) && (
              <div className="btn-row">
                {actions.map((a) =>
                  a.href ? (
                    <a key={a.label} className={`btn ${a.primary ? 'primary' : 'secondary'}`} href={a.href}>
                      {a.icon}
                      <span>{a.label}</span>
                    </a>
                  ) : (
                    <Button key={a.label} variant={a.primary ? 'primary' : 'secondary'} to={a.to} icon={a.icon}>
                      {a.label}
                    </Button>
                  ),
                )}
              </div>
            )}
            {guide && (
              <Link to={`/help/guides/${guide}`} className="small row nowrap gap-xs">
                <BookOpen size={14} aria-hidden="true" />
                <span>{t('hub.readGuide')}</span>
              </Link>
            )}
          </div>
        )}

        {!asking ? (
          <Button variant="ghost" onClick={() => setAsking(true)} icon={<MessageCircle size={16} />}>
            {t('hub.stillNeedHelp')}
          </Button>
        ) : (
          <form className="stack" onSubmit={send}>
            {answerFirst && <span className="bold">{t('hub.messageUs')}</span>}
            {choices.length > 0 && (
              <fieldset className="help-ride-pick stack-xs">
                <legend className="muted small">{t('hub.whichRide')}</legend>
                <div className="chips">
                  {choices.map((r) => (
                    <label key={r.id} className={`chip ${linked === r.id ? 'on' : ''}`}>
                      <input type="radio" name="help-ride" className="sr-only" checked={linked === r.id} onChange={() => setLinked(r.id)} />
                      {fmtDay(r.pickup_at, ft)} · {fmtTime(r.pickup_at, ft)}
                    </label>
                  ))}
                  <label className={`chip ${linked == null ? 'on' : ''}`}>
                    <input type="radio" name="help-ride" className="sr-only" checked={linked == null} onChange={() => setLinked(null)} />
                    {t('hub.notAboutRide')}
                  </label>
                </div>
              </fieldset>
            )}
            <Field label={t('hub.whatHappened')} error={missing ? t('hub.messageRequired') : undefined}>
              <textarea
                className="input"
                rows={4}
                value={body}
                onChange={(e) => {
                  setBody(e.target.value);
                  setMissing(false);
                }}
                placeholder={t('hub.messagePlaceholder')}
              />
            </Field>
            <PhotoAttach fileId={photo} onChange={setPhoto} disabled={busy} />
            <ErrorBox error={error} />
            <Button variant="primary" type="submit" disabled={busy} block>
              {t('hub.send')}
            </Button>
            <p className="muted small center">{t('hub.replyNote')}</p>
          </form>
        )}
      </div>
    </Modal>
  );
}
