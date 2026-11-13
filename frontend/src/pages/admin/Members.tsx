import { FormEvent, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { del, post, put, useApi } from '../../api';
import { useAuth } from '../../auth';
import {
  Avatar,
  Button,
  Card,
  CardSkeleton,
  EmptyState,
  ErrorBox,
  Modal,
  Page,
  PageHeader,
  Pill,
  Tabs,
  useAction,
  useConfirm,
  useToast,
} from '../../components/ui';
import { fmtAgo, ROLE_LABEL, useFormatT } from '../../format';

interface User {
  id: number;
  name: string;
  email: string;
  phone: string;
  roles: string[];
  status: string;
  created_at: string;
  passkeys: number;
  groups: string;
}

interface Group {
  id: number;
  name: string;
  is_all: boolean;
}

interface InviteItem {
  id: number;
  email: string | null;
  url: string | null;
  kind: string;
  roles: string[];
  status: string;
  created_at: string;
}

interface RecoveryItem {
  id: number;
  name: string | null;
  email: string;
  reason: string;
  status: string;
  created_at: string;
}

const ROLES = ['admin', 'transport', 'finance', 'leader', 'driver'];

export default function Members() {
  const { t } = useTranslation('admin');
  const { has, me } = useAuth();
  const [tab, setTab] = useState<'members' | 'invites' | 'recovery'>('members');

  if (!has('admin') || !me?.mfa_ok) {
    return (
      <Page>
        <EmptyState title={t('members.noPermission')} />
      </Page>
    );
  }

  return (
    <Page>
      <PageHeader title={t('members.title')} subtitle={t('members.desc')} />
      <Tabs
        value={tab}
        onChange={(v) => setTab(v as 'members' | 'invites' | 'recovery')}
        items={[
          { value: 'members', label: t('members.membersTab') },
          { value: 'invites', label: t('members.invitesTab') },
          { value: 'recovery', label: t('members.recoveryTab') },
        ]}
      />
      {tab === 'members' && <MemberList />}
      {tab === 'invites' && <Invites />}
      {tab === 'recovery' && <Recovery />}
    </Page>
  );
}

function MemberList() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const [q, setQ] = useState('');
  const { data, loading, reload } = useApi<User[]>(
    `/admin/users?q=${encodeURIComponent(q)}`,
  );
  const [modalUser, setModalUser] = useState<{
    user: User;
    mode: 'edit' | 'approve';
  } | null>(null);

  return (
    <div className="stack">
      <input
        className="input"
        placeholder={t('members.search')}
        value={q}
        onChange={(e) => setQ(e.target.value)}
      />

      <Card>
        {loading ? (
          <CardSkeleton count={4} />
        ) : !data?.length ? (
          <EmptyState title={t('members.none')} />
        ) : (
          <div className="table-wrap">
            <table className="table responsive sticky-header">
              <thead>
                <tr>
                  <th>{t('members.name')}</th>
                  <th>{t('members.email')}</th>
                  <th>{t('members.role')}</th>
                  <th>{t('members.group')}</th>
                  <th>{t('members.passkey')}</th>
                  <th>{t('members.actions')}</th>
                </tr>
              </thead>
              <tbody>
                {data.map((u) => (
                  <tr key={u.id}>
                    <td data-label={t('members.name')} className="bold">
                      <div className="row nowrap">
                        <Avatar name={u.name} size="sm" />
                        <span>{u.name}</span>
                      </div>
                    </td>
                    <td data-label={t('members.email')}>{u.email}</td>
                    <td data-label={t('members.role')}>
                      <div className="chips">
                        {u.roles.length > 0 ? (
                          u.roles.map((r) => (
                            <Pill key={r} tone="blue">
                              {ft(ROLE_LABEL[r] ?? r)}
                            </Pill>
                          ))
                        ) : (
                          <Pill tone="gray">{t('members.member')}</Pill>
                        )}
                      </div>
                    </td>
                    <td data-label={t('members.group')} className="small">
                      {u.groups || <span className="muted">—</span>}
                    </td>
                    <td data-label={t('members.passkey')} className="mono">
                      {u.passkeys}
                    </td>
                    <td data-label={t('members.actions')}>
                      {u.status === 'pending' ? (
                        <Button
                          variant="primary"
                          size="sm"
                          onClick={() => setModalUser({ user: u, mode: 'approve' })}
                        >
                          {t('members.approveMember')}
                        </Button>
                      ) : (
                        <Button
                          variant="secondary"
                          size="sm"
                          onClick={() => setModalUser({ user: u, mode: 'edit' })}
                        >
                          {t('members.edit')}
                        </Button>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Card>

      {modalUser && (
        <MemberFormModal
          user={modalUser.user}
          mode={modalUser.mode}
          onClose={() => setModalUser(null)}
          onSuccess={() => {
            setModalUser(null);
            reload();
          }}
        />
      )}
    </div>
  );
}

/** Merged MemberFormModal handling both Edit and Approve modes */
function MemberFormModal({
  user,
  mode,
  onClose,
  onSuccess,
}: {
  user: User;
  mode: 'edit' | 'approve';
  onClose: () => void;
  onSuccess: () => void;
}) {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const { data: groups } = useApi<Group[]>('/groups');

  const [roles, setRoles] = useState<string[]>(
    mode === 'edit' ? user.roles : [],
  );

  const initialGroups = user.groups ? user.groups.split('、') : [];
  const [gids, setGids] = useState<number[] | null>(
    mode === 'edit' ? null : [],
  );

  const selectedG =
    gids ??
    (groups?.filter((g) => initialGroups.includes(g.name)).map((g) => g.id) ?? []);

  const { busy, error, run } = useAction();

  const toggleRole = (r: string) => {
    setRoles((curr) =>
      curr.includes(r) ? curr.filter((x) => x !== r) : [...curr, r],
    );
  };

  const toggleGroup = (id: number) => {
    const next = selectedG.includes(id)
      ? selectedG.filter((x) => x !== id)
      : [...selectedG, id];
    setGids(next);
  };

  const save = () =>
    run(
      () =>
        put(`/admin/users/${user.id}`, {
          roles,
          status: 'active',
          group_ids: selectedG,
        }).then(onSuccess),
      mode === 'approve'
        ? t('members.memberApproved')
        : t('members.memberUpdated'),
    );

  const modalTitle =
    mode === 'approve'
      ? t('members.modeApprove', { name: user.name })
      : t('members.modeEdit', { name: user.name });

  return (
    <Modal title={modalTitle} onClose={onClose}>
      <div className="stack-sm">
        <p className="muted small">{user.email}</p>

        <div>
          <strong className="small">{t('members.role')}</strong>
          <div className="chips">
            {ROLES.map((r) => (
              <button
                key={r}
                type="button"
                role="checkbox"
                aria-checked={roles.includes(r)}
                className={`chip ${roles.includes(r) ? 'on' : ''}`}
                onClick={() => toggleRole(r)}
              >
                {ft(ROLE_LABEL[r] ?? r)}
              </button>
            ))}
          </div>
        </div>

        <div>
          <strong className="small">{t('members.group')}</strong>
          <div className="chips">
            {groups
              ?.filter((g) => !g.is_all)
              .map((g) => (
                <button
                  key={g.id}
                  type="button"
                  role="checkbox"
                  aria-checked={selectedG.includes(g.id)}
                  className={`chip ${selectedG.includes(g.id) ? 'on' : ''}`}
                  onClick={() => toggleGroup(g.id)}
                >
                  {g.name}
                </button>
              ))}
          </div>
        </div>

        <ErrorBox error={error} />

        <Button variant="primary" block disabled={busy} onClick={save}>
          {mode === 'approve'
            ? t('members.approveMember')
            : t('members.update')}
        </Button>
      </div>
    </Modal>
  );
}

function Invites() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const toast = useToast();
  const confirm = useConfirm();
  const { data, loading, reload } = useApi<InviteItem[]>('/admin/invites');
  const [link, setLink] = useState<string | null>(null);
  const { busy, error, run } = useAction();

  const copy = (s: string) => {
    navigator.clipboard?.writeText(s);
    toast.success(t('members.copied'));
  };

  const submit = (e: FormEvent) => {
    e.preventDefault();
    run(async () => {
      const r = await post<{ url: string }>('/admin/invites', {});
      setLink(r.url);
      reload();
    }, t('members.inviteCreated'));
  };

  const revoke = async (invite: InviteItem) => {
    const ok = await confirm({
      title: t('members.revoke'),
      body: t('members.deleteGroupConfirm', { name: invite.email || t('members.openInvite') }),
      confirmLabel: t('members.revoke'),
      tone: 'danger',
    });
    if (ok) {
      run(() => del(`/admin/invites/${invite.id}`).then(reload), t('members.revoked'));
    }
  };

  return (
    <div className="stack">
      <form className="card stack" onSubmit={submit}>
        <p className="muted small">{t('members.inviteHint')}</p>
        <ErrorBox error={error} />
        <Button variant="primary" disabled={busy} type="submit">
          {t('members.createInvite')}
        </Button>
        {link && (
          <div className="notice ok stack-sm">
            <span>{t('members.inviteLink')}</span>
            <code className="text-break">{link}</code>
            <Button
              variant="secondary"
              size="sm"
              type="button"
              onClick={() => copy(link)}
            >
              {t('members.copy')}
            </Button>
          </div>
        )}
      </form>

      {/* Invites list as cards */}
      <Card className="stack">
        <h3>{t('members.inviteLog')}</h3>
        {loading ? (
          <CardSkeleton count={3} />
        ) : !data?.length ? (
          <EmptyState title={t('members.emptyInvites')} />
        ) : (
          <div className="list">
            {data.map((i) => (
              <div key={i.id} className="item">
                <div className="grow">
                  <div className="bold">
                    {i.email || t('members.openInvite')}{' '}
                    {i.kind === 'recovery' && (
                      <Pill tone="blue">{t('members.recoveryTab')}</Pill>
                    )}
                  </div>
                  <div className="muted tiny">
                    {i.roles.map((r: string) => ft(ROLE_LABEL[r] ?? r)).join('、') ||
                      t('members.member')}{' '}
                    · {fmtAgo(i.created_at, ft)}
                  </div>
                </div>
                {i.status === 'open' ? (
                  <div className="btn-row">
                    {/* Recovery links are shown once, on approval, and never listed. */}
                    {i.url && (
                      <Button variant="secondary" size="sm" onClick={() => copy(i.url!)}>
                        {t('members.copy')}
                      </Button>
                    )}
                    <Button
                      variant="danger"
                      size="sm"
                      onClick={() => revoke(i)}
                    >
                      {t('members.revoke')}
                    </Button>
                  </div>
                ) : (
                  <Pill tone={i.status === 'used' ? 'green' : 'gray'}>
                    {i.status === 'used' ? t('members.used') : t('members.expired')}
                  </Pill>
                )}
              </div>
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}

function Recovery() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const { data, loading, reload } = useApi<RecoveryItem[]>('/admin/recovery');
  const [link, setLink] = useState<string | null>(null);
  const { busy, error, run } = useAction();

  return (
    <div className="stack">
      <div className="notice info">{t('members.recoveryNotice')}</div>
      {link && (
        <div className="notice ok text-break">
          {t('members.recoveryLink', { link })}
        </div>
      )}
      <ErrorBox error={error} />

      <Card>
        {loading ? (
          <CardSkeleton count={3} />
        ) : !data?.length ? (
          <EmptyState title={t('members.noneRecovery')} />
        ) : (
          <div className="list">
            {data.map((r) => (
              <div key={r.id} className="item">
                <div className="grow">
                  <div className="bold">
                    {r.name ?? t('members.nonMember')} · {r.email}
                  </div>
                  <div className="small">{r.reason}</div>
                  <div className="muted tiny">{fmtAgo(r.created_at, ft)}</div>
                </div>
                {r.status === 'pending' || r.status === 'awaiting_second' ? (
                  <div className="btn-row">
                    {r.status === 'awaiting_second' && (
                      <span className="muted tiny">
                        {t('members.secondAdminNotice')}
                      </span>
                    )}
                    <Button
                      variant="primary"
                      size="sm"
                      disabled={busy || !r.name}
                      onClick={() =>
                        run(async () => {
                          const x = await post<{
                            url?: string;
                            awaiting_second?: boolean;
                          }>(`/admin/recovery/${r.id}/approve`);
                          if (x.url) setLink(x.url);
                          reload();
                        })
                      }
                    >
                      {r.status === 'awaiting_second'
                        ? t('members.confirmSecond')
                        : t('members.approve')}
                    </Button>
                    <Button
                      variant="secondary"
                      size="sm"
                      disabled={busy}
                      onClick={() =>
                        run(() =>
                          post(`/admin/recovery/${r.id}/reject`).then(reload),
                        )
                      }
                    >
                      {t('members.reject')}
                    </Button>
                  </div>
                ) : (
                  <Pill tone={r.status === 'approved' ? 'green' : 'gray'}>
                    {r.status === 'approved'
                      ? t('members.approved')
                      : t('members.rejected')}
                  </Pill>
                )}
              </div>
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}
