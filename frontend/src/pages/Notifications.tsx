import { useCallback, useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { safeInternalPath } from '../guideLinks';
import {
  Bell,
  BellOff,
  BellRing,
  Calendar,
  CarFront,
  CheckCheck,
  CreditCard,
  Heart,
  Mail,
  MailOpen,
  Megaphone,
  MessageSquare,
  ShieldAlert,
  Smartphone,
  Trash2,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { api, post, useApi } from '../api';
import { useAuth } from '../auth';
import { PwaInstallModal } from '../components/PwaInstallModal';
import {
  BackToTop,
  Button,
  EmptyState,
  InfiniteSentinel,
  Notice,
  Page,
  PageHeader,
  Spinner,
  useAction,
  useInfiniteScroll,
} from '../components/ui';
import { fmtAgo, parseLocal, useFormatT } from '../format';
import { enablePushNotifications, usePushNotifications } from '../pushNotifications';

interface Notif {
  id: number;
  title: string;
  body: string;
  link: string;
  read: boolean;
  created_at: string;
  category?: string;
  event_type?: string;
  priority?: string;
  entity_type?: string;
  entity_id?: number | null;
}

interface NotifsPage {
  items: Notif[];
  next_before: number | null;
  unread?: number;
}

const CATEGORIES = [
  { key: '', labelKey: 'catAll' },
  { key: 'ride', labelKey: 'catRide' },
  { key: 'event', labelKey: 'catEvent' },
  { key: 'cost', labelKey: 'catCost' },
  { key: 'help', labelKey: 'catHelp' },
  { key: 'family', labelKey: 'catFamily' },
  { key: 'admin', labelKey: 'catAdmin' },
  { key: 'announcement', labelKey: 'catAnnouncement' },
] as const;

const klDateFormatter = new Intl.DateTimeFormat('en-CA', {
  timeZone: 'Asia/Kuala_Lumpur',
  year: 'numeric',
  month: '2-digit',
  day: '2-digit',
});

function klDateKey(input: string | Date): string {
  let d: Date;
  if (typeof input === 'string') {
    const trimmed = input.trim();
    if (!trimmed) return '';
    d = /[zZ]|[+-]\d{2}:?\d{2}$/.test(trimmed) ? new Date(trimmed) : parseLocal(trimmed);
  } else {
    d = input;
  }
  if (Number.isNaN(d.getTime())) {
    return typeof input === 'string' ? input.slice(0, 10) : '';
  }
  return klDateFormatter.format(d);
}

function getNotifIcon(n: Notif) {
  switch (n.category) {
    case 'ride':
      return <CarFront size={18} />;
    case 'event':
      return <Calendar size={18} />;
    case 'cost':
      return <CreditCard size={18} />;
    case 'help':
      return <MessageSquare size={18} />;
    case 'family':
      return <Heart size={18} />;
    case 'admin':
      return <ShieldAlert size={18} />;
    case 'announcement':
      return <Megaphone size={18} />;
    default:
      break;
  }
  const l = (n.link || '').toLowerCase();
  const title = (n.title || '').toLowerCase();
  if (l.includes('ride') || title.includes('ride')) return <CarFront size={18} />;
  if (l.includes('event') || title.includes('event')) return <Calendar size={18} />;
  if (l.includes('settlement') || l.includes('share') || l.includes('cost')) {
    return <CreditCard size={18} />;
  }
  if (l.includes('help') || l.includes('chat')) return <MessageSquare size={18} />;
  return <Bell size={18} />;
}

function buildNotifsPath(unreadOnly: boolean, category: string, before?: number | null): string {
  if (!unreadOnly && !category && before == null) {
    return '/notifications';
  }
  const params = new URLSearchParams({ limit: '30' });
  if (before != null) params.set('before', String(before));
  if (unreadOnly) params.set('unread_only', 'true');
  if (category) params.set('category', category);
  return `/notifications?${params.toString()}`;
}

export default function Notifications() {
  const { t } = useTranslation('notifications');
  const ft = useFormatT();
  const [unreadOnly, setUnreadOnly] = useState(false);
  const [category, setCategory] = useState('');
  const listPath = buildNotifsPath(unreadOnly, category);
  const { data, loading, reload } = useApi<NotifsPage | Notif[]>(
    listPath,
    20000,
    ['notify', 'resume'],
  );
  const [olderItems, setOlderItems] = useState<Notif[]>([]);
  const [nextBefore, setNextBefore] = useState<number | null>(null);
  const [loadingMore, setLoadingMore] = useState(false);
  const [loadMoreError, setLoadMoreError] = useState(false);
  const [readOverrides, setReadOverrides] = useState<Record<number, boolean>>({});
  const [deletedIds, setDeletedIds] = useState<number[]>([]);
  const filterGenRef = useRef(0);

  useEffect(() => {
    filterGenRef.current += 1;
    setOlderItems([]);
    setNextBefore(null);
    setLoadingMore(false);
    setLoadMoreError(false);
    setReadOverrides({});
    setDeletedIds([]);
  }, [listPath]);

  useEffect(() => {
    if (data) {
      setReadOverrides((cur) => (Object.keys(cur).length === 0 ? cur : {}));
      setDeletedIds((cur) => (cur.length === 0 ? cur : []));
      if (!Array.isArray(data)) {
        // Only update the cursor from page 1 when older pages have not been appended yet;
        // otherwise polling page 1 would reset nextBefore and re-fetch duplicate page-2 items.
        setNextBefore((cur) => (olderItems.length > 0 ? cur : data.next_before));
      }
    }
  }, [data, olderItems.length]);

  const { me, refresh } = useAuth();
  const push = usePushNotifications();
  const [dismissed, setDismissed] = useState(() => {
    try {
      return sessionStorage.getItem('gr_notif_prompt_dismissed') === '1';
    } catch {
      return false;
    }
  });
  const [enabling, setEnabling] = useState(false);
  const [pushFeedback, setPushFeedback] = useState<{
    tone: 'success' | 'warning';
    text: string;
  } | null>(null);
  const [showInstallModal, setShowInstallModal] = useState(false);
  const { run } = useAction();
  const nav = useNavigate();

  const deletedSet = new Set(deletedIds);
  const initialItems = Array.isArray(data) ? data : (data?.items ?? []);
  const seenIds = new Set<number>();
  const items: Notif[] = [];
  for (const raw of [...initialItems, ...olderItems]) {
    if (seenIds.has(raw.id) || deletedSet.has(raw.id)) continue;
    seenIds.add(raw.id);
    const read = readOverrides[raw.id] !== undefined ? readOverrides[raw.id] : raw.read;
    if (unreadOnly && read) continue;
    items.push({ ...raw, read });
  }

  const loadMore = useCallback(async () => {
    if (nextBefore == null || loadingMore) return;
    const gen = filterGenRef.current;
    setLoadingMore(true);
    setLoadMoreError(false);
    try {
      const res = await api<NotifsPage | Notif[]>(
        buildNotifsPath(unreadOnly, category, nextBefore),
      );
      if (filterGenRef.current !== gen) return;
      const newItems = Array.isArray(res) ? res : res.items;
      const nb = Array.isArray(res) ? null : res.next_before;
      setOlderItems((prev) => {
        const existing = new Set(prev.map((x) => x.id));
        return [...prev, ...newItems.filter((x) => !existing.has(x.id))];
      });
      setNextBefore(nb);
    } catch {
      if (filterGenRef.current !== gen) return;
      setLoadMoreError(true);
    } finally {
      if (filterGenRef.current === gen) {
        setLoadingMore(false);
      }
    }
  }, [nextBefore, loadingMore, unreadOnly, category]);

  const { sentinelRef } = useInfiniteScroll({
    hasMore: nextBefore != null && !loadMoreError,
    loading: loadingMore || loading,
    onLoadMore: loadMore,
  });

  const open = async (n: Notif) => {
    if (!n.read) {
      setReadOverrides((prev) => ({ ...prev, [n.id]: true }));
      setOlderItems((prev) => prev.map((x) => (x.id === n.id ? { ...x, read: true } : x)));
      await run(() => post(`/notifications/${n.id}/read`));
    }
    refresh();
    const safeLink = safeInternalPath(n.link, null);
    if (safeLink) {
      nav(safeLink);
    } else {
      reload();
    }
  };

  const toggleRead = async (n: Notif) => {
    const nextRead = !n.read;
    setReadOverrides((prev) => ({ ...prev, [n.id]: nextRead }));
    setOlderItems((prev) => prev.map((x) => (x.id === n.id ? { ...x, read: nextRead } : x)));
    await run(async () => {
      await post(`/notifications/${n.id}/${nextRead ? 'read' : 'unread'}`);
      reload();
      refresh();
    });
  };

  const removeNotif = async (n: Notif) => {
    setDeletedIds((prev) => (prev.includes(n.id) ? prev : [...prev, n.id]));
    setOlderItems((prev) => prev.filter((x) => x.id !== n.id));
    await run(async () => {
      await api(`/notifications/${n.id}`, { method: 'DELETE' });
      reload();
      refresh();
    });
  };

  const markAllRead = async () => {
    const patch: Record<number, boolean> = {};
    for (const item of items) {
      patch[item.id] = true;
    }
    setReadOverrides((prev) => ({ ...prev, ...patch }));
    setOlderItems((prev) => prev.map((x) => ({ ...x, read: true })));
    await run(async () => {
      await (category
        ? post('/notifications/read-all', { category })
        : post('/notifications/read-all'));
      reload();
      refresh();
    });
  };

  const dismissPrompt = () => {
    setDismissed(true);
    try {
      sessionStorage.setItem('gr_notif_prompt_dismissed', '1');
    } catch {
      /* storage blocked */
    }
  };

  const deniedCopy =
    push.platform === 'ios'
      ? t('pushDeniedIos')
      : push.platform === 'android'
        ? t('pushDeniedAndroid')
        : t('pushDeniedDesktop');

  const handleEnablePush = async () => {
    setEnabling(true);
    setPushFeedback(null);
    try {
      const res = await enablePushNotifications();
      if (res.ok) {
        setPushFeedback({
          tone: 'success',
          text: t(res.mode === 'push' ? 'pushEnabled' : 'pushEnabledLocal'),
        });
        void refresh();
      } else if (res.reason === 'needs_install_ios') {
        setShowInstallModal(true);
      } else if (res.reason === 'denied') {
        setPushFeedback({ tone: 'warning', text: deniedCopy });
      } else if (res.reason === 'dismissed') {
        setPushFeedback({ tone: 'warning', text: t('pushDismissed') });
      } else if (res.reason === 'subscribe_failed') {
        setPushFeedback({ tone: 'warning', text: t('pushSubscribeFailed') });
      }
    } finally {
      setEnabling(false);
    }
  };

  const canShowBanner = Boolean(
    me && me.status !== 'pending' && me.push_opt_in !== false && !dismissed,
  );
  const needsReconnect = push.availability === 'granted' && push.isSubscribed === false;
  const showPromptCard =
    canShowBanner &&
    pushFeedback?.tone !== 'success' &&
    (push.availability === 'prompt' || needsReconnect);
  const showInstallCard = canShowBanner && push.availability === 'needs_install_ios';
  const showDeniedBanner = canShowBanner && push.availability === 'denied';
  const bannerShown = showPromptCard || showInstallCard || showDeniedBanner;

  const pageUnread = !Array.isArray(data) && typeof data?.unread === 'number' ? data.unread : undefined;
  const hasUnread =
    (pageUnread !== undefined ? pageUnread > 0 : (me?.unread ?? 0) > 0) ||
    items.some((n) => !n.read);

  const nowDate = new Date();
  const todayKl = klDateKey(nowDate);
  const yesterdayKl = klDateKey(new Date(nowDate.getTime() - 86_400_000));
  const todayNotifs: Notif[] = [];
  const yesterdayNotifs: Notif[] = [];
  const earlierNotifs: Notif[] = [];
  for (const n of items) {
    const dk = klDateKey(n.created_at);
    if (dk === todayKl) {
      todayNotifs.push(n);
    } else if (dk === yesterdayKl) {
      yesterdayNotifs.push(n);
    } else {
      earlierNotifs.push(n);
    }
  }

  const renderNotifRow = (n: Notif) => (
    <div
      key={n.id}
      className={`notif-row ${n.priority === 'urgent' ? 'notif-urgent' : ''}`}
    >
      <button
        type="button"
        className={`notif-item ${n.read ? '' : 'notif-unread'}`}
        onClick={() => void open(n)}
      >
        <div className="notif-icon-wrap" aria-hidden="true">
          {getNotifIcon(n)}
        </div>
        <div className="grow min-w-0">
          <div className={`ellipsis ${n.read ? '' : 'bold'}`}>{n.title}</div>
          {n.body && <div className="muted small clamp-2">{n.body}</div>}
          <div className="muted tiny">{fmtAgo(n.created_at, ft)}</div>
        </div>
        <span aria-hidden className={`unread-dot ${n.read ? 'read' : ''}`} />
      </button>
      <div className="notif-row-actions">
        <button
          type="button"
          className="notif-action-btn"
          aria-label={t(n.read ? 'markUnread' : 'markRead')}
          title={t(n.read ? 'markUnread' : 'markRead')}
          onClick={() => void toggleRead(n)}
        >
          {n.read ? <Mail size={15} /> : <MailOpen size={15} />}
        </button>
        <button
          type="button"
          className="notif-action-btn"
          aria-label={t('deleteNotif')}
          title={t('deleteNotif')}
          onClick={() => void removeNotif(n)}
        >
          <Trash2 size={15} />
        </button>
      </div>
    </div>
  );

  const sections = [
    { key: 'today', label: t('today'), rows: todayNotifs },
    { key: 'yesterday', label: t('yesterday'), rows: yesterdayNotifs },
    { key: 'earlier', label: t('earlier'), rows: earlierNotifs },
  ];

  return (
    <Page width="narrow">
      <div className="stack">
        <PageHeader
          title={t('title')}
          actions={
            hasUnread ? (
              <Button
                variant="secondary"
                size="sm"
                onClick={() => void markAllRead()}
                icon={<CheckCheck size={16} />}
              >
                {t('readAll')}
              </Button>
            ) : undefined
          }
        />

        {showPromptCard && (
          <section
            className="card notif-push-banner"
            aria-label={t(needsReconnect ? 'pushReconnectTitle' : 'pushPromptTitle')}
          >
            <div className="notif-push-banner-head">
              <div className="notif-push-banner-icon" aria-hidden="true">
                <BellRing size={20} />
              </div>
              <div className="stack-xs grow min-w-0">
                <div className="bold">
                  {t(needsReconnect ? 'pushReconnectTitle' : 'pushPromptTitle')}
                </div>
                <p className="muted small">
                  {t(needsReconnect ? 'pushReconnectDesc' : 'pushPromptDesc')}
                </p>
              </div>
            </div>
            {pushFeedback && (
              <p className="muted tiny" role="status">
                {pushFeedback.text}
              </p>
            )}
            <div className="notif-push-banner-actions">
              <Button
                variant="primary"
                size="sm"
                loading={enabling}
                onClick={handleEnablePush}
              >
                <BellRing size={15} aria-hidden="true" />
                {t(enabling ? 'enablingNotifications' : 'enableNotifications')}
              </Button>
              <Button variant="ghost" size="sm" onClick={dismissPrompt}>
                {t('notNow')}
              </Button>
            </div>
          </section>
        )}

        {showInstallCard && (
          <section
            className="card notif-push-banner"
            aria-label={t('pushInstallIosTitle')}
          >
            <div className="notif-push-banner-head">
              <div className="notif-push-banner-icon" aria-hidden="true">
                <Smartphone size={20} />
              </div>
              <div className="stack-xs grow min-w-0">
                <div className="bold">{t('pushInstallIosTitle')}</div>
                <p className="muted small">{t('pushInstallIosDesc')}</p>
              </div>
            </div>
            <div className="notif-push-banner-actions">
              <Button
                variant="primary"
                size="sm"
                onClick={() => setShowInstallModal(true)}
              >
                <Smartphone size={15} aria-hidden="true" />
                {t('pushInstallIosAction')}
              </Button>
              <Button variant="ghost" size="sm" onClick={dismissPrompt}>
                {t('notNow')}
              </Button>
            </div>
          </section>
        )}

        {showDeniedBanner && (
          <Notice tone="warning" icon={<BellOff size={18} aria-hidden="true" />}>
            <div className="stack-xs">
              <div className="bold">{t('pushDeniedTitle')}</div>
              <p className="small">{deniedCopy}</p>
              <div>
                <Button variant="ghost" size="sm" onClick={dismissPrompt}>
                  {t('notNow')}
                </Button>
              </div>
            </div>
          </Notice>
        )}

        {!bannerShown && pushFeedback && (
          <Notice tone={pushFeedback.tone} role="status">
            {pushFeedback.text}
          </Notice>
        )}

        <div className="notif-filters">
          <div className="notif-filter-row" role="group" aria-label={t('title')}>
            <button
              type="button"
              className={`notif-chip ${!unreadOnly ? 'active' : ''}`}
              aria-pressed={!unreadOnly}
              onClick={() => setUnreadOnly(false)}
            >
              {t('filterAll')}
            </button>
            <button
              type="button"
              className={`notif-chip ${unreadOnly ? 'active' : ''}`}
              aria-pressed={unreadOnly}
              onClick={() => setUnreadOnly(true)}
            >
              {t('filterUnread')}
            </button>
          </div>
          <div className="notif-filter-row">
            {CATEGORIES.map((c) => (
              <button
                key={c.key || 'all'}
                type="button"
                className={`notif-chip ${category === c.key ? 'active' : ''}`}
                aria-pressed={category === c.key}
                onClick={() => setCategory(c.key)}
              >
                {t(c.labelKey)}
              </button>
            ))}
          </div>
        </div>

        {loading ? (
          <Spinner />
        ) : !items.length ? (
          <EmptyState title={t('none')} />
        ) : (
          <>
            <div className="stack">
              {sections.map(
                (sec) =>
                  sec.rows.length > 0 && (
                    <div key={sec.key}>
                      <div className="notif-date-header">{sec.label}</div>
                      <div className="list">{sec.rows.map(renderNotifRow)}</div>
                    </div>
                  ),
              )}
            </div>
            {loadMoreError && (
              <Notice tone="warning">
                <div className="row between wrap gap-sm items-center">
                  <span className="small">{t('loadMoreError')}</span>
                  <Button variant="ghost" size="sm" onClick={() => void loadMore()}>
                    {t('retry')}
                  </Button>
                </div>
              </Notice>
            )}
            <InfiniteSentinel
              hasMore={nextBefore != null && !loadMoreError}
              loading={loadingMore}
              sentinelRef={sentinelRef}
              empty={!items.length}
            />
            <BackToTop />
          </>
        )}
      </div>
      {showInstallModal && (
        <PwaInstallModal
          onClose={() => setShowInstallModal(false)}
          onNotificationsEnabled={() => void refresh()}
        />
      )}
    </Page>
  );
}
