import { useNavigate } from 'react-router-dom';
import { post, useApi } from '../api';
import { useAuth } from '../auth';
import { Empty, Spinner } from '../components/ui';
import { fmtAgo, useFormatT } from '../format';
import { useTranslation } from 'react-i18next';

interface Notif { id: number; title: string; body: string; link: string; read: boolean; created_at: string }

export default function Notifications() {
  const { t } = useTranslation('notifications');
  const ft = useFormatT();
  const { data, loading, reload } = useApi<Notif[]>('/notifications', 20000);
  const { refresh } = useAuth();
  const nav = useNavigate();
  const open = async (n: Notif) => {
    if (!n.read) await post(`/notifications/${n.id}/read`).catch(() => {});
    refresh();
    if (n.link) nav(n.link);
    else reload();
  };
  return (
    <div className="stack" style={{ maxWidth: 820 }}>
      <div className="page-head">
        <h1>{t('title')}</h1>
        <button className="btn sm" onClick={async () => { await post('/notifications/read-all'); reload(); refresh(); }}>{t('readAll')}</button>
      </div>
      {loading ? <Spinner /> : !data?.length ? <Empty>{t('none')}</Empty> : (
        <div className="card"><div className="list">
          {data.map((n) => (
            <button key={n.id} className="item" onClick={() => open(n)} style={{ background: 'none', border: 'none', borderBottom: '1px solid var(--border)', textAlign: 'left', cursor: 'pointer', width: '100%' }}>
              <span aria-hidden style={{ width: 10, height: 10, borderRadius: 5, background: n.read ? 'transparent' : 'var(--danger)', flex: 'none' }} />
              <div className="grow">
                <div className={n.read ? '' : 'bold'}>{n.title}</div>
                {n.body && <div className="muted small">{n.body}</div>}
              </div>
              <span className="muted tiny">{fmtAgo(n.created_at, ft)}</span>
            </button>
          ))}
        </div></div>
      )}
    </div>
  );
}
