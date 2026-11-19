import { FormEvent, useEffect, useState } from 'react';
import { useNavigate, useParams, useSearchParams } from 'react-router-dom';
import { post, put, useApi } from '../api';
import { useAuth } from '../auth';
import { Button, ErrorBox, Field, Page, PageHeader, useAction } from '../components/ui';
import { EVENT_KIND, useFormatT } from '../format';
import { useTranslation } from 'react-i18next';
import { AddressInput } from '../components/AddressInput';
import PhotoAttach from '../components/PhotoAttach';
import { ordinary, treeOrder, withDescendants } from '../groupTree';
import { canAuthoriseMileage } from '../perms';

interface Group {
  id: number;
  name: string;
  description: string;
  is_all: boolean;
  member_count: number;
  parent_id: number | null;
  is_visitor: boolean;
}

interface ExistingEvent {
  id: number;
  title: string;
  kind: string;
  description: string;
  starts_at: string;
  location: string;
  capacity: number;
  cost_note: string;
  signup_deadline: string | null;
  group_ids: number[];
  as_group_ids: number[];
  photo_file_id: number | null;
  attendance_enabled: boolean;
  mileage_claimable?: boolean;
  created_by?: { id: number };
}

export default function EventForm() {
  const { t } = useTranslation('eventForm');
  const { me } = useAuth();
  const canMileage = canAuthoriseMileage(me?.roles);
  const ft = useFormatT();
  const { id } = useParams();
  const [params] = useSearchParams();
  const duplicateId = !id ? params.get('duplicate') : null;
  const nav = useNavigate();
  const { data: groups } = useApi<Group[]>('/groups');
  const { data: existing } = useApi<ExistingEvent>(
    id ? `/events/${id}` : duplicateId ? `/events/${duplicateId}` : null,
  );
  const [f, setF] = useState({
    title: '',
    kind: 'badminton',
    description: '',
    date: '',
    time: '',
    location: '',
    capacity: '12',
    cost_note: '',
    deadline: '',
    group_ids: [] as number[],
    as_group_ids: [] as number[],
    photo_file_id: null as number | null,
    attendance_enabled: true,
    mileage_claimable: false,
    repeat_frequency: 'none' as 'none' | 'weekly' | 'biweekly' | 'monthly',
    repeat_count: '4',
  });
  const [narrow, setNarrow] = useState(false);
  const { busy, error, run } = useAction();

  useEffect(() => {
    if (!existing) return;
    const [d = '', t2 = ''] = existing.starts_at.split(/[T ]/);
    const keepPhoto = !duplicateId || (me?.id != null && existing.created_by?.id === me.id);
    setF({
      title: existing.title,
      kind: existing.kind,
      description: existing.description,
      date: duplicateId ? '' : d,
      time: t2,
      location: existing.location,
      capacity: String(existing.capacity),
      cost_note: existing.cost_note,
      deadline: duplicateId ? '' : existing.signup_deadline ?? '',
      group_ids: existing.group_ids,
      as_group_ids: existing.as_group_ids ?? [],
      photo_file_id: keepPhoto ? (existing.photo_file_id ?? null) : null,
      attendance_enabled: existing.attendance_enabled ?? true,
      mileage_claimable: existing.mileage_claimable ?? false,
      repeat_frequency: 'none',
      repeat_count: '4',
    });
    setNarrow((existing.as_group_ids ?? []).length > 0);
  }, [existing, duplicateId, me?.id]);

  const set = (k: keyof typeof f) => (e: { target: { value: string } }) =>
    setF({ ...f, [k]: e.target.value });

  const toggle = (g: number) =>
    setF({
      ...f,
      group_ids: f.group_ids.includes(g)
        ? f.group_ids.filter((x) => x !== g)
        : [...f.group_ids, g],
    });

  const all = ordinary(groups ?? []);
  // "Joining as" can be any group the audience reaches: the chosen groups and every group
  // below them, or every group when the event is for all members.
  const forEveryone = all.some((g) => g.is_all && f.group_ids.includes(g.id));
  const reach = new Set<number>(forEveryone ? all.map((g) => g.id) : f.group_ids.flatMap((id) => [...withDescendants(all, id)]));
  const joinable = treeOrder(all.filter((g) => reach.has(g.id) && !g.is_all)).map((x) => x.group);
  const toggleJoin = (g: number) =>
    setF({
      ...f,
      as_group_ids: f.as_group_ids.includes(g) ? f.as_group_ids.filter((x) => x !== g) : [...f.as_group_ids, g],
    });

  const submit = (e: FormEvent) => {
    e.preventDefault();
    const body = {
      title: f.title,
      kind: f.kind,
      description: f.description,
      starts_at: `${f.date}T${f.time}`,
      location: f.location,
      capacity: Number(f.capacity) || 0,
      cost_note: f.cost_note,
      signup_deadline: f.deadline || null,
      group_ids: f.group_ids,
      photo_file_id: f.photo_file_id,
      attendance_enabled: f.attendance_enabled,
      mileage_claimable: f.mileage_claimable,
      // Empty: people choose from the audience groups and their sub-groups.
      as_group_ids: narrow ? f.as_group_ids.filter((g) => joinable.some((j) => j.id === g)) : [],
      repeat_frequency: !id && f.repeat_frequency !== 'none' ? f.repeat_frequency : undefined,
      repeat_count: !id && f.repeat_frequency !== 'none' ? Math.max(2, Math.min(12, Number(f.repeat_count) || 2)) : undefined,
    };
    run(async () => {
      if (id) {
        await put(`/events/${id}`, body);
        nav(`/events/${id}`);
      } else {
        const r = await post<{ id: number }>('/events', body);
        nav(`/events/${r.id}`);
      }
    }, id ? t('updated') : t('published'));
  };

  return (
    <Page width="medium">
      <div className="stack">
        <PageHeader
          title={id ? t('edit') : t('create')}
          back={id ? `/events/${id}` : '/events'}
        />

        <form className="card stack" onSubmit={submit}>
          <div className="form-grid two">
            <div className="full">
              <Field label={t('name')}>
                <input
                  className="input"
                  required
                  value={f.title}
                  onChange={set('title')}
                  placeholder={t('namePlaceholder')}
                />
              </Field>
            </div>
            <Field label={t('type')}>
              <select className="input" value={f.kind} onChange={set('kind')}>
                {Object.entries(EVENT_KIND).map(([k, v]) => (
                  <option key={k} value={k}>
                    {ft(v.label)}
                  </option>
                ))}
              </select>
            </Field>
            <Field label={t('capacity')} hint={t('capacityHint')}>
              <input
                className="input"
                type="number"
                min={0}
                value={f.capacity}
                onChange={set('capacity')}
              />
            </Field>
            <Field label={t('date')}>
              <input
                className="input"
                type="date"
                required
                value={f.date}
                onChange={set('date')}
              />
            </Field>
            <Field label={t('time')}>
              <input
                className="input"
                type="time"
                required
                value={f.time}
                onChange={set('time')}
              />
            </Field>
            <div className="full">
              <AddressInput
                id="field-location"
                label={t('location')}
                required
                value={f.location}
                onChange={(v) => setF((cur) => ({ ...cur, location: v }))}
              />
            </div>
            <Field label={t('cost')}>
              <input
                className="input"
                value={f.cost_note}
                onChange={set('cost_note')}
                placeholder={t('costPlaceholder')}
              />
            </Field>
            <Field label={t('deadline')}>
              <input
                className="input"
                type="datetime-local"
                value={f.deadline}
                onChange={set('deadline')}
              />
            </Field>
            <div className="full">
              <Field label={t('description')}>
                <textarea
                  className="input"
                  value={f.description}
                  onChange={set('description')}
                  rows={3}
                />
              </Field>
            </div>
            <div className="full">
              <Field label={t('photo')}>
                <PhotoAttach
                  ns="eventForm"
                  fileId={f.photo_file_id}
                  onChange={(photo_file_id) => setF({ ...f, photo_file_id })}
                  disabled={busy}
                />
              </Field>
            </div>
            <div className="full">
              <Field label={t('attendanceModule')} hint={t('attendanceHint')}>
                <label className="row nowrap gap-sm">
                  <input
                    type="checkbox"
                    checked={f.attendance_enabled}
                    onChange={(e) => setF({ ...f, attendance_enabled: e.target.checked })}
                  />
                  <span>{t('enableAttendance')}</span>
                </label>
              </Field>
            </div>
            <div className="full">
              <Field
                label={t('mileageClaimable')}
                hint={canMileage ? t('mileageClaimableHint') : t('mileageClaimableRestrictedHint')}
              >
                <label className={`row nowrap gap-sm ${!canMileage ? 'opacity-70 cursor-not-allowed' : ''}`}>
                  <input
                    type="checkbox"
                    checked={f.mileage_claimable}
                    disabled={!canMileage}
                    onChange={(e) => setF({ ...f, mileage_claimable: e.target.checked })}
                  />
                  <span>{t('enableMileageClaimable')}</span>
                </label>
              </Field>
            </div>
            {!id && (
              <>
                <Field label={t('repeatFrequency')}>
                  <select
                    className="input"
                    value={f.repeat_frequency}
                    onChange={(e) => setF({ ...f, repeat_frequency: e.target.value as 'none' | 'weekly' | 'biweekly' | 'monthly' })}
                  >
                    <option value="none">{t('repeatNone')}</option>
                    <option value="weekly">{t('repeatWeekly')}</option>
                    <option value="biweekly">{t('repeatBiweekly')}</option>
                    <option value="monthly">{t('repeatMonthly')}</option>
                  </select>
                </Field>
                {f.repeat_frequency !== 'none' && (
                  <Field label={t('repeatCount')}>
                    <input
                      className="input"
                      type="number"
                      min={2}
                      max={12}
                      value={f.repeat_count}
                      onChange={set('repeat_count')}
                    />
                  </Field>
                )}
              </>
            )}
          </div>

          <div className="stack-sm">
            <strong>{t('groups')}</strong>
            <p className="muted small">{t('groupsHint')}</p>
            <div className="grid cols-3">
              {treeOrder(all).map(({ group: g, depth }) => (
                <label key={g.id} className={`check ${f.group_ids.includes(g.id) ? 'on' : ''}`}>
                  <input
                    type="checkbox"
                    checked={f.group_ids.includes(g.id)}
                    onChange={() => toggle(g.id)}
                  />
                  <span>
                    {depth > 0 && <span className="muted" aria-hidden="true">{'↳ '.repeat(depth)}</span>}
                    {g.name}
                    {!g.is_all && <span className="muted tiny"> · {g.member_count} {t('peopleCount')}</span>}
                  </span>
                </label>
              ))}
            </div>
          </div>

          {joinable.length > 1 && (
            <div className="stack-sm">
              <strong>{t('joinAs')}</strong>
              <p className="muted small">{t('joinAsHint')}</p>
              <label className="row nowrap small">
                <input type="checkbox" checked={narrow} onChange={(e) => setNarrow(e.target.checked)} />
                <span>{t('joinAsNarrow')}</span>
              </label>
              {narrow && (
                <div className="grid cols-3">
                  {joinable.map((g) => (
                    <label key={g.id} className={`check ${f.as_group_ids.includes(g.id) ? 'on' : ''}`}>
                      <input type="checkbox" checked={f.as_group_ids.includes(g.id)} onChange={() => toggleJoin(g.id)} />
                      <span>{g.name}</span>
                    </label>
                  ))}
                </div>
              )}
            </div>
          )}

          <ErrorBox error={error} />
          <Button variant="primary" block type="submit" loading={busy} size="lg">
            {id ? t('save') : t('publish')}
          </Button>
        </form>
      </div>
    </Page>
  );
}
