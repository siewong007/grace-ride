import { useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Bell,
  Calendar,
  CarFront,
  CheckCheck,
  CreditCard,
  MessageSquare,
} from 'lucide-react';
import { api, post, useApi } from '../api';
import { useAuth } from '../auth';
import {
  BackToTop,
  Button,
  EmptyState,
  InfiniteSentinel,
  Page,
  PageHeader,
  Spinner,
  useAction,
  useInfiniteScroll,
} from '../components/ui';
import { fmtAgo, todayStr, useFormatT } from '../format';
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

  const { refresh } = useAuth();
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
    </Page>
  );
}
