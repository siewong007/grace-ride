import { FormEvent, useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { post, put, useApi } from '../api';
import { Button, ErrorBox, Field, Page, PageHeader, useAction } from '../components/ui';
import { EVENT_KIND, useFormatT } from '../format';
import { useTranslation } from 'react-i18next';
import { AddressInput } from '../components/AddressInput';

interface Group {
  id: number;
  name: string;
  description: string;
  is_all: boolean;
  member_count: number;
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
}

export default function EventForm() {
  const { t } = useTranslation('eventForm');
  const ft = useFormatT();
  const { id } = useParams();
  const nav = useNavigate();
  const { data: groups } = useApi<Group[]>('/groups');
  const { data: existing } = useApi<ExistingEvent>(id ? `/events/${id}` : null);
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
  });
  const { busy, error, run } = useAction();

  useEffect(() => {
    if (!existing) return;
    const [d, t2] = existing.starts_at.split('T');
    setF({
      title: existing.title,
      kind: existing.kind,
      description: existing.description,
      date: d,
      time: t2,
      location: existing.location,
      capacity: String(existing.capacity),
      cost_note: existing.cost_note,
      deadline: existing.signup_deadline ?? '',
      group_ids: existing.group_ids,
    });
  }, [existing]);

  const set = (k: keyof typeof f) => (e: { target: { value: string } }) =>
    setF({ ...f, [k]: e.target.value });

  const toggle = (g: number) =>
    setF({
      ...f,
      group_ids: f.group_ids.includes(g)
        ? f.group_ids.filter((x) => x !== g)
        : [...f.group_ids, g],
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
                    {v.emoji} {ft(v.label)}
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
          </div>

          <div className="stack-sm">
            <strong>{t('groups')}</strong>
            <p className="muted small">{t('groupsHint')}</p>
            <div className="grid cols-3">
              {groups?.map((g) => (
                <label key={g.id} className={`check ${f.group_ids.includes(g.id) ? 'on' : ''}`}>
                  <input
                    type="checkbox"
                    checked={f.group_ids.includes(g.id)}
                    onChange={() => toggle(g.id)}
                  />
                  <span>
                    {g.name}
                    {!g.is_all && <span className="muted tiny"> · {g.member_count} {t('peopleCount')}</span>}
                  </span>
                </label>
              ))}
            </div>
          </div>

          <ErrorBox error={error} />
          <Button variant="primary" block type="submit" loading={busy} size="lg">
            {id ? t('save') : t('publish')}
          </Button>
        </form>
      </div>
    </Page>
  );
}
