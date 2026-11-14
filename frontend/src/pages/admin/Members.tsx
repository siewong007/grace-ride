import { FormEvent, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Download } from 'lucide-react';
import { del, post, put, useApi } from '../../api';
import { useAuth } from '../../auth';
import { useTableParams } from '../../components/tableState';
import {
  Avatar,
  Button,
  Card,
  CardSkeleton,
  EmptyState,
  NoAccess,
  ErrorBox,
  Field,
  Modal,
  Page,
  PageHeader,
  Pagination,
  Pill,
  Tabs,
  TableColumn,
  TableFilter,
  UniversalTable,
  useAction,
  useConfirm,
  useToast,
} from '../../components/ui';
import { exportToCsv, fmtAgo, roleLabel, todayStr, useFormatT } from '../../format';
import { useRoleOptions } from './roleData';

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

export default function Members() {
  const { t } = useTranslation('admin');
  const { can } = useAuth();
  // Each tab is its own page permission; a member sees the ones they hold.
  type Tab = 'members' | 'invites' | 'recovery';
  const tabs = [
    { value: 'members' as Tab, label: t('members.membersTab'), show: can('admin_members') },
    { value: 'invites' as Tab, label: t('members.invitesTab'), show: can('admin_invites') },
    { value: 'recovery' as Tab, label: t('members.recoveryTab'), show: can('admin_recovery') },
  ].filter((x) => x.show);
  const [params, setParams] = useSearchParams();
  const currentTab = params.get('tab') as Tab;
  const tab = tabs.some((x) => x.value === currentTab) ? currentTab : tabs[0]?.value;

  const handleTabChange = (nextTab: Tab) => {
    const p = new URLSearchParams(params);
    if (nextTab !== 'members') p.set('tab', nextTab);
    else p.delete('tab');
    p.delete('page');
    p.delete('limit');
    p.delete('q');
    p.delete('sort');
    p.delete('dir');
    setParams(p, { replace: true });
  };

  if (!tab) {
    return (
      <Page>
        <NoAccess />
      </Page>
    );
  }


  return (
    <Page>
      <PageHeader title={t('members.title')} subtitle={t('members.desc')} />
      <Tabs
        value={tab}
        onChange={(v) => handleTabChange(v as Tab)}
        items={tabs.map(({ value, label }) => ({ value, label }))}
      />
      {tab === 'members' && <MemberList />}
      {tab === 'invites' && <Invites />}
      {tab === 'recovery' && <Recovery />}
    </Page>
  );
}

interface PaginatedUsers {
  items: User[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

function MemberList() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const { can, me } = useAuth();
  const canEdit = can('admin_members', 'update');
  // A super admin's account is changed only by another super admin (the server enforces it).
  const isSuper = !!me?.roles?.includes('superadmin');
  const editable = (u: User) => isSuper || !u.roles.includes('superadmin');
  const { names } = useRoleOptions();
  const {
    page,
    pageSize,
    sort,
    filters,
    setPage,
    setPageSize,
    setSort,
    setFilter,
    clearFilters,
    hasFilters,
    queryString,
  } = useTableParams({
    defaultPageSize: 50,
    defaultSort: { key: 'name', dir: 'asc' },
    filterKeys: ['q'],
  });

  // The API requires explicit pagination parameters to return totals.
  const apiQuery = new URLSearchParams(queryString);
  apiQuery.set('page', String(page));
  apiQuery.set('limit', String(pageSize));
  const { data, loading, reload } = useApi<PaginatedUsers | User[]>(
    `/admin/users?${apiQuery}`,
    undefined,
    { latestOnly: true },
  );
  const [initialLoadComplete, setInitialLoadComplete] = useState(false);
  if (!loading && !initialLoadComplete) setInitialLoadComplete(true);

  const [modalUser, setModalUser] = useState<{
    user: User;
    mode: 'edit' | 'approve';
  } | null>(null);

  const users = Array.isArray(data) ? data : data?.items ?? [];
  const total = Array.isArray(data) ? data.length : data?.total ?? 0;
  const totalPages = Array.isArray(data) ? 1 : data?.total_pages ?? 1;

  const filterDefs: TableFilter[] = [
    { type: 'text', key: 'q', label: t('members.search') },
  ];

  const handleExport = () => {
    const headers = ['ID', 'Name', 'Email', 'Phone', 'Roles', 'Status', 'Passkeys', 'Groups', 'Joined'];
    const rows = users.map((u) => [
      u.id,
      u.name,
      u.email,
      u.phone,
      u.roles.join('; '),
      u.status,
      u.passkeys,
      u.groups,
      u.created_at,
    ]);
    exportToCsv(`grace-ride-members-${todayStr()}`, headers, rows);
  };

  const columns: TableColumn<User>[] = [
    {
      key: 'name',
      header: t('members.name'),
      sortable: true,
      render: (u) => (
        <div className="row nowrap">
          <Avatar name={u.name} size="sm" />
          <span>{u.name}</span>
        </div>
      ),
    },
    {
      key: 'email',
      header: t('members.email'),
      sortable: true,
      render: (u) => u.email,
    },
    {
      key: 'role',
      header: t('members.role'),
      render: (u) => (
        <div className="chips">
          {u.roles.length > 0 ? (
            u.roles.map((r) => (
              <Pill key={r} tone="blue">
                {roleLabel(r, ft, names)}
              </Pill>
            ))
          ) : (
            <Pill tone="gray">{t('members.member')}</Pill>
          )}
        </div>
      ),
    },
    {
      key: 'group',
      header: t('members.group'),
      render: (u) => u.groups || <span className="muted">—</span>,
      cellClassName: 'small',
    },
    {
      key: 'passkey',
      header: t('members.passkey'),
      render: (u) => u.passkeys,
      cellClassName: 'mono',
    },
  ];
  if (canEdit) {
    columns.push({
      key: 'actions',
      header: t('members.actions'),
      render: (u) =>
        !editable(u) ? null : (
          <>
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
          </>
        ),
    });
  }

  return (
    <div className="stack">
      <Card>
        {loading && !initialLoadComplete ? (
          <CardSkeleton count={4} />
        ) : (
          <UniversalTable
            columns={columns}
            items={loading ? [] : users}
            itemKey={(u) => u.id}
            sort={sort}
            onSortChange={setSort}
            filters={filterDefs}
            filterValues={filters}
            onFilterChange={setFilter}
            onClearFilters={clearFilters}
            toolbarActions={!loading && users.length > 0 ? (
              <Button variant="secondary" size="sm" onClick={handleExport} icon={<Download size={16} />}>
                {t('common:exportCsv', 'Export CSV')}
              </Button>
            ) : undefined}
            emptyContent={loading ? <CardSkeleton count={4} /> : (
                <EmptyState
                  title={t('members.none')}
                  action={hasFilters ? <Button variant="secondary" onClick={clearFilters}>{t('common:clearFilters')}</Button> : undefined}
                />
              )}
          />
        )}
        {!loading && users.length > 0 && (
          <Pagination
            page={page}
            totalPages={totalPages}
            total={total}
            pageSize={pageSize}
            onPageChange={setPage}
            onPageSizeChange={setPageSize}
          />
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
  const { can } = useAuth();
  // Giving and taking roles decides what a member may do: it needs the roles page's update permission.
  const canAssign = can('admin_roles', 'update');
  const { assignable, names } = useRoleOptions();
  const { data: groups } = useApi<Group[]>('/groups');

  const [roles, setRoles] = useState<string[]>(
    mode === 'edit' ? user.roles : [],
  );
  const [status, setStatus] = useState(mode === 'approve' ? 'active' : user.status);

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
          status,
          ...(groups ? { group_ids: selectedG } : {}),
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

        {mode === 'edit' && (
          <Field label={t('members.status')}>
            <select className="input" value={status} onChange={(e) => setStatus(e.target.value)}>
              <option value="active">{t('members.active')}</option>
              <option value="disabled">{t('members.disabled')}</option>
            </select>
          </Field>
        )}

        <div>
          <strong className="small">{t('members.role')}</strong>
          <div className="chips">
            {(canAssign ? assignable : roles).map((r) => (
              <button
                key={r}
                type="button"
                role="checkbox"
                aria-checked={roles.includes(r)}
                disabled={!canAssign}
                className={`chip ${roles.includes(r) ? 'on' : ''}`}
                onClick={() => toggleRole(r)}
              >
                {roleLabel(r, ft, names)}
              </button>
            ))}
          </div>
          {!canAssign && <p className="muted tiny">{t('members.rolesReadOnly')}</p>}
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
  const { can } = useAuth();
  const { data, loading, reload } = useApi<InviteItem[]>('/admin/invites');
  const [link, setLink] = useState<string | null>(null);
  const { busy, error, run } = useAction();

  const copy = async (s: string) => {
    try {
      await navigator.clipboard.writeText(s);
      toast.success(t('members.copied'));
    } catch {
      // Clipboard access is unavailable outside secure browser contexts.
    }
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
      body: t('members.revokeInviteConfirm', { email: invite.email || t('members.openInvite') }),
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
        {can('admin_invites', 'create') && (
          <Button variant="primary" disabled={busy} type="submit">
            {t('members.createInvite')}
          </Button>
        )}
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
                    {i.roles.map((r: string) => roleLabel(r, ft)).join('、') ||
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
                    {can('admin_invites', 'delete') && (
                      <Button variant="danger" size="sm" onClick={() => revoke(i)}>
                        {t('members.revoke')}
                      </Button>
                    )}
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
  const { can } = useAuth();
  const canDecide = can('admin_recovery', 'update');
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
                      disabled={busy || !r.name || !canDecide}
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
                      disabled={busy || !canDecide}
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
