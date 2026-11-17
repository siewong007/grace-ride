import { FormEvent, useState } from 'react';
import { Pin } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { del, post, put, useApi } from '../../api';
import { useAuth } from '../../auth';
import {
  Button,
  Card,
  CardSkeleton,
  EmptyState,
  ErrorBox,
  Field,
  Modal,
  Page,
  PageHeader,
  Pill,
  useAction,
  useConfirm,
} from '../../components/ui';
import { fmtAgo, useFormatT } from '../../format';
import { ADMIN_HOME } from '../../staffNav';

interface AnnouncementItem {
  id: number;
  title: string;
  body: string;
  pinned: boolean;
  by: string;
  created_at: string;
}

export default function Announcements() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const confirm = useConfirm();
  const { me, can } = useAuth();

  const { data, loading, reload } = useApi<AnnouncementItem[]>('/announcements');
  const [f, setF] = useState({
    title: '',
    body: '',
    pinned: false,
    notify_all: true,
  });
  const [editing, setEditing] = useState<AnnouncementItem | null>(null);
  const { busy, error, run } = useAction();
  const canWrite = can('admin_announcements', 'create');

  const togglePin = (a: AnnouncementItem) =>
    run(
      () => put(`/announcements/${a.id}`, { title: a.title, body: a.body, pinned: !a.pinned }).then(reload),
      a.pinned ? t('announcements.unpinned') : t('announcements.pinnedDone'),
    );

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    // Notifying everyone cannot be taken back, so say so before it happens.
    if (f.notify_all) {
      const ok = await confirm({
        title: t('announcements.notifyConfirmTitle'),
        body: t('announcements.notifyConfirmBody', { title: f.title.trim() }),
        confirmLabel: t('announcements.add'),
        tone: 'primary',
      });
      if (!ok) return;
    }
    run(
      () =>
        post('/announcements', f).then(() => {
          setF({ title: '', body: '', pinned: false, notify_all: true });
          reload();
        }),
      t('announcements.addDone'),
    );
  };

  const remove = async (a: AnnouncementItem) => {
    const ok = await confirm({
      title: t('announcements.delete'),
      body: t('announcements.deleteConfirm'),
      confirmLabel: t('announcements.delete'),
      tone: 'danger',
    });
    if (ok) {
      run(() => del(`/announcements/${a.id}`).then(reload));
    }
  };

  return (
    <Page>
      <PageHeader
        title={t('announcements.title')}
        subtitle={t('announcements.desc')}
        back={ADMIN_HOME}
      />

      <div className="split">
        {/* Creation form */}
        {can('admin_announcements', 'create') && <form className="card stack" onSubmit={submit}>
          <Field label={t('announcements.titleLabel')}>
            <input
              className="input"
              required
              value={f.title}
              onChange={(e) => setF({ ...f, title: e.target.value })}
            />
          </Field>
          <Field label={t('announcements.bodyLabel')}>
            <textarea
              className="input"
              required
              rows={4}
              value={f.body}
              onChange={(e) => setF({ ...f, body: e.target.value })}
            />
          </Field>
          <label className={`check ${f.pinned ? 'on' : ''}`}>
            <input
              type="checkbox"
              checked={f.pinned}
              onChange={(e) => setF({ ...f, pinned: e.target.checked })}
            />
            {t('announcements.pinned')}
          </label>
          <label className={`check ${f.notify_all ? 'on' : ''}`}>
            <input
              type="checkbox"
              checked={f.notify_all}
              onChange={(e) => setF({ ...f, notify_all: e.target.checked })}
            />
            {t('announcements.notifyAll')}
          </label>
          <ErrorBox error={error} />
          <Button variant="primary" disabled={busy} type="submit">
            {t('announcements.add')}
          </Button>
        </form>}

        <div className="stack">
          {/* What members will see */}
          {canWrite && <Card className="stack-sm">
            <div className="row between">
              <span className="muted small">{t('announcements.preview')}</span>
              {f.pinned && (
                <Pill tone="amber" icon={<Pin size={12} />}>
                  {t('announcements.pinned')}
                </Pill>
              )}
            </div>
            <h3>{f.title || t('announcements.titleLabel')}</h3>
            <p className="pre-wrap">{f.body || t('announcements.bodyLabel')}</p>
            <span className="muted small">
              {t('announcements.by', { by: me?.name || 'Staff', ago: ft('format:justNow') })}
            </span>
          </Card>}

          {/* Published announcements */}
          <section>
            <div className="section-head">
              <h2>{t('announcements.published')}</h2>
            </div>
            {loading ? (
              <CardSkeleton count={2} />
            ) : !data?.length ? (
              <EmptyState title={t('announcements.empty')} description={can('admin_announcements', 'create') ? t('announcements.emptyDesc') : undefined} />
            ) : (
              <div className="list">
                {data.map((a) => (
                  <div key={a.id} className="item item-stack announcement-item">
                    <div className="grow min-w-0 stack-xs">
                      <strong className="row nowrap">
                        {a.pinned && <Pin size={14} aria-label={t('announcements.pinned')} />}
                        <span>{a.title}</span>
                      </strong>
                      <p className="pre-wrap">{a.body}</p>
                      <span className="muted small">
                        {t('announcements.by', { by: a.by, ago: fmtAgo(a.created_at, ft) })}
                      </span>
                    </div>
                    <div className="item-actions">
                      {canWrite && (
                        <div className="btn-row">
                          <Button variant="ghost" size="sm" onClick={() => setEditing(a)}>
                            {t('announcements.edit')}
                          </Button>
                          <Button variant="ghost" size="sm" disabled={busy} onClick={() => togglePin(a)}>
                            {a.pinned ? t('announcements.unpin') : t('announcements.pin')}
                          </Button>
                        </div>
                      )}
                      {can('admin_announcements', 'delete') && (
                        <Button variant="ghost" size="sm" className="text-danger" onClick={() => remove(a)}>
                          {t('announcements.delete')}
                        </Button>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </section>
        </div>
      </div>

      {editing && (
        <EditAnnouncement
          item={editing}
          onClose={() => setEditing(null)}
          onSaved={() => {
            setEditing(null);
            reload();
          }}
        />
      )}
    </Page>
  );
}

function EditAnnouncement({
  item,
  onClose,
  onSaved,
}: {
  item: AnnouncementItem;
  onClose: () => void;
  onSaved: () => void;
}) {
  const { t } = useTranslation('admin');
  const [f, setF] = useState({ title: item.title, body: item.body, pinned: item.pinned });
  const { busy, error, run } = useAction();
  return (
    <Modal title={t('announcements.editTitle')} description={t('announcements.editHint')} onClose={onClose}>
      <form
        className="stack"
        onSubmit={(e) => {
          e.preventDefault();
          run(() => put(`/announcements/${item.id}`, f).then(onSaved), t('announcements.updated'));
        }}
      >
        <Field label={t('announcements.titleLabel')}>
          <input className="input" required value={f.title} onChange={(e) => setF({ ...f, title: e.target.value })} />
        </Field>
        <Field label={t('announcements.bodyLabel')}>
          <textarea className="input" required rows={5} value={f.body} onChange={(e) => setF({ ...f, body: e.target.value })} />
        </Field>
        <label className={`check ${f.pinned ? 'on' : ''}`}>
          <input type="checkbox" checked={f.pinned} onChange={(e) => setF({ ...f, pinned: e.target.checked })} />
          {t('announcements.pinned')}
        </label>
        <ErrorBox error={error} />
        <Button variant="primary" type="submit" disabled={busy || !f.title.trim() || !f.body.trim()}>
          {t('announcements.save')}
        </Button>
      </form>
    </Modal>
  );
}
