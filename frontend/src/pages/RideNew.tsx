import { FormEvent, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { post, useApi } from '../api';
import { ErrorBox, Field, useAction } from '../components/ui';
import { todayStr } from '../format';
import { useTranslation } from 'react-i18next';

interface Area { id: number; name: string; active: boolean }

export default function RideNew() {
  const { t } = useTranslation('rideNew');
  const nav = useNavigate();
  const [params] = useSearchParams();
  const eventId = params.get('event');
  const { data: areas } = useApi<Area[]>('/areas');
  const { data: event } = useApi<{ id: number; title: string; starts_at: string; location: string }>(eventId ? `/events/${eventId}` : null);
  const [f, setF] = useState({ date: '', time: '', area: '', address: '', destination: '', seats: 1, assistance: '', notes: '' });
  const set = (k: keyof typeof f) => (e: { target: { value: string } }) => setF({ ...f, [k]: e.target.value });
  const { busy, error, run } = useAction();

  const [prefilled, setPrefilled] = useState(false);
  if (event && !prefilled) {
    const [d, t2] = event.starts_at.split('T');
    const [h, m] = t2.split(':').map(Number);
    const mins = Math.max(0, h * 60 + m - 40);
    setF((x) => ({ ...x, date: d, time: `${String(Math.floor(mins / 60)).padStart(2, '0')}:${String(mins % 60).padStart(2, '0')}`, destination: event.location }));
    setPrefilled(true);
  }

  const submit = (e: FormEvent) => {
    e.preventDefault();
    run(async () => {
      const r = await post<{ id: number }>('/rides', {
        pickup_at: `${f.date}T${f.time}`,
        pickup_area_id: Number(f.area),
        pickup_address: f.address,
        destination: f.destination,
        seats: Number(f.seats),
        assistance: f.assistance,
        notes: f.notes,
        event_id: eventId ? Number(eventId) : undefined,
      });
      nav(`/rides/${r.id}`, { replace: true });
    }, t('submitted'));
  };

  return (
    <div className="stack" style={{ maxWidth: 760 }}>
      <div className="page-head">
        <div>
          <h1>{t('title')}</h1>
          <p>{event ? t('eventLabel', { title: event.title }) : t('desc')}</p>
        </div>
      </div>
      <form className="card stack" onSubmit={submit}>
        <div className="form-grid two">
          <Field label={t('date')}>
            <input className="input" type="date" required value={f.date} onChange={set('date')} min={todayStr()} />
          </Field>
          <Field label={t('time')} hint={event ? t('timeHintEvent') : undefined}>
            <input className="input" type="time" required value={f.time} onChange={set('time')} />
          </Field>
          <Field label={t('area')} hint={t('areaHint')}>
            <select className="input" required value={f.area} onChange={set('area')}>
              <option value="">{t('chooseDriver')}</option>
              {areas?.filter((a) => a.active).map((a) => <option key={a.id} value={a.id}>{a.name}</option>)}
            </select>
          </Field>
          <Field label={t('seats')}>
            <select className="input" value={f.seats} onChange={set('seats')}>
              {[1, 2, 3, 4].map((n) => <option key={n} value={n}>{n} {t('seatsUnit')}</option>)}
            </select>
          </Field>
          <div className="full">
            <Field label={t('address')} hint={t('addressHint')}>
              <input className="input" required value={f.address} onChange={set('address')} placeholder={t('addressPlaceholder')} />
            </Field>
          </div>
          <div className="full">
            <Field label={t('destination')}>
              <input className="input" required value={f.destination} onChange={set('destination')} placeholder={t('destinationPlaceholder')} />
            </Field>
          </div>
          <div className="full">
            <Field label={t('assistance')} hint={t('assistanceHint')}>
              <input className="input" value={f.assistance} onChange={set('assistance')} />
            </Field>
          </div>
          <div className="full">
            <Field label={t('notes')}>
              <textarea className="input" value={f.notes} onChange={set('notes')} />
            </Field>
          </div>
        </div>
        <ErrorBox error={error} />
        <button className="btn primary block" disabled={busy}>{busy ? t('submitting') : t('submitBtn')}</button>
        <p className="muted small center">{t('note')}</p>
      </form>
    </div>
  );
}
