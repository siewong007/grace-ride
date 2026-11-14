import { Fragment, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Plus, ShieldCheck, Trash2 } from 'lucide-react';
import { del, post, put, useApi } from '../../api';
import { useAuth } from '../../auth';
import {
  Button,
  Card,
  CardSkeleton,
  ErrorBox,
  Field,
  Notice,
  Page,
  PageHeader,
  Pill,
  useAction,
  useConfirm,
} from '../../components/ui';
import { roleLabel, useFormatT } from '../../format';
import { ACTIONS, AREAS, type Act } from '../../perms';
import type { CatalogPage, RoleDef, RolesResponse } from './roleData';

/** Stands in for the role being created until the server gives it a key. */
const NEW = '__new__';

const BLANK: RoleDef = {
  key: NEW,
  name: '',
  description: '',
  system: false,
  implicit: false,
  permissions: [],
  locked: [],
  members: 0,
};

/** Roles & permissions: who may view, create, update or delete on each page. */
export default function Roles() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const { can, refresh } = useAuth();
  const { data, loading, error, reload } = useApi<RolesResponse>('/admin/roles');
  const [picked, setPicked] = useState<string | null>(null);

  const roles = data?.roles ?? [];
  const current = picked === NEW ? BLANK : roles.find((r) => r.key === picked) ?? roles[0];

  // Editing a role can change what the signed-in member may do, so reload both.
  const afterChange = async (key: string | null) => {
    reload();
    await refresh();
    setPicked(key);
  };

  return (
    <Page>
      <PageHeader
        title={t('roles.title')}
        subtitle={t('roles.desc')}
        actions={
          can('admin_roles', 'create') ? (
            <Button variant="primary" icon={<Plus size={18} />} onClick={() => setPicked(NEW)}>
              {t('roles.newRole')}
            </Button>
          ) : undefined
        }
      />
      <ErrorBox error={error} />
      {loading || !data || !current ? (
        <CardSkeleton count={2} />
      ) : (
        <div className="roles-layout">
          <Card className="stack-sm">
            <h3>{t('roles.listTitle')}</h3>
            <div className="list">
              {roles.map((r) => (
                <button
                  key={r.key}
                  type="button"
                  aria-current={r.key === current.key}
                  className={`item role-item ${r.key === current.key ? 'on' : ''}`}
                  onClick={() => setPicked(r.key)}
                >
                  <span className="grow">
                    <span className="bold">{roleLabel(r.key, ft, { [r.key]: r.name })}</span>
                    <span className="muted tiny role-item-sub">
                      {r.implicit ? t('roles.everyone') : t('roles.memberCount', { count: r.members })}
                    </span>
                  </span>
                  {r.system && <Pill tone="gray">{t('roles.builtIn')}</Pill>}
                </button>
              ))}
            </div>
          </Card>
          <RoleEditor key={current.key} role={current} catalog={data.catalog} onChanged={afterChange} />
        </div>
      )}
    </Page>
  );
}

function RoleEditor({
  role,
  catalog,
  onChanged,
}: {
  role: RoleDef;
  catalog: CatalogPage[];
  onChanged: (key: string | null) => void;
}) {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const { can } = useAuth();
  const confirm = useConfirm();
  const { busy, error, run } = useAction();
  const isNew = role.key === NEW;
  // The super admin role is fixed: it always holds everything, so there is nothing to edit.
  const editable = !role.fixed && (isNew ? can('admin_roles', 'create') : can('admin_roles', 'update'));

  const [name, setName] = useState(role.name);
  const [description, setDescription] = useState(role.description);
  const [granted, setGranted] = useState(() => new Set(role.permissions));

  const toggle = (key: string) =>
    setGranted((prev) => {
      const next = new Set(prev);
      if (!next.delete(key)) next.add(key);
      return next;
    });

  /** Clicking a page name ticks every action on it, or clears them if all are ticked. */
  const toggleRow = (p: CatalogPage) =>
    setGranted((prev) => {
      const keys = p.actions.map((a) => `${p.page}.${a}`).filter((k) => !role.locked.includes(k));
      const all = keys.every((k) => prev.has(k));
      const next = new Set(prev);
      for (const k of keys) {
        if (all) next.delete(k);
        else next.add(k);
      }
      return next;
    });

  const save = () =>
    run(async () => {
      const body = { name: name.trim(), description: description.trim(), permissions: [...granted] };
      if (isNew) {
        const made = await post<{ key: string }>('/admin/roles', body);
        onChanged(made.key);
      } else {
        await put(`/admin/roles/${role.key}`, body);
        onChanged(role.key);
      }
    }, isNew ? t('roles.created') : t('roles.saved'));

  const remove = async () => {
    const ok = await confirm({
      title: t('roles.delete'),
      body: t('roles.deleteConfirm', { name: role.name, count: role.members }),
      confirmLabel: t('roles.delete'),
      tone: 'danger',
    });
    if (ok) run(() => del(`/admin/roles/${role.key}`).then(() => onChanged(null)), t('roles.deleted'));
  };

  const title = isNew ? t('roles.newRole') : roleLabel(role.key, ft, { [role.key]: role.name });

  return (
    <Card className="stack">
      <div className="row between nowrap">
        <h3>{title}</h3>
        {role.system && <Pill tone="gray">{t('roles.builtIn')}</Pill>}
      </div>

      {role.implicit && <Notice tone="info">{t('roles.everyoneHint')}</Notice>}
      {role.fixed && <Notice tone="info" icon={<ShieldCheck size={18} />}>{t('roles.fixedHint')}</Notice>}
      {!editable && !role.fixed && <Notice tone="info">{t('roles.readOnly')}</Notice>}

      {!role.system && (
        <div className="form-grid two">
          <Field label={t('roles.name')}>
            <input
              className="input"
              value={name}
              maxLength={40}
              disabled={!editable}
              placeholder={t('roles.namePlaceholder')}
              onChange={(e) => setName(e.target.value)}
            />
          </Field>
          <Field label={t('roles.description')}>
            <input
              className="input"
              value={description}
              maxLength={200}
              disabled={!editable}
              onChange={(e) => setDescription(e.target.value)}
            />
          </Field>
        </div>
      )}

      <div className="table-wrap">
        <table className="table perm-matrix">
          <thead>
            <tr>
              <th scope="col">{t('roles.page')}</th>
              {ACTIONS.map((a) => (
                <th key={a} scope="col">{t(`roles.act.${a}`)}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {AREAS.map((area) => (
              <Fragment key={area}>
                <tr className="perm-area">
                  <th colSpan={ACTIONS.length + 1} scope="colgroup">
                    <span>{t(`roles.area.${area}`)}</span>
                    <span className="muted tiny perm-area-hint">{t(`roles.areaHint.${area}`)}</span>
                  </th>
                </tr>
                {catalog
                  .filter((p) => p.area === area)
                  .map((p) => {
                    const pageName = t(`roles.pageName.${p.page}`);
                    return (
                      <tr key={p.page}>
                        <th scope="row" className="perm-page">
                          <button type="button" className="perm-page-btn" disabled={!editable} onClick={() => toggleRow(p)}>
                            {pageName}
                          </button>
                          <div className="muted tiny">{t(`roles.pageHint.${p.page}`)}</div>
                        </th>
                        {ACTIONS.map((a: Act) => {
                          const key = `${p.page}.${a}`;
                          const locked = role.locked.includes(key);
                          return (
                            <td key={a}>
                              {p.actions.includes(a) ? (
                                <input
                                  type="checkbox"
                                  aria-label={`${pageName} · ${t(`roles.act.${a}`)}`}
                                  checked={granted.has(key)}
                                  disabled={!editable || locked}
                                  onChange={() => toggle(key)}
                                />
                              ) : (
                                <span className="muted" aria-hidden="true">—</span>
                              )}
                            </td>
                          );
                        })}
                      </tr>
                    );
                  })}
              </Fragment>
            ))}
          </tbody>
        </table>
      </div>

      <ErrorBox error={error} />

      {editable && (
        <div className="btn-row">
          <Button variant="primary" disabled={busy || (!role.system && !name.trim())} onClick={save}>
            {isNew ? t('roles.create') : t('roles.save')}
          </Button>
          {!isNew && !role.system && can('admin_roles', 'delete') && (
            <Button variant="danger" disabled={busy} icon={<Trash2 size={16} />} onClick={remove}>
              {t('roles.delete')}
            </Button>
          )}
        </div>
      )}
    </Card>
  );
}
