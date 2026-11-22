import { FormEvent, useEffect, useRef, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import {
  Bell,
  Check,
  Copy,
  ExternalLink,
  Send,
} from 'lucide-react';
import { del, post, put, useApi } from '../../api';
import { useAuth } from '../../auth';
import { ordinary, treeOrder, withDescendants } from '../../groupTree';
import {
  Button,
  Card,
  CardSkeleton,
  EmptyState,
  ErrorBox,
  Field,
  Modal,
  NoAccess,
  Page,
  PageHeader,
  Pill,
  Tabs,
  useAction,
  useConfirm,
  useToast,
} from '../../components/ui';
import { ADMIN_HOME } from '../../staffNav';
import AreaFares, { fareCoverage, type FaresResponse } from './AreaFares';
import AreaPhotos from './AreaPhotos';

interface Group {
  id: number;
  name: string;
  description: string;
  is_all: boolean;
  member_count: number;
  parent_id: number | null;
  is_visitor: boolean;
}

/** Groups `group` may be part of: ordinary groups outside its own branch. */
function parentChoices(all: Group[], group?: Group): Group[] {
  const own = group ? withDescendants(all, group.id) : new Set<number>();
  return ordinary(all).filter((g) => !g.is_all && !own.has(g.id));
}

interface Area {
  id: number;
  name: string;
  active: boolean;
}


interface SystemSettingsResponse {
  audit_retention_days: number;
  chat_retention_days: number;
  max_live_drivers: number;
  session_timeout_mins: number;
}

interface AlertSettingsData {
  telegram_bot_configured?: boolean;
  telegram_bot_username?: string;
  alert_webhook_url?: string;
  alert_webhook_configured?: boolean;
  vapid_public_key?: string;
  vapid_private_configured?: boolean;
  push_subscriptions_count?: number;
}

type TabKey = 'fares' | 'photos' | 'rates' | 'org' | 'alerts';

export default function Settings() {
  const { t } = useTranslation('admin');
  const { can } = useAuth();
  const [params, setParams] = useSearchParams();

  // Groups, areas, and rates settings are separate permissions; each section shows for whoever holds it.
  const groups = can('admin_groups');
  const areas = can('admin_areas');
  const rates = can('admin_settings');
  const fares = can('admin_fares');
  const photos = can('admin_area_photos');

  if (!groups && !areas && !rates && !fares && !photos) {
    return (
      <Page>
        <NoAccess />
      </Page>
    );
  }

  const availableTabs: { value: TabKey; label: string }[] = [];
  if (fares) {
    availableTabs.push({ value: 'fares', label: t('settings.tabFares') });
  }
  if (photos) {
    availableTabs.push({ value: 'photos', label: t('settings.tabAreaPhotos') });
  }
  if (rates) {
    availableTabs.push({ value: 'rates', label: t('settings.tabRatesLimits') });
  }
  if (groups || areas) {
    availableTabs.push({ value: 'org', label: t('settings.tabGroupsAreas') });
  }
  if (rates) {
    availableTabs.push({ value: 'alerts', label: t('settings.tabAlerts') });
  }

  const tabParam = params.get('tab') as TabKey;
  const activeTab = availableTabs.some((t) => t.value === tabParam)
    ? tabParam
    : availableTabs[0]?.value ?? 'rates';

  const handleTabChange = (next: string) => {
    const nextParams = new URLSearchParams(params);
    if (next === availableTabs[0]?.value) {
      nextParams.delete('tab');
    } else {
      nextParams.set('tab', next);
    }
    setParams(nextParams, { replace: true });
  };

  return (
    <Page>
      <PageHeader title={t('settings.title')} subtitle={t('settings.desc')} back={ADMIN_HOME} />

      <div className="stack gap-lg">
        <SettingsSummaryStrip
          rates={rates}
          areas={areas}
          fares={fares}
          onSelectTab={(k) => handleTabChange(k)}
        />

        {availableTabs.length > 1 && (
          <Tabs
            value={activeTab}
            onChange={handleTabChange}
            items={availableTabs}
            ariaLabel={t('settings.title')}
          />
        )}

        {fares && (
          <div
            role="tabpanel"
            id="panel-fares"
            aria-labelledby="tab-fares"
            className="tabpanel stack gap-lg"
            hidden={activeTab !== 'fares'}
          >
            <AreaFares />
          </div>
        )}

        {photos && (
          <div
            role="tabpanel"
            id="panel-photos"
            aria-labelledby="tab-photos"
            className="tabpanel stack gap-lg"
            hidden={activeTab !== 'photos'}
          >
            {activeTab === 'photos' && <AreaPhotos />}
          </div>
        )}

        {rates && (
          <div
            role="tabpanel"
            id="panel-rates"
            aria-labelledby="tab-rates"
            className="tabpanel stack gap-lg"
            hidden={activeTab !== 'rates'}
          >
            <SystemRetentionSettings />
          </div>
        )}

        {(groups || areas) && (
          <div
            role="tabpanel"
            id="panel-org"
            aria-labelledby="tab-org"
            className="tabpanel stack gap-lg"
            hidden={activeTab !== 'org'}
          >
            <div className="split">
              {groups && <Groups />}
              {areas && <Areas />}
            </div>
          </div>
        )}

        {rates && (
          <div
            role="tabpanel"
            id="panel-alerts"
            aria-labelledby="tab-alerts"
            className="tabpanel stack gap-lg"
            hidden={activeTab !== 'alerts'}
          >
            <AlertChannelSettings />
          </div>
        )}
      </div>
    </Page>
  );
}

function SettingsSummaryStrip({
  rates,
  areas,
  fares,
  onSelectTab,
}: {
  rates: boolean;
  areas: boolean;
  fares: boolean;
  onSelectTab: (tab: TabKey) => void;
}) {
  const { t } = useTranslation('admin');
  const areasData = useApi<Area[]>(areas || fares ? '/areas' : null).data;
  const faresData = useApi<FaresResponse>(fares ? '/admin/settings/fares' : null).data;
  const systemData = useApi<SystemSettingsResponse>(rates ? '/admin/settings/system' : null).data;
  const alertsData = useApi<AlertSettingsData>(rates ? '/admin/settings/alerts' : null).data;

  const totalAreas = areasData?.length ?? 0;
  const activeAreas = (areasData ?? []).filter((a) => a.active).length;

  const coverage = fareCoverage(areasData ?? [], faresData?.fares ?? []);

  const auditDays = systemData?.audit_retention_days;
  const sessionTimeout = systemData?.session_timeout_mins;

  const cfg = alertsData && !Array.isArray(alertsData) ? alertsData : null;
  const channelsCount = cfg
    ? (cfg.vapid_private_configured ? 1 : 0) +
      (cfg.telegram_bot_configured ? 1 : 0) +
      (cfg.alert_webhook_configured || Boolean(cfg.alert_webhook_url) ? 1 : 0)
    : 0;

  return (
    <div className="stat-strip" aria-label="Configuration overview">
      {areas && (
        <button
          type="button"
          className="card stat link text-left"
          onClick={() => onSelectTab('org')}
        >
          <div className="label">{t('settings.statAreasOpen')}</div>
          <div className="num">
            {activeAreas}/{totalAreas}
          </div>
          <div className="muted tiny">{t('settings.statActiveCount', { count: activeAreas })}</div>
        </button>
      )}

      {fares && (
        <button
          type="button"
          className="card stat link text-left"
          onClick={() => onSelectTab('fares')}
        >
          <div className="label">{t('settings.statFares')}</div>
          <div className="num">
            {coverage.set}/{coverage.total}
          </div>
          <div className="muted tiny">{t('settings.statFaresHint')}</div>
        </button>
      )}

      {rates && (
        <>
          <button
            type="button"
            className="card stat link text-left"
            onClick={() => onSelectTab('rates')}
          >
            <div className="label">{t('settings.statAuditChat')}</div>
            <div className="num">{auditDays != null ? `${auditDays}d` : '—'}</div>
            <div className="muted tiny">
              {sessionTimeout != null ? `${sessionTimeout}m auto-lock` : ''}
            </div>
          </button>

          <button
            type="button"
            className="card stat link text-left"
            onClick={() => onSelectTab('alerts')}
          >
            <div className="label">{t('settings.statAlertChannels')}</div>
            <div className="num">{channelsCount}/3</div>
            <div className="muted tiny">
              {t('settings.statReadyCount', { count: channelsCount })}
            </div>
          </button>
        </>
      )}
    </div>
  );
}

function Groups() {
  const { t } = useTranslation('admin');
  const confirm = useConfirm();
  const { can } = useAuth();
  const { data, loading, reload } = useApi<Group[]>('/groups');
  const [name, setName] = useState('');
  const [parent, setParent] = useState('');
  const [members, setMembers] = useState<Group | null>(null);
  const [editing, setEditing] = useState<Group | null>(null);
  const byId = new Map((data ?? []).map((g) => [g.id, g]));
  const { busy, error, run } = useAction();

  const removeGroup = async (g: Group) => {
    const ok = await confirm({
      title: t('settings.delete'),
      body: t('members.deleteGroupConfirm', { name: g.name }),
      confirmLabel: t('settings.delete'),
      tone: 'danger',
    });
    if (ok) {
      run(() => del(`/admin/groups/${g.id}`).then(reload), t('members.deleted'));
    }
  };

  return (
    <Card className="stack">
      <div className="row between wrap gap-xs">
        <h3>{t('settings.groups')}</h3>
        {data && (
          <span className="muted tiny">
            {t('settings.memberCount', { count: data.length })}
          </span>
        )}
      </div>

      {loading ? (
        <CardSkeleton count={2} />
      ) : !data?.length ? (
        <EmptyState title={t('settings.emptyGroups')} />
      ) : (
        <div className="list">
          {treeOrder(data).map(({ group: g, depth }) => (
            <div
              key={g.id}
              className="item group-row"
              style={{ paddingInlineStart: depth ? `${depth * 1.25}rem` : undefined }}
            >
              <div className="grow">
                <div className="bold">
                  {depth > 0 && (
                    <span className="muted" aria-hidden="true">
                      ↳{' '}
                    </span>
                  )}
                  {g.name}{' '}
                  {g.is_all && <Pill tone="blue">{t('settings.allMembers')}</Pill>}
                  {g.is_visitor && <Pill tone="neutral">{t('settings.visitorPill')}</Pill>}
                </div>
                <div className="muted tiny">
                  {g.is_all
                    ? t('settings.allDesc')
                    : g.is_visitor
                      ? t('settings.visitorDesc')
                      : t('settings.memberCount', { count: g.member_count })}
                  {g.parent_id != null &&
                    byId.get(g.parent_id) &&
                    ` · ${t('settings.partOfName', { name: byId.get(g.parent_id)!.name })}`}
                  {g.description && !g.is_visitor && ` · ${g.description}`}
                </div>
              </div>
              {!g.is_all && !g.is_visitor && (
                <div className="group-actions">
                  {can('admin_groups', 'update') && (
                    <Button variant="ghost" size="sm" onClick={() => setEditing(g)}>
                      {t('settings.edit')}
                    </Button>
                  )}
                  {can('admin_groups', 'update') && (
                    <Button variant="secondary" size="sm" onClick={() => setMembers(g)}>
                      {t('settings.membersBtn')}
                    </Button>
                  )}
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {can('admin_groups', 'create') && (
        <form
          className="row group-add"
          onSubmit={(e) => {
            e.preventDefault();
            run(
              () =>
                post('/admin/groups', { name, parent_id: parent ? Number(parent) : null }).then(
                  () => {
                    setName('');
                    setParent('');
                    reload();
                  },
                ),
              t('settings.added'),
            );
          }}
        >
          <input
            className="input grow"
            placeholder={t('settings.groupNamePlaceholder')}
            aria-label={t('settings.groupName')}
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
          />
          <select
            className="input"
            value={parent}
            onChange={(e) => setParent(e.target.value)}
            aria-label={t('settings.partOf')}
          >
            <option value="">{t('settings.topLevel')}</option>
            {parentChoices(data ?? []).map((g) => (
              <option key={g.id} value={g.id}>
                {t('settings.partOfName', { name: g.name })}
              </option>
            ))}
          </select>
          <Button variant="primary" disabled={busy} type="submit">
            {t('settings.addGroup')}
          </Button>
        </form>
      )}

      <ErrorBox error={error} />
      {editing && (
        <GroupEdit
          group={editing}
          choices={parentChoices(data ?? [], editing)}
          onDelete={
            can('admin_groups', 'delete')
              ? () => {
                  const g = editing;
                  setEditing(null);
                  removeGroup(g);
                }
              : undefined
          }
          onClose={() => setEditing(null)}
          onSaved={() => {
            setEditing(null);
            reload();
          }}
        />
      )}
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

function GroupEdit({
  group,
  choices,
  onClose,
  onSaved,
  onDelete,
}: {
  group: Group;
  choices: Group[];
  onClose: () => void;
  onSaved: () => void;
  /** Deleting sits here, apart from the everyday buttons, and only for who may delete. */
  onDelete?: () => void;
}) {
  const { t } = useTranslation('admin');
  const [name, setName] = useState(group.name);
  const [description, setDescription] = useState(group.description);
  const [parent, setParent] = useState(group.parent_id != null ? String(group.parent_id) : '');
  const { busy, error, run } = useAction();
  return (
    <Modal title={t('settings.editGroup')} onClose={onClose}>
      <form
        className="stack"
        onSubmit={(e) => {
          e.preventDefault();
          run(
            () =>
              put(`/admin/groups/${group.id}`, {
                name,
                description,
                parent_id: parent ? Number(parent) : null,
              }).then(onSaved),
            t('settings.saved'),
          );
        }}
      >
        <Field label={t('settings.groupName')}>
          <input
            className="input"
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
          />
        </Field>
        <Field label={t('settings.groupDescription')}>
          <input
            className="input"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
          />
        </Field>
        <Field label={t('settings.partOf')} hint={t('settings.partOfHint')}>
          <select className="input" value={parent} onChange={(e) => setParent(e.target.value)}>
            <option value="">{t('settings.topLevel')}</option>
            {choices.map((g) => (
              <option key={g.id} value={g.id}>
                {g.name}
              </option>
            ))}
          </select>
        </Field>
        <ErrorBox error={error} />
        <Button variant="primary" type="submit" disabled={busy}>
          {t('settings.save')}
        </Button>
        {onDelete && (
          <div className="stack-xs">
            <Button variant="danger" type="button" onClick={onDelete}>
              {t('settings.delete')}
            </Button>
            <p className="muted tiny">{t('settings.deleteGroupHint')}</p>
          </div>
        )}
      </form>
    </Modal>
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
  const { data: all } = useApi<{ id: number; name: string; status?: string }[]>('/admin/users');
  const [sel, setSel] = useState<number[] | null>(null);
  const [q, setQ] = useState('');
  const chosen = sel ?? current?.map((c) => c.id) ?? [];
  const { busy, error, run } = useAction();

  const toggleUser = (id: number) => {
    setSel(chosen.includes(id) ? chosen.filter((x) => x !== id) : [...chosen, id]);
  };

  const save = () =>
    run(
      () => put(`/admin/groups/${group.id}/members`, { user_ids: chosen }).then(onClose),
      t('members.memberUpdated'),
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
        type="search"
        aria-label={t('settings.search')}
        placeholder={t('settings.search')}
        value={q}
        onChange={(e) => setQ(e.target.value)}
      />
      <div className="stack-sm group-members-list">
        {all
          ?.filter(
            (u) =>
              (u.status == null || u.status === 'active') &&
              u.name.toLowerCase().includes(q.trim().toLowerCase()),
          )
          .map((u) => (
            <label key={u.id} className={`check ${chosen.includes(u.id) ? 'on' : ''}`}>
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
      <Button variant="primary" block disabled={busy || !all || !current} onClick={save}>
        {t('settings.save')}
      </Button>
    </Modal>
  );
}

export function Areas() {
  const { t } = useTranslation('admin');
  const { can } = useAuth();
  const { data, loading, reload } = useApi<Area[]>('/areas');
  const [name, setName] = useState('');
  const [edit, setEdit] = useState<Area | null>(null);
  const { busy, error, run } = useAction();

  const openCount = (data ?? []).filter((a) => a.active).length;

  return (
    <Card className="stack">
      <div className="row between wrap gap-xs">
        <h3>{t('settings.areas')}</h3>
        {data && (
          <span className="muted tiny">
            {t('settings.statActiveCount', { count: openCount })}
          </span>
        )}
      </div>

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
              disabled={!can('admin_areas', 'update')}
              onClick={() => setEdit(a)}
            >
              <span>{a.name}</span>
              {!a.active && <span className="muted tiny"> ({t('settings.inactive')})</span>}
            </button>
          ))}
        </div>
      )}

      {can('admin_areas', 'create') && (
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
      )}

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
          <ErrorBox error={error} />
          <Button
            variant="primary"
            block
            disabled={busy || !edit.name.trim()}
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

function SystemRetentionSettings() {
  const { t } = useTranslation('admin');
  const { can } = useAuth();
  const canUpdate = can('admin_settings', 'update');
  const { data, loading, reload } = useApi<SystemSettingsResponse>('/admin/settings/system');
  const [auditDays, setAuditDays] = useState<string>('');
  const [chatDays, setChatDays] = useState<string>('');
  const [maxDrivers, setMaxDrivers] = useState<string>('');
  const [sessionTimeout, setSessionTimeout] = useState<string>('');
  const [initialized, setInitialized] = useState(false);
  const initialRef = useRef({ audit: '', chat: '', drivers: '', timeout: '' });
  const { busy, error, run } = useAction();

  useEffect(() => {
    if (data && !initialized) {
      const a = String(data.audit_retention_days);
      const c = String(data.chat_retention_days);
      const d = String(data.max_live_drivers);
      const s = String(data.session_timeout_mins ?? 15);
      setAuditDays(a);
      setChatDays(c);
      setMaxDrivers(d);
      setSessionTimeout(s);
      initialRef.current = { audit: a, chat: c, drivers: d, timeout: s };
      setInitialized(true);
    }
  }, [data, initialized]);

  const isDirty =
    initialized &&
    (auditDays !== initialRef.current.audit ||
      chatDays !== initialRef.current.chat ||
      maxDrivers !== initialRef.current.drivers ||
      sessionTimeout !== initialRef.current.timeout);

  // Out-of-range numbers are explained, not silently changed to something else on save.
  const limits = {
    audit: { min: 30, max: 3650 },
    chat: { min: 7, max: 3650 },
    drivers: { min: 1, max: 100 },
    timeout: { min: 0, max: 1440 },
  };
  const check = (value: string, { min, max }: { min: number; max: number }) => {
    const n = Number(value);
    return value.trim() !== '' && Number.isInteger(n) && n >= min && n <= max
      ? undefined
      : t('settings.rangeError', { min, max });
  };
  const errors = {
    audit: check(auditDays, limits.audit),
    chat: check(chatDays, limits.chat),
    drivers: check(maxDrivers, limits.drivers),
    timeout: check(sessionTimeout, limits.timeout),
  };
  const systemValid = !Object.values(errors).some(Boolean);

  const save = (e: FormEvent) => {
    e.preventDefault();
    if (!systemValid) return;
    run(
      () =>
        put('/admin/settings/system', {
          audit_retention_days: Number(auditDays),
          chat_retention_days: Number(chatDays),
          max_live_drivers: Number(maxDrivers),
          session_timeout_mins: Number(sessionTimeout),
        }).then(() => {
          initialRef.current = {
            audit: auditDays,
            chat: chatDays,
            drivers: maxDrivers,
            timeout: sessionTimeout,
          };
          reload();
        }),
      t('settings.systemSaved'),
    );
  };

  const auditPresets = [
    { label: t('settings.daysPreset90'), val: 90 },
    { label: t('settings.daysPreset1y'), val: 365 },
    { label: t('settings.daysPreset2y'), val: 730 },
    { label: t('settings.daysPreset5y'), val: 1825 },
  ];
  const chatPresets = [
    { label: t('settings.daysPreset30'), val: 30 },
    { label: t('settings.daysPreset90'), val: 90 },
    { label: t('settings.daysPreset180'), val: 180 },
    { label: t('settings.daysPreset1y'), val: 365 },
  ];
  const timeoutPresets = [
    { label: t('settings.minsDisabled'), val: 0 },
    { label: t('settings.mins15'), val: 15 },
    { label: t('settings.mins30'), val: 30 },
    { label: t('settings.mins60'), val: 60 },
    { label: t('settings.mins240'), val: 240 },
  ];

  return (
    <Card className="stack">
      <div>
        <h3>{t('settings.systemTitle')}</h3>
        <p className="muted small">{t('settings.systemDesc')}</p>
      </div>

      {loading && !initialized ? (
        <CardSkeleton count={2} />
      ) : (
        <form onSubmit={save} className="stack">
          <div className="form-grid two">
            <Field
              label={t('settings.auditRetention')}
              hint={t('settings.auditRetentionHint')}
              error={errors.audit}
            >
              <input
                aria-label={t('settings.auditRetention')}
                className="input"
                type="number"
                min="30"
                max="3650"
                step="1"
                value={auditDays}
                disabled={!canUpdate || busy}
                onChange={(e) => setAuditDays(e.target.value)}
                required
              />
              <div className="preset-chips">
                {auditPresets.map((p) => (
                  <button
                    key={p.val}
                    type="button"
                    className={`preset-chip ${Number(auditDays) === p.val ? 'on' : ''}`}
                    disabled={!canUpdate || busy}
                    onClick={() => setAuditDays(String(p.val))}
                  >
                    {p.label}
                  </button>
                ))}
              </div>
            </Field>

            <Field
              label={t('settings.chatRetention')}
              hint={t('settings.chatRetentionHint')}
              error={errors.chat}
            >
              <input
                aria-label={t('settings.chatRetention')}
                className="input"
                type="number"
                min="7"
                max="3650"
                step="1"
                value={chatDays}
                disabled={!canUpdate || busy}
                onChange={(e) => setChatDays(e.target.value)}
                required
              />
              <div className="preset-chips">
                {chatPresets.map((p) => (
                  <button
                    key={p.val}
                    type="button"
                    className={`preset-chip ${Number(chatDays) === p.val ? 'on' : ''}`}
                    disabled={!canUpdate || busy}
                    onClick={() => setChatDays(String(p.val))}
                  >
                    {p.label}
                  </button>
                ))}
              </div>
            </Field>

            <Field
              label={t('settings.maxLiveDrivers')}
              hint={t('settings.maxLiveDriversHint')}
              error={errors.drivers}
            >
              <div className="input-affix-wrap">
                <input
                  aria-label={t('settings.maxLiveDrivers')}
                  className="input"
                  type="number"
                  min="1"
                  max="100"
                  step="1"
                  value={maxDrivers}
                  disabled={!canUpdate || busy}
                  onChange={(e) => setMaxDrivers(e.target.value)}
                  required
                />
                <button
                  type="button"
                  className="input-affix-btn"
                  disabled={!canUpdate || busy || Number(maxDrivers) <= 1}
                  onClick={() => setMaxDrivers(String(Math.max(1, (Number(maxDrivers) || 10) - 1)))}
                  aria-label="Decrease max live drivers"
                >
                  −
                </button>
                <button
                  type="button"
                  className="input-affix-btn"
                  disabled={!canUpdate || busy || Number(maxDrivers) >= 100}
                  onClick={() =>
                    setMaxDrivers(String(Math.min(100, (Number(maxDrivers) || 10) + 1)))
                  }
                  aria-label="Increase max live drivers"
                >
                  +
                </button>
              </div>
            </Field>

            <Field
              label={t('settings.sessionTimeout')}
              hint={t('settings.sessionTimeoutHint')}
              error={errors.timeout}
            >
              <input
                aria-label={t('settings.sessionTimeout')}
                className="input"
                type="number"
                min="0"
                max="1440"
                step="1"
                value={sessionTimeout}
                disabled={!canUpdate || busy}
                onChange={(e) => setSessionTimeout(e.target.value)}
                required
              />
              <div className="preset-chips">
                {timeoutPresets.map((p) => (
                  <button
                    key={p.val}
                    type="button"
                    className={`preset-chip ${Number(sessionTimeout) === p.val ? 'on' : ''}`}
                    disabled={!canUpdate || busy}
                    onClick={() => setSessionTimeout(String(p.val))}
                  >
                    {p.label}
                  </button>
                ))}
              </div>
            </Field>
          </div>

          <ErrorBox error={error} />

          <div className="settings-submit-bar">
            <div className="dirty-indicator-box">
              {isDirty ? (
                <span className="dirty-indicator dirty">
                  <span className="dirty-dot" aria-hidden="true" />
                  <span>{t('settings.unsavedChanges')}</span>
                </span>
              ) : (
                <span className="dirty-indicator clean">
                  <Check size={16} aria-hidden="true" />
                  <span>{t('settings.allChangesSaved')}</span>
                </span>
              )}
            </div>

            {canUpdate && (
              <div className="settings-submit-actions">
                {isDirty && (
                  <Button
                    variant="ghost"
                    type="button"
                    disabled={busy}
                    onClick={() => {
                      setAuditDays(initialRef.current.audit);
                      setChatDays(initialRef.current.chat);
                      setMaxDrivers(initialRef.current.drivers);
                      setSessionTimeout(initialRef.current.timeout);
                    }}
                  >
                    {t('settings.reset')}
                  </Button>
                )}
                <Button
                  variant="primary"
                  type="submit"
                  disabled={busy || !systemValid || !isDirty}
                  loading={busy}
                >
                  {t('settings.saveSystem')}
                </Button>
              </div>
            )}
          </div>
        </form>
      )}
    </Card>
  );
}

function AlertChannelSettings() {
  const { t } = useTranslation('admin');
  const toast = useToast();
  const { can } = useAuth();
  const confirm = useConfirm();
  const canUpdate = can('admin_settings', 'update');
  const { data, loading, reload } = useApi<AlertSettingsData>('/admin/settings/alerts');
  const cfg = data && !Array.isArray(data) ? data : null;
  const [tgUser, setTgUser] = useState('');
  const [tgToken, setTgToken] = useState('');
  const [clearToken, setClearToken] = useState(false);
  const [webhookUrl, setWebhookUrl] = useState('');
  const [clearWebhook, setClearWebhook] = useState(false);
  const [vapidKey, setVapidKey] = useState('');
  const [vapidPrivateKey, setVapidPrivateKey] = useState('');
  const [initialized, setInitialized] = useState(false);
  const initialRef = useRef({ tgUser: '', webhookUrl: '', vapidKey: '' });
  const { busy, error, run } = useAction();

  useEffect(() => {
    if (cfg && !initialized) {
      const u = cfg.telegram_bot_username ?? '';
      const v = cfg.vapid_public_key ?? '';
      const w = cfg.alert_webhook_url ?? '';
      setTgUser(u);
      setVapidKey(v);
      setWebhookUrl(w);
      initialRef.current = { tgUser: u, webhookUrl: w, vapidKey: v };
      setInitialized(true);
    }
  }, [cfg, initialized]);

  const isDirty =
    initialized &&
    (tgUser !== initialRef.current.tgUser ||
      tgToken.trim() !== '' ||
      clearToken ||
      webhookUrl !== initialRef.current.webhookUrl ||
      clearWebhook ||
      vapidKey !== initialRef.current.vapidKey ||
      vapidPrivateKey.trim() !== '');

  const save = (e: FormEvent) => {
    e.preventDefault();
    run(async () => {
      await put('/admin/settings/alerts', {
        telegram_bot_username: tgUser.trim(),
        telegram_bot_token: tgToken.trim() || undefined,
        clear_telegram_bot_token: clearToken,
        alert_webhook_url: webhookUrl.trim() || undefined,
        clear_alert_webhook_url: clearWebhook,
        vapid_public_key: vapidKey.trim(),
        vapid_private_key: vapidPrivateKey.trim() || undefined,
      });
      setTgToken('');
      setVapidPrivateKey('');
      setClearToken(false);
      setWebhookUrl('');
      setClearWebhook(false);
      setInitialized(false);
      await reload();
    }, t('settings.alertsSaved'));
  };

  const copyVapid = async () => {
    if (!vapidKey) return;
    try {
      await navigator.clipboard.writeText(vapidKey);
      toast.success(t('settings.copied'));
    } catch {
      // Ignored if insecure context
    }
  };

  const rotateVapidKeys = async () => {
    const ok = await confirm({
      title: t('settings.rotateVapidConfirmTitle'),
      body: t('settings.rotateVapidConfirmBody'),
      confirmLabel: t('settings.rotateVapid'),
      tone: 'danger',
    });
    if (!ok) return;
    run(async () => {
      await put('/admin/settings/alerts', { rotate_vapid_keys: true });
      setVapidPrivateKey('');
      setInitialized(false);
      await reload();
    }, t('settings.vapidRotated'));
  };

  return (
    <Card className="stack">
      <div>
        <h3>{t('settings.alertsTitle')}</h3>
        <p className="muted small">{t('settings.alertsDesc')}</p>
      </div>

      {loading && !initialized ? (
        <CardSkeleton count={2} />
      ) : (
        <form onSubmit={save} className="stack">
          {/* Service 1: Web Push */}
          <div className="integration-card">
            <div className="integration-header">
              <div className="integration-title">
                <Bell size={20} className="text-[var(--primary)]" aria-hidden="true" />
                <div>
                  <div className="bold">{t('settings.webPushTitle')}</div>
                  <div className="muted small">{t('settings.webPushDesc')}</div>
                </div>
              </div>
              <Pill tone={cfg?.vapid_private_configured ? 'success' : 'neutral'}>
                {cfg?.vapid_private_configured
                  ? t('settings.pushConfiguredCount', {
                      count: cfg.push_subscriptions_count ?? 0,
                    })
                  : t('settings.pushNotConfigured')}
              </Pill>
            </div>

            <div className="form-grid two">
              <Field label={t('settings.vapidPublicKey')}>
                <div className="input-affix-wrap">
                  <input
                    aria-label={t('settings.vapidPublicKey')}
                    className="input mono"
                    disabled={!canUpdate || busy}
                    value={vapidKey}
                    onChange={(e) => setVapidKey(e.target.value)}
                  />
                  <button
                    type="button"
                    className="input-affix-btn"
                    disabled={!vapidKey}
                    onClick={copyVapid}
                    title={t('settings.copyVapid')}
                    aria-label={t('settings.copyVapid')}
                  >
                    <Copy size={16} />
                  </button>
                </div>
              </Field>

              <Field
                label={t('settings.vapidPrivateKey')}
                hint={t('settings.vapidPrivateKeyHint')}
              >
                <input
                  aria-label={t('settings.vapidPrivateKey')}
                  className="input mono"
                  type="password"
                  autoComplete="off"
                  disabled={!canUpdate || busy}
                  value={vapidPrivateKey}
                  onChange={(e) => setVapidPrivateKey(e.target.value)}
                  placeholder={cfg?.vapid_private_configured ? '••••••••' : ''}
                />
              </Field>
            </div>

            {canUpdate && (
              <div className="row wrap justify-end">
                <Button
                  variant="danger"
                  size="sm"
                  type="button"
                  disabled={busy}
                  onClick={rotateVapidKeys}
                >
                  {t('settings.rotateVapid')}
                </Button>
              </div>
            )}
          </div>

          {/* Service 2: Telegram Bot */}
          <div className="integration-card">
            <div className="integration-header">
              <div className="integration-title">
                <Send size={20} className="text-[var(--primary)]" aria-hidden="true" />
                <div>
                  <div className="bold">{t('settings.telegramTitle')}</div>
                  <div className="muted small">{t('settings.telegramDesc')}</div>
                </div>
              </div>
              <Pill tone={cfg?.telegram_bot_configured ? 'success' : 'neutral'}>
                {cfg?.telegram_bot_configured
                  ? t('settings.tgConfigured')
                  : t('settings.tgNotConfigured')}
              </Pill>
            </div>

            <div className="form-grid two">
              <Field label={t('settings.tgBotUsername')}>
                <input
                  aria-label={t('settings.tgBotUsername')}
                  className="input"
                  disabled={!canUpdate || busy}
                  value={tgUser}
                  onChange={(e) => setTgUser(e.target.value)}
                  placeholder="GraceRideBot"
                />
              </Field>

              <Field label={t('settings.tgBotToken')} hint={t('settings.tgBotTokenHint')}>
                <input
                  aria-label={t('settings.tgBotToken')}
                  className="input"
                  type="password"
                  autoComplete="off"
                  disabled={!canUpdate || busy || clearToken}
                  value={tgToken}
                  onChange={(e) => setTgToken(e.target.value)}
                  placeholder={cfg?.telegram_bot_configured ? '••••••••' : ''}
                />
              </Field>
            </div>

            {canUpdate && cfg?.telegram_bot_configured && (
              <label className="row gap-xs items-center small">
                <input
                  type="checkbox"
                  checked={clearToken}
                  onChange={(e) => setClearToken(e.target.checked)}
                />
                <span>{t('settings.tgClearToken')}</span>
              </label>
            )}
          </div>

          {/* Service 3: WhatsApp / Alert Webhook */}
          <div className="integration-card">
            <div className="integration-header">
              <div className="integration-title">
                <ExternalLink size={20} className="text-[var(--primary)]" aria-hidden="true" />
                <div>
                  <div className="bold">{t('settings.webhookTitle')}</div>
                  <div className="muted small">{t('settings.webhookDesc')}</div>
                </div>
              </div>
              <Pill
                tone={
                  cfg?.alert_webhook_configured || Boolean(cfg?.alert_webhook_url)
                    ? 'success'
                    : 'neutral'
                }
              >
                {cfg?.alert_webhook_configured || Boolean(cfg?.alert_webhook_url)
                  ? t('settings.tgConfigured')
                  : t('settings.tgNotConfigured')}
              </Pill>
            </div>

            <Field label={t('settings.webhookUrl')} hint={t('settings.webhookUrlHint')}>
              <input
                aria-label={t('settings.webhookUrl')}
                className="input"
                disabled={!canUpdate || busy || clearWebhook}
                value={webhookUrl}
                onChange={(e) => setWebhookUrl(e.target.value)}
                placeholder={cfg?.alert_webhook_url || 'https://example.org/hooks/grace-ride'}
              />
            </Field>

            {canUpdate && (cfg?.alert_webhook_configured || Boolean(cfg?.alert_webhook_url)) && (
              <label className="row gap-xs items-center small">
                <input
                  type="checkbox"
                  checked={clearWebhook}
                  onChange={(e) => setClearWebhook(e.target.checked)}
                />
                <span>{t('settings.webhookClear')}</span>
              </label>
            )}
          </div>

          <ErrorBox error={error} />

          <div className="settings-submit-bar">
            <div className="dirty-indicator-box">
              {isDirty ? (
                <span className="dirty-indicator dirty">
                  <span className="dirty-dot" aria-hidden="true" />
                  <span>{t('settings.unsavedChanges')}</span>
                </span>
              ) : (
                <span className="dirty-indicator clean">
                  <Check size={16} aria-hidden="true" />
                  <span>{t('settings.allChangesSaved')}</span>
                </span>
              )}
            </div>

            {canUpdate && (
              <div className="settings-submit-actions">
                {isDirty && (
                  <Button
                    variant="ghost"
                    type="button"
                    disabled={busy}
                    onClick={() => {
                      setTgUser(initialRef.current.tgUser);
                      setTgToken('');
                      setClearToken(false);
                      setWebhookUrl(initialRef.current.webhookUrl);
                      setClearWebhook(false);
                      setVapidKey(initialRef.current.vapidKey);
                      setVapidPrivateKey('');
                    }}
                  >
                    {t('settings.reset')}
                  </Button>
                )}
                <Button
                  variant="primary"
                  type="submit"
                  disabled={busy || !isDirty}
                  loading={busy}
                >
                  {t('settings.saveAlerts')}
                </Button>
              </div>
            )}
          </div>
        </form>
      )}
    </Card>
  );
}
