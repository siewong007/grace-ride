import { useState } from 'react';
import { del, post, put, useApi } from '../../api';
import { useAuth } from '../../auth';
import { Empty, ErrorBox, Field, Modal, Pill, useAction } from '../../components/ui';
import { useTranslation } from 'react-i18next';

interface Group { id: number; name: string; description: string; is_all: boolean; member_count: number }
interface Area { id: number; name: string; active: boolean }

export default function Settings() {
  const { t } = useTranslation('admin');
  const { has, me } = useAuth();
  if (!has('admin') || !me?.mfa_ok) return <Empty>{t('settings.noPermission')}</Empty>;
  return (
    <div className="stack">
      <div className="page-head"><div><h1>{t('settings.title')}</h1><p>{t('settings.desc')}</p></div></div>
      <div className="split">
        <Groups t={t} />
        <Areas t={t} />
      </div>
    </div>
  );
}

function Groups({ t }: { t: (k: string, o?: any) => string }) {
  const { data, reload } = useApi<Group[]>('/groups');
  const [name, setName] = useState('');
  const [members, setMembers] = useState<Group | null>(null);
  const { busy, error, run } = useAction();
  return (
    <div className="card stack">
      <h3>{t('settings.groups')}</h3>
      <div className="list">
        {data?.map((g) => (
          <div key={g.id} className="item">
            <div className="grow">
              <div className="bold">{g.name} {g.is_all && <Pill tone="blue">{t('settings.allMembers')}</Pill>}</div>
              <div className="muted tiny">{g.is_all ? t('settings.allDesc') : t('settings.memberCount', { count: g.member_count })}{g.description && ` · ${g.description}`}</div>
            </div>
            {!g.is_all && <>
              <button className="btn sm" onClick={() => setMembers(g)}>{t('settings.membersBtn')}</button>
              <button className="btn sm danger" onClick={() => confirm(t('settings.deleteGroupConfirm', { name: g.name })) && run(() => del(`/admin/groups/${g.id}`).then(reload), t('settings.deleted'))}>{t('settings.delete')}</button>
            </>}
          </div>
        ))}
      </div>
      <form className="row nowrap" onSubmit={(e) => { e.preventDefault(); run(() => post('/admin/groups', { name }).then(() => { setName(''); reload(); }), t('settings.added')); }}>
        <input className="input grow" placeholder={t('settings.groupNamePlaceholder')} value={name} onChange={(e) => setName(e.target.value)} required />
        <button className="btn primary" disabled={busy}>{t('settings.addGroup')}</button>
      </form>
      <ErrorBox error={error} />
      {members && <GroupMembers group={members} t={t} onClose={() => { setMembers(null); reload(); }} />}
    </div>
  );
}

function GroupMembers({ group, t, onClose }: { group: Group; t: (k: string, o?: any) => string; onClose: () => void }) {
  const { data: current } = useApi<{ id: number; name: string }[]>(`/admin/groups/${group.id}/members`);
  const { data: all } = useApi<{ id: number; name: string; status: string }[]>('/admin/users');
  const [sel, setSel] = useState<number[] | null>(null);
  const [q, setQ] = useState('');
  const chosen = sel ?? current?.map((c) => c.id) ?? [];
  const { busy, error, run } = useAction();
  return (
    <Modal title={`${t('settings.groupMembers', { name: group.name, count: chosen.length })}`} onClose={onClose}>
      <input className="input" placeholder={t('settings.search')} value={q} onChange={(e) => setQ(e.target.value)} />
      <div className="stack-sm" style={{ maxHeight: 360, overflowY: 'auto' }}>
        {all?.filter((u) => u.status === 'active' && u.name.includes(q)).map((u) => (
          <label key={u.id} className={`check ${chosen.includes(u.id) ? 'on' : ''}`}>
            <input type="checkbox" checked={chosen.includes(u.id)} onChange={() => setSel(chosen.includes(u.id) ? chosen.filter((x) => x !== u.id) : [...chosen, u.id])} />
            {u.name}
          </label>
        ))}
      </div>
      <ErrorBox error={error} />
      <button className="btn primary" disabled={busy} onClick={() => run(() => put(`/admin/groups/${group.id}/members`, { user_ids: chosen }).then(onClose), t('settings.memberUpdated'))}>{t('settings.save')}</button>
    </Modal>
  );
}

function Areas({ t }: { t: (k: string, o?: any) => string }) {
  const { data, reload } = useApi<Area[]>('/areas');
  const [name, setName] = useState('');
  const [edit, setEdit] = useState<Area | null>(null);
  const { busy, error, run } = useAction();
  return (
    <div className="card stack">
      <h3>{t('settings.areas')}</h3>
      <div className="chips">
        {data?.map((a) => (
          <button key={a.id} className={`chip ${a.active ? 'on' : ''}`} onClick={() => setEdit(a)}>{a.name}{!a.active && `（${t('settings.inactive')}）`}</button>
        ))}
      </div>
      <form className="row nowrap" onSubmit={(e) => { e.preventDefault(); run(() => post('/admin/areas', { name }).then(() => { setName(''); reload(); }), t('settings.added')); }}>
        <input className="input grow" placeholder={t('settings.areaPlaceholder')} value={name} onChange={(e) => setName(e.target.value)} required />
        <button className="btn primary" disabled={busy}>{t('settings.addArea')}</button>
      </form>
      <ErrorBox error={error} />
      {edit && (
        <Modal title={t('settings.editArea')} onClose={() => setEdit(null)}>
          <Field label={t('settings.areaName')}>
            <input className="input" value={edit.name} onChange={(e) => setEdit({ ...edit, name: e.target.value })} />
          </Field>
          <label className={`check ${edit.active ? 'on' : ''}`}><input type="checkbox" checked={edit.active} onChange={(e) => setEdit({ ...edit, active: e.target.checked })} />{t('settings.openSignup')}</label>
          <button className="btn primary" onClick={() => run(() => put(`/admin/areas/${edit.id}`, edit).then(() => { setEdit(null); reload(); }), t('settings.saved'))}>{t('settings.saveArea')}</button>
        </Modal>
      )}
    </div>
  );
}
