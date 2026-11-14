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
  User,
  Users,
  X,
} from 'lucide-react';
import { post, useApi } from '../api';
import {
  Button,
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
import { AddressInput } from '../components/AddressInput';

interface Area {
  id: number;
  name: string;
  active: boolean;
}

interface ProxyRider {
  id: number;
  name: string;
  phone: string;
  relationship: string;
  notes: string;
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
  const { data: frequentRiders } = useApi<ProxyRider[]>('/proxy-riders');

  const [isProxy, setIsProxy] = useState(false);
  const [riderName, setRiderName] = useState('');
  const [riderPhone, setRiderPhone] = useState('');
  const [riderRelationship, setRiderRelationship] = useState('parent');
  const [saveProxyRider, setSaveProxyRider] = useState(false);

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

  const selectFrequentRider = (fr: ProxyRider) => {
    setRiderName(fr.name);
    if (fr.phone) setRiderPhone(fr.phone);
    if (fr.relationship) setRiderRelationship(fr.relationship);
    if (fr.notes && !notes) setNotes(fr.notes);
  };

  const submit = (e: FormEvent) => {
    e.preventDefault();

    const newErrors: Record<string, string> = {
      date: validateField('date', date),
      time: validateField('time', time),
      area: validateField('area', area),
      address: validateField('address', address),
      destination: validateField('destination', destination),
      riderName: isProxy && !riderName.trim() ? t('fieldRequired') : '',
    };

    setTouched({
      date: true,
      time: true,
      area: true,
      address: true,
      destination: true,
      riderName: isProxy,
    });
    setErrors(newErrors);

    const firstErrKey = [
      isProxy ? 'riderName' : '',
      'date',
      'time',
      'area',
      'address',
      'destination',
    ].find((k) => k && newErrors[k]);

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
        is_proxy: isProxy,
        rider_name: isProxy ? riderName.trim() : undefined,
        rider_phone: isProxy ? riderPhone.trim() : undefined,
        rider_relationship: isProxy ? riderRelationship : undefined,
        save_proxy_rider: isProxy ? saveProxyRider : false,
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
          {/* Section 0: Who is this ride for? */}
          <section className="form-section stack">
            <h3>{t('riderTypeSection')}</h3>
            <div className="segmented-control" role="tablist">
              <button
                type="button"
                className={`segmented-item ${!isProxy ? 'active' : ''}`}
                onClick={() => setIsProxy(false)}
              >
                <User size={16} aria-hidden="true" />
                <span>{t('forMyself')}</span>
              </button>
              <button
                type="button"
                className={`segmented-item ${isProxy ? 'active' : ''}`}
                onClick={() => setIsProxy(true)}
              >
                <Users size={16} aria-hidden="true" />
                <span>{t('forOther')}</span>
              </button>
            </div>

            {isProxy && (
              <div
                className="card inset stack gap-md"
                style={{
                  background: 'var(--surface-2)',
                  padding: 'var(--space-3)',
                  borderRadius: 'var(--radius)',
                }}
              >
                {frequentRiders && frequentRiders.length > 0 && (
                  <div className="stack gap-xs">
                    <label className="small muted bold">{t('frequentRiders')}</label>
                    <div className="row wrap gap-xs">
                      {frequentRiders.map((fr) => (
                        <button
                          key={fr.id}
                          type="button"
                          className={`chip ${riderName === fr.name ? 'active' : ''}`}
                          onClick={() => selectFrequentRider(fr)}
                        >
                          👵 {fr.name} {fr.relationship ? `(${fr.relationship})` : ''}
                        </button>
                      ))}
                    </div>
                  </div>
                )}

                <div className="form-grid two">
                  <div>
                    <Field label={t('riderName')}>
                      <input
                        id="field-riderName"
                        className="input"
                        type="text"
                        required
                        value={riderName}
                        placeholder={t('riderNamePlaceholder')}
                        onChange={(e) => {
                          setRiderName(e.target.value);
                          if (touched.riderName) {
                            setErrors((cur) => ({
                              ...cur,
                              riderName: e.target.value.trim() ? '' : t('fieldRequired'),
                            }));
                          }
                        }}
                        onBlur={() => {
                          setTouched((cur) => ({ ...cur, riderName: true }));
                          setErrors((cur) => ({
                            ...cur,
                            riderName: riderName.trim() ? '' : t('fieldRequired'),
                          }));
                        }}
                        aria-invalid={touched.riderName && !!errors.riderName}
                      />
                    </Field>
                    {touched.riderName && errors.riderName && (
                      <div className="field-error-msg" role="alert">
                        {errors.riderName}
                      </div>
                    )}
                  </div>

                  <div>
                    <Field label={t('relationship')}>
                      <select
                        className="input select"
                        value={riderRelationship}
                        onChange={(e) => setRiderRelationship(e.target.value)}
                      >
                        <option value="parent">{t('relParent')}</option>
                        <option value="cell_member">{t('relCellMember')}</option>
                        <option value="spouse">{t('relSpouse')}</option>
                        <option value="friend">{t('relFriend')}</option>
                        <option value="other">{t('relOther')}</option>
                      </select>
                    </Field>
                  </div>
                </div>

                <Field label={t('riderPhone')} hint={t('riderPhoneHint')}>
                  <input
                    id="field-riderPhone"
                    className="input"
                    type="tel"
                    value={riderPhone}
                    placeholder={t('riderPhonePlaceholder')}
                    onChange={(e) => setRiderPhone(e.target.value)}
                  />
                </Field>

                <label className="row gap-xs items-center small">
                  <input
                    type="checkbox"
                    checked={saveProxyRider}
                    onChange={(e) => setSaveProxyRider(e.target.checked)}
                  />
                  <span>{t('saveFrequentRider')}</span>
                </label>
              </div>
            )}
          </section>

          {/* Section 1: When */}
          <section className="form-section stack">
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
          </section>

          {/* Section 2: Where */}
          <section className="form-section stack">
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
                <AddressInput
                  id="field-address"
                  label={t('address')}
                  hint={
                    <span className="row nowrap">
                      <Lock size={12} aria-hidden="true" />
                      <span>{t('addressHint')}</span>
                    </span>
                  }
                  required
                  value={address}
                  onChange={(v) => {
                    setAddress(v);
                    if (touched.address) setErrors((c) => ({ ...c, address: validateField('address', v) }));
                  }}
                  onBlur={(v) => handleBlur('address', v)}
                  placeholder={t('addressPlaceholder')}
                  invalid={touched.address && !!errors.address}
                />
                {touched.address && errors.address && (
                  <div className="field-error-msg" role="alert">{errors.address}</div>
                )}
              </div>

              <div className="full">
                <AddressInput
                  id="field-destination"
                  label={t('destination')}
                  required
                  value={destination}
                  onChange={(v) => {
                    setDestination(v);
                    if (touched.destination) setErrors((c) => ({ ...c, destination: validateField('destination', v) }));
                  }}
                  onBlur={(v) => handleBlur('destination', v)}
                  placeholder={t('destinationPlaceholder')}
                  invalid={touched.destination && !!errors.destination}
                />
                {touched.destination && errors.destination && (
                  <div className="field-error-msg" role="alert">{errors.destination}</div>
                )}
              </div>
            </div>
          </section>

          {/* Section 3: Who */}
          <section className="form-section stack">
            <h3>{t('whoSection')}</h3>
            <div className="stepper" role="group" aria-label={t('whoSection')}>
              <IconButton
                label={t('decreaseSeats')}
                icon={<Minus size={20} />}
                disabled={seats <= 1}
                onClick={() => setSeats((s) => Math.max(1, s - 1))}
              />
              <span className="stepper-value bold">{t('seatCount', { count: seats })}</span>
              <IconButton
                label={t('increaseSeats')}
                icon={<Plus size={20} />}
                disabled={seats >= 6}
                onClick={() => setSeats((s) => Math.min(6, s + 1))}
              />
            </div>
          </section>

          {/* Section 4: Needs & Notes */}
          <section className="form-section stack">
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
          </section>

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
          </div>
          <p className="muted small center">{t('note')}</p>
        </form>
      </div>
    </Page>
  );
}
