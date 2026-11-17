import { useEffect, useState } from 'react';
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
  const code = params.get('code') || '';
  const { t } = useTranslation('eventDetail');
  const ft = useFormatT();
  const { me } = useAuth();
  const { data: info, error: infoErr } = useApi<CheckinInfo>(id ? `/events/${id}/checkin` : null);
  const [checkedIn, setCheckedIn] = useState(false);
  const [alreadyAttended, setAlreadyAttended] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!id || !code) return;
    setBusy(true);
    post<{ ok: boolean; already_attended: boolean }>(`/events/${id}/checkin`, { code })
      .then((res) => {
        setCheckedIn(true);
        setAlreadyAttended(res.already_attended);
      })
      .catch((err) => {
        setError(err);
      })
      .finally(() => setBusy(false));
  }, [id, code]);

  const title = info?.title || t('attendanceModule');

  // If pending or not active (non-member)
  const isNonMember = me && me.status !== 'active';

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
                {t('manageTitle', 'Back')}
              </Button>
            </div>
          ) : checkedIn || info?.my_attended ? (
            <div className="stack items-center">
              <div style={{ color: '#10b981', marginBottom: '0.5rem' }}>
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
                {t('manageTitle')}
              </Button>
            </div>
          ) : error || infoErr ? (
            <div className="stack items-center" style={{ width: '100%' }}>
              <div style={{ color: 'var(--danger)', marginBottom: '0.5rem' }}>
                <AlertTriangle size={56} />
              </div>
              <h3>{t('attendanceDisabled')}</h3>
              <ErrorBox error={error || infoErr} />
              {!code && (
                <p className="muted small">{t('scanQrModalDesc')}</p>
              )}
              <Button variant="secondary" block to={`/events/${id}`}>
                {t('manageTitle')}
              </Button>
            </div>
          ) : null}
        </Card>
      </div>
    </Page>
  );
}
