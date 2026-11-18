import { FormEvent, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { del, post, put, useApi } from '../../api';
import { useAuth } from '../../auth';
import { ordinary, treeOrder, withDescendants } from '../../groupTree';
import {
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
  Pill,
  useAction,
  useConfirm,
} from '../../components/ui';
import { ADMIN_HOME } from '../../staffNav';

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

export default function Settings() {
  const { t } = useTranslation('admin');
  const { can } = useAuth();
  // Groups, areas, and rates settings are separate permissions; each section shows for whoever holds it.
  const groups = can('admin_groups');
  const areas = can('admin_areas');
  const rates = can('admin_settings');

  if (!groups && !areas && !rates) {
    return (
      <Page>
        <NoAccess />
      </Page>
    );
  }

  return (
    <Page>
      <PageHeader title={t('settings.title')} subtitle={t('settings.desc')} back={ADMIN_HOME} />
      <div className="stack gap-lg">
        {(groups || areas) && (
          <div className="split">
            {groups && <Groups />}
            {areas && <Areas />}
          </div>
        )}
        {rates && <ReimbursementRates />}
        {rates && <SystemRetentionSettings />}
        {rates && <AlertChannelSettings />}
      </div>
    </Page>
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
      <h3>{t('settings.groups')}</h3>
      {loading ? (
        <CardSkeleton count={2} />
      ) : !data?.length ? (
        <EmptyState title={t('settings.emptyGroups')} />
      ) : (
        <div className="list">
          {treeOrder(data).map(({ group: g, depth }) => (
            <div key={g.id} className="item group-row" style={{ paddingInlineStart: depth ? `${depth * 1.25}rem` : undefined }}>
              <div className="grow">
                <div className="bold">
                  {depth > 0 && <span className="muted" aria-hidden="true">↳ </span>}
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
                  {g.parent_id != null && byId.get(g.parent_id) && ` · ${t('settings.partOfName', { name: byId.get(g.parent_id)!.name })}`}
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

      {can('admin_groups', 'create') && <form
        className="row group-add"
        onSubmit={(e) => {
          e.preventDefault();
          run(
            () =>
              post('/admin/groups', { name, parent_id: parent ? Number(parent) : null }).then(() => {
                setName('');
                setParent('');
                reload();
              }),
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
        <select className="input" value={parent} onChange={(e) => setParent(e.target.value)} aria-label={t('settings.partOf')}>
          <option value="">{t('settings.topLevel')}</option>
          {parentChoices(data ?? []).map((g) => (
            <option key={g.id} value={g.id}>{t('settings.partOfName', { name: g.name })}</option>
          ))}
        </select>
        <Button variant="primary" disabled={busy} type="submit">
          {t('settings.addGroup')}
        </Button>
      </form>}

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
            () => put(`/admin/groups/${group.id}`, { name, description, parent_id: parent ? Number(parent) : null }).then(onSaved),
            t('settings.saved'),
          );
        }}
      >
        <Field label={t('settings.groupName')}>
          <input className="input" value={name} onChange={(e) => setName(e.target.value)} required />
        </Field>
        <Field label={t('settings.groupDescription')}>
          <input className="input" value={description} onChange={(e) => setDescription(e.target.value)} />
        </Field>
        <Field label={t('settings.partOf')} hint={t('settings.partOfHint')}>
          <select className="input" value={parent} onChange={(e) => setParent(e.target.value)}>
            <option value="">{t('settings.topLevel')}</option>
            {choices.map((g) => (
              <option key={g.id} value={g.id}>{g.name}</option>
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
  const { data: all } = useApi<{ id: number; name: string; status?: string }[]>(
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
          ?.filter((u) => (u.status == null || u.status === 'active') && u.name.toLowerCase().includes(q.trim().toLowerCase()))
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
      <Button variant="primary" block disabled={busy || !all || !current} onClick={save}>
        {t('settings.save')}
      </Button>
    </Modal>
  );
}

function Areas() {
  const { t } = useTranslation('admin');
  const { can } = useAuth();
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
              disabled={!can('admin_areas', 'update')}
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

      {can('admin_areas', 'create') && <form
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
      </form>}

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

interface RateEntry {
  vehicle_type: string;
  rate_cents_per_km: number;
  updated_at: string;
}

interface RatesResponse {
  rates: RateEntry[];
}

function ReimbursementRates() {
  const { t } = useTranslation('admin');
  const { can } = useAuth();
  const canUpdate = can('admin_settings', 'update');
  const { data, loading, reload } = useApi<RatesResponse>('/admin/settings/rates');
  const [standardRate, setStandardRate] = useState<string>('');
  const [otherRate, setOtherRate] = useState<string>('');
  const [initialized, setInitialized] = useState(false);
  const { busy, error, run } = useAction();

  // Stored as cents per km, shown and typed in ringgit.
  const toRm = (cents: number) => (cents / 100).toFixed(2);
  const parseRm = (v: string) => {
    const n = Number(v);
    return v.trim() !== '' && Number.isFinite(n) && n >= 0 ? Math.round(n * 100) : null;
  };
  const stdError = standardRate !== '' && parseRm(standardRate) === null ? t('settings.rateError') : undefined;
  const othError = otherRate !== '' && parseRm(otherRate) === null ? t('settings.rateError') : undefined;
  const ratesValid = parseRm(standardRate) !== null && parseRm(otherRate) !== null;

  useEffect(() => {
    if (data?.rates && !initialized) {
      const std = data.rates.find((r) => r.vehicle_type === 'standard');
      const oth = data.rates.find((r) => r.vehicle_type === 'other');
      if (std) setStandardRate(toRm(std.rate_cents_per_km));
      if (oth) setOtherRate(toRm(oth.rate_cents_per_km));
      setInitialized(true);
    }
  }, [data, initialized]);

  const save = (e: FormEvent) => {
    e.preventDefault();
    const stdNum = parseRm(standardRate);
    const othNum = parseRm(otherRate);
    if (stdNum === null || othNum === null) return;
    run(
      () =>
        put('/admin/settings/rates', {
          rates: [
            { vehicle_type: 'standard', rate_cents_per_km: stdNum },
            { vehicle_type: 'other', rate_cents_per_km: othNum },
          ],
        }).then(reload),
      t('settings.ratesSaved'),
    );
  };

  return (
    <Card className="stack">
      <div>
        <h3>{t('settings.ratesTitle')}</h3>
        <p className="muted small">{t('settings.ratesDesc')}</p>
      </div>

      {loading && !initialized ? (
        <CardSkeleton count={2} />
      ) : (
        <form onSubmit={save} className="stack">
          <div className="grid cols-2">
            <Field label={t('settings.rateStandard')} error={stdError}>
              <input
                className="input"
                type="number"
                inputMode="decimal"
                min="0"
                step="0.01"
                value={standardRate}
                disabled={!canUpdate || busy}
                onChange={(e) => setStandardRate(e.target.value)}
                required
              />
            </Field>
            <Field label={t('settings.rateOther')} error={othError}>
              <input
                className="input"
                type="number"
                inputMode="decimal"
                min="0"
                step="0.01"
                value={otherRate}
                disabled={!canUpdate || busy}
                onChange={(e) => setOtherRate(e.target.value)}
                required
              />
            </Field>
          </div>

          <ErrorBox error={error} />

          {canUpdate && (
            <div className="btn-row">
              <Button variant="primary" type="submit" disabled={busy || !ratesValid} loading={busy}>
                {t('settings.saveRates')}
              </Button>
            </div>
          )}
        </form>
      )}
    </Card>
  );
}

interface SystemSettingsResponse {
  audit_retention_days: number;
  chat_retention_days: number;
  max_live_drivers: number;
  session_timeout_mins: number;
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
  const { busy, error, run } = useAction();

  useEffect(() => {
    if (data && !initialized) {
      setAuditDays(String(data.audit_retention_days));
      setChatDays(String(data.chat_retention_days));
      setMaxDrivers(String(data.max_live_drivers));
      setSessionTimeout(String(data.session_timeout_mins ?? 15));
      setInitialized(true);
    }
  }, [data, initialized]);

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
        }).then(reload),
      t('settings.systemSaved'),
    );
  };

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
          <div className="grid cols-4">
            <Field
              label={t('settings.auditRetention')}
              hint={t('settings.auditRetentionHint')}
              error={errors.audit}
            >
              <input
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
            </Field>

            <Field
              label={t('settings.chatRetention')}
              hint={t('settings.chatRetentionHint')}
              error={errors.chat}
            >
              <input
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
            </Field>

            <Field
              label={t('settings.maxLiveDrivers')}
              hint={t('settings.maxLiveDriversHint')}
              error={errors.drivers}
            >
              <input
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
            </Field>

            <Field
              label={t('settings.sessionTimeout')}
              hint={t('settings.sessionTimeoutHint')}
              error={errors.timeout}
            >
              <input
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
            </Field>
          </div>

          <ErrorBox error={error} />

          {canUpdate && (
            <div className="btn-row">
              <Button variant="primary" type="submit" disabled={busy || !systemValid} loading={busy}>
                {t('settings.saveSystem')}
              </Button>
            </div>
          )}
        </form>
      )}
    </Card>
  );
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

function AlertChannelSettings() {
  const { t } = useTranslation('admin');
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
  const { busy, error, run } = useAction();

  useEffect(() => {
    if (cfg && !initialized) {
      setTgUser(cfg.telegram_bot_username ?? '');
      setVapidKey(cfg.vapid_public_key ?? '');
      setInitialized(true);
    }
  }, [cfg, initialized]);

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
      <div className="row between wrap gap-sm">
        <div>
          <h3>{t('settings.alertsTitle')}</h3>
          <p className="muted small">{t('settings.alertsDesc')}</p>
        </div>
        {cfg && (
          <div className="row wrap gap-xs">
            <Pill tone={cfg.telegram_bot_configured ? 'success' : 'neutral'}>
              {cfg.telegram_bot_configured ? t('settings.tgConfigured') : t('settings.tgNotConfigured')}
            </Pill>
            <Pill tone={cfg.vapid_private_configured ? 'success' : 'neutral'}>
              {cfg.vapid_private_configured
                ? t('settings.pushConfiguredCount', { count: cfg.push_subscriptions_count ?? 0 })
                : t('settings.pushNotConfigured')}
            </Pill>
          </div>
        )}
      </div>

      {loading && !initialized ? (
        <CardSkeleton count={2} />
      ) : (
        <form onSubmit={save} className="stack">
          <div className="form-grid two">
            <Field label={t('settings.tgBotUsername')}>
              <input
                className="input"
                disabled={!canUpdate || busy}
                value={tgUser}
                onChange={(e) => setTgUser(e.target.value)}
                placeholder="GraceRideBot"
              />
            </Field>
            <Field label={t('settings.tgBotToken')} hint={t('settings.tgBotTokenHint')}>
              <input
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
          <Field label={t('settings.webhookUrl')} hint={t('settings.webhookUrlHint')}>
            <input
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
          <div className="form-grid two">
            <Field label={t('settings.vapidPublicKey')}>
              <input
                className="input mono"
                disabled={!canUpdate || busy}
                value={vapidKey}
                onChange={(e) => setVapidKey(e.target.value)}
              />
            </Field>
            <Field label={t('settings.vapidPrivateKey')} hint={t('settings.vapidPrivateKeyHint')}>
              <input
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
          <ErrorBox error={error} />
          {canUpdate && (
            <div className="btn-row">
              <Button variant="primary" type="submit" disabled={busy} loading={busy}>
                {t('settings.saveAlerts')}
              </Button>
              <Button variant="secondary" type="button" disabled={busy} onClick={rotateVapidKeys}>
                {t('settings.rotateVapid')}
              </Button>
            </div>
          )}
        </form>
      )}
    </Card>
  );
}
