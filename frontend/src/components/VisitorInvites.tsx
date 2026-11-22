// "Bringing a visitor" on the event page: book a ride for a friend without an account and
// send them a one-time link (backend/src/routes/visitors.rs).
import { FormEvent, useState } from 'react';
import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Copy, Share2, UserPlus } from 'lucide-react';
import { post, useApi } from '../api';
import { Button, ErrorBox, Field, Modal, Pill, useAction, useConfirm, useToast } from './ui';
import { fmtDateTime, parseLocal, useFormatT } from '../format';

interface VisitorPass {
  id: number;
  name: string;
  phone: string;
  ride_id: number | null;
  ride_status: string | null;
  host: { id: number; name: string };
  state: 'open' | 'claimed' | 'not_yet' | 'closed';
  closed_reason: string | null;
  valid_from: string;
}

interface Area {
  id: number;
  name: string;
  active: boolean;
}

/** One hour before the event, as the date and time fields want it. */
function defaultPickup(startsAt: string): { date: string; time: string } {
  const d = parseLocal(startsAt);
  d.setHours(d.getHours() - 1);
  const pad = (n: number) => String(n).padStart(2, '0');
  return { date: `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`, time: `${pad(d.getHours())}:${pad(d.getMinutes())}` };
}

export default function VisitorInvites({ event }: { event: { id: number; title: string; starts_at: string } }) {
  const { t } = useTranslation('visit');
  const ft = useFormatT();
  const confirm = useConfirm();
  const { data, reload } = useApi<VisitorPass[]>(`/events/${event.id}/visitors`);
  const [form, setForm] = useState(false);
  const [sent, setSent] = useState<{ name: string; url: string } | null>(null);
  const { busy, error, run } = useAction();

  const label = (p: VisitorPass) =>
    p.state === 'closed'
      ? t(`state.${p.closed_reason ?? 'expired'}`)
      : p.state === 'not_yet'
        ? t('state.not_yet', { when: fmtDateTime(p.valid_from, ft) })
        : t(`state.${p.state}`);
  const tone = (p: VisitorPass) => (p.state === 'closed' ? 'neutral' : p.state === 'claimed' ? 'success' : 'info');

  const reissue = (p: VisitorPass) =>
    run(async () => {
      const r = await post<{ url: string }>(`/visitor-passes/${p.id}/reissue`);
      setSent({ name: p.name, url: r.url });
      reload();
    });
  const turnOff = async (p: VisitorPass) => {
    const ok = await confirm({
      title: t('turnOffTitle', { name: p.name }),
      body: t('turnOffBody'),
      confirmLabel: t('turnOff'),
      tone: 'danger',
    });
    if (ok) run(() => post(`/visitor-passes/${p.id}/revoke`).then(reload));
  };

  return (
    <section className="detail-section stack">
      <div>
        <h3>{t('bringTitle')}</h3>
        <p className="muted small">{t('bringDesc')}</p>
        {!!data?.length && <p className="muted small">{t('bringConsent')}</p>}
      </div>
      {!!data?.length && (
        <div className="list">
          {data.map((p) => (
            <div key={p.id} className="item visitor-row">
              <div className="grow min-w-0">
                <div className="bold">{p.name}</div>
                <div className="muted tiny row nowrap gap-sm">
                  <Pill tone={tone(p)}>{label(p)}</Pill>
                  {p.ride_id != null && <Link to={`/rides/${p.ride_id}`}>{t('viewRide')}</Link>}
                </div>
              </div>
              {p.state !== 'closed' && (
                <div className="btn-row">
                  <Button variant="ghost" size="sm" disabled={busy} onClick={() => reissue(p)}>{t('newLink')}</Button>
                  <Button variant="ghost" size="sm" className="quiet-danger" disabled={busy} onClick={() => turnOff(p)}>{t('turnOff')}</Button>
                </div>
              )}
            </div>
          ))}
        </div>
      )}
      <ErrorBox error={error} />
      <div>
        <Button variant="secondary" size="sm" icon={<UserPlus size={16} />} onClick={() => setForm(true)}>
          {t('inviteButton')}
        </Button>
      </div>
      {form && (
        <InviteForm
          event={event}
          onClose={() => setForm(false)}
          onCreated={(name, url) => {
            setForm(false);
            setSent({ name, url });
            reload();
          }}
        />
      )}
      {sent && <SendLink event={event.title} {...sent} onClose={() => setSent(null)} />}
    </section>
  );
}

function InviteForm({
  event,
  onClose,
  onCreated,
}: {
  event: { id: number; starts_at: string };
  onClose: () => void;
  onCreated: (name: string, url: string) => void;
}) {
  const { t } = useTranslation('visit');
  const { data: areas } = useApi<Area[]>('/areas');
  const start = defaultPickup(event.starts_at);
  const [f, setF] = useState({ name: '', phone: '', date: start.date, time: start.time, area: '', address: '' });
  const { busy, error, run } = useAction();
  const set = (k: keyof typeof f) => (e: { target: { value: string } }) => setF({ ...f, [k]: e.target.value });

  const submit = (e: FormEvent) => {
    e.preventDefault();
    run(async () => {
      const r = await post<{ url: string }>(`/events/${event.id}/visitors`, {
        name: f.name,
        phone: f.phone,
        pickup_at: `${f.date}T${f.time}`,
        pickup_area_id: Number(f.area),
        pickup_address: f.address,
      });
      onCreated(f.name.trim(), r.url);
    });
  };

  return (
    <Modal title={t('formTitle')} onClose={onClose}>
      <form className="stack" onSubmit={submit}>
        <Field label={t('name')}>
          <input className="input" required value={f.name} onChange={set('name')} autoComplete="off" />
        </Field>
        <Field label={t('phone')}>
          <input className="input" type="tel" value={f.phone} onChange={set('phone')} autoComplete="off" />
        </Field>
        <div className="form-grid two">
          <Field label={t('date')}>
            <input className="input" type="date" required value={f.date} onChange={set('date')} />
          </Field>
          <Field label={t('pickupTime')}>
            <input className="input" type="time" required value={f.time} onChange={set('time')} />
          </Field>
        </div>
        <Field label={t('area')}>
          <select className="input" required value={f.area} onChange={set('area')}>
            <option value="">{t('areaPlaceholder')}</option>
            {areas?.filter((a) => a.active).map((a) => (
              <option key={a.id} value={a.id}>{a.name}</option>
            ))}
          </select>
        </Field>
        <Field label={t('address')}>
          <input className="input" required value={f.address} onChange={set('address')} />
        </Field>
        <ErrorBox error={error} />
        <Button variant="primary" type="submit" disabled={busy}>{t('create')}</Button>
      </form>
    </Modal>
  );
}

function SendLink({ event, name, url, onClose }: { event: string; name: string; url: string; onClose: () => void }) {
  const { t } = useTranslation('visit');
  const toast = useToast();
  const text = t('shareText', { name, event, url });
  return (
    <Modal title={t('sendTitle', { name })} description={t('sendBody')} onClose={onClose}>
      <div className="stack">
        <input className="input mono" readOnly value={url} aria-label={t('copy')} onFocus={(e) => e.target.select()} />
        <div className="btn-row">
          <Button
            variant="primary"
            icon={<Share2 size={18} />}
            onClick={() => window.open(`https://wa.me/?text=${encodeURIComponent(text)}`, '_blank', 'noopener,noreferrer')}
          >
            {t('share')}
          </Button>
          <Button
            variant="secondary"
            icon={<Copy size={18} />}
            onClick={() => navigator.clipboard?.writeText(url).then(() => toast(t('copied'), 'success'))}
          >
            {t('copy')}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
