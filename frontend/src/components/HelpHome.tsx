// The member help home: today's ride when there is one, replies waiting, "Get help with"
// topics (each opens an answer-first sheet), guides, your requests, and the emergency line.
import { useState } from 'react';
import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AlertTriangle, ChevronRight, Phone } from 'lucide-react';
import { Ride } from './RideCard';
import { StatusPill } from './ui';
import { TOPICS, TopicKey } from './HelpTopicSheet';
import { fmtAgo, fmtTime, HELP_STATUS, RIDE_STATUS, RIDE_STATUS_PASSENGER, todayStr, useFormatT } from '../format';
import type { GuideSummary } from '../pages/HelpGuides';
import GuideListRow from './GuideListRow';
import { guideStatus } from '../guideProgress';
import type { TicketSummary } from '../pages/Help';

const SHOW = 3;
const ENDED = ['completed', 'cancelled', 'no_show', 'alt_transport'];

export default function HelpHome({
  rides,
  requests,
  guides,
  onTopic,
}: {
  rides: Ride[];
  requests: TicketSummary[];
  guides: GuideSummary[];
  onTopic: (topic: TopicKey, rideId?: number) => void;
}) {
  const { t } = useTranslation('help');
  const ft = useFormatT();
  const [allGuides, setAllGuides] = useState(false);
  const [allRequests, setAllRequests] = useState(false);

  const today = todayStr();
  const live = rides.filter((r) => !ENDED.includes(r.status));
  const todayRide = live.find((r) => r.status === 'in_progress' || r.pickup_at.startsWith(today));
  const offer = live.some((r) => r.status === 'pending_passenger');
  const topics = TOPICS.filter((x) => (x.key === 'driverOffer' ? offer : true));

  // A reply waiting for the member is shown once, at the top, not again below.
  const waiting = requests.filter((r) => r.status === 'answered');
  const rest = requests
    .filter((r) => r.status !== 'answered')
    .sort((a, b) => Number(a.status === 'closed') - Number(b.status === 'closed'));
  const shownRequests = allRequests ? rest : rest.slice(0, SHOW);

  // Guides already started come first.
  const started = (g: GuideSummary) => Number(guideStatus(g.slug, g.step_count).state === 'started');
  const ordered = [...guides].sort((a, b) => started(b) - started(a));
  const shownGuides = allGuides ? ordered : ordered.slice(0, SHOW);

  const d = todayRide?.driver;
  const approved = todayRide && ['confirmed', 'in_progress'].includes(todayRide.status);

  return (
    <div className="help-home">
      {todayRide && (
        <section className="card help-today" aria-labelledby="help-today-title">
          <div className="row between nowrap">
            <span id="help-today-title" className="muted small">{t('hub.todayRide')}</span>
            <StatusPill map={todayRide.viewer?.is_passenger ? RIDE_STATUS_PASSENGER : RIDE_STATUS} status={todayRide.status} />
          </div>
          <p className="bold help-today-when">
            {t('hub.todayLine', { time: fmtTime(todayRide.pickup_at, ft), dest: todayRide.destination })}
          </p>
          <div className="btn-row">
            {approved && d?.phone && (
              <a className="btn secondary" href={`tel:${d.phone}`}>
                <Phone size={16} aria-hidden="true" />
                <span>{t('hub.callDriver', { name: d.name })}</span>
              </a>
            )}
            <button type="button" className="btn secondary" onClick={() => onTopic('ride', todayRide.id)}>
              {t('hub.helpWithRide')}
            </button>
          </div>
        </section>
      )}

      {waiting.length > 0 && (
        <section className="help-block">
          <h2 className="help-label">{t('hub.repliesWaiting')}</h2>
          <div className="help-rows">
            {waiting.map((tk) => (
              <Link key={tk.id} to={`/help/${tk.id}`} className="help-row">
                <span className="help-dot" aria-hidden="true" />
                <span className="grow min-w-0">
                  <span className="bold block clamp-2">{tk.subject}</span>
                  <span className="muted small block clamp-2">
                    {tk.last_body ? `“${tk.last_body}” · ` : ''}
                    {fmtAgo(tk.last_at || tk.updated_at, ft)}
                  </span>
                </span>
                <ChevronRight size={18} className="muted" aria-hidden="true" />
              </Link>
            ))}
          </div>
        </section>
      )}

      <section className="help-block">
        <h2 className="help-label">{t('hub.getHelpWith')}</h2>
        <div className="help-rows">
          {topics.map((x) => (
            <button key={x.key} type="button" className="help-row" onClick={() => onTopic(x.key)}>
              <span className="help-row-icon" aria-hidden="true">{x.icon}</span>
              <span className="grow">{t(`hub.topic.${x.key}`)}</span>
              <ChevronRight size={18} className="muted" aria-hidden="true" />
            </button>
          ))}
        </div>
      </section>

      {guides.length > 0 && (
        <section className="help-block" id="guides-section">
          <div className="help-label-row">
            <h2 className="help-label">{t('guideTab')}</h2>
            {guides.length > SHOW && (
              <button type="button" className="link-btn small" onClick={() => setAllGuides(!allGuides)}>
                {allGuides ? t('hub.showLess') : t('hub.seeAll', { count: guides.length })}
              </button>
            )}
          </div>
          <div className="help-rows">
            {shownGuides.map((g) => <GuideListRow key={g.id} guide={g} showSummary={false} />)}
          </div>
        </section>
      )}

      {rest.length > 0 && (
        <section className="help-block">
          <div className="help-label-row">
            <h2 className="help-label">{t('hub.yourRequests')}</h2>
            {rest.length > SHOW && (
              <button type="button" className="link-btn small" onClick={() => setAllRequests(!allRequests)}>
                {allRequests ? t('hub.showLess') : t('hub.seeAll', { count: rest.length })}
              </button>
            )}
          </div>
          <div className="help-rows">
            {shownRequests.map((tk) => (
              <Link key={tk.id} to={`/help/${tk.id}`} className="help-row">
                <span className="grow min-w-0">
                  <span className="block clamp-2">{tk.subject}</span>
                  <span className="muted small block">{fmtAgo(tk.updated_at, ft)}</span>
                </span>
                <StatusPill map={HELP_STATUS} status={tk.status} />
              </Link>
            ))}
          </div>
        </section>
      )}

      <p className="help-emergency small">
        <span className="row nowrap gap-xs">
          <AlertTriangle size={16} aria-hidden="true" />
          <span>{t('hub.emergency')}</span>
        </span>
        <a href="tel:+60379573888" className="bold nowrap">{t('hub.callTransport', { phone: t('callTransportShort') })}</a>
      </p>
    </div>
  );
}
