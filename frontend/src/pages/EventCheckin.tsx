import { FormEvent, useCallback, useEffect, useState } from 'react';
import { useParams, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AlertTriangle, ArrowRight, Calendar, CheckCircle2, MapPin } from 'lucide-react';
import { post, useApi } from '../api';
import { useAuth } from '../auth';
import { Button, Card, ErrorBox, Page, PageHeader, Spinner } from '../components/ui';
import { fmtDateTime, useFormatT } from '../format';

interface CheckinInfo {
  id: number;
  title: string;
  starts_at: string;
  location: string;
  attendance_enabled: boolean;
  my_attended: boolean;
  is_member: boolean;
}

export default function EventCheckin() {
  const { id } = useParams();
  const [params] = useSearchParams();
  const queryCode = params.get('code') || '';
  const { t } = useTranslation('eventDetail');
  const ft = useFormatT();
  const { me } = useAuth();
  const { data: info, error: infoErr } = useApi<CheckinInfo>(id ? `/events/${id}/checkin` : null);
  const [manualCode, setManualCode] = useState('');
  const [checkedIn, setCheckedIn] = useState(false);
  const [alreadyAttended, setAlreadyAttended] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  // Pending or disabled accounts cannot check in.
  const isNonMember = Boolean(me && me.status !== 'active');

  const submitCheckin = useCallback(
    (codeToSubmit: string) => {
      if (!id || !codeToSubmit.trim() || isNonMember) return;
      setBusy(true);
      setError(null);
      post<{ ok: boolean; already_attended: boolean }>(`/events/${id}/checkin`, { code: codeToSubmit.trim() })
        .then((res) => {
          setCheckedIn(true);
          setAlreadyAttended(res.already_attended);
        })
        .catch((err) => {
          setError(err instanceof Error ? err.message : String(err));
        })
        .finally(() => setBusy(false));
    },
    [id, isNonMember],
  );

  useEffect(() => {
    if (!id || !queryCode || isNonMember) return;
    submitCheckin(queryCode);
  }, [id, queryCode, isNonMember, submitCheckin]);

  const title = info?.title || t('attendanceModule');

  const handleManualSubmit = (e: FormEvent) => {
    e.preventDefault();
    submitCheckin(manualCode);
  };

  return (
    <Page width="narrow">
      <div className="stack items-center" style={{ maxWidth: '480px', margin: '2rem auto' }}>
        <PageHeader title={title} back={`/events/${id}`} />

        <Card className="stack text-center items-center" style={{ width: '100%', padding: '2rem' }}>
          {busy ? (
            <div className="stack items-center">
              <Spinner />
              <p className="muted small">{t('submitCheckin')}...</p>
            </div>
          ) : isNonMember ? (
            <div className="stack items-center">
              <div style={{ color: 'var(--danger)', marginBottom: '1rem' }}>
                <AlertTriangle size={56} />
              </div>
              <h3>{t('membersOnlyNotice')}</h3>
              <p className="muted small">{t('attendanceDisabled')}</p>
              <Button variant="secondary" to="/" block>
                {t('home', { ns: 'chrome' })}
              </Button>
            </div>
          ) : checkedIn || info?.my_attended ? (
            <div className="stack items-center">
              <div style={{ color: 'var(--success)', marginBottom: '0.5rem' }}>
                <CheckCircle2 size={64} />
              </div>
              <h2>{alreadyAttended || info?.my_attended ? t('alreadyAttendedNote') : t('checkinSuccess')}</h2>
              <p className="muted small">
                {t('participateNoRide')}
              </p>
              {info && (
                <div className="card flat tight stack-sm text-left" style={{ width: '100%', margin: '1rem 0' }}>
                  <div className="bold">{info.title}</div>
                  <div className="muted small row nowrap gap-sm">
                    <Calendar size={14} />
                    <span>{fmtDateTime(info.starts_at, ft)}</span>
                  </div>
                  <div className="muted small row nowrap gap-sm">
                    <MapPin size={14} />
                    <span>{info.location}</span>
                  </div>
                </div>
              )}
              <Button variant="primary" block to={`/events/${id}`} icon={<ArrowRight size={16} />}>
                {t('backToEvent')}
              </Button>
            </div>
          ) : info && !info.attendance_enabled ? (
            <div className="stack items-center" style={{ width: '100%' }}>
              <div style={{ color: 'var(--danger)', marginBottom: '0.5rem' }}>
                <AlertTriangle size={56} />
              </div>
              <h3>{t('attendanceDisabled')}</h3>
              <Button variant="secondary" block to={`/events/${id}`}>
                {t('backToEvent')}
              </Button>
            </div>
          ) : error || infoErr ? (
            <div className="stack items-center" style={{ width: '100%' }}>
              <div style={{ color: 'var(--danger)', marginBottom: '0.5rem' }}>
                <AlertTriangle size={56} />
              </div>
              <h3>{info?.attendance_enabled === false ? t('attendanceDisabled') : t('checkinFailedTitle')}</h3>
              <ErrorBox error={error || infoErr} />
              <form className="stack-sm" style={{ width: '100%' }} onSubmit={handleManualSubmit}>
                <input
                  className="input"
                  type="text"
                  placeholder={t('enterCodePlaceholder')}
                  aria-label={t('enterCodePlaceholder')}
                  value={manualCode}
                  onChange={(e) => setManualCode(e.target.value)}
                />
                <Button variant="primary" block type="submit" disabled={!manualCode.trim()}>
                  {t('submitCheckin')}
                </Button>
              </form>
              <Button variant="secondary" block to={`/events/${id}`}>
                {t('backToEvent')}
              </Button>
            </div>
          ) : (
            <div className="stack items-center" style={{ width: '100%' }}>
              <p className="muted small">{t('scanQrModalDesc')}</p>
              <form className="stack-sm" style={{ width: '100%' }} onSubmit={handleManualSubmit}>
                <input
                  className="input"
                  type="text"
                  placeholder={t('enterCodePlaceholder')}
                  aria-label={t('enterCodePlaceholder')}
                  value={manualCode}
                  onChange={(e) => setManualCode(e.target.value)}
                />
                <Button variant="primary" block type="submit" disabled={!manualCode.trim()}>
                  {t('submitCheckin')}
                </Button>
              </form>
              <Button variant="secondary" block to={`/events/${id}`}>
                {t('backToEvent')}
              </Button>
            </div>
          )}
        </Card>
      </div>
    </Page>
  );
}
