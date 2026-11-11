import { FormEvent, useState } from 'react';
import { del, post, put, useApi } from '../../api';
import { useAuth } from '../../auth';
import { Empty, ErrorBox, Modal, Pill, Spinner, Tabs, useAction } from '../../components/ui';
import { fmtAgo, ROLE_LABEL, useFormatT } from '../../format';
import { useTranslation } from 'react-i18next';

interface User { id: number; name: string; email: string; phone: string; roles: string[]; status: string; created_at: string; passkeys: number; groups: string }
interface Group { id: number; name: string; is_all: boolean }
const ROLES = ['admin', 'transport', 'finance', 'leader', 'driver'];

export default function Members() {
  const { t } = useTranslation('admin');
  const { has, me } = useAuth();
  const [tab, setTab] = useState<'members' | 'invites' | 'recovery'>('members');
  if (!has('admin') || !me?.mfa_ok) return <Empty>{t('members.noPermission')}</Empty>;
  return (
    <div className="stack">
      <div className="page-head"><div><h1>{t('members.title')}</h1><p>{t('members.desc')}</p></div></div>
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
    </div>
  );
}

function MemberList() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const [q, setQ] = useState('');
  const { data, loading, reload } = useApi<User[]>(`/admin/users?q=${encodeURIComponent(q)}`);
  const [edit, setEdit] = useState<User | null>(null);
  const [approve, setApprove] = useState<User | null>(null);
  return (
    <div className="stack">
      <input className="input" placeholder={t('members.search')} value={q} onChange={(e) => setQ(e.target.value)} />
      <div className="card">
        {loading ? <Spinner /> : !data?.length ? <Empty>{t('members.none')}</Empty> : (
          <div className="table-wrap">
            <table className="table responsive">
              <thead><tr><th>{t('members.name')}</th><th>{t('members.email')}</th><th>{t('members.role')}</th><th>{t('members.group')}</th><th>{t('members.passkey')}</th><th></th></tr></thead>
              <tbody>
                {data.map((u) => (
                  <tr key={u.id}>
                    <td data-label={t('members.name')} className="bold">{u.name} {u.status === 'pending' && <Pill>{t('members.pending')}</Pill>}{u.status === 'disabled' && <Pill tone="red">{t('members.statusDisabled')}</Pill>}</td>
                    <td data-label={t('members.email')}>{u.email}</td>
                    <td data-label={t('members.role')}>{u.roles.map((r) => ft(ROLE_LABEL[r])).join('、') || t('members.member')}</td>
                    <td data-label={t('members.group')} className="small">{u.groups}</td>
                    <td data-label={t('members.passkey')}>{u.passkeys}</td>
                    <td>{u.status === 'pending'
                      ? <button className="btn sm primary" onClick={() => setApprove(u)}>{t('members.approveMember')}</button>
                      : <button className="btn sm" onClick={() => setEdit(u)}>{t('members.edit')}</button>}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
      {edit && <EditUser user={edit} onClose={() => { setEdit(null); reload(); }} />}
      {approve && <ApproveUser user={approve} onClose={() => { setApprove(null); reload(); }} />}
    </div>
  );
}

function EditUser({ user, onClose }: { user: User; onClose: () => void }) {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const { data: groups } = useApi<Group[]>('/groups');
  const [roles, setRoles] = useState<string[]>(user.roles);
  const initialGroups = user.groups ? user.groups.split('、') : [];
  const [gids, setGids] = useState<number[] | null>(null);
  const selectedG = gids ?? (groups?.filter((g) => initialGroups.includes(g.name)).map((g) => g.id) ?? []);
  const { busy, error, run } = useAction();
  const save = () => run(
    () => put(`/admin/users/${user.id}`, { roles, status: 'active', group_ids: gids }).then(onClose),
    t('members.memberUpdated'),
  );
  return (
    <Modal title={`${t('members.edit')} ${user.name}`} onClose={onClose}>
      <div className="stack-sm">
        <strong>{t('members.role')}</strong>
        <div className="chips">
          {ROLES.map((r) => (
            <button key={r} type="button" className={`chip ${roles.includes(r) ? 'on' : ''}`} onClick={() => setRoles(roles.includes(r) ? roles.filter((x) => x !== r) : [...roles, r])}>{ft(ROLE_LABEL[r])}</button>
          ))}
        </div>
        <strong>{t('members.group')}</strong>
        <div className="chips">
          {groups?.filter((g) => !g.is_all).map((g) => (
            <button key={g.id} type="button" className={`chip ${selectedG.includes(g.id) ? 'on' : ''}`} onClick={() => setGids(selectedG.includes(g.id) ? selectedG.filter((x) => x !== g.id) : [...selectedG, g.id])}>{g.name}</button>
          ))}
        </div>
        <ErrorBox error={error} />
        <button className="btn primary" disabled={busy} onClick={save}>{t('members.update')}</button>
      </div>
    </Modal>
  );
}

function ApproveUser({ user, onClose }: { user: User; onClose: () => void }) {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const { data: groups } = useApi<Group[]>('/groups');
  const [roles, setRoles] = useState<string[]>([]);
  const [gids, setGids] = useState<number[]>([]);
  const { busy, error, run } = useAction();
  const save = () => run(
    () => put(`/admin/users/${user.id}`, { roles, status: 'active', group_ids: gids }).then(onClose),
    t('members.memberApproved'),
  );
  return (
    <Modal title={`${t('members.approveMember')} ${user.name}`} onClose={onClose}>
      <div className="stack-sm">
        <p className="muted">{user.email}</p>
        <strong>{t('members.role')}</strong>
        <div className="chips">
          {ROLES.map((r) => (
            <button key={r} type="button" className={`chip ${roles.includes(r) ? 'on' : ''}`} onClick={() => setRoles(roles.includes(r) ? roles.filter((x) => x !== r) : [...roles, r])}>{ft(ROLE_LABEL[r])}</button>
          ))}
        </div>
        <strong>{t('members.group')}</strong>
        <div className="chips">
          {groups?.filter((g) => !g.is_all).map((g) => (
            <button key={g.id} type="button" className={`chip ${gids.includes(g.id) ? 'on' : ''}`} onClick={() => setGids(gids.includes(g.id) ? gids.filter((x) => x !== g.id) : [...gids, g.id])}>{g.name}</button>
          ))}
        </div>
        <ErrorBox error={error} />
        <button className="btn primary" disabled={busy} onClick={save}>{t('members.approveMember')}</button>
      </div>
    </Modal>
  );
}

function Invites() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const { data, reload } = useApi<any[]>('/admin/invites');
  const [link, setLink] = useState<string | null>(null);
  const { busy, error, run } = useAction();
  const copy = (s: string) => { navigator.clipboard?.writeText(s); };
  const submit = (e: FormEvent) => {
    e.preventDefault();
    run(async () => {
      const r = await post<{ url: string }>('/admin/invites', {});
      setLink(r.url);
      reload();
    }, t('members.inviteCreated'));
  };
  return (
    <div className="stack">
      <form className="card stack" onSubmit={submit}>
        <p className="muted">{t('members.inviteHint')}</p>
        <ErrorBox error={error} />
        <button className="btn primary" disabled={busy} type="submit">{t('members.createInvite')}</button>
        {link && (
          <div className="notice ok stack-sm">
            <span>{t('members.inviteLink')}</span>
            <code style={{ wordBreak: 'break-all' }}>{link}</code>
            <button type="button" className="btn sm" onClick={() => copy(link)}>{t('members.copy')}</button>
          </div>
        )}
      </form>
      <div className="card">
        <h3 style={{ marginBottom: 8 }}>{t('members.inviteLog')}</h3>
        <div className="list">
          {data?.map((i) => (
            <div key={i.id} className="item">
              <div className="grow">
                <div className="bold">{i.email || t('members.openInvite')} {i.kind === 'recovery' && <Pill tone="blue">{t('members.recovery')}</Pill>}</div>
                <div className="muted tiny">{i.roles.map((r: string) => ft(ROLE_LABEL[r])).join('、') || t('members.member')} · {fmtAgo(i.created_at, ft)}</div>
              </div>
              {i.status === 'open' ? (
                <>
                  <button className="btn sm" onClick={() => copy(i.url)}>{t('members.copy')}</button>
                  <button className="btn sm danger" onClick={() => run(() => del(`/admin/invites/${i.id}`).then(reload), t('members.revoked'))}>{t('members.revoke')}</button>
                </>
              ) : <Pill tone={i.status === 'used' ? 'green' : 'gray'}>{i.status === 'used' ? t('members.used') : t('members.expired')}</Pill>}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

function Recovery() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const { data, reload } = useApi<any[]>('/admin/recovery');
  const [link, setLink] = useState<string | null>(null);
  const { busy, error, run } = useAction();
  return (
    <div className="stack">
      <div className="notice info">{t('members.recoveryNotice')}</div>
      {link && <div className="notice ok" style={{ wordBreak: 'break-all' }}>{t('members.recoveryLink', { link })}</div>}
      <ErrorBox error={error} />
      <div className="card">
        {!data?.length ? <Empty>{t('members.noneRecovery')}</Empty> : (
          <div className="list">
            {data.map((r) => (
              <div key={r.id} className="item">
                <div className="grow">
                  <div className="bold">{r.name ?? t('members.nonMember')} · {r.email}</div>
                  <div className="small">{r.reason}</div>
                  <div className="muted tiny">{fmtAgo(r.created_at, ft)}</div>
                </div>
                {r.status === 'pending' || r.status === 'awaiting_second' ? (
                  <div className="btn-row">
                    {r.status === 'awaiting_second' && <span className="muted tiny">{t('members.secondAdminNotice')}</span>}
                    <button className="btn sm primary" disabled={busy || !r.name} onClick={() => run(async () => {
                      const x = await post<{ url?: string; awaiting_second?: boolean }>(`/admin/recovery/${r.id}/approve`);
                      if (x.url) setLink(x.url);
                      reload();
                    })}>{r.status === 'awaiting_second' ? t('members.confirmSecond') : t('members.approve')}</button>
                    <button className="btn sm" disabled={busy} onClick={() => run(() => post(`/admin/recovery/${r.id}/reject`).then(reload))}>{t('members.reject')}</button>
                  </div>
                ) : <Pill tone={r.status === 'approved' ? 'green' : 'gray'}>{r.status === 'approved' ? t('members.approved') : t('members.rejected')}</Pill>}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
