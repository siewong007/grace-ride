import { FormEvent, useEffect, useState } from 'react';
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
  const { can } = useAuth();
  // Groups, areas, and rates settings are separate permissions; each section shows for whoever holds it.
  const groups = can('admin_groups');
  const areas = can('admin_areas');
  const rates = can('admin_settings');

  if (!groups && !areas && !rates) {
    return (
      <Page>
        <EmptyState title={t('settings.noPermission')} />
      </Page>
    );
  }

  return (
    <Page>
      <PageHeader title={t('settings.title')} subtitle={t('settings.desc')} />
      <div className="stack gap-lg">
        {(groups || areas) && (
          <div className="split">
            {groups && <Groups />}
            {areas && <Areas />}
          </div>
        )}
        {rates && <ReimbursementRates />}
        {rates && <SystemRetentionSettings />}
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
  const [members, setMembers] = useState<Group | null>(null);
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
                  {can('admin_groups', 'update') && (
                    <Button variant="secondary" size="sm" onClick={() => setMembers(g)}>
                      {t('settings.membersBtn')}
                    </Button>
                  )}
                  {can('admin_groups', 'delete') && (
                    <Button variant="danger" size="sm" onClick={() => removeGroup(g)}>
                      {t('settings.delete')}
                    </Button>
                  )}
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {can('admin_groups', 'create') && <form
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
      </form>}

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
        placeholder={t('settings.search')}
        value={q}
        onChange={(e) => setQ(e.target.value)}
      />
      <div className="stack-sm group-members-list">
        {all
          ?.filter((u) => (u.status == null || u.status === 'active') && u.name.includes(q))
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

  useEffect(() => {
    if (data?.rates && !initialized) {
      const std = data.rates.find((r) => r.vehicle_type === 'standard');
      const oth = data.rates.find((r) => r.vehicle_type === 'other');
      if (std) setStandardRate(String(std.rate_cents_per_km));
      if (oth) setOtherRate(String(oth.rate_cents_per_km));
      setInitialized(true);
    }
  }, [data, initialized]);

  const save = (e: FormEvent) => {
    e.preventDefault();
    const stdNum = Math.max(0, parseInt(standardRate, 10) || 0);
    const othNum = Math.max(0, parseInt(otherRate, 10) || 0);
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
            <Field label={t('settings.rateStandard')}>
              <input
                className="input"
                type="number"
                min="0"
                step="1"
                value={standardRate}
                disabled={!canUpdate || busy}
                onChange={(e) => setStandardRate(e.target.value)}
                required
              />
            </Field>
            <Field label={t('settings.rateOther')}>
              <input
                className="input"
                type="number"
                min="0"
                step="1"
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
              <Button variant="primary" type="submit" disabled={busy} loading={busy}>
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

  const save = (e: FormEvent) => {
    e.preventDefault();
    const auditNum = Math.max(30, Math.min(3650, parseInt(auditDays, 10) || 730));
    const chatNum = Math.max(7, Math.min(3650, parseInt(chatDays, 10) || 180));
    const driversNum = Math.max(1, Math.min(100, parseInt(maxDrivers, 10) || 10));
    const timeoutNum = Math.max(0, Math.min(1440, parseInt(sessionTimeout, 10) ?? 15));

    run(
      () =>
        put('/admin/settings/system', {
          audit_retention_days: auditNum,
          chat_retention_days: chatNum,
          max_live_drivers: driversNum,
          session_timeout_mins: timeoutNum,
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
              <Button variant="primary" type="submit" disabled={busy} loading={busy}>
                {t('settings.saveSystem')}
              </Button>
            </div>
          )}
        </form>
      )}
    </Card>
  );
}
