import { FormEvent, useState } from 'react';
import { Pin } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { del, post, useApi } from '../../api';
import { useAuth } from '../../auth';
import {
  Button,
  Card,
  CardSkeleton,
  EmptyState,
  ErrorBox,
  Field,
  Page,
  PageHeader,
  Pill,
  useAction,
  useConfirm,
} from '../../components/ui';
import { fmtAgo, useFormatT } from '../../format';

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
  const { busy, error, run } = useAction();

  const submit = (e: FormEvent) => {
    e.preventDefault();
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
          <Card className="stack-sm">
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
          </Card>

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
                  <div key={a.id} className="item announcement-item">
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
                    {can('admin_announcements', 'delete') && (
                      <Button variant="ghost" size="sm" className="text-danger" onClick={() => remove(a)}>
                        {t('announcements.delete')}
                      </Button>
                    )}
                  </div>
                ))}
              </div>
            )}
          </section>
        </div>
      </div>
    </Page>
  );
}
