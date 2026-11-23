import { useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Bell,
  BellOff,
  BellRing,
  Calendar,
  CarFront,
  CheckCheck,
  CreditCard,
  MessageSquare,
  Smartphone,
} from 'lucide-react';
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
import { fmtAgo, todayStr, useFormatT } from '../format';
import { enablePushNotifications, usePushNotifications } from '../pushNotifications';
import { useTranslation } from 'react-i18next';

interface Notif {
  id: number;
  title: string;
  body: string;
  link: string;
  read: boolean;
  created_at: string;
}

interface NotifsPage {
  items: Notif[];
  next_before: number | null;
}

export default function Notifications() {
  const { t } = useTranslation('notifications');
  const ft = useFormatT();
  const { data, loading, reload } = useApi<NotifsPage | Notif[]>('/notifications', 20000);
  const [olderItems, setOlderItems] = useState<Notif[]>([]);
  const [nextBefore, setNextBefore] = useState<number | null>(null);
  const [loadingMore, setLoadingMore] = useState(false);

  useEffect(() => {
    if (data && !Array.isArray(data)) {
      // Only update the cursor from page 1 when older pages have not been appended yet;
      // otherwise polling page 1 would reset nextBefore and re-fetch duplicate page-2 items.
      setNextBefore((cur) => (olderItems.length > 0 ? cur : data.next_before));
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
  const [pushFeedback, setPushFeedback] = useState<{ tone: 'success' | 'warning'; text: string } | null>(null);
  const [showInstallModal, setShowInstallModal] = useState(false);
  const { run } = useAction();
  const nav = useNavigate();

  const initialItems = Array.isArray(data) ? data : data?.items ?? [];
  const seenIds = new Set<number>();
  const items: Notif[] = [];
  for (const n of [...initialItems, ...olderItems]) {
    if (!seenIds.has(n.id)) {
      seenIds.add(n.id);
      items.push(n);
    }
  }

  const loadMore = useCallback(async () => {
    if (nextBefore == null || loadingMore) return;
    setLoadingMore(true);
    try {
      const res = await api<NotifsPage | Notif[]>(`/notifications?before=${nextBefore}&limit=30`);
      const newItems = Array.isArray(res) ? res : res.items;
      const nb = Array.isArray(res) ? null : res.next_before;
      setOlderItems((prev) => {
        const existing = new Set(prev.map((x) => x.id));
        return [...prev, ...newItems.filter((x) => !existing.has(x.id))];
      });
      setNextBefore(nb);
    } catch {
      // ignore
    } finally {
      setLoadingMore(false);
    }
  }, [nextBefore, loadingMore]);

  const { sentinelRef } = useInfiniteScroll({
    hasMore: nextBefore != null,
    loading: loadingMore || loading,
    onLoadMore: loadMore,
  });

  const open = async (n: Notif) => {
    if (!n.read) {
      await run(() => post(`/notifications/${n.id}/read`));
      setOlderItems((prev) => prev.map((x) => (x.id === n.id ? { ...x, read: true } : x)));
    }
    refresh();
    if (
      n.link &&
      n.link.startsWith('/') &&
      !n.link.startsWith('//') &&
      !n.link.startsWith('/api/') &&
      !n.link.includes(':')
    ) {
      nav(n.link);
    } else {
      reload();
    }
  };

  const markAllRead = async () => {
    await run(async () => {
      await post('/notifications/read-all');
      setOlderItems((prev) => prev.map((x) => ({ ...x, read: true })));
      reload();
      refresh();
    });
  };

  const getNotifIcon = (n: Notif) => {
    const l = (n.link || '').toLowerCase();
    const title = (n.title || '').toLowerCase();
    if (l.includes('ride') || title.includes('ride')) return <CarFront size={18} />;
    if (l.includes('event') || title.includes('event')) return <Calendar size={18} />;
    if (l.includes('settlement') || l.includes('share') || l.includes('cost')) {
      return <CreditCard size={18} />;
    }
    if (l.includes('help') || l.includes('chat')) return <MessageSquare size={18} />;
    return <Bell size={18} />;
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

  const today = todayStr();
  const todayNotifs = items.filter((n) => n.created_at.slice(0, 10) === today);
  const earlierNotifs = items.filter((n) => n.created_at.slice(0, 10) !== today);

  return (
    <Page width="narrow">
      <div className="stack">
        <PageHeader
          title={t('title')}
          actions={
            items.some((n) => !n.read) ? (
              <Button
                variant="ghost"
                size="sm"
                onClick={markAllRead}
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

        {loading ? (
          <Spinner />
        ) : !items.length ? (
          <EmptyState title={t('none')} />
        ) : (
          <>
            <div className="stack">
              {todayNotifs.length > 0 && (
                <div>
                  <div className="notif-date-header">{t('today')}</div>
                  <div className="list">
                    {todayNotifs.map((n) => (
                      <button
                        key={n.id}
                        type="button"
                        className={`notif-item ${n.read ? '' : 'notif-unread'}`}
                        onClick={() => open(n)}
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
                    ))}
                  </div>
                </div>
              )}

              {earlierNotifs.length > 0 && (
                <div>
                  <div className="notif-date-header">{t('earlier')}</div>
                  <div className="list">
                    {earlierNotifs.map((n) => (
                      <button
                        key={n.id}
                        type="button"
                        className={`notif-item ${n.read ? '' : 'notif-unread'}`}
                        onClick={() => open(n)}
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
                    ))}
                  </div>
                </div>
              )}
            </div>
            <InfiniteSentinel
              hasMore={nextBefore != null}
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
