import { FormEvent, useState } from 'react';
import { Pin } from 'lucide-react';
import { del, post, useApi } from '../../api';
import { ErrorBox, Field, useAction, useConfirm } from '../../components/ui';
import { fmtAgo, useFormatT } from '../../format';
import { useTranslation } from 'react-i18next';

export default function Announcements() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const confirm = useConfirm();
  const { data, reload } = useApi<any[]>('/announcements');
  const [f, setF] = useState({ title: '', body: '', pinned: false, notify_all: true });
  const { busy, error, run } = useAction();
  const submit = (e: FormEvent) => {
    e.preventDefault();
    run(() => post('/announcements', f).then(() => { setF({ title: '', body: '', pinned: false, notify_all: true }); reload(); }), t('announcements.addDone'));
  };
  return (
    <div className="stack">
      <div className="page-head"><h1>{t('announcements.title')}</h1></div>
      <div className="split">
        <form className="card stack" onSubmit={submit}>
          <Field label={t('announcements.titleLabel')}>
            <input className="input" required value={f.title} onChange={(e) => setF({ ...f, title: e.target.value })} />
          </Field>
          <Field label={t('announcements.bodyLabel')}>
            <textarea className="input" required value={f.body} onChange={(e) => setF({ ...f, body: e.target.value })} />
          </Field>
          <label className={`check ${f.pinned ? 'on' : ''}`}><input type="checkbox" checked={f.pinned} onChange={(e) => setF({ ...f, pinned: e.target.checked })} />{t('announcements.pinned')}</label>
          <label className={`check ${f.notify_all ? 'on' : ''}`}><input type="checkbox" checked={f.notify_all} onChange={(e) => setF({ ...f, notify_all: e.target.checked })} />{t('announcements.notifyAll')}</label>
          <ErrorBox error={error} />
          <button className="btn primary" disabled={busy} type="submit">{t('announcements.add')}</button>
        </form>
        <div className="stack-sm">
          {data?.map((a) => (
            <div key={a.id} className="card stack-sm">
              <div className="row between">
                <strong className="row nowrap" style={{ gap: 'var(--space-1)' }}>
                  {a.pinned && <Pin size={14} />}
                  <span>{a.title}</span>
                </strong>
                <button
                  className="btn sm danger"
                  onClick={async () => {
                    const ok = await confirm({
                      title: t('announcements.delete'),
                      body: t('announcements.deleteConfirm'),
                      confirmLabel: t('announcements.delete'),
                      tone: 'danger',
                    });
                    if (ok) run(() => del(`/announcements/${a.id}`).then(reload));
                  }}
                >
                  {t('announcements.delete')}
                </button>
              </div>
              <p className="small" style={{ whiteSpace: 'pre-wrap' }}>{a.body}</p>
              <span className="muted tiny">{t('announcements.by', { by: a.by, ago: fmtAgo(a.created_at, ft) })}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
