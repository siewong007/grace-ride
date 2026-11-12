import { FormEvent, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import {
  Accessibility,
  Eye,
  Footprints,
  HeartHandshake,
  Lock,
  Minus,
  Plus,
  X,
} from 'lucide-react';
import { post, useApi } from '../api';
import {
  Button,
  Card,
  ErrorBox,
  Field,
  IconButton,
  Notice,
  Page,
  PageHeader,
  useAction,
} from '../components/ui';
import { todayStr } from '../format';
import { useTranslation } from 'react-i18next';

interface Area {
  id: number;
  name: string;
  active: boolean;
}

export default function RideNew() {
  const { t } = useTranslation('rideNew');
  const nav = useNavigate();
  const [params] = useSearchParams();
  const eventId = params.get('event');
  const { data: areas } = useApi<Area[]>('/areas');
  const { data: event } = useApi<{ id: number; title: string; starts_at: string; location: string }>(
    eventId ? `/events/${eventId}` : null,
  );

  const [date, setDate] = useState('');
  const [time, setTime] = useState('');
  const [area, setArea] = useState('');
  const [address, setAddress] = useState('');
  const [destination, setDestination] = useState('');
  const [seats, setSeats] = useState(1);
  const [selectedChips, setSelectedChips] = useState<string[]>([]);
  const [customAssistance, setCustomAssistance] = useState('');
  const [notes, setNotes] = useState('');

  const [showEventNotice, setShowEventNotice] = useState(true);
  const [touched, setTouched] = useState<Record<string, boolean>>({});
  const [errors, setErrors] = useState<Record<string, string>>({});
  const { busy, error, run } = useAction();

  const [prefilled, setPrefilled] = useState(false);
  if (event && !prefilled) {
    const [d, t2] = event.starts_at.split('T');
    const [h, m] = t2.split(':').map(Number);
    const mins = Math.max(0, h * 60 + m - 40);
    setDate(d);
    setTime(`${String(Math.floor(mins / 60)).padStart(2, '0')}:${String(mins % 60).padStart(2, '0')}`);
    setDestination(event.location);
    setPrefilled(true);
  }

  const validateField = (_name: string, val: string): string => {
    if (!val || !val.trim()) {
      return t('fieldRequired');
    }
    return '';
  };

  const handleBlur = (name: string, val: string) => {
    setTouched((cur) => ({ ...cur, [name]: true }));
    const err = validateField(name, val);
    setErrors((cur) => ({ ...cur, [name]: err }));
  };

  const toggleChip = (chipLabel: string) => {
    setSelectedChips((cur) =>
      cur.includes(chipLabel) ? cur.filter((c) => c !== chipLabel) : [...cur, chipLabel],
    );
  };

  const submit = (e: FormEvent) => {
    e.preventDefault();

    const newErrors: Record<string, string> = {
      date: validateField('date', date),
      time: validateField('time', time),
      area: validateField('area', area),
      address: validateField('address', address),
      destination: validateField('destination', destination),
    };

    setTouched({
      date: true,
      time: true,
      area: true,
      address: true,
      destination: true,
    });
    setErrors(newErrors);

    const firstErrKey = ['date', 'time', 'area', 'address', 'destination'].find(
      (k) => newErrors[k],
    );

    if (firstErrKey) {
      const el = document.getElementById(`field-${firstErrKey}`);
      el?.focus();
      el?.scrollIntoView?.({ behavior: 'smooth', block: 'center' });
      return;
    }

    const assistanceParts = [...selectedChips];
    if (customAssistance.trim()) {
      assistanceParts.push(customAssistance.trim());
    }
    const combinedAssistance = assistanceParts.join(', ');

    run(async () => {
      const r = await post<{ id: number }>('/rides', {
        pickup_at: `${date}T${time}`,
        pickup_area_id: Number(area),
        pickup_address: address,
        destination,
        seats,
        assistance: combinedAssistance,
        notes,
        event_id: eventId ? Number(eventId) : undefined,
      });
      nav(`/rides/${r.id}`, { replace: true });
    }, t('submitted'));
  };

  return (
    <Page width="narrow">
      <div className="stack">
        <PageHeader
          title={t('title')}
          subtitle={event ? t('eventLabel', { title: event.title }) : t('desc')}
          back="/rides"
        />

        {showEventNotice && event && (
          <Notice tone="info" className="row between nowrap">
            <div>
              <strong>{t('eventLabel', { title: event.title })}</strong>
              <div className="small muted">{t('timeHintEvent')}</div>
            </div>
            <IconButton
              size="sm"
              variant="ghost"
              label={t('dismiss')}
              icon={<X size={16} />}
              onClick={() => setShowEventNotice(false)}
            />
          </Notice>
        )}

        <form onSubmit={submit} noValidate className="stack">
          {/* Card 1: When */}
          <Card className="stack">
            <h3>{t('whenSection')}</h3>
            <div className="form-grid two">
              <div>
                <Field label={t('date')}>
                  <input
                    id="field-date"
                    className="input"
                    type="date"
                    required
                    value={date}
                    onChange={(e) => {
                      setDate(e.target.value);
                      if (touched.date) setErrors((c) => ({ ...c, date: validateField('date', e.target.value) }));
                    }}
                    onBlur={(e) => handleBlur('date', e.target.value)}
                    min={todayStr()}
                    aria-invalid={touched.date && !!errors.date}
                  />
                </Field>
                {touched.date && errors.date && (
                  <div className="field-error-msg" role="alert">{errors.date}</div>
                )}
              </div>

              <div>
                <Field label={t('time')} hint={event ? t('timeHintEvent') : undefined}>
                  <input
                    id="field-time"
                    className="input"
                    type="time"
                    required
                    value={time}
                    onChange={(e) => {
                      setTime(e.target.value);
                      if (touched.time) setErrors((c) => ({ ...c, time: validateField('time', e.target.value) }));
                    }}
                    onBlur={(e) => handleBlur('time', e.target.value)}
                    aria-invalid={touched.time && !!errors.time}
                  />
                </Field>
                {touched.time && errors.time && (
                  <div className="field-error-msg" role="alert">{errors.time}</div>
                )}
              </div>
            </div>
          </Card>

          {/* Card 2: Where */}
          <Card className="stack">
            <h3>{t('whereSection')}</h3>
            <div className="form-grid two">
              <div className="full">
                <Field label={t('area')} hint={t('areaHint')}>
                  <select
                    id="field-area"
                    className="input"
                    required
                    value={area}
                    onChange={(e) => {
                      setArea(e.target.value);
                      if (touched.area) setErrors((c) => ({ ...c, area: validateField('area', e.target.value) }));
                    }}
                    onBlur={(e) => handleBlur('area', e.target.value)}
                    aria-invalid={touched.area && !!errors.area}
                  >
                    <option value="">{t('chooseDriver')}</option>
                    {areas?.filter((a) => a.active).map((a) => (
                      <option key={a.id} value={a.id}>{a.name}</option>
                    ))}
                  </select>
                </Field>
                {touched.area && errors.area && (
                  <div className="field-error-msg" role="alert">{errors.area}</div>
                )}
              </div>

              <div className="full">
                <Field
                  label={t('address')}
                  hint={
                    <span className="row nowrap">
                      <Lock size={12} aria-hidden="true" />
                      <span>{t('addressHint')}</span>
                    </span>
                  }
                >
                  <input
                    id="field-address"
                    className="input"
                    required
                    value={address}
                    onChange={(e) => {
                      setAddress(e.target.value);
                      if (touched.address) setErrors((c) => ({ ...c, address: validateField('address', e.target.value) }));
                    }}
                    onBlur={(e) => handleBlur('address', e.target.value)}
                    placeholder={t('addressPlaceholder')}
                    aria-invalid={touched.address && !!errors.address}
                  />
                </Field>
                {touched.address && errors.address && (
                  <div className="field-error-msg" role="alert">{errors.address}</div>
                )}
              </div>

              <div className="full">
                <Field label={t('destination')}>
                  <input
                    id="field-destination"
                    className="input"
                    required
                    value={destination}
                    onChange={(e) => {
                      setDestination(e.target.value);
                      if (touched.destination) setErrors((c) => ({ ...c, destination: validateField('destination', e.target.value) }));
                    }}
                    onBlur={(e) => handleBlur('destination', e.target.value)}
                    placeholder={t('destinationPlaceholder')}
                    aria-invalid={touched.destination && !!errors.destination}
                  />
                </Field>
                {touched.destination && errors.destination && (
                  <div className="field-error-msg" role="alert">{errors.destination}</div>
                )}
              </div>
            </div>
          </Card>

          {/* Card 3: Who */}
          <Card className="stack">
            <h3>{t('whoSection')}</h3>
            <div className="stepper" role="group" aria-label={t('whoSection')}>
              <IconButton
                label={t('decreaseSeats')}
                icon={<Minus size={20} />}
                disabled={seats <= 1}
                onClick={() => setSeats((s) => Math.max(1, s - 1))}
              />
              <span className="stepper-value bold">{seats} {t('seatsUnit')}</span>
              <IconButton
                label={t('increaseSeats')}
                icon={<Plus size={20} />}
                disabled={seats >= 6}
                onClick={() => setSeats((s) => Math.min(6, s + 1))}
              />
            </div>
          </Card>

          {/* Card 4: Needs & Notes */}
          <Card className="stack">
            <h3>{t('needsSection')}</h3>
            <Field label={t('assistance')} hint={t('assistanceHint')}>
              <div className="chips-grid">
                <button
                  type="button"
                  className={`chip-toggle ${selectedChips.includes(t('needWheelchair')) ? 'active' : ''}`}
                  onClick={() => toggleChip(t('needWheelchair'))}
                  aria-pressed={selectedChips.includes(t('needWheelchair'))}
                >
                  <Accessibility size={16} aria-hidden="true" />
                  <span>{t('needWheelchair')}</span>
                </button>
                <button
                  type="button"
                  className={`chip-toggle ${selectedChips.includes(t('needWalkingAid')) ? 'active' : ''}`}
                  onClick={() => toggleChip(t('needWalkingAid'))}
                  aria-pressed={selectedChips.includes(t('needWalkingAid'))}
                >
                  <Footprints size={16} aria-hidden="true" />
                  <span>{t('needWalkingAid')}</span>
                </button>
                <button
                  type="button"
                  className={`chip-toggle ${selectedChips.includes(t('needVisualAssistance')) ? 'active' : ''}`}
                  onClick={() => toggleChip(t('needVisualAssistance'))}
                  aria-pressed={selectedChips.includes(t('needVisualAssistance'))}
                >
                  <Eye size={16} aria-hidden="true" />
                  <span>{t('needVisualAssistance')}</span>
                </button>
                <button
                  type="button"
                  className={`chip-toggle ${selectedChips.includes(t('needHelpInOut')) ? 'active' : ''}`}
                  onClick={() => toggleChip(t('needHelpInOut'))}
                  aria-pressed={selectedChips.includes(t('needHelpInOut'))}
                >
                  <HeartHandshake size={16} aria-hidden="true" />
                  <span>{t('needHelpInOut')}</span>
                </button>
              </div>
            </Field>

            <Field label={t('otherAssistance')}>
              <input
                className="input"
                value={customAssistance}
                onChange={(e) => setCustomAssistance(e.target.value)}
                placeholder={t('otherAssistancePlaceholder')}
              />
            </Field>

            <Field label={t('notes')}>
              <textarea
                className="input"
                value={notes}
                onChange={(e) => setNotes(e.target.value)}
                rows={3}
              />
            </Field>
          </Card>

          {/* Sticky Submit Bar on Phone */}
          <div className="ride-new-submit-bar">
            <ErrorBox error={error} />
            <Button
              variant="primary"
              block
              type="submit"
              loading={busy}
              size="lg"
            >
              {busy ? t('submitting') : t('submitBtn')}
            </Button>
            <p className="muted small center">{t('note')}</p>
          </div>
        </form>
      </div>
    </Page>
  );
}
