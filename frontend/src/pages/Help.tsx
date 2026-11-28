import { useEffect, useMemo, useRef, useState } from 'react';
import { Link, useNavigate, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import {
  ArrowUpDown,
  BookOpen,
  ChevronDown,
  ChevronRight,
  MessageCircle,
  Plus,
  Search,
  X,
} from 'lucide-react';
import { useApi } from '../api';
import { useAuth } from '../auth';
import { canAnyAdminPage } from '../perms';
import { Ride } from '../components/RideCard';
import HelpComposer from '../components/HelpComposer';
import HelpHome from '../components/HelpHome';
import HelpTopicSheet, { TOPICS, TopicKey } from '../components/HelpTopicSheet';
import GuideArranger from '../components/GuideArranger';
import GuideListRow from '../components/GuideListRow';
import { useGuideRunner } from '../components/GuideRunner';
import { PageChromeContext } from '../components/PageChrome';
import HelpDetail from './HelpDetail';
import type { GuideStep, GuideSummary, TicketSummary } from '../types/help';
import { usePick } from '../hooks/usePick';
import {
  Button,
  Card,
  CardSkeleton,
  EmptyState,
  ListSkeleton,
  Page,
  PageHeader,
  StatusPill,
} from '../components/ui';
import { fmtAgo, HELP_STATUS, HELP_STATUS_STAFF, useFormatT } from '../format';

export type { TicketSummary };

interface TicketCounts {
  open: number;
  answered: number;
  closed: number;
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
  const { me, can } = useAuth();
  const [params] = useSearchParams();

  const isHelpDesk = can('admin_help', 'view') || can('admin_help', 'update');
  const isStaff = isHelpDesk || canAnyAdminPage(can);
  const isEditor = can('admin_help', 'update');

  // Search input & ref
  const searchInputRef = useRef<HTMLInputElement>(null);
  const [searchQuery, setSearchQuery] = useState('');

  // Member topic sheet (answer first, then ask)
  const [topic, setTopic] = useState<{ key: TopicKey; rideId: number | null; text?: string } | null>(null);

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
  const [arranging, setArranging] = useState(false);

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
    if (r && !isHelpDesk) {
      setTopic({ key: 'ride', rideId: Number(r) });
    } else if (r) {
      setComposerRideId(Number(r));
      setComposerSubject(t('relatedRide', { rideId: r }));
      setComposerOpen(true);
    }
  }, [params, t, isHelpDesk]);

  // Data fetching:
  // 1. Guides with steps for instant search (reuse GuideRunnerProvider's loaded guides for non-staff)
  const guideRunner = useGuideRunner();
  const reuseRunnerGuides = guideRunner.hasProvider && !isStaff;
  const guidesApi = useApi<(GuideSummary & { steps?: GuideStep[] })[]>(
    reuseRunnerGuides ? null : `/help/guides?full=true${isStaff ? '&include_drafts=true' : ''}`,
  );
  const guides = useMemo(
    () => (reuseRunnerGuides ? guideRunner.guides : guidesApi.data) ?? [],
    [reuseRunnerGuides, guideRunner.guides, guidesApi.data],
  );
  const guidesLoading = reuseRunnerGuides ? guideRunner.guidesLoading : guidesApi.loading;
  const reloadGuides = reuseRunnerGuides ? guideRunner.reloadGuides : guidesApi.reload;

  // 2. Member's own requests
  const ownRequestsApi = useApi<TicketSummary[]>('/help', 10000);
  const ownRequests = useMemo(() => ownRequestsApi.data ?? [], [ownRequestsApi.data]);

  // 3. Member's upcoming rides
  const ridesApi = useApi<Ride[]>(can('rides') ? '/rides' : null, 30000);
  const memberRides = ridesApi.data ?? [];

  // 4. Staff inbox requests
  const staffStatusParam = staffFilter === 'all' ? '' : `&status=${staffFilter}`;
  const staffSearchParam = inboxQuery.trim() ? `&q=${encodeURIComponent(inboxQuery.trim())}` : '';
  const staffRequestsApi = useApi<TicketSummary[]>(
    isHelpDesk && !viewAsMember
      ? `/help?all=true${allStaffScope ? '' : '&assigned=me'}${staffStatusParam}${staffSearchParam}`
      : null,
    10000,
  );
  const staffRequests = useMemo(() => staffRequestsApi.data ?? [], [staffRequestsApi.data]);

  // 5. Staff counts
  const countsApi = useApi<TicketCounts>(
    isHelpDesk && !viewAsMember ? `/help/counts?all=true${allStaffScope ? '' : '&assigned=me'}` : null,
    10000,
  );
  const counts = {
    open: countsApi.data?.open ?? 0,
    answered: countsApi.data?.answered ?? 0,
    closed: countsApi.data?.closed ?? 0,
  };
  const totalCounts = counts.open + counts.answered + counts.closed;

  // Auto-select first ticket in 2-pane mode on desktop
  useEffect(() => {
    if (isDesktop && isHelpDesk && !viewAsMember && staffRequests.length > 0 && selectedTicketId === null) {
      setSelectedTicketId(staffRequests[0].id);
    }
  }, [isDesktop, isHelpDesk, viewAsMember, staffRequests, selectedTicketId]);

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

  const memberView = !isHelpDesk || viewAsMember;
  // Members can find a help topic by searching for it ("late", "visitor", "pay"...).
  const q = searchQuery.trim().toLowerCase();
  const matchedTopics = q
    ? TOPICS.filter((x) => x.key !== 'driverOffer' && t(`hub.topic.${x.key}`).toLowerCase().includes(q))
    : [];

  const openComposer = (subj = '', rId: number | null = null) => {
    setComposerSubject(subj);
    setComposerRideId(rId);
    setComposerOpen(true);
  };


  // Filtered guides by audience
  const displayedGuides = guides.filter((g) => {
    if (!isStaff) return g.audience === 'member' && g.published;
    if (guideAudienceFilter === 'all') return true;
    return g.audience === guideAudienceFilter;
  });

  return (
    <Page width="wide">
      <div className="stack">
        <PageHeader
          title={isHelpDesk && !viewAsMember ? t('title') : t('howCanWeHelp')}
          subtitle={isHelpDesk && !viewAsMember ? t('desc') : undefined}
          back={isHelpDesk && !viewAsMember ? (can('admin_dashboard') ? '/admin' : '/') : '/me'}
          actions={
            isHelpDesk ? (
              <Button
                variant="secondary"
                size="sm"
                onClick={() => setViewAsMember(!viewAsMember)}
              >
                {viewAsMember ? t('exitViewAsMember') : t('viewAsMember')}
              </Button>
            ) : undefined
          }
        />

        {/* Global Search Bar (press / to focus) */}
        <div className={`help-search-box ${isHelpDesk && !viewAsMember ? '' : 'is-member'}`}>
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

        {/* Search Results Mode: topics, guides, steps and requests as one list of rows */}
        {searchQuery.trim() && searchResults ? (
          <div className="help-home help-search-results">
            {memberView && matchedTopics.length > 0 && (
              <section className="help-block">
                <h2 className="help-label">{t('hub.getHelpWith')}</h2>
                <div className="help-rows">
                  {matchedTopics.map((x) => (
                    <button key={x.key} type="button" className="help-row" onClick={() => setTopic({ key: x.key, rideId: null })}>
                      <span className="help-row-icon" aria-hidden="true">{x.icon}</span>
                      <span className="grow"><HighlightMatch text={t(`hub.topic.${x.key}`)} query={searchQuery} /></span>
                      <ChevronRight size={18} className="muted" aria-hidden="true" />
                    </button>
                  ))}
                </div>
              </section>
            )}

            {searchResults.guides.length > 0 && (
              <section className="help-block">
                <h2 className="help-label">{t('searchGroupGuides')}</h2>
                <div className="help-rows">
                  {searchResults.guides.map(({ guide: g, snippet }) => (
                    <Link key={g.id} to={`/help/guides/${g.slug}`} className="help-row">
                      <span className="help-row-icon" aria-hidden="true"><BookOpen size={18} /></span>
                      <span className="grow min-w-0">
                        <span className="block clamp-2"><HighlightMatch text={pick(g.title_zh, g.title_en)} query={searchQuery} /></span>
                        <span className="muted small block clamp-2"><HighlightMatch text={snippet} query={searchQuery} /></span>
                      </span>
                      <ChevronRight size={18} className="muted" aria-hidden="true" />
                    </Link>
                  ))}
                </div>
              </section>
            )}

            {searchResults.steps.length > 0 && (
              <section className="help-block">
                <h2 className="help-label">{t('searchGroupSteps')}</h2>
                <div className="help-rows">
                  {searchResults.steps.map(({ guide: g, stepIndex, title, body }) => (
                    <Link key={`${g.id}-${stepIndex}`} to={`/help/guides/${g.slug}#step-${stepIndex}`} className="help-row">
                      <span className="help-step-no" aria-hidden="true">{stepIndex + 1}</span>
                      <span className="grow min-w-0">
                        <span className="block clamp-2"><HighlightMatch text={title} query={searchQuery} /></span>
                        <span className="muted small block clamp-2">
                          {pick(g.title_zh, g.title_en)}
                          {body ? ' · ' : ''}
                          {body && <HighlightMatch text={body} query={searchQuery} />}
                        </span>
                      </span>
                      <ChevronRight size={18} className="muted" aria-hidden="true" />
                    </Link>
                  ))}
                </div>
              </section>
            )}

            {searchResults.requests.length > 0 && (
              <section className="help-block">
                <h2 className="help-label">{t('searchGroupRequests')}</h2>
                <div className="help-rows">
                  {searchResults.requests.map(({ ticket: tk, snippet }) => (
                    <Link key={tk.id} to={`/help/${tk.id}`} className="help-row">
                      <span className="grow min-w-0">
                        <span className="block clamp-2"><HighlightMatch text={tk.subject} query={searchQuery} /></span>
                        <span className="muted small block clamp-2"><HighlightMatch text={snippet} query={searchQuery} /></span>
                      </span>
                      <StatusPill map={HELP_STATUS} status={tk.status} />
                    </Link>
                  ))}
                </div>
              </section>
            )}

            {searchResults.total === 0 && !(memberView && matchedTopics.length) && (
              <p className="muted">{t('searchNoResults', { q: searchQuery })}</p>
            )}
            {/* Always a way out: ask, with what they searched for already written. */}
            <button
              type="button"
              className="help-row help-ask-row"
              onClick={() => (memberView ? setTopic({ key: 'other', rideId: null, text: searchQuery.trim() }) : openComposer(searchQuery))}
            >
              <span className="help-row-icon" aria-hidden="true"><MessageCircle size={18} /></span>
              <span className="grow">
                <span className="block bold">{t('hub.cantFind')}</span>
                <span className="muted small block">{t('hub.cantFindDesc')}</span>
              </span>
              <ChevronRight size={18} className="muted" aria-hidden="true" />
            </button>
          </div>
        ) : (
          /* Normal Content Mode */
          <div className="stack">
            {/* STAFF INBOX (Section 3) */}
            {isHelpDesk && !viewAsMember && (
              <section className="stack">
                <div className="help-inbox-bar">
                  <h2>{t('inboxTitle')}</h2>
                  <div className="help-inbox-actions">
                    <div className="segmented-control">
                      <button
                        type="button"
                        aria-pressed={allStaffScope}
                        className={`segmented-item ${allStaffScope ? 'active' : ''}`}
                        onClick={() => setAllStaffScope(true)}
                      >
                        {t('allHelp')}
                      </button>
                      <button
                        type="button"
                        aria-pressed={!allStaffScope}
                        className={`segmented-item ${!allStaffScope ? 'active' : ''}`}
                        onClick={() => setAllStaffScope(false)}
                      >
                        {t('claimedByMeFilter')}
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
                    aria-pressed={staffFilter === 'open'}
                    className={`chip ${staffFilter === 'open' ? 'on' : ''}`}
                    onClick={() => setStaffFilter('open')}
                  >
                    {t('filterNeedsReply', { count: counts.open })}
                  </button>
                  <button
                    type="button"
                    aria-pressed={staffFilter === 'answered'}
                    className={`chip ${staffFilter === 'answered' ? 'on' : ''}`}
                    onClick={() => setStaffFilter('answered')}
                  >
                    {t('filterAnswered', { count: counts.answered })}
                  </button>
                  <button
                    type="button"
                    aria-pressed={staffFilter === 'closed'}
                    className={`chip ${staffFilter === 'closed' ? 'on' : ''}`}
                    onClick={() => setStaffFilter('closed')}
                  >
                    {t('filterResolved', { count: counts.closed })}
                  </button>
                  <button
                    type="button"
                    aria-pressed={staffFilter === 'all'}
                    className={`chip ${staffFilter === 'all' ? 'on' : ''}`}
                    onClick={() => setStaffFilter('all')}
                  >
                    {t('filterAll', { count: totalCounts })}
                  </button>
                </div>

                {/* Search input for inbox */}
                <input
                  className="input"
                  aria-label={t('searchInboxPlaceholder')}
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
                                    {tk.last_staff ? `${t('staffReplyPrefix')} ` : ''}
                                    {tk.last_body}
                                  </div>
                                )}
                                <div className="help-row-status">
                                  <StatusPill map={HELP_STATUS_STAFF} status={tk.status} />
                              {tk.assignee_name && (
                                <span className="muted tiny help-row-claim">
                                  {tk.assignee_id === me?.id ? t('claimedByYou') : t('claimedBy', { name: tk.assignee_name })}
                                </span>
                              )}
                                </div>
                              </div>
                            </button>
                          ))}
                        </div>
                      )}
                    </Card>

                    {/* Right Pane: Selected Thread */}
                    <div className="help-two-pane-right">
                      {selectedTicketId ? (
                        <PageChromeContext.Provider value={null}>
                          <HelpDetail
                            key={selectedTicketId}
                            ticketId={selectedTicketId}
                            isTwoPane
                            onStatusChange={() => {
                              staffRequestsApi.reload();
                              countsApi.reload();
                            }}
                          />
                        </PageChromeContext.Provider>
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
                              <div className="help-row-status">
                                <StatusPill map={HELP_STATUS_STAFF} status={tk.status} />
                              {tk.assignee_name && (
                                <span className="muted tiny help-row-claim">
                                  {tk.assignee_id === me?.id ? t('claimedByYou') : t('claimedBy', { name: tk.assignee_name })}
                                </span>
                              )}
                              </div>
                            </div>
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
                    aria-expanded={myOwnOpen}
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
                              <div className="help-row-status">
                                <StatusPill map={HELP_STATUS} status={tk.status} />
                              </div>
                            </div>
                          </Link>
                        ))
                      )}
                    </div>
                  )}
                </div>
              </section>
            )}

            {/* MEMBER HELP HOME */}
            {(!isHelpDesk || viewAsMember) && (
              <HelpHome
                rides={memberRides}
                requests={ownRequests}
                guides={displayedGuides.filter((g) => g.published)}
                onTopic={(key, rideId) => setTopic({ key, rideId: rideId ?? null })}
              />
            )}

            {/* GUIDES SECTION (help desk: audiences, arranging, new guides) */}
            {isHelpDesk && !viewAsMember && (
            <section id="guides-section" className="stack">
              <div className="help-inbox-bar">
                <h2>{t('guideTab')}</h2>
                <div className="help-inbox-actions">
                  {isStaff && (
                    <div className="segmented-control">
                      <button
                        type="button"
                        aria-pressed={guideAudienceFilter === 'all'}
                        className={`segmented-item ${guideAudienceFilter === 'all' ? 'active' : ''}`}
                        onClick={() => setGuideAudienceFilter('all')}
                      >
                        {t('audienceAll')}
                      </button>
                      <button
                        type="button"
                        aria-pressed={guideAudienceFilter === 'member'}
                        className={`segmented-item ${guideAudienceFilter === 'member' ? 'active' : ''}`}
                        onClick={() => setGuideAudienceFilter('member')}
                      >
                        {t('audienceMembers')}
                      </button>
                      <button
                        type="button"
                        aria-pressed={guideAudienceFilter === 'staff'}
                        className={`segmented-item ${guideAudienceFilter === 'staff' ? 'active' : ''}`}
                        onClick={() => setGuideAudienceFilter('staff')}
                      >
                        {t('audienceStaff')}
                      </button>
                    </div>
                  )}
                  {isEditor && !arranging && guides.length > 1 && (
                    <Button
                      variant="secondary"
                      size="sm"
                      icon={<ArrowUpDown size={16} />}
                      onClick={() => setArranging(true)}
                    >
                      {t('arrangeTitle')}
                    </Button>
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

              {arranging ? (
                <GuideArranger
                  guides={guides}
                  onDone={(saved) => {
                    setArranging(false);
                    if (saved) reloadGuides();
                  }}
                />
              ) : guidesLoading ? (
                <CardSkeleton count={3} />
              ) : !displayedGuides.length ? (
                <EmptyState title={t('guideNone')} />
              ) : (
                <div className="help-rows">
                  {displayedGuides.map((g) => <GuideListRow key={g.id} guide={g} />)}
                </div>
              )}
            </section>

            )}
          </div>
        )}

        {topic && (
          <HelpTopicSheet
            topic={topic.key}
            rideId={topic.rideId}
            initialText={topic.text}
            rides={memberRides}
            guideSlugs={new Set(displayedGuides.filter((g) => g.published).map((g) => g.slug))}
            onClose={() => setTopic(null)}
            onSent={(id) => {
              ownRequestsApi.reload();
              nav(`/help/${id}`);
            }}
          />
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
              if (isHelpDesk) staffRequestsApi.reload();
              nav(`/help/${newId}`);
            }}
          />
        )}
      </div>
    </Page>
  );
}
