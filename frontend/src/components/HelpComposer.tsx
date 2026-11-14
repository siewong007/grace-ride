import { FormEvent, useEffect, useState } from 'react';
import PhotoAttach from './PhotoAttach';
import { useNavigate, Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { BookOpen, User, X } from 'lucide-react';
import { post, useApi } from '../api';
import { useAuth } from '../auth';
import { Ride } from './RideCard';
import { Button, ErrorBox, Field, Modal, useAction, useToast } from './ui';
import { fmtDay, fmtTime, useFormatT } from '../format';
import { GuideSummary, usePick } from '../pages/HelpGuides';

const DRAFT_KEY = 'graceride_help_draft';

interface HelpComposerProps {
  open?: boolean;
  onClose: () => void;
  initialSubject?: string;
  initialRideId?: number | null;
  guides?: GuideSummary[];
  onCreated?: (ticketId: number) => void;
}

interface MemberUser {
  id: number;
  name: string;
}

export default function HelpComposer({
  onClose,
  initialSubject = '',
  initialRideId = null,
  guides = [],
  onCreated,
}: HelpComposerProps) {
  const { t } = useTranslation('help');
  const { t: tc } = useTranslation();
  const ft = useFormatT();
  const pick = usePick();
  const nav = useNavigate();
  const { can } = useAuth();
  const toast = useToast();
  const { busy, error, run } = useAction();
  const isStaffEditor = can('admin_help', 'update');

  const [subject, setSubject] = useState(initialSubject);
  const [body, setBody] = useState('');
  const [rideId, setRideId] = useState<number | null>(initialRideId);
  const [photo, setPhoto] = useState<number | null>(null);

  // Staff on-behalf creation
  const [onBehalf, setOnBehalf] = useState(false);
  const [memberQuery, setMemberQuery] = useState('');
  const [selectedMember, setSelectedMember] = useState<MemberUser | null>(null);

  // Load member's recent rides for the ride picker
  const ridesApi = useApi<Ride[]>(can('rides') ? '/rides' : null);
  const recentRides = ridesApi.data ?? [];

  // Member search for staff
  const membersApi = useApi<MemberUser[]>(
    isStaffEditor && onBehalf && memberQuery.trim()
      ? `/help/members?q=${encodeURIComponent(memberQuery.trim())}`
      : null,
  );
  const memberResults = membersApi.data ?? [];

  // Restore draft on mount if not overridden by initial props
  useEffect(() => {
    if (initialSubject || initialRideId != null) return;
    try {
      const saved = localStorage.getItem(DRAFT_KEY);
      if (saved) {
        const parsed = JSON.parse(saved);
        if (parsed.subject) setSubject(parsed.subject);
        if (parsed.body) setBody(parsed.body);
        if (parsed.rideId != null) setRideId(parsed.rideId);
      }
    } catch {
      // Ignore corrupt storage
    }
  }, [initialSubject, initialRideId]);

  // Persist draft on changes
  useEffect(() => {
    if (!subject && !body && rideId == null) {
      localStorage.removeItem(DRAFT_KEY);
      return;
    }
    try {
      localStorage.setItem(DRAFT_KEY, JSON.stringify({ subject, body, rideId }));
    } catch {
      // Ignore storage errors
    }
  }, [subject, body, rideId]);

  // Find suggested guides based on subject
  const suggestedGuides = (() => {
    const q = subject.trim().toLowerCase();
    if (q.length < 2) return [];
    const words = q.split(/\s+/).filter(Boolean);
    return guides
      .filter((g) => {
        const title = pick(g.title_zh, g.title_en).toLowerCase();
        const summary = pick(g.summary_zh, g.summary_en).toLowerCase();
        return words.some((w) => title.includes(w) || summary.includes(w));
      })
      .slice(0, 3);
  })();

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    run(async () => {
      const payload: {
        subject: string;
        body: string;
        ride_id?: number;
        for_user_id?: number;
        file_id?: number;
      } = {
        subject: subject.trim(),
        body: body.trim(),
      };
      if (rideId != null) {
        payload.ride_id = Number(rideId);
      }
      if (photo != null) payload.file_id = photo;
      if (isStaffEditor && onBehalf && selectedMember) {
        payload.for_user_id = selectedMember.id;
      }

      const res = await post<{ id: number }>('/help', payload);
      try {
        localStorage.removeItem(DRAFT_KEY);
      } catch {
        // ignore
      }
      toast(t('submitted'));
      onClose();
      if (onCreated) {
        onCreated(res.id);
      } else {
        nav(`/help/${res.id}`);
      }
    });
  };

  return (
    <Modal
      title={isStaffEditor && onBehalf ? t('raiseForMember') : t('ask')}
      onClose={onClose}
      size="md"
    >
      <form onSubmit={handleSubmit} className="stack">
        {isStaffEditor && (
          <div className="form-section">
            <label className="check">
              <input
                type="checkbox"
                checked={onBehalf}
                onChange={(e) => {
                  setOnBehalf(e.target.checked);
                  if (!e.target.checked) setSelectedMember(null);
                }}
              />
              <span>{t('raiseForMember')}</span>
            </label>

            {onBehalf && (
              <div className="stack-xs" style={{ marginTop: 'var(--space-2)' }}>
                {selectedMember ? (
                  <div className="row between nowrap pill-box">
                    <span className="row nowrap gap-xs bold small">
                      <User size={16} /> {selectedMember.name} (#{selectedMember.id})
                    </span>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => setSelectedMember(null)}
                      aria-label={t('clearMember')}
                    >
                      <X size={14} />
                    </Button>
                  </div>
                ) : (
                  <div>
                    <input
                      className="input"
                      placeholder={t('memberSearchPlaceholder')}
                      value={memberQuery}
                      onChange={(e) => setMemberQuery(e.target.value)}
                      autoFocus
                    />
                    {memberResults.length > 0 && (
                      <div className="card tight stack-xs" style={{ marginTop: 'var(--space-1)', maxHeight: 150, overflowY: 'auto' }}>
                        {memberResults.map((m) => (
                          <button
                            key={m.id}
                            type="button"
                            className="item"
                            style={{ textAlign: 'left', width: '100%', cursor: 'pointer' }}
                            onClick={() => {
                              setSelectedMember(m);
                              setMemberQuery('');
                            }}
                          >
                            <span className="bold">{m.name}</span>
                            <span className="muted tiny"> #{m.id}</span>
                          </button>
                        ))}
                      </div>
                    )}
                  </div>
                )}
              </div>
            )}
          </div>
        )}

        <Field label={t('subject')}>
          <input
            className="input"
            required
            value={subject}
            onChange={(e) => setSubject(e.target.value)}
            placeholder={t('subjectPlaceholder', 'What do you need help with?')}
          />
        </Field>

        {suggestedGuides.length > 0 && (
          <div className="notice info stack-xs">
            <div className="bold small">{t('suggestedGuides')}</div>
            <div className="stack-xs">
              {suggestedGuides.map((g) => (
                <Link
                  key={g.id}
                  to={`/help/guides/${g.slug}`}
                  onClick={onClose}
                  className="row nowrap gap-xs small"
                >
                  <BookOpen size={14} aria-hidden="true" />
                  <span>{pick(g.title_zh, g.title_en)}</span>
                </Link>
              ))}
            </div>
          </div>
        )}

        <Field label={t('body')}>
          <textarea
            className="input"
            required
            rows={4}
            value={body}
            onChange={(e) => setBody(e.target.value)}
            placeholder={t('bodyPlaceholder')}
          />
        </Field>
        <PhotoAttach fileId={photo} onChange={setPhoto} disabled={busy} />

        {recentRides.length > 0 && (
          <Field label={t('linkedRideLabel')}>
            <select
              className="input"
              value={rideId ?? ''}
              onChange={(e) => setRideId(e.target.value ? Number(e.target.value) : null)}
            >
              <option value="">{t('noLinkedRide')}</option>
              {recentRides.map((r) => (
                <option key={r.id} value={r.id}>
                  #{r.id} · {fmtDay(r.pickup_at, ft)} {fmtTime(r.pickup_at, ft)} → {r.destination}
                </option>
              ))}
            </select>
          </Field>
        )}

        <ErrorBox error={error} />

        <div className="btn-row">
          <Button variant="primary" type="submit" loading={busy}>
            {t('submit')}
          </Button>
          <Button variant="secondary" type="button" onClick={onClose} disabled={busy}>
            {tc('cancel')}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
