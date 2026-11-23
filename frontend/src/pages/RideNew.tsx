import { FormEvent, useEffect, useMemo, useRef, useState } from 'react';
import { Link, useNavigate, useSearchParams } from 'react-router-dom';
import {
  Accessibility,
  ChevronDown,
  ChevronUp,
  Eye,
  Footprints,
  HeartHandshake,
  Lock,
  Minus,
  Plus,
  User,
  Users,
} from 'lucide-react';
import { post, useApi } from '../api';
import {
  Button,
  EmptyState,
  ErrorBox,
  Field,
  IconButton,
  Page,
  PageHeader,
  useAction,
  useToast,
} from '../components/ui';
import type { Ride } from '../components/RideCard';
import type { SavedPlace } from '../types/domain';
import { fmtDateTime, todayStr, useFormatT } from '../format';
import { useTranslation } from 'react-i18next';
import { AddressInput } from '../components/AddressInput';
import { AreaPhotoStrip } from '../components/AreaPhotos';

interface Area {
  id: number;
  name: string;
  active: boolean;
  photo_count?: number;
}

interface ProxyRider {
  id: number;
  name: string;
  phone: string;
  relationship: string;
  notes: string;
  area_id?: number | null;
  pickup_address?: string;
  gate_notes?: string;
}

interface EventData {
  id: number;
  title: string;
  starts_at: string;
  location: string;
  my_signup?: { status: string } | null;
}

/** A ride is asked for from an event page. Opened without `?event=` (an old link or
 *  bookmark) there is nothing to attach it to, so point the member at the events list. */
export default function RideNew() {
  const { t } = useTranslation('rideNew');
  const [params] = useSearchParams();
  if (!params.get('event')) {
    return (
      <Page width="narrow">
        <div className="stack">
          <PageHeader
            title={t('title')}
            subtitle={t('desc')}
            back="/events"
          />
          <EmptyState
            title={t('eventFirstTitle')}
            description={t('eventFirstDesc')}
            action={<Button variant="primary" to="/events">{t('browseEvents')}</Button>}
          />
        </div>
      </Page>
    );
  }
  return <RideNewForm />;
}

function RideNewForm() {
  const { t } = useTranslation('rideNew');
  const ft = useFormatT();
  const nav = useNavigate();
  const [params] = useSearchParams();
  const eventId = params.get('event');
  const { data: areas } = useApi<Area[]>('/areas');
  const { data: event } = useApi<EventData>(
    eventId ? `/events/${eventId}` : null,
  );
  const { data: frequentRiders } = useApi<ProxyRider[]>('/proxy-riders');
  const { data: savedPlacesRaw } = useApi<SavedPlace[]>('/saved-places');
  const savedPlaces = useMemo(
    () => (Array.isArray(savedPlacesRaw) ? savedPlacesRaw : []),
    [savedPlacesRaw],
  );
  const preferredPlace = useMemo(
    () =>
      savedPlaces.find((sp) => sp.is_default === 1) ??
      (savedPlaces.length === 1 ? savedPlaces[0] : null),
    [savedPlaces],
  );
  const { data: recentRides } = useApi<Ride[]>('/rides');
  const toast = useToast();

  const [isProxy, setIsProxy] = useState(() => params.get('proxy') === '1');
  const [riderName, setRiderName] = useState('');
  const [riderPhone, setRiderPhone] = useState('');
  const [riderRelationship, setRiderRelationship] = useState('parent');
  const [saveProxyRider, setSaveProxyRider] = useState(false);

  const [date, setDate] = useState('');
  const [time, setTime] = useState('');
  const [tripType, setTripType] = useState<'outbound' | 'round' | 'return'>('outbound');
  const [includeReturn, setIncludeReturn] = useState(false);
  const [returnTime, setReturnTime] = useState('12:30');
  const [area, setArea] = useState('');
  const [address, setAddress] = useState('');
  const [destination, setDestination] = useState('');
  const [seats, setSeats] = useState(1);
  const [selectedChips, setSelectedChips] = useState<string[]>([]);
  const [customAssistance, setCustomAssistance] = useState('');
  const [notes, setNotes] = useState('');
  const [needsOpen, setNeedsOpen] = useState(false);
  const [saveAddress, setSaveAddress] = useState(false);
  const [saveAddressLabel, setSaveAddressLabel] = useState('');

  const [autoFilledHint, setAutoFilledHint] = useState<string | null>(null);
  const [touched, setTouched] = useState<Record<string, boolean>>({});
  const [errors, setErrors] = useState<Record<string, string>>({});
  const { busy, error, run } = useAction();
  const proxyAppliedRef = useRef<{ area?: string; address?: string; notes?: string }>({});
  const savedPlaceAppliedRef = useRef<{ notes?: string }>({});
  const prefilledPlaceRef = useRef(false);

  const hasNeedsContent =
    selectedChips.length > 0 || customAssistance.trim().length > 0 || notes.trim().length > 0;
  useEffect(() => {
    if (hasNeedsContent) setNeedsOpen(true);
  }, [hasNeedsContent]);

  const defaultReturnFor = (hhmm: string): string => {
    const parts = hhmm.split(':');
    if (parts.length !== 2) return '12:30';
    const h = Number(parts[0]);
    const m = Number(parts[1]);
    if (Number.isNaN(h) || Number.isNaN(m)) return '12:30';
    const mins = h * 60 + m;
    if (mins < 12 * 60 && returnTime > hhmm) return returnTime;
    const retMins = Math.min(23 * 60 + 59, mins + 120);
    return `${String(Math.floor(retMins / 60)).padStart(2, '0')}:${String(retMins % 60).padStart(2, '0')}`;
  };

  const handleTripTypeChange = (newType: 'outbound' | 'round' | 'return') => {
    setTripType(newType);
    setIncludeReturn(newType === 'round');
    if (!event) {
      if (newType === 'round' && time) {
        setReturnTime((cur) => (cur && cur > time ? cur : defaultReturnFor(time)));
      }
      return;
    }
    const [, t2 = '09:00'] = event.starts_at.split(/[T ]/);
    const [h = 9, m = 0] = t2.split(':').map(Number);
    if (newType === 'return') {
      const retMins = Math.min(23 * 60 + 59, h * 60 + m + 120);
      const retHhmm = `${String(Math.floor(retMins / 60)).padStart(2, '0')}:${String(retMins % 60).padStart(2, '0')}`;
      setTime(retHhmm);
      setAddress(event.location);
      setDestination(preferredPlace?.address || lastOwnRide?.pickup_address || '');
      if (!area && preferredPlace?.area_id) setArea(String(preferredPlace.area_id));
    } else {
      const mins = Math.max(0, h * 60 + m - 40);
      const hhmm = `${String(Math.floor(mins / 60)).padStart(2, '0')}:${String(mins % 60).padStart(2, '0')}`;
      setTime(hhmm);
      setDestination(event.location);
      if (address === event.location) {
        setAddress(preferredPlace?.address || lastOwnRide?.pickup_address || '');
      }
      if (newType === 'round') {
        const retMins = Math.min(23 * 60 + 59, h * 60 + m + 120);
        setReturnTime(`${String(Math.floor(retMins / 60)).padStart(2, '0')}:${String(retMins % 60).padStart(2, '0')}`);
      }
    }
  };

  const [prefilled, setPrefilled] = useState(false);
  useEffect(() => {
    if (!event || prefilled) return;
    const [d = '', t2 = '09:00'] = event.starts_at.split(/[T ]/);
    const [h = 9, m = 0] = t2.split(':').map(Number);
    const mins = Math.max(0, h * 60 + m - 40);
    const hhmm = `${String(Math.floor(mins / 60)).padStart(2, '0')}:${String(mins % 60).padStart(2, '0')}`;
    setDate(d);
    setTime(hhmm);
    if (mins >= 12 * 60) {
      const retMins = Math.min(23 * 60 + 59, mins + 120);
      setReturnTime(`${String(Math.floor(retMins / 60)).padStart(2, '0')}:${String(retMins % 60).padStart(2, '0')}`);
    }
    setDestination(event.location);
    setPrefilled(true);
  }, [event, prefilled]);

  useEffect(() => {
    if (prefilledPlaceRef.current || isProxy || area || address || tripType === 'return') return;
    if (preferredPlace && (preferredPlace.area_id || preferredPlace.address)) {
      prefilledPlaceRef.current = true;
      if (preferredPlace.area_id) setArea(String(preferredPlace.area_id));
      if (preferredPlace.address) setAddress(preferredPlace.address);
      if (preferredPlace.gate_notes && !notes.includes(preferredPlace.gate_notes)) {
        setNotes((cur) => (cur ? `${cur}\n${preferredPlace.gate_notes}` : preferredPlace.gate_notes));
      }
      savedPlaceAppliedRef.current = { notes: preferredPlace.gate_notes || undefined };
      setAutoFilledHint(t('autoPrefilledPlace', { label: preferredPlace.label }));
      return;
    }
    const ownRide = (recentRides ?? []).find((r) => !r.is_proxy && r.area?.id && r.pickup_address);
    if (ownRide) {
      prefilledPlaceRef.current = true;
      setArea(String(ownRide.area.id));
      setAddress(ownRide.pickup_address ?? '');
      setAutoFilledHint(t('autoPrefilledLast'));
    }
  }, [isProxy, area, address, tripType, preferredPlace, recentRides, notes, t]);

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

  const stripAppliedNotes = (curNotes: string, applied: string) => {
    if (!applied) return curNotes;
    if (curNotes === applied) return '';
    return curNotes
      .split(/\n/)
      .filter((line) => line.trim() !== applied.trim())
      .join('\n')
      .trim();
  };

  const selectFrequentRider = (fr: ProxyRider) => {
    const prev = proxyAppliedRef.current;
    setRiderName(fr.name);
    setRiderPhone(fr.phone);
    setRiderRelationship(fr.relationship || 'parent');

    const nextArea = fr.area_id ? String(fr.area_id) : '';
    if (fr.area_id || prev.area === area) {
      setArea(nextArea);
      setErrors((cur) => ({ ...cur, area: '' }));
    }

    const nextAddr = fr.pickup_address || '';
    if (fr.pickup_address || prev.address === address) {
      setAddress(nextAddr);
      setErrors((cur) => ({ ...cur, address: '' }));
    }

    const extraNotes = [fr.gate_notes, fr.notes].filter(Boolean).join(' · ');
    setNotes((cur) => {
      const stripped = stripAppliedNotes(cur, prev.notes ?? '');
      if (!extraNotes) return stripped;
      if (stripped.includes(extraNotes)) return stripped;
      return stripped ? `${stripped}\n${extraNotes}` : extraNotes;
    });
    proxyAppliedRef.current = {
      area: nextArea || undefined,
      address: nextAddr || undefined,
      notes: extraNotes || undefined,
    };
  };

  const switchToMyself = () => {
    const prev = proxyAppliedRef.current;
    setIsProxy(false);
    setRiderName('');
    setRiderPhone('');
    setSaveProxyRider(false);
    if (prev.area && area === prev.area) setArea('');
    if (prev.address && address === prev.address) setAddress('');
    if (prev.notes) setNotes((cur) => stripAppliedNotes(cur, prev.notes ?? ''));
    proxyAppliedRef.current = {};
  };

  const applySavedPlace = (sp: SavedPlace) => {
    const prevNotes = savedPlaceAppliedRef.current.notes ?? '';
    if (sp.area_id) {
      setArea(String(sp.area_id));
      setErrors((cur) => ({ ...cur, area: '' }));
    }
    if (tripType === 'return') {
      setDestination(sp.address);
      setErrors((cur) => ({ ...cur, destination: '' }));
    } else {
      setAddress(sp.address);
      setErrors((cur) => ({ ...cur, address: '' }));
    }
    setNotes((cur) => {
      const stripped = stripAppliedNotes(cur, prevNotes);
      if (!sp.gate_notes) return stripped;
      if (stripped.includes(sp.gate_notes)) return stripped;
      return stripped ? `${stripped}\n${sp.gate_notes}` : sp.gate_notes;
    });
    savedPlaceAppliedRef.current = { notes: sp.gate_notes || undefined };
    setSaveAddress(false);
    setSaveAddressLabel('');
    toast(t('savedPlaceApplied', { label: sp.label }));
  };

  const memberAddr = tripType === 'return' ? destination : address;
  const trimmedMemberAddr = memberAddr.trim();
  const matchesExistingSavedPlace = savedPlaces.some(
    (sp) => sp.address.trim().toLowerCase() === trimmedMemberAddr.toLowerCase(),
  );
  const canOfferSaveAddress =
    !isProxy &&
    trimmedMemberAddr.length > 0 &&
    !matchesExistingSavedPlace &&
    savedPlaces.length < 50 &&
    (!event || trimmedMemberAddr.toLowerCase() !== event.location.trim().toLowerCase());
  const addressLabelPresets = [t('placePresetHome'), t('placePresetWork'), t('placePresetParents')];

  const lastOwnRide = (recentRides ?? []).find((r) => !r.is_proxy && r.area?.id);
  const quickDestList = Array.from(
    new Set(
      [
        t('churchDest'),
        ...(recentRides ?? []).map((r) => r.destination?.trim()).filter((d): d is string => Boolean(d)),
      ],
    ),
  ).slice(0, 3);

  const applyLastPickup = () => {
    if (!lastOwnRide) return;
    setArea(String(lastOwnRide.area.id));
    setErrors((cur) => ({ ...cur, area: '' }));
    if (lastOwnRide.pickup_address) {
      setAddress(lastOwnRide.pickup_address);
      setErrors((cur) => ({ ...cur, address: '' }));
    }
    toast(t('lastPickupApplied'));
  };

  const submit = (e: FormEvent) => {
    e.preventDefault();

    const returnInvalid = includeReturn && (!returnTime.trim() || (Boolean(time) && returnTime <= time));
    const newErrors: Record<string, string> = {
      date: validateField('date', date),
      time: validateField('time', time),
      returnTime: returnInvalid
        ? !returnTime.trim()
          ? t('fieldRequired')
          : t('rides.INVALID_PICKUP_TIME', { ns: 'errors' })
        : '',
      area: validateField('area', area),
      address: validateField('address', address),
      destination: validateField('destination', destination),
      riderName: isProxy && !riderName.trim() ? t('fieldRequired') : '',
    };

    setTouched({
      date: true,
      time: true,
      returnTime: includeReturn,
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
      includeReturn ? 'returnTime' : '',
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

    const chipLabels: Record<string, string> = {
      wheelchair: t('needWheelchair'),
      walking: t('needWalkingAid'),
      visual: t('needVisualAssistance'),
      helpInOut: t('needHelpInOut'),
    };
    const trimmedCustom = customAssistance.trim();
    const customLower = trimmedCustom.toLowerCase();
    let canonicalAssistance = '';
    if (selectedChips.includes('wheelchair') || /\bwheelchair\b/.test(customLower)) {
      canonicalAssistance = 'wheelchair';
    } else if (/\b(stroller|child\s*seat|infant|baby)\b/.test(customLower)) {
      canonicalAssistance = 'stroller';
    } else if (/\boxygen\b/.test(customLower)) {
      canonicalAssistance = 'oxygen';
    } else if (selectedChips.length > 0 || trimmedCustom) {
      canonicalAssistance = 'other';
    }

    const descParts = [
      ...selectedChips.map((k) => chipLabels[k]).filter(Boolean),
      trimmedCustom,
    ];
    const extraDesc = descParts.join(', ');
    const cleanNotes = notes.trim();
    const finalNotes =
      extraDesc && !cleanNotes.includes(extraDesc)
        ? cleanNotes
          ? `${extraDesc} · ${cleanNotes}`
          : extraDesc
        : cleanNotes;

    run(async () => {
      const r = await post<{ id: number }>('/rides', {
        pickup_at: `${date}T${time}`,
        pickup_area_id: Number(area),
        pickup_address: address,
        destination,
        seats,
        assistance: canonicalAssistance,
        notes: finalNotes,
        event_id: eventId ? Number(eventId) : undefined,
        leg: tripType === 'return' ? 'return' : 'outbound',
        is_proxy: isProxy,
        rider_name: isProxy ? riderName.trim() : undefined,
        rider_phone: isProxy ? riderPhone.trim() : undefined,
        rider_relationship: isProxy ? riderRelationship : undefined,
        save_proxy_rider: isProxy ? saveProxyRider : false,
        return_pickup_at: includeReturn && date && returnTime ? `${date}T${returnTime}` : undefined,
        return_pickup_area_id: includeReturn ? Number(area) : undefined,
        return_pickup_address: includeReturn ? destination.trim() : undefined,
        return_destination: includeReturn ? address.trim() : undefined,
      });
      if (canOfferSaveAddress && saveAddress && saveAddressLabel.trim()) {
        await post('/saved-places', {
          label: saveAddressLabel.trim(),
          area_id: area ? Number(area) : null,
          address: trimmedMemberAddr,
          gate_notes: '',
          is_default: savedPlaces.length === 0,
        }).catch(() => {});
      }
      nav(`/rides/${r.id}`, { replace: true });
    }, t('submitted'));
  };

  if (event && event.my_signup?.status !== 'going') {
    return (
      <Page width="narrow">
        <div className="stack">
          <PageHeader
            title={t('title')}
            subtitle={t('eventLabel', { title: event.title })}
            back={eventId ? `/events/${eventId}` : '/events'}
          />
          <EmptyState
            title={t('signupRequiredTitle')}
            description={t('signupRequiredDesc', { title: event.title })}
            action={
              <Button variant="primary" to={`/events/${eventId}`}>
                {t('goToEvent')}
              </Button>
            }
          />
        </div>
      </Page>
    );
  }

  return (
    <Page width="narrow">
      <div className="stack">
        <PageHeader
          title={t('title')}
          subtitle={event ? `${event.title} · ${fmtDateTime(event.starts_at, ft)} · ${event.location}` : t('desc')}
          back={eventId ? `/events/${eventId}` : '/events'}
        />

        <form onSubmit={submit} noValidate className="stack ride-new-form">
          {/* Section 0: Who is this ride for? */}
          <section className="form-section stack">
            <h3>{t('riderTypeSection')}</h3>
            <div className="segmented-control wrap-phone" role="group" aria-label={t('riderTypeSection')}>
              <button
                type="button"
                aria-pressed={!isProxy}
                className={`segmented-item ${!isProxy ? 'active' : ''}`}
                onClick={switchToMyself}
              >
                <User size={16} aria-hidden="true" />
                <span>{t('forMyself')}</span>
              </button>
              <button
                type="button"
                aria-pressed={isProxy}
                className={`segmented-item ${isProxy ? 'active' : ''}`}
                onClick={() => setIsProxy(true)}
              >
                <Users size={16} aria-hidden="true" />
                <span>{t('forOther')}</span>
              </button>
            </div>

            {isProxy && (
              <div className="proxy-panel stack">
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
                          {fr.name} {fr.relationship ? `(${fr.relationship})` : ''}
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
            <div className="segmented-control wrap-phone" role="group" aria-label={t('tripTypeLabel')}>
              <button
                type="button"
                className={`segmented-item ${tripType === 'outbound' ? 'active' : ''}`}
                aria-pressed={tripType === 'outbound'}
                onClick={() => handleTripTypeChange('outbound')}
              >
                <span>{t('tripTypeOutbound')}</span>
              </button>
              <button
                type="button"
                className={`segmented-item ${tripType === 'round' ? 'active' : ''}`}
                aria-pressed={tripType === 'round'}
                onClick={() => handleTripTypeChange('round')}
              >
                <span>{t('tripTypeRound')}</span>
              </button>
              <button
                type="button"
                className={`segmented-item ${tripType === 'return' ? 'active' : ''}`}
                aria-pressed={tripType === 'return'}
                onClick={() => handleTripTypeChange('return')}
              >
                <span>{t('tripTypeReturn')}</span>
              </button>
            </div>
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
                <Field label={t('time')} hint={tripType === 'return' ? t('timeHintReturn') : event ? t('timeHintEvent') : undefined}>
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
            {tripType === 'round' && (
              <div className="form-grid two">
                <div>
                  <Field label={t('returnTime')} hint={t('returnHint')}>
                    <input
                      id="field-returnTime"
                      className="input"
                      type="time"
                      value={returnTime}
                      onChange={(e) => {
                        setReturnTime(e.target.value);
                        if (touched.returnTime) {
                          const bad = !e.target.value.trim() || (Boolean(time) && e.target.value <= time);
                          setErrors((cur) => ({
                            ...cur,
                            returnTime: bad
                              ? !e.target.value.trim()
                                ? t('fieldRequired')
                                : t('rides.INVALID_PICKUP_TIME', { ns: 'errors' })
                              : '',
                          }));
                        }
                      }}
                      aria-invalid={touched.returnTime && !!errors.returnTime}
                    />
                  </Field>
                  {touched.returnTime && errors.returnTime && (
                    <div className="field-error-msg" role="alert">{errors.returnTime}</div>
                  )}
                </div>
              </div>
            )}
          </section>

          {/* Section 2: Where */}
          <section className="form-section stack">
            <div className="row between wrap gap-xs">
              <div>
                <h3>{t('whereSection')}</h3>
                {autoFilledHint && !isProxy && <p className="muted tiny">{autoFilledHint}</p>}
              </div>
              <div className="row wrap gap-xs items-center">
                {!isProxy &&
                  savedPlaces.map((sp) => (
                    <button
                      key={sp.id}
                      type="button"
                      className={`chip ${(tripType === 'return' ? destination : address) === sp.address ? 'on' : ''}`}
                      onClick={() => applySavedPlace(sp)}
                    >
                      {sp.label}
                    </button>
                  ))}
                {!isProxy && lastOwnRide && (
                  <button
                    type="button"
                    className="chip"
                    onClick={applyLastPickup}
                  >
                    {t('useLastPickup', { area: lastOwnRide.area.name })}
                  </button>
                )}
                {!isProxy && (
                  <Link to="/me?section=places" className="saved-addr-manage-link">
                    {t('manageSavedPlaces')}
                  </Link>
                )}
              </div>
            </div>
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
                {(() => {
                  const chosen = areas?.find((a) => String(a.id) === area);
                  return chosen && (chosen.photo_count ?? 0) > 0 ? (
                    <AreaPhotoStrip areaId={chosen.id} areaName={chosen.name} />
                  ) : null;
                })()}
              </div>

              <div className="full">
                <AddressInput
                  id="field-address"
                  label={t('address')}
                  hint={
                    <span className="row nowrap min-w-0">
                      <Lock size={12} aria-hidden="true" />
                      <span className="min-w-0">{t('addressHint')}</span>
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
                {!event && quickDestList.length > 0 && (
                  <div className="chips" role="group" aria-label={t('quickDestinations')}>
                    {quickDestList.map((d) => (
                      <button
                        key={d}
                        type="button"
                        className={`chip ${destination === d ? 'active' : ''}`}
                        onClick={() => {
                          setDestination(d);
                          setErrors((c) => ({ ...c, destination: '' }));
                        }}
                      >
                        {d}
                      </button>
                    ))}
                  </div>
                )}
                {touched.destination && errors.destination && (
                  <div className="field-error-msg" role="alert">{errors.destination}</div>
                )}
              </div>

              {canOfferSaveAddress && (
                <div className="full stack-sm">
                  <label className={`check ${saveAddress ? 'on' : ''}`}>
                    <input
                      type="checkbox"
                      checked={saveAddress}
                      onChange={(e) => setSaveAddress(e.target.checked)}
                    />
                    <span>{t('saveAddressForNextTime')}</span>
                  </label>
                  {saveAddress && (
                    <div className="stack-sm">
                      <div className="chips">
                        {addressLabelPresets.map((preset) => (
                          <button
                            key={preset}
                            type="button"
                            className={`chip ${saveAddressLabel.trim() === preset ? 'on' : ''}`}
                            onClick={() => setSaveAddressLabel(preset)}
                          >
                            {preset}
                          </button>
                        ))}
                      </div>
                      <Field label={t('saveAddressLabel')}>
                        <input
                          className="input"
                          maxLength={40}
                          value={saveAddressLabel}
                          onChange={(e) => setSaveAddressLabel(e.target.value)}
                          placeholder={t('saveAddressLabelPlaceholder')}
                        />
                      </Field>
                    </div>
                  )}
                </div>
              )}
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

          {/* Section 4: Needs & Notes (collapsible optional disclosure) */}
          <section className="form-section stack">
            <button
              type="button"
              className="disclosure-toggle row between items-center"
              aria-expanded={needsOpen}
              aria-controls="ride-needs-panel"
              onClick={() => setNeedsOpen((v) => !v)}
            >
              <div className="stack-none text-left">
                <h3>{t('needsSection')}</h3>
                <span className="muted tiny">
                  {hasNeedsContent
                    ? t('needsAddedSummary', {
                        count:
                          selectedChips.length +
                          (customAssistance.trim() ? 1 : 0) +
                          (notes.trim() ? 1 : 0),
                      })
                    : t('needsOptionalHint')}
                </span>
              </div>
              {needsOpen ? <ChevronUp size={20} aria-hidden="true" /> : <ChevronDown size={20} aria-hidden="true" />}
            </button>
            {needsOpen && (
              <div id="ride-needs-panel" className="stack">
                <Field label={t('assistance')} hint={t('assistanceHint')}>
                  <div className="chips-grid">
                    <button
                      type="button"
                      className={`chip-toggle ${selectedChips.includes('wheelchair') ? 'active' : ''}`}
                      onClick={() => toggleChip('wheelchair')}
                      aria-pressed={selectedChips.includes('wheelchair')}
                    >
                      <Accessibility size={16} aria-hidden="true" />
                      <span>{t('needWheelchair')}</span>
                    </button>
                    <button
                      type="button"
                      className={`chip-toggle ${selectedChips.includes('walking') ? 'active' : ''}`}
                      onClick={() => toggleChip('walking')}
                      aria-pressed={selectedChips.includes('walking')}
                    >
                      <Footprints size={16} aria-hidden="true" />
                      <span>{t('needWalkingAid')}</span>
                    </button>
                    <button
                      type="button"
                      className={`chip-toggle ${selectedChips.includes('visual') ? 'active' : ''}`}
                      onClick={() => toggleChip('visual')}
                      aria-pressed={selectedChips.includes('visual')}
                    >
                      <Eye size={16} aria-hidden="true" />
                      <span>{t('needVisualAssistance')}</span>
                    </button>
                    <button
                      type="button"
                      className={`chip-toggle ${selectedChips.includes('helpInOut') ? 'active' : ''}`}
                      onClick={() => toggleChip('helpInOut')}
                      aria-pressed={selectedChips.includes('helpInOut')}
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
              </div>
            )}
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
