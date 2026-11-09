import { useApi } from '../../api';
import LiveMap from '../../components/LiveMap';
import { Empty, ErrorBox, Pill } from '../../components/ui';
import { fmtAgo, fmtTime, parseLocal, useFormatT } from '../../format';
import { useTranslation } from 'react-i18next';

interface Live { max: number; drivers: { driver_id: number; name: string; lat: number | null; lng: number | null; updated_at: string | null; rides: number; first_pickup: string }[] }

export default function AdminLive() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const { data, error } = useApi<Live>('/admin/live', 15000);
  const pts = (data?.drivers ?? []).filter((d) => d.lat != null && d.lng != null).map((d) => ({
    id: d.driver_id, lat: d.lat!, lng: d.lng!, label: d.name, detail: `${d.rides} ${t('live.trips', { rides: d.rides })} · 更新 ${d.updated_at ? fmtAgo(d.updated_at, ft) : '—'}`,
  }));
  const stale = (s: string | null) => !s || Date.now() - parseLocal(s).getTime() > 2 * 60 * 1000;

  return (
    <div className="stack">
      <div className="page-head">
        <div><h1>{t('live.title')}</h1><p>{t('live.desc')}</p></div>
        <Pill tone="teal">{data?.drivers.length ?? 0} / {data?.max ?? 10} {t('live.drivers')}</Pill>
      </div>
      <ErrorBox error={error} />
      <div className="split">
        <LiveMap points={pts} tall />
        <div className="card">
          <h3 style={{ marginBottom: 8 }}>{t('live.driversList')}</h3>
          {!data?.drivers.length ? <Empty>{t('live.none')}</Empty> : (
            <div className="list">
              {data.drivers.map((d) => (
                <div key={d.driver_id} className="item">
                  <div className="grow">
                    <div className="bold">{d.name}</div>
                    <div className="muted tiny">{d.rides} 项行程 · 首站 {fmtTime(d.first_pickup, ft)}</div>
                  </div>
                  {stale(d.updated_at) ? <Pill tone="amber">{d.updated_at ? `${fmtAgo(d.updated_at, ft)}${t('live.updated')}` : t('live.noLocation')}</Pill> : <Pill tone="green">{t('live.normal')}</Pill>}
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
