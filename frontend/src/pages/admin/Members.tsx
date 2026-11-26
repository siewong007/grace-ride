import { FormEvent, useEffect, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Download, Share2 } from 'lucide-react';
import { del, post, put, useApi } from '../../api';
import { useAuth } from '../../auth';
import { useTableParams } from '../../components/tableState';
import { useExportAll } from '../../components/useExportAll';
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
import { fmtAgo, roleLabel, todayStr, useFormatT } from '../../format';
import { useRoleOptions } from './roleData';
import { ADMIN_HOME, useStaffNav } from '../../staffNav';

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
  photo_file_id?: number | null;
  avatar_color?: string | null;
  avatar_icon?: string | null;
  family_name?: string | null;
}

interface Group {
  id: number;
  name: string;
  is_all: boolean;
  /** The group visitors join events as: set by the system, not given to members by hand. */
  is_visitor?: boolean;
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

/** Copy text to the clipboard and say whether it worked; the link is still on screen if it did not. */
function useCopy() {
  const { t } = useTranslation('admin');
  const toast = useToast();
  return async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      toast.success(t('members.copied'));
    } catch {
      // Clipboard access is unavailable outside secure browser contexts.
      toast.error(t('members.copyFailed'));
    }
  };
}

export default function Members() {
  const { t } = useTranslation('admin');
  const { can } = useAuth();
  // Each tab is its own page permission; a member sees the ones they hold.
  type Tab = 'members' | 'invites' | 'recovery';
  const { counts } = useStaffNav();
  const tabs = [
    { value: 'members' as Tab, label: t('members.membersTab'), show: can('admin_members'), count: counts?.pending_members || undefined },
    { value: 'invites' as Tab, label: t('members.invitesTab'), show: can('admin_invites'), count: counts?.open_invites || undefined },
    { value: 'recovery' as Tab, label: t('members.recoveryTab'), show: can('admin_recovery'), count: counts?.pending_recovery || undefined },
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
      <PageHeader title={t('members.title')} subtitle={t('members.desc')} back={ADMIN_HOME} />
      <Tabs
        value={tab}
        onChange={(v) => handleTabChange(v as Tab)}
        items={tabs.map(({ value, label, count }) => ({ value, label, count }))}
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
  const { names, assignable } = useRoleOptions();
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
    filterKeys: ['q', 'status', 'role'],
  });

  // The API requires explicit pagination parameters to return totals.
  const apiQuery = new URLSearchParams(queryString);
  apiQuery.set('page', String(page));
  apiQuery.set('limit', String(pageSize));
  const { data, loading, reload } = useApi<PaginatedUsers | User[]>(
    `/admin/users?${apiQuery}`,
  );
  const [initialLoadComplete, setInitialLoadComplete] = useState(false);
  if (!loading && !initialLoadComplete) setInitialLoadComplete(true);

  const [modalUser, setModalUser] = useState<{
    user: User;
    mode: 'edit' | 'approve';
  } | null>(null);
  const [selectedIds, setSelectedIds] = useState<number[]>([]);
  const [batchModalOpen, setBatchModalOpen] = useState(false);

  useEffect(() => {
    setSelectedIds([]);
  }, [page, pageSize, queryString]);

  const users = Array.isArray(data) ? data : data?.items ?? [];
  const editableUsers = users.filter((u) => editable(u));
  const total = Array.isArray(data) ? data.length : data?.total ?? 0;
  const totalPages = Array.isArray(data) ? 1 : data?.total_pages ?? 1;
  const selectedUsers = users.filter((u) => editable(u) && selectedIds.includes(u.id));

  const filterDefs: TableFilter[] = [
    { type: 'text', key: 'q', label: t('members.search') },
    {
      type: 'select',
      key: 'status',
      label: t('members.status'),
      allLabel: t('members.allStatus'),
      options: [
        { value: 'pending', label: t('members.pending') },
        { value: 'active', label: t('members.active') },
        { value: 'disabled', label: t('members.disabled') },
      ],
    },
    {
      type: 'select',
      key: 'role',
      label: t('members.role'),
      allLabel: t('members.allRoles'),
      options: assignable.map((r) => ({ value: r, label: roleLabel(r, ft, names) })),
    },
  ];

  const exporter = useExportAll<User>();
  const handleExport = () =>
    exporter.run({
      path: '/admin/users',
      query: apiQuery,
      filename: `grace-ride-members-${todayStr()}`,
      headers: ['ID', 'Name', 'Email', 'Phone', 'Roles', 'Status', 'Passkeys', 'Groups', 'Joined'],
      row: (u) => [u.id, u.name, u.email, u.phone, u.roles.join('; '), u.status, u.passkeys, u.groups, u.created_at],
    });

  const columns: TableColumn<User>[] = [];
  if (canEdit) {
    columns.push({
      key: 'select',
      header: (
        <input
          type="checkbox"
          aria-label={t('members.selectAll')}
          checked={editableUsers.length > 0 && editableUsers.every((u) => selectedIds.includes(u.id))}
          ref={(el) => {
            if (el) {
              const count = editableUsers.filter((u) => selectedIds.includes(u.id)).length;
              el.indeterminate = count > 0 && count < editableUsers.length;
            }
          }}
          onChange={(e) => {
            if (e.target.checked) {
              setSelectedIds(Array.from(new Set([...selectedIds, ...editableUsers.map((u) => u.id)])));
            } else {
              const pageIds = new Set(editableUsers.map((u) => u.id));
              setSelectedIds(selectedIds.filter((id) => !pageIds.has(id)));
            }
          }}
        />
      ),
      headerClassName: 'col-select',
      cellClassName: 'col-select',
      render: (u) =>
        editable(u) ? (
          <input
            type="checkbox"
            aria-label={`Select ${u.name}`}
            checked={selectedIds.includes(u.id)}
            onChange={(e) => {
              if (e.target.checked) {
                setSelectedIds((prev) => [...prev, u.id]);
              } else {
                setSelectedIds((prev) => prev.filter((id) => id !== u.id));
              }
            }}
          />
        ) : null,
    });
  }
  columns.push(
    {
      key: 'name',
      header: t('members.name'),
      sortable: true,
      render: (u) => (
        <div className="row nowrap">
          <Avatar name={u.name} fileId={u.photo_file_id} color={u.avatar_color} icon={u.avatar_icon} size="sm" />
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
  );
  if (canEdit) {
    columns.push({
      key: 'actions',
      header: t('members.actions'),
      headerClassName: 'col-sticky-end',
      cellClassName: 'col-sticky-end',
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
      <div className="quick-date-chips row wrap gap-xs" role="group" aria-label={t('members.status')}>
        <button
          type="button"
          className={`chip ${!filters.status ? 'on' : ''}`}
          onClick={() => setFilter('status', '')}
        >
          {t('members.allStatus')}
        </button>
        <button
          type="button"
          className={`chip ${filters.status === 'pending' ? 'on' : ''}`}
          onClick={() => setFilter('status', 'pending')}
        >
          {t('members.pending')}
        </button>
        <button
          type="button"
          className={`chip ${filters.status === 'active' ? 'on' : ''}`}
          onClick={() => setFilter('status', 'active')}
        >
          {t('members.active')}
        </button>
      </div>

      <Card>
        {loading && !initialLoadComplete ? (
          <CardSkeleton count={4} />
        ) : (
          <UniversalTable
            columns={columns}
            items={loading ? [] : users}
            itemKey={(u) => u.id}
            compactRow={(u) => (
              <>
                {canEdit && editable(u) && (
                  <input
                    type="checkbox"
                    aria-label={`Select ${u.name}`}
                    checked={selectedIds.includes(u.id)}
                    onChange={(e) => {
                      if (e.target.checked) setSelectedIds((prev) => [...prev, u.id]);
                      else setSelectedIds((prev) => prev.filter((id) => id !== u.id));
                    }}
                  />
                )}
                <Avatar name={u.name} fileId={u.photo_file_id} color={u.avatar_color} icon={u.avatar_icon} size="sm" />
                <div className="grow min-w-0">
                  <div className="bold ellipsis">{u.name}</div>
                  <div className="muted small ellipsis">{u.email}</div>
                  <div className="muted tiny ellipsis">
                    {u.roles.length > 0
                      ? u.roles.map((r) => roleLabel(r, ft, names)).join(' · ')
                      : t('members.member')}
                    {u.groups ? ` · ${u.groups}` : ''}
                  </div>
                </div>
                {canEdit && editable(u) && (
                  u.status === 'pending' ? (
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
                  )
                )}
              </>
            )}
            sort={sort}
            onSortChange={setSort}
            filters={filterDefs}
            filterValues={filters}
            onFilterChange={setFilter}
            onClearFilters={clearFilters}
            toolbarActions={!loading && total > 0 ? (
              <Button variant="secondary" size="sm" onClick={handleExport} loading={exporter.busy} icon={<Download size={16} />}>
                {t('common:exportAll', { count: total })}
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

      {selectedIds.length > 0 && (
        <div className="table-batch-bar" role="region" aria-label="Batch actions">
          <span className="batch-count bold">{t('members.selectedCount', { count: selectedIds.length })}</span>
          <div className="row gap-xs">
            {canEdit && (
              <Button
                variant="primary"
                size="sm"
                onClick={() => setBatchModalOpen(true)}
              >
                {t('members.batchApprove', { count: selectedIds.length })}
              </Button>
            )}
            <Button variant="ghost" size="sm" onClick={() => setSelectedIds([])}>
              {t('members.batchClear')}
            </Button>
          </div>
        </div>
      )}

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

      {batchModalOpen && selectedUsers.length > 0 && (
        <BatchApproveModal
          selectedUsers={selectedUsers}
          onClose={() => setBatchModalOpen(false)}
          onSuccess={() => {
            setBatchModalOpen(false);
            setSelectedIds([]);
            reload();
          }}
        />
      )}
    </div>
  );
}

/** Batch approve modal for rapidly triaging multiple new members */
function BatchApproveModal({
  selectedUsers,
  onClose,
  onSuccess,
}: {
  selectedUsers: User[];
  onClose: () => void;
  onSuccess: () => void;
}) {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const { can, me } = useAuth();
  const confirm = useConfirm();
  const canAssign = can('admin_roles', 'update');
  const { assignable, names } = useRoleOptions();
  const { data: groups } = useApi<Group[]>('/groups');
  const [selectedGroups, setSelectedGroups] = useState<number[]>([]);
  const [groupsTouched, setGroupsTouched] = useState(false);
  const [role, setRole] = useState<string>('member');
  const { busy, error, run } = useAction();

  const toggleGroup = (id: number) => {
    setGroupsTouched(true);
    setSelectedGroups((curr) =>
      curr.includes(id) ? curr.filter((g) => g !== id) : [...curr, id],
    );
  };

  const handleBatchSubmit = async () => {
    const viceNeedsAck =
      !!me?.roles?.includes('vice_president') &&
      !me.roles.includes('president') &&
      !me.roles.includes('superadmin');
    const promotingAdmin = canAssign && role === 'admin' && selectedUsers.some((u) => !u.roles.includes('admin'));
    if (viceNeedsAck && promotingAdmin) {
      const ok = await confirm({
        title: t('members.presidentAckTitle'),
        body: t('members.presidentAckBody', { name: selectedUsers.map((u) => u.name).join(', ') }),
        confirmLabel: t('members.presidentAckConfirm'),
        tone: 'primary',
      });
      if (!ok) return;
    }
    run(async () => {
      const payloadRoles = role === 'member' ? [] : [role];
      await Promise.all(
        selectedUsers.map((u) =>
          put(`/admin/users/${u.id}`, {
            roles: canAssign ? payloadRoles : u.roles,
            status: 'active',
            ...(groups && (groupsTouched || u.status === 'pending') ? { group_ids: selectedGroups } : {}),
            ...(viceNeedsAck && promotingAdmin && !u.roles.includes('admin')
              ? { president_acknowledged: true }
              : {}),
          }),
        ),
      );
      onSuccess();
    }, t('members.batchApproveSuccess', { count: selectedUsers.length }));
  };

  return (
    <Modal title={t('members.batchApproveTitle')} onClose={onClose}>
      <div className="stack gap-md">
        <p className="muted small">
          {t('members.batchApproveDesc', { count: selectedUsers.length })}
        </p>

        <div className="card flat tight stack-xs">
          <div className="label small bold">{t('members.selectedCount', { count: selectedUsers.length })}:</div>
          <div className="chips">
            {selectedUsers.map((u) => (
              <Pill key={u.id} tone="gray">
                {u.name}
              </Pill>
            ))}
          </div>
        </div>

        {canAssign && (
          <div className="stack-xs">
            <label className="label small bold">{t('members.batchAssignRole')}</label>
            <div className="chips">
              <button
                type="button"
                className={`chip ${role === 'member' ? 'on' : ''}`}
                onClick={() => setRole('member')}
              >
                {t('members.member')}
              </button>
              {assignable.map((r) => (
                <button
                  key={r}
                  type="button"
                  className={`chip ${role === r ? 'on' : ''}`}
                  onClick={() => setRole(r)}
                >
                  {roleLabel(r, ft, names)}
                </button>
              ))}
            </div>
          </div>
        )}

        <div className="stack-xs">
          <label className="label small bold">{t('members.batchAssignGroups')}</label>
          <div className="chips">
            {(groups ?? [])
              .filter((g) => !g.is_all && !g.is_visitor)
              .map((g) => {
              const active = selectedGroups.includes(g.id);
              return (
                <button
                  key={g.id}
                  type="button"
                  className={`chip ${active ? 'on' : ''}`}
                  onClick={() => toggleGroup(g.id)}
                >
                  {g.name}
                </button>
              );
            })}
          </div>
        </div>

        <ErrorBox error={error} />

        <div className="row end gap-xs">
          <Button variant="ghost" onClick={onClose} disabled={busy}>
            {t('common:cancel')}
          </Button>
          <Button
            variant="primary"
            onClick={handleBatchSubmit}
            loading={busy}
          >
            {t('members.batchConfirm', { count: selectedUsers.length })}
          </Button>
        </div>
      </div>
    </Modal>
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
  const { can, me } = useAuth();
  const confirm = useConfirm();
  // Giving and taking roles decides what a member may do: it needs the roles page's update permission.
  const canAssign = can('admin_roles', 'update');
  const { assignable, names } = useRoleOptions();
  const { data: groups } = useApi<Group[]>('/groups');

  const [roles, setRoles] = useState<string[]>(
    mode === 'edit' ? user.roles : [],
  );
  const [status, setStatus] = useState(mode === 'approve' ? 'active' : user.status);

  // A Vice-President making an ordinary member an administrator must first
  // confirm the President has acknowledged it. The server enforces the flag.
  const viceNeedsAck =
    !!me?.roles?.includes('vice_president') &&
    !me.roles.includes('president') &&
    !me.roles.includes('superadmin');
  const promotingToAdmin = !user.roles.includes('admin') && roles.includes('admin');

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

  const save = async () => {
    const needsAck = viceNeedsAck && promotingToAdmin;
    if (needsAck) {
      const ok = await confirm({
        title: t('members.presidentAckTitle'),
        body: t('members.presidentAckBody', { name: user.name }),
        confirmLabel: t('members.presidentAckConfirm'),
        tone: 'primary',
      });
      if (!ok) return;
    }
    run(
      () =>
        put(`/admin/users/${user.id}`, {
          roles,
          status,
          ...(groups ? { group_ids: selectedG } : {}),
          ...(needsAck ? { president_acknowledged: true } : {}),
        }).then(onSuccess),
      mode === 'approve'
        ? t('members.memberApproved')
        : t('members.memberUpdated'),
    );
  };

  const modalTitle =
    mode === 'approve'
      ? t('members.modeApprove', { name: user.name })
      : t('members.modeEdit', { name: user.name });

  return (
    <Modal title={modalTitle} onClose={onClose}>
      <div className="stack-sm">
        <p className="muted small">{user.email}</p>
        {user.family_name ? (
          <p className="muted small">
            {t('members.family')}: {user.family_name}
          </p>
        ) : null}

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
              ?.filter((g) => !g.is_all && !g.is_visitor)
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
  const confirm = useConfirm();
  const { can } = useAuth();
  const { data, loading, reload } = useApi<InviteItem[]>('/admin/invites');
  const [link, setLink] = useState<string | null>(null);
  const { busy, error, run } = useAction();
  const copy = useCopy();

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
            <div className="btn-row">
              <Button
                variant="secondary"
                size="sm"
                type="button"
                onClick={() => copy(link)}
              >
                {t('members.copy')}
              </Button>
              <Button
                variant="secondary"
                size="sm"
                type="button"
                className="btn-whatsapp"
                icon={<Share2 size={16} />}
                onClick={() =>
                  window.open(
                    `https://wa.me/?text=${encodeURIComponent(
                      `${t('members.inviteShareText')}\n\n${link}`
                    )}`,
                    '_blank',
                    'noopener,noreferrer'
                  )
                }
              >
                {t('members.shareWhatsapp')}
              </Button>
            </div>
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
              <div key={i.id} className="item item-stack">
                <div className="grow min-w-0">
                  <div className="bold text-break">
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
                      <>
                        <Button variant="secondary" size="sm" onClick={() => copy(i.url!)}>
                          {t('members.copy')}
                        </Button>
                        <Button
                          variant="secondary"
                          size="sm"
                          className="btn-whatsapp"
                          icon={<Share2 size={16} />}
                          onClick={() =>
                            window.open(
                              `https://wa.me/?text=${encodeURIComponent(
                                `${t('members.inviteShareText')}\n\n${i.url}`
                              )}`,
                              '_blank',
                              'noopener,noreferrer'
                            )
                          }
                        >
                          {t('members.shareWhatsapp')}
                        </Button>
                      </>
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
  const confirm = useConfirm();
  const copy = useCopy();
  const canDecide = can('admin_recovery', 'update');
  const { data, loading, reload } = useApi<RecoveryItem[]>('/admin/recovery');
  const [link, setLink] = useState<string | null>(null);
  const { busy, error, run } = useAction();

  // Approving signs the member out everywhere and issues a login link; rejecting is final.
  // Both are asked about first, naming who it is for.
  const approve = async (r: RecoveryItem) => {
    const who = `${r.name ?? r.email} (${r.email})`;
    const second = r.status === 'awaiting_second';
    const ok = await confirm({
      title: second ? t('members.confirmSecond') : t('members.approve'),
      body: t(second ? 'members.confirmSecondBody' : 'members.approveRecoveryConfirm', { who }),
      confirmLabel: second ? t('members.confirmSecond') : t('members.approve'),
      tone: 'primary',
    });
    if (!ok) return;
    run(async () => {
      const x = await post<{ url?: string; awaiting_second?: boolean }>(`/admin/recovery/${r.id}/approve`);
      if (x.url) setLink(x.url);
      reload();
    });
  };

  const reject = async (r: RecoveryItem) => {
    const ok = await confirm({
      title: t('members.reject'),
      body: t('members.rejectRecoveryConfirm', { who: `${r.name ?? r.email} (${r.email})` }),
      confirmLabel: t('members.reject'),
      tone: 'danger',
    });
    if (ok) run(() => post(`/admin/recovery/${r.id}/reject`).then(reload));
  };

  return (
    <div className="stack">
      <div className="notice info">{t('members.recoveryNotice')}</div>
      {link && (
        <div className="notice ok stack-sm">
          <span>{t('members.recoveryLinkLabel')}</span>
          <code className="text-break">{link}</code>
          <div className="btn-row">
            <Button variant="secondary" size="sm" type="button" onClick={() => copy(link)}>
              {t('members.copy')}
            </Button>
            <Button variant="ghost" size="sm" type="button" onClick={() => setLink(null)}>
              {t('members.dismiss')}
            </Button>
          </div>
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
              <div key={r.id} className="item item-stack">
                <div className="grow min-w-0">
                  <div className="bold text-break">
                    {r.name ?? t('members.nonMember')} · {r.email}
                  </div>
                  <div className="small text-break">{r.reason}</div>
                  <div className="muted tiny">{fmtAgo(r.created_at, ft)}</div>
                </div>
                {r.status === 'pending' || r.status === 'awaiting_second' ? (
                  <div className="item-actions">
                    {r.status === 'awaiting_second' && (
                      <span className="muted tiny">
                        {t('members.secondAdminNotice')}
                      </span>
                    )}
                    <div className="btn-row">
                      <Button
                        variant="primary"
                        size="sm"
                        disabled={busy || !r.name || !canDecide}
                        onClick={() => approve(r)}
                      >
                        {r.status === 'awaiting_second'
                          ? t('members.confirmSecond')
                          : t('members.approve')}
                      </Button>
                      <Button
                        variant="secondary"
                        size="sm"
                        disabled={busy || !canDecide}
                        onClick={() => reject(r)}
                      >
                        {t('members.reject')}
                      </Button>
                    </div>
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
