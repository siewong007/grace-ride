import { useEffect, useMemo, useRef, useState } from 'react';
import { Link, useNavigate, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import {
  BookOpen,
  ChevronDown,
  ChevronRight,
  Phone,
  Plus,
  Search,
  X,
} from 'lucide-react';
import { useApi } from '../api';
import { useAuth } from '../auth';
import { Ride } from '../components/RideCard';
import HelpComposer from '../components/HelpComposer';
import HelpDetail from './HelpDetail';
import { GuideStep, GuideSummary, usePick } from './HelpGuides';
import {
  Button,
  Card,
  CardSkeleton,
  EmptyState,
  ListSkeleton,
  Page,
  Pill,
  StatusPill,
} from '../components/ui';
import { fmtAgo, fmtTime, HELP_STATUS, RIDE_STATUS, todayStr, useFormatT } from '../format';

export { HelpDetail };

export interface TicketSummary {
  id: number;
  subject: string;
  user_id: number;
  user_name: string;
  status: string;
  ride_id: number | null;
  created_at: string;
  updated_at: string;
  last_body?: string | null;
  last_staff?: boolean | null;
  last_at?: string | null;
}

interface TicketCounts {
  open: number;
  answered: number;
  closed: number;
}

function getGuideProgress(slug: string): number {
  try {
    const raw = localStorage.getItem(`graceride_guide_${slug}_done`);
    if (!raw) return 0;
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.length : 0;
  } catch {
    return 0;
  }
}

function HighlightMatch({ text, query }: { text: string; query: string }) {
  if (!query.trim() || !text) return <>{text}</>;
  const escaped = query.trim().replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const parts = text.split(new RegExp(`(${escaped})`, 'gi'));
  return (
    <>
      {parts.map((part, i) =>
        part.toLowerCase() === query.trim().toLowerCase() ? (
          <mark key={i} className="search-highlight">
            {part}
          </mark>
        ) : (
          part
        ),
      )}
    </>
  );
}

export default function Help() {
  const { t } = useTranslation('help');
  const { t: tc } = useTranslation();
  const ft = useFormatT();
  const pick = usePick();
  const nav = useNavigate();
  const { can } = useAuth();
  const [params] = useSearchParams();

  const isStaff = can('admin_help', 'view');
  const isEditor = can('admin_help', 'update');

  // Search input & ref
  const searchInputRef = useRef<HTMLInputElement>(null);
  const [searchQuery, setSearchQuery] = useState('');

  // Composer modal state
  const [composerOpen, setComposerOpen] = useState(false);
  const [composerSubject, setComposerSubject] = useState('');
  const [composerRideId, setComposerRideId] = useState<number | null>(null);

  // Staff inbox filters
  const [viewAsMember, setViewAsMember] = useState(false);
  const [staffFilter, setStaffFilter] = useState<'open' | 'answered' | 'closed' | 'all'>('open');
  const [allStaffScope, setAllStaffScope] = useState(true);
  const [inboxQuery, setInboxQuery] = useState('');
  const [selectedTicketId, setSelectedTicketId] = useState<number | null>(null);
  const [myOwnOpen, setMyOwnOpen] = useState(false);

  // Audience filter for guides
  const [guideAudienceFilter, setGuideAudienceFilter] = useState<'all' | 'member' | 'staff'>('all');

  // Responsive desktop detection for 2-pane inbox
  const [isDesktop, setIsDesktop] = useState(
    typeof window !== 'undefined' ? window.innerWidth >= 1200 : false,
  );
  useEffect(() => {
    const onResize = () => setIsDesktop(window.innerWidth >= 1200);
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, []);

  // Shortcut "/" focuses search box
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (
        e.key === '/' &&
        document.activeElement?.tagName !== 'INPUT' &&
        document.activeElement?.tagName !== 'TEXTAREA'
      ) {
        e.preventDefault();
        searchInputRef.current?.focus();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, []);

  // Deep-linking: ?tab=guides scrolls to guides, ?ride=N opens composer
  useEffect(() => {
    const tab = params.get('tab');
    if (tab === 'guides') {
      setTimeout(() => {
        document.getElementById('guides-section')?.scrollIntoView({ behavior: 'smooth' });
      }, 100);
    }
    const r = params.get('ride');
    if (r) {
      setComposerRideId(Number(r));
      setComposerSubject(t('relatedRide', { rideId: r }));
      setComposerOpen(true);
    }
  }, [params, t]);

  // Data fetching:
  // 1. Guides with steps for instant search
  const guidesApi = useApi<(GuideSummary & { steps?: GuideStep[] })[]>(
    `/help/guides?full=true${isStaff ? '&include_drafts=true' : ''}`,
  );
  const guides = guidesApi.data ?? [];

  // 2. Member's own requests
  const ownRequestsApi = useApi<TicketSummary[]>('/help', 10000);
  const ownRequests = ownRequestsApi.data ?? [];

  // 3. Member's upcoming rides
  const ridesApi = useApi<Ride[]>(can('rides') ? '/rides' : null, 30000);
  const memberRides = ridesApi.data ?? [];

  // Today or in-progress ride
  const today = todayStr();
  const todayRide = memberRides.find(
    (r) => r.status === 'in_progress' || (r.pickup_at && r.pickup_at.startsWith(today)),
  );

  // 4. Staff inbox requests
  const staffStatusParam = staffFilter === 'all' ? '' : `&status=${staffFilter}`;
  const staffSearchParam = inboxQuery.trim() ? `&q=${encodeURIComponent(inboxQuery.trim())}` : '';
  const staffRequestsApi = useApi<TicketSummary[]>(
    isStaff && !viewAsMember
      ? `/help?all=${allStaffScope}${staffStatusParam}${staffSearchParam}`
      : null,
    10000,
  );
  const staffRequests = staffRequestsApi.data ?? [];

  // 5. Staff counts
  const countsApi = useApi<TicketCounts>(
    isStaff && !viewAsMember ? `/help/counts?all=${allStaffScope}` : null,
    10000,
  );
  const counts = countsApi.data ?? { open: 0, answered: 0, closed: 0 };
  const totalCounts = counts.open + counts.answered + counts.closed;

  // Auto-select first ticket in 2-pane mode on desktop
  useEffect(() => {
    if (isDesktop && isStaff && !viewAsMember && staffRequests.length > 0 && selectedTicketId === null) {
      setSelectedTicketId(staffRequests[0].id);
    }
  }, [isDesktop, isStaff, viewAsMember, staffRequests, selectedTicketId]);

  // Answered tickets waiting for member
  const waitingForYou = ownRequests.filter((r) => r.status === 'answered');

  // Instant browser search matching
  const searchResults = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return null;

    const matchedGuides: { guide: GuideSummary; snippet: string }[] = [];
    const matchedSteps: { guide: GuideSummary; stepIndex: number; title: string; body: string }[] = [];
    const matchedRequests: { ticket: TicketSummary; snippet: string }[] = [];

    // Search guides & steps
    for (const g of guides) {
      const ten = (g.title_en || '').toLowerCase();
      const tzh = (g.title_zh || '').toLowerCase();
      const sen = (g.summary_en || '').toLowerCase();
      const szh = (g.summary_zh || '').toLowerCase();

      if (ten.includes(q) || tzh.includes(q) || sen.includes(q) || szh.includes(q)) {
        matchedGuides.push({
          guide: g,
          snippet: pick(g.summary_zh, g.summary_en) || pick(g.title_zh, g.title_en),
        });
      }

      if (g.steps && Array.isArray(g.steps)) {
        g.steps.forEach((s, idx) => {
          const sten = (s.title_en || '').toLowerCase();
          const stzh = (s.title_zh || '').toLowerCase();
          const sben = (s.body_en || '').toLowerCase();
          const sbzh = (s.body_zh || '').toLowerCase();

          if (sten.includes(q) || stzh.includes(q) || sben.includes(q) || sbzh.includes(q)) {
            matchedSteps.push({
              guide: g,
              stepIndex: idx,
              title: pick(s.title_zh, s.title_en),
              body: pick(s.body_zh, s.body_en),
            });
          }
        });
      }
    }

    // Search member's requests
    for (const r of ownRequests) {
      const subj = (r.subject || '').toLowerCase();
      const last = (r.last_body || '').toLowerCase();
      if (subj.includes(q) || last.includes(q)) {
        matchedRequests.push({
          ticket: r,
          snippet: r.last_body || r.subject,
        });
      }
    }

    const total = matchedGuides.length + matchedSteps.length + matchedRequests.length;
    return {
      guides: matchedGuides,
      steps: matchedSteps,
      requests: matchedRequests,
      total,
    };
  }, [searchQuery, guides, ownRequests, pick]);

  const openComposer = (subj = '', rId: number | null = null) => {
    setComposerSubject(subj);
    setComposerRideId(rId);
    setComposerOpen(true);
  };

  const commonQuestions = [
    { key: 'driverLate', text: t('commonDriverLate') },
    { key: 'changePickup', text: t('commonChangePickup') },
    { key: 'cancelRide', text: t('commonCancelRide') },
    { key: 'payment', text: t('commonPayment') },
    { key: 'somethingElse', text: t('commonSomethingElse') },
  ];

  // Filtered guides by audience
  const displayedGuides = guides.filter((g) => {
    if (!isStaff) return g.audience === 'member' && g.published;
    if (guideAudienceFilter === 'all') return true;
    return g.audience === guideAudienceFilter;
  });

  return (
    <Page width="wide">
      <div className="stack">
        {/* Header with title and staff view toggle */}
        <div className="row between nowrap page-head-title">
          <div>
            <h1>{isStaff && !viewAsMember ? t('title') : t('howCanWeHelp')}</h1>
            <p className="muted">{t('desc')}</p>
          </div>
          {isStaff && (
            <div className="btn-row nowrap">
              <Button
                variant="secondary"
                size="sm"
                onClick={() => setViewAsMember(!viewAsMember)}
              >
                {viewAsMember ? t('exitViewAsMember') : t('viewAsMember')}
              </Button>
            </div>
          )}
        </div>

        {/* Global Search Bar (press / to focus) */}
        <div className="help-search-box">
          <Search size={18} className="help-search-icon" aria-hidden="true" />
          <input
            ref={searchInputRef}
            type="search"
            className="input help-search-input"
            placeholder={t('searchPlaceholder')}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            aria-label={t('searchPlaceholder')}
          />
          {searchQuery && (
            <button
              type="button"
              className="icon-btn help-search-clear"
              onClick={() => setSearchQuery('')}
              aria-label={tc('clear')}
            >
              <X size={16} />
            </button>
          )}
        </div>
        <div className="sr-only" aria-live="polite">
          {searchQuery.trim() && searchResults
            ? t('searchResultsCount', { count: searchResults.total })
            : ''}
        </div>

        {/* Search Results Mode */}
        {searchQuery.trim() && searchResults ? (
          <div className="stack">
            <h2>{t('searchResults')}</h2>
            {searchResults.total === 0 ? (
              <EmptyState
                title={t('searchNoResults', { q: searchQuery })}
                description={t('searchNoResultsDesc')}
                action={
                  <Button
                    variant="primary"
                    onClick={() => openComposer(searchQuery)}
                  >
                    {t('contactUs')}
                  </Button>
                }
              />
            ) : (
              <div className="stack">
                {searchResults.guides.length > 0 && (
                  <section className="stack-xs">
                    <h3>{t('searchGroupGuides')} ({searchResults.guides.length})</h3>
                    <Card className="stack">
                      <div className="list">
                        {searchResults.guides.map(({ guide: g, snippet }) => (
                          <Link
                            key={g.id}
                            to={`/help/guides/${g.slug}`}
                            className="item"
                          >
                            <div className="grow min-w-0">
                              <div className="bold ellipsis">
                                <HighlightMatch
                                  text={pick(g.title_zh, g.title_en)}
                                  query={searchQuery}
                                />
                              </div>
                              <div className="muted tiny ellipsis">
                                <HighlightMatch text={snippet} query={searchQuery} />
                              </div>
                            </div>
                            <span className="muted tiny">
                              {t('guideSteps', { count: g.step_count })}
                            </span>
                          </Link>
                        ))}
                      </div>
                    </Card>
                  </section>
                )}

                {searchResults.steps.length > 0 && (
                  <section className="stack-xs">
                    <h3>{t('searchGroupSteps')} ({searchResults.steps.length})</h3>
                    <Card className="stack">
                      <div className="list">
                        {searchResults.steps.map(({ guide: g, stepIndex, title, body }) => (
                          <Link
                            key={`${g.id}-${stepIndex}`}
                            to={`/help/guides/${g.slug}#step-${stepIndex}`}
                            className="item"
                          >
                            <div className="grow min-w-0">
                              <div className="bold ellipsis">
                                <span className="muted">{pick(g.title_zh, g.title_en)} · </span>
                                <span>{t('stepNumber', { n: stepIndex + 1 })}: </span>
                                <HighlightMatch text={title} query={searchQuery} />
                              </div>
                              {body && (
                                <div className="muted tiny ellipsis">
                                  <HighlightMatch text={body} query={searchQuery} />
                                </div>
                              )}
                            </div>
                            <ChevronRight size={16} className="muted" />
                          </Link>
                        ))}
                      </div>
                    </Card>
                  </section>
                )}

                {searchResults.requests.length > 0 && (
                  <section className="stack-xs">
                    <h3>{t('searchGroupRequests')} ({searchResults.requests.length})</h3>
                    <Card className="stack">
                      <div className="list">
                        {searchResults.requests.map(({ ticket: tk, snippet }) => (
                          <Link key={tk.id} to={`/help/${tk.id}`} className="item">
                            <div className="grow min-w-0">
                              <div className="bold ellipsis">
                                <HighlightMatch text={tk.subject} query={searchQuery} />
                              </div>
                              <div className="muted tiny ellipsis">
                                <HighlightMatch text={snippet} query={searchQuery} />
                              </div>
                            </div>
                            <StatusPill map={HELP_STATUS} status={tk.status} />
                          </Link>
                        ))}
                      </div>
                    </Card>
                  </section>
                )}
              </div>
            )}
          </div>
        ) : (
          /* Normal Content Mode */
          <div className="stack">
            {/* STAFF INBOX (Section 3) */}
            {isStaff && !viewAsMember && (
              <section className="stack">
                <div className="row between nowrap">
                  <h2>{t('inboxTitle')}</h2>
                  <div className="row nowrap gap-sm">
                    <div className="segmented-control">
                      <button
                        type="button"
                        className={`segmented-item ${allStaffScope ? 'active' : ''}`}
                        onClick={() => setAllStaffScope(true)}
                      >
                        {t('allHelp')}
                      </button>
                      <button
                        type="button"
                        className={`segmented-item ${!allStaffScope ? 'active' : ''}`}
                        onClick={() => setAllStaffScope(false)}
                      >
                        {t('mine')}
                      </button>
                    </div>
                    {isEditor && (
                      <Button
                        variant="primary"
                        size="sm"
                        icon={<Plus size={16} />}
                        onClick={() => openComposer()}
                      >
                        {t('ask')}
                      </Button>
                    )}
                  </div>
                </div>

                {/* Filter chips with counts */}
                <div className="chips">
                  <button
                    type="button"
                    className={`chip ${staffFilter === 'open' ? 'on' : ''}`}
                    onClick={() => setStaffFilter('open')}
                  >
                    {t('filterNeedsReply', { count: counts.open })}
                  </button>
                  <button
                    type="button"
                    className={`chip ${staffFilter === 'answered' ? 'on' : ''}`}
                    onClick={() => setStaffFilter('answered')}
                  >
                    {t('filterAnswered', { count: counts.answered })}
                  </button>
                  <button
                    type="button"
                    className={`chip ${staffFilter === 'closed' ? 'on' : ''}`}
                    onClick={() => setStaffFilter('closed')}
                  >
                    {t('filterResolved', { count: counts.closed })}
                  </button>
                  <button
                    type="button"
                    className={`chip ${staffFilter === 'all' ? 'on' : ''}`}
                    onClick={() => setStaffFilter('all')}
                  >
                    {t('filterAll', { count: totalCounts })}
                  </button>
                </div>

                {/* Search input for inbox */}
                <input
                  className="input"
                  placeholder={t('searchInboxPlaceholder')}
                  value={inboxQuery}
                  onChange={(e) => setInboxQuery(e.target.value)}
                />

                {/* Staff Layout: 2-pane on desktop, list on phone/tablet */}
                {isDesktop ? (
                  <div className="help-two-pane">
                    {/* Left Pane: Tickets List */}
                    <Card className="stack">
                      {staffRequestsApi.loading ? (
                        <ListSkeleton count={4} />
                      ) : !staffRequests.length ? (
                        <EmptyState title={t('none')} />
                      ) : (
                        <div className="list">
                          {staffRequests.map((tk) => (
                            <button
                              key={tk.id}
                              type="button"
                              className={`item ${selectedTicketId === tk.id ? 'active' : ''}`}
                              onClick={() => setSelectedTicketId(tk.id)}
                            >
                              <div className="grow min-w-0">
                                <div className="row between nowrap">
                                  <span className="bold ellipsis">{tk.user_name}</span>
                                  <span className="muted tiny">
                                    {fmtAgo(tk.last_at || tk.updated_at, ft)}
                                  </span>
                                </div>
                                <div className="small ellipsis">{tk.subject}</div>
                                {tk.last_body && (
                                  <div className="muted tiny ellipsis">
                                    {tk.last_staff ? '↳ Staff: ' : ''}
                                    {tk.last_body}
                                  </div>
                                )}
                              </div>
                              <StatusPill map={HELP_STATUS} status={tk.status} />
                            </button>
                          ))}
                        </div>
                      )}
                    </Card>

                    {/* Right Pane: Selected Thread */}
                    <div className="help-two-pane-right">
                      {selectedTicketId ? (
                        <HelpDetail
                          ticketId={selectedTicketId}
                          isTwoPane
                          onStatusChange={() => {
                            staffRequestsApi.reload();
                            countsApi.reload();
                          }}
                        />
                      ) : (
                        <Card className="stack center pad-lg">
                          <p className="muted">{t('selectTicket', 'Select a request to view')}</p>
                        </Card>
                      )}
                    </div>
                  </div>
                ) : (
                  /* Mobile/Tablet: Flat List linking to /help/:id */
                  <Card className="stack">
                    {staffRequestsApi.loading ? (
                      <ListSkeleton count={4} />
                    ) : !staffRequests.length ? (
                      <EmptyState title={t('none')} />
                    ) : (
                      <div className="list">
                        {staffRequests.map((tk) => (
                          <Link key={tk.id} to={`/help/${tk.id}`} className="item">
                            <div className="grow min-w-0">
                              <div className="row between nowrap">
                                <span className="bold ellipsis">{tk.user_name}</span>
                                <span className="muted tiny">
                                  {fmtAgo(tk.last_at || tk.updated_at, ft)}
                                </span>
                              </div>
                              <div className="small ellipsis">{tk.subject}</div>
                              {tk.last_body && (
                                <div className="muted tiny ellipsis">{tk.last_body}</div>
                              )}
                            </div>
                            <StatusPill map={HELP_STATUS} status={tk.status} />
                          </Link>
                        ))}
                      </div>
                    )}
                  </Card>
                )}

                {/* Collapsible: Staff's own requests */}
                <div className="card stack-xs" style={{ marginTop: 'var(--space-2)' }}>
                  <button
                    type="button"
                    className="row between nowrap"
                    style={{ background: 'none', border: 'none', cursor: 'pointer', width: '100%', padding: 'var(--space-2) 0' }}
                    onClick={() => setMyOwnOpen(!myOwnOpen)}
                  >
                    <span className="bold small">{t('myOwnRequests')}</span>
                    <ChevronDown
                      size={18}
                      style={{
                        transform: myOwnOpen ? 'rotate(180deg)' : 'none',
                        transition: 'transform var(--dur-fast)',
                      }}
                    />
                  </button>
                  {myOwnOpen && (
                    <div className="list" style={{ marginTop: 'var(--space-2)' }}>
                      {!ownRequests.length ? (
                        <p className="muted small">{t('none')}</p>
                      ) : (
                        ownRequests.map((tk) => (
                          <Link key={tk.id} to={`/help/${tk.id}`} className="item">
                            <div className="grow min-w-0">
                              <div className="bold ellipsis">{tk.subject}</div>
                              <div className="muted tiny">{fmtAgo(tk.updated_at, ft)}</div>
                            </div>
                            <StatusPill map={HELP_STATUS} status={tk.status} />
                          </Link>
                        ))
                      )}
                    </div>
                  )}
                </div>
              </section>
            )}

            {/* MEMBER HOME EXPERIENCE */}
            {(!isStaff || viewAsMember) && (
              <div className="stack">
                {/* Active Ride Today Banner */}
                {todayRide && (
                  <div className="card notice info stack-xs">
                    <div className="row between nowrap">
                      <span className="bold small">
                        {t('rideTodayBanner', {
                          time: fmtTime(todayRide.pickup_at, ft),
                          dest: todayRide.destination,
                        })}
                      </span>
                      <StatusPill map={RIDE_STATUS} status={todayRide.status} />
                    </div>
                    <div className="btn-row" style={{ marginTop: 'var(--space-2)' }}>
                      <Button
                        variant="primary"
                        size="sm"
                        onClick={() =>
                          openComposer(t('relatedRide', { rideId: todayRide.id }), todayRide.id)
                        }
                      >
                        {t('getHelpWithRide')}
                      </Button>
                      {todayRide.driver?.phone && (
                        <a
                          className="btn sm secondary row nowrap gap-xs"
                          href={`tel:${todayRide.driver.phone}`}
                        >
                          <Phone size={14} />
                          <span>{t('callDriver')}</span>
                        </a>
                      )}
                    </div>
                  </div>
                )}

                {/* Urgent Strip */}
                <div className="help-urgent-strip">
                  <div className="row nowrap gap-xs align-center">
                    <span className="bold small">{t('urgentPrefix')}</span>
                    <span className="muted small">{t('urgent')}</span>
                  </div>
                  <a className="btn sm secondary row nowrap gap-xs" href="tel:+60379573888">
                    <Phone size={14} aria-hidden="true" />
                    <span>{t('callTransportShort')}</span>
                  </a>
                </div>

                {/* Waiting For You (answered requests lifted to top) */}
                {waitingForYou.length > 0 && (
                  <section className="stack-xs">
                    <h3 className="row nowrap gap-xs tone-blue">
                      <span>{t('waitingForYouCount', { count: waitingForYou.length })}</span>
                    </h3>
                    <Card className="stack">
                      <div className="list">
                        {waitingForYou.map((tk) => (
                          <Link key={tk.id} to={`/help/${tk.id}`} className="item">
                            <div className="grow min-w-0">
                              <div className="bold ellipsis">{tk.subject}</div>
                              <div className="muted tiny">
                                {tk.last_body ? `"${tk.last_body}" · ` : ''}
                                {fmtAgo(tk.last_at || tk.updated_at, ft)}
                              </div>
                            </div>
                            <ChevronRight size={18} className="muted" />
                          </Link>
                        ))}
                      </div>
                    </Card>
                  </section>
                )}

                {/* Common questions chips */}
                <section className="stack-xs">
                  <span className="muted bold tiny">{t('commonQuestions')}</span>
                  <div className="chips">
                    {commonQuestions.map((q) => (
                      <button
                        key={q.key}
                        type="button"
                        className="chip"
                        onClick={() => openComposer(q.text)}
                      >
                        {q.text}
                      </button>
                    ))}
                  </div>
                </section>
              </div>
            )}

            {/* GUIDES SECTION */}
            <section id="guides-section" className="stack">
              <div className="row between nowrap">
                <h2>{t('guideTab')}</h2>
                <div className="row nowrap gap-sm">
                  {isStaff && (
                    <div className="segmented-control">
                      <button
                        type="button"
                        className={`segmented-item ${guideAudienceFilter === 'all' ? 'active' : ''}`}
                        onClick={() => setGuideAudienceFilter('all')}
                      >
                        {t('audienceAll')}
                      </button>
                      <button
                        type="button"
                        className={`segmented-item ${guideAudienceFilter === 'member' ? 'active' : ''}`}
                        onClick={() => setGuideAudienceFilter('member')}
                      >
                        {t('audienceMembers')}
                      </button>
                      <button
                        type="button"
                        className={`segmented-item ${guideAudienceFilter === 'staff' ? 'active' : ''}`}
                        onClick={() => setGuideAudienceFilter('staff')}
                      >
                        {t('audienceStaff')}
                      </button>
                    </div>
                  )}
                  {isEditor && (
                    <Button
                      variant="secondary"
                      size="sm"
                      to="/help/guides/new"
                      icon={<Plus size={16} />}
                    >
                      {t('guideNew')}
                    </Button>
                  )}
                </div>
              </div>

              {guidesApi.loading ? (
                <CardSkeleton count={3} />
              ) : !displayedGuides.length ? (
                <EmptyState title={t('guideNone')} />
              ) : (
                <div className="grid cols-3">
                  {displayedGuides.map((g) => {
                    const progress = getGuideProgress(g.slug);
                    const totalSteps = g.step_count || 1;
                    const hasProgress = progress > 0 && progress < totalSteps;

                    return (
                      <Link
                        key={g.id}
                        to={`/help/guides/${g.slug}`}
                        className="card link stack-xs"
                      >
                        <div className="row between nowrap">
                          <div className="bold ellipsis row nowrap gap-xs">
                            <BookOpen size={16} aria-hidden="true" />
                            <span>{pick(g.title_zh, g.title_en)}</span>
                          </div>
                          {!g.published && (
                            <Pill tone="warning">{t('guideDraft')}</Pill>
                          )}
                        </div>

                        <p className="muted small ellipsis-2">
                          {pick(g.summary_zh, g.summary_en)}
                        </p>

                        {hasProgress ? (
                          <div className="stack-xs" style={{ marginTop: 'var(--space-2)' }}>
                            <div className="row between nowrap tiny muted">
                              <span>{t('continueGuide')}</span>
                              <span>
                                {t('guideProgress', { done: progress, total: totalSteps })}
                              </span>
                            </div>
                            <div className="guide-progress-bar">
                              <div
                                className="guide-progress-fill"
                                style={{
                                  width: `${Math.round((progress / totalSteps) * 100)}%`,
                                }}
                              />
                            </div>
                          </div>
                        ) : (
                          <span className="muted tiny" style={{ marginTop: 'var(--space-2)' }}>
                            {t('guideSteps', { count: totalSteps })}
                          </span>
                        )}
                      </Link>
                    );
                  })}
                </div>
              )}
            </section>

            {/* MY REQUESTS SECTION (shown to members or in viewAsMember mode) */}
            {(!isStaff || viewAsMember) && (
              <section className="stack">
                <div className="row between nowrap">
                  <h2>{t('myRequests')}</h2>
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => openComposer()}
                  >
                    {t('contactUs')}
                  </Button>
                </div>

                {ownRequestsApi.loading ? (
                  <ListSkeleton count={3} />
                ) : !ownRequests.length ? (
                  <EmptyState
                    title={t('noRequestsYet')}
                    description={t('noRequestsDesc')}
                    action={
                      <Button
                        variant="primary"
                        onClick={() => openComposer()}
                      >
                        {t('contactUs')}
                      </Button>
                    }
                  />
                ) : (
                  <Card className="stack">
                    <div className="list">
                      {ownRequests.map((tk) => (
                        <Link key={tk.id} to={`/help/${tk.id}`} className="item">
                          <div className="grow min-w-0">
                            <div className="bold ellipsis">{tk.subject}</div>
                            <div className="muted tiny">
                              {tk.last_body ? `${tk.last_body} · ` : ''}
                              {fmtAgo(tk.updated_at, ft)}
                            </div>
                          </div>
                          <StatusPill map={HELP_STATUS} status={tk.status} />
                        </Link>
                      ))}
                    </div>
                  </Card>
                )}
              </section>
            )}
          </div>
        )}

        {/* Contact Composer Modal */}
        {composerOpen && (
          <HelpComposer
            onClose={() => setComposerOpen(false)}
            initialSubject={composerSubject}
            initialRideId={composerRideId}
            guides={guides}
            onCreated={(newId) => {
              ownRequestsApi.reload();
              if (isStaff) staffRequestsApi.reload();
              nav(`/help/${newId}`);
            }}
          />
        )}
      </div>
    </Page>
  );
}
