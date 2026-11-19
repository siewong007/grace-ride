import { FormEvent, useState } from 'react';
import { Pin, Users } from 'lucide-react';
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
import { ordinary, treeOrder, type TreeGroup } from '../../groupTree';
import { ADMIN_HOME } from '../../staffNav';

interface AnnouncementItem {
  id: number;
  title: string;
  body: string;
  pinned: boolean;
  by: string;
  created_at: string;
  /** Only for `groups` (and their sub-groups). False: for every member. */
  targeted?: boolean;
  groups?: { id: number; name: string }[];
}

interface Group extends TreeGroup {
  member_count: number;
}

const groupIdsOf = (a: AnnouncementItem) => (a.groups ?? []).map((g) => g.id);

export default function Announcements() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const confirm = useConfirm();
  const { me, can } = useAuth();

  const { data, loading, reload } = useApi<AnnouncementItem[]>('/announcements?manage=true');
  const { data: groupData } = useApi<Group[]>('/groups');
  const groups = ordinary(groupData ?? []);
  const blank = { title: '', body: '', pinned: false, notify_all: true, group_ids: [] as number[] };
  const [f, setF] = useState(blank);
  const audienceNames = groups.filter((g) => f.group_ids.includes(g.id)).map((g) => g.name).join(', ');
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
    // Notifying cannot be taken back, so say who will hear about it before it happens.
    if (f.notify_all) {
      const ok = await confirm({
        title: f.group_ids.length ? t('announcements.notifyGroupsConfirmTitle') : t('announcements.notifyConfirmTitle'),
        body: f.group_ids.length
          ? t('announcements.notifyGroupsConfirmBody', { title: f.title.trim(), groups: audienceNames })
          : t('announcements.notifyConfirmBody', { title: f.title.trim() }),
        confirmLabel: t('announcements.add'),
        tone: 'primary',
      });
      if (!ok) return;
    }
    run(
      () =>
        post('/announcements', f).then(() => {
          setF(blank);
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
          <AudiencePicker groups={groups} value={f.group_ids} onChange={(group_ids) => setF({ ...f, group_ids })} />
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
            {f.group_ids.length ? t('announcements.notifyGroups') : t('announcements.notifyAll')}
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
              <div className="row">
                <AudiencePill names={audienceNames} />
                {f.pinned && (
                  <Pill tone="amber" icon={<Pin size={12} />}>
                    {t('announcements.pinned')}
                  </Pill>
                )}
              </div>
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
                      <div className="row">
                        <AudiencePill
                          names={(a.groups ?? []).map((g) => g.name).join(', ')}
                          hidden={a.targeted && !a.groups?.length}
                        />
                        <span className="muted small">
                          {t('announcements.by', { by: a.by, ago: fmtAgo(a.created_at, ft) })}
                        </span>
                      </div>
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
          groups={groups}
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

/** Who an announcement is for: chosen groups (and their sub-groups), or nobody chosen for everyone. */
function AudiencePicker({
  groups,
  value,
  onChange,
}: {
  groups: Group[];
  value: number[];
  onChange: (ids: number[]) => void;
}) {
  const { t } = useTranslation('admin');
  const toggle = (id: number) => onChange(value.includes(id) ? value.filter((x) => x !== id) : [...value, id]);
  return (
    <fieldset className="stack-sm">
      <legend>
        <strong>{t('announcements.audience')}</strong>
      </legend>
      <p className="muted small">{t('announcements.audienceHint')}</p>
      <div className="grid cols-2">
        {treeOrder(groups).map(({ group: g, depth }) => (
          <label key={g.id} className={`check ${value.includes(g.id) ? 'on' : ''}`}>
            <input type="checkbox" checked={value.includes(g.id)} onChange={() => toggle(g.id)} />
            <span>
              {depth > 0 && <span className="muted" aria-hidden="true">{'↳ '.repeat(depth)}</span>}
              {g.name}
              {!g.is_all && <span className="muted tiny"> · {t('announcements.peopleCount', { n: g.member_count })}</span>}
            </span>
          </label>
        ))}
      </div>
    </fieldset>
  );
}

/** "Everyone", or the groups an announcement is for. `hidden`: its groups were all deleted. */
function AudiencePill({ names, hidden = false }: { names: string; hidden?: boolean }) {
  const { t } = useTranslation('admin');
  return (
    <Pill tone={names || hidden ? 'blue' : 'gray'} icon={<Users size={12} />}>
      {hidden ? t('announcements.audienceGone') : names ? t('announcements.forGroups', { groups: names }) : t('announcements.forEveryone')}
    </Pill>
  );
}

function EditAnnouncement({
  item,
  groups,
  onClose,
  onSaved,
}: {
  item: AnnouncementItem;
  groups: Group[];
  onClose: () => void;
  onSaved: () => void;
}) {
  const { t } = useTranslation('admin');
  const [f, setF] = useState({ title: item.title, body: item.body, pinned: item.pinned, group_ids: groupIdsOf(item) });
  const { busy, error, run } = useAction();
  return (
    <Modal title={t('announcements.editTitle')} description={t('announcements.editHint')} onClose={onClose}>
      <form
        className="stack"
        onSubmit={(e) => {
          e.preventDefault();
          // Send the audience only when it was changed: an announcement whose groups were
          // all deleted is hidden, and a typo fix must not quietly open it to everyone.
          const original = groupIdsOf(item);
          const same = f.group_ids.length === original.length && f.group_ids.every((g) => original.includes(g));
          const { group_ids, ...rest } = f;
          run(() => put(`/announcements/${item.id}`, same ? rest : { ...rest, group_ids }).then(onSaved), t('announcements.updated'));
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
        <AudiencePicker groups={groups} value={f.group_ids} onChange={(group_ids) => setF({ ...f, group_ids })} />
        <ErrorBox error={error} />
        <Button variant="primary" type="submit" disabled={busy || !f.title.trim() || !f.body.trim()}>
          {t('announcements.save')}
        </Button>
      </form>
    </Modal>
  );
}
