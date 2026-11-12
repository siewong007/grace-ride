import { useState } from 'react';
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

interface Group {
  id: number;
  name: string;
  description: string;
  is_all: boolean;
  member_count: number;
}

interface Area {
  id: number;
  name: string;
  active: boolean;
}

export default function Settings() {
  const { t } = useTranslation('admin');
  const { has, me } = useAuth();

  if (!has('admin') || !me?.mfa_ok) {
    return (
      <Page>
        <EmptyState title={t('settings.noPermission')} />
      </Page>
    );
  }

  return (
    <Page>
      <PageHeader title={t('settings.title')} subtitle={t('settings.desc')} />
      <div className="split">
        <Groups />
        <Areas />
      </div>
    </Page>
  );
}

function Groups() {
  const { t } = useTranslation('admin');
  const confirm = useConfirm();
  const { data, loading, reload } = useApi<Group[]>('/groups');
  const [name, setName] = useState('');
  const [members, setMembers] = useState<Group | null>(null);
  const { busy, error, run } = useAction();

  const removeGroup = async (g: Group) => {
    const ok = await confirm({
      title: t('settings.delete'),
      body: t('settings.deleteGroupConfirm', { name: g.name }),
      confirmLabel: t('settings.delete'),
      tone: 'danger',
    });
    if (ok) {
      run(() => del(`/admin/groups/${g.id}`).then(reload), t('settings.deleted'));
    }
  };

  return (
    <Card className="stack">
      <h3>{t('settings.groups')}</h3>
      {loading ? (
        <CardSkeleton count={2} />
      ) : !data?.length ? (
        <EmptyState title={t('settings.emptyGroups')} />
      ) : (
        <div className="list">
          {data.map((g) => (
            <div key={g.id} className="item">
              <div className="grow">
                <div className="bold">
                  {g.name}{' '}
                  {g.is_all && <Pill tone="blue">{t('settings.allMembers')}</Pill>}
                </div>
                <div className="muted tiny">
                  {g.is_all
                    ? t('settings.allDesc')
                    : t('settings.memberCount', { count: g.member_count })}
                  {g.description && ` · ${g.description}`}
                </div>
              </div>
              {!g.is_all && (
                <div className="btn-row">
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => setMembers(g)}
                  >
                    {t('settings.membersBtn')}
                  </Button>
                  <Button
                    variant="danger"
                    size="sm"
                    onClick={() => removeGroup(g)}
                  >
                    {t('settings.delete')}
                  </Button>
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      <form
        className="row nowrap"
        onSubmit={(e) => {
          e.preventDefault();
          run(
            () =>
              post('/admin/groups', { name }).then(() => {
                setName('');
                reload();
              }),
            t('settings.added'),
          );
        }}
      >
        <input
          className="input grow"
          placeholder={t('settings.groupNamePlaceholder')}
          value={name}
          onChange={(e) => setName(e.target.value)}
          required
        />
        <Button variant="primary" disabled={busy} type="submit">
          {t('settings.addGroup')}
        </Button>
      </form>

      <ErrorBox error={error} />
      {members && (
        <GroupMembers
          group={members}
          onClose={() => {
            setMembers(null);
            reload();
          }}
        />
      )}
    </Card>
  );
}

function GroupMembers({
  group,
  onClose,
}: {
  group: Group;
  onClose: () => void;
}) {
  const { t } = useTranslation('admin');
  const { data: current } = useApi<{ id: number; name: string }[]>(
    `/admin/groups/${group.id}/members`,
  );
  const { data: all } = useApi<{ id: number; name: string; status: string }[]>(
    '/admin/users',
  );
  const [sel, setSel] = useState<number[] | null>(null);
  const [q, setQ] = useState('');
  const chosen = sel ?? current?.map((c) => c.id) ?? [];
  const { busy, error, run } = useAction();

  const toggleUser = (id: number) => {
    setSel(chosen.includes(id) ? chosen.filter((x) => x !== id) : [...chosen, id]);
  };

  const save = () =>
    run(
      () =>
        put(`/admin/groups/${group.id}/members`, { user_ids: chosen }).then(
          onClose,
        ),
      t('settings.memberUpdated'),
    );

  return (
    <Modal
      title={t('settings.groupMembers', {
        name: group.name,
        count: chosen.length,
      })}
      onClose={onClose}
    >
      <input
        className="input"
        placeholder={t('settings.search')}
        value={q}
        onChange={(e) => setQ(e.target.value)}
      />
      <div className="stack-sm group-members-list">
        {all
          ?.filter((u) => u.status === 'active' && u.name.includes(q))
          .map((u) => (
            <label
              key={u.id}
              className={`check ${chosen.includes(u.id) ? 'on' : ''}`}
            >
              <input
                type="checkbox"
                checked={chosen.includes(u.id)}
                onChange={() => toggleUser(u.id)}
              />
              {u.name}
            </label>
          ))}
      </div>
      <ErrorBox error={error} />
      <Button variant="primary" block disabled={busy} onClick={save}>
        {t('settings.save')}
      </Button>
    </Modal>
  );
}

function Areas() {
  const { t } = useTranslation('admin');
  const { data, loading, reload } = useApi<Area[]>('/areas');
  const [name, setName] = useState('');
  const [edit, setEdit] = useState<Area | null>(null);
  const { busy, error, run } = useAction();

  return (
    <Card className="stack">
      <h3>{t('settings.areas')}</h3>
      {loading ? (
        <CardSkeleton count={2} />
      ) : !data?.length ? (
        <EmptyState title={t('settings.emptyAreas')} />
      ) : (
        <div className="chips">
          {data.map((a) => (
            <button
              key={a.id}
              type="button"
              className={`chip ${a.active ? 'on' : ''}`}
              onClick={() => setEdit(a)}
            >
              <span>{a.name}</span>
              {!a.active && (
                <span className="muted tiny"> ({t('settings.inactive')})</span>
              )}
            </button>
          ))}
        </div>
      )}

      <form
        className="row nowrap"
        onSubmit={(e) => {
          e.preventDefault();
          run(
            () =>
              post('/admin/areas', { name }).then(() => {
                setName('');
                reload();
              }),
            t('settings.added'),
          );
        }}
      >
        <input
          className="input grow"
          placeholder={t('settings.areaPlaceholder')}
          value={name}
          onChange={(e) => setName(e.target.value)}
          required
        />
        <Button variant="primary" disabled={busy} type="submit">
          {t('settings.addArea')}
        </Button>
      </form>

      <ErrorBox error={error} />

      {edit && (
        <Modal title={t('settings.editArea')} onClose={() => setEdit(null)}>
          <Field label={t('settings.areaName')}>
            <input
              className="input"
              value={edit.name}
              onChange={(e) => setEdit({ ...edit, name: e.target.value })}
            />
          </Field>
          <label className={`check ${edit.active ? 'on' : ''}`}>
            <input
              type="checkbox"
              checked={edit.active}
              onChange={(e) => setEdit({ ...edit, active: e.target.checked })}
            />
            {t('settings.openSignup')}
          </label>
          <Button
            variant="primary"
            block
            onClick={() =>
              run(
                () =>
                  put(`/admin/areas/${edit.id}`, edit).then(() => {
                    setEdit(null);
                    reload();
                  }),
                t('settings.saved'),
              )
            }
          >
            {t('settings.saveArea')}
          </Button>
        </Modal>
      )}
    </Card>
  );
}
