import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useApi } from '../../api';
import LiveMap, { MapPoint } from '../../components/LiveMap';
import {
  CardSkeleton,
  EmptyState,
  ErrorBox,
  Page,
  PageHeader,
  Pill,
} from '../../components/ui';
import { fmtAgo, fmtTime, parseLocal, useFormatT } from '../../format';

interface Live {
  max: number;
  drivers: {
    driver_id: number;
    name: string;
    lat: number | null;
    lng: number | null;
    updated_at: string | null;
    rides: number;
    first_pickup: string;
  }[];
}

export default function AdminLive() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const [focusPoint, setFocusPoint] = useState<{ lat: number; lng: number } | null>(null);
  const [selectedId, setSelectedId] = useState<number | null>(null);

  const { data, error, loading } = useApi<Live>('/admin/live', 15000);

  const pts: MapPoint[] = (data?.drivers ?? [])
    .filter((d) => d.lat != null && d.lng != null)
    .map((d) => ({
      id: d.driver_id,
      lat: d.lat!,
      lng: d.lng!,
      label: d.name,
      detail: `${t('live.trips', { rides: d.rides })} · ${t('live.updated')} ${d.updated_at ? fmtAgo(d.updated_at, ft) : '—'}`,
      kind: 'car',
    }));

  const stale = (s: string | null) =>
    !s || Date.now() - parseLocal(s).getTime() > 2 * 60 * 1000;

  const selectDriver = (d: Live['drivers'][0]) => {
    setSelectedId(d.driver_id);
    if (d.lat != null && d.lng != null) {
      setFocusPoint({ lat: d.lat, lng: d.lng });
    }
  };

  return (
    <Page>
      <PageHeader
        title={t('live.title')}
        subtitle={t('live.desc')}
        actions={
          <Pill tone="teal">
            {data?.drivers.length ?? 0} / {data?.max ?? 10} {t('live.drivers')}
          </Pill>
        }
      />
      <ErrorBox error={error} />

      <div className="split">
        <div className="stack">
          <LiveMap points={pts} tall focusPoint={focusPoint} />

          {/* Legend */}
          <div className="map-legend" role="region" aria-label={t('live.legendTitle')}>
            <span>{t('live.legendTitle')}:</span>
            <span className="map-legend-item">
              <span className="map-legend-dot active" aria-hidden="true" />
              <span>{t('live.legendActive')}</span>
            </span>
            <span className="map-legend-item">
              <span className="map-legend-dot stale" aria-hidden="true" />
              <span>{t('live.legendStale')}</span>
            </span>
            <span className="map-legend-item">
              <span className="map-legend-dot passenger" aria-hidden="true" />
              <span>{t('live.legendPassenger')}</span>
            </span>
          </div>
        </div>

        {/* Side list of active drivers */}
        <section>
          <div className="section-head">
            <h2>{t('live.driversList')}</h2>
          </div>
          {loading ? (
            <CardSkeleton count={3} />
          ) : !data?.drivers.length ? (
            <EmptyState title={t('live.none')} />
          ) : (
            <div className="list">
              {data.drivers.map((d) => {
                const isSelected = selectedId === d.driver_id;
                return (
                  <button
                    key={d.driver_id}
                    type="button"
                    className={`item clickable text-left ${isSelected ? 'active' : ''}`}
                    onClick={() => selectDriver(d)}
                  >
                    <div className="grow">
                      <div className="bold">{d.name}</div>
                      <div className="muted tiny">
                        {t('live.driverTripsSummary', {
                          rides: d.rides,
                          time: fmtTime(d.first_pickup, ft),
                        })}
                      </div>
                    </div>
                    {stale(d.updated_at) ? (
                      <Pill tone="amber">
                        {d.updated_at
                          ? t('live.lastSeen', { time: fmtAgo(d.updated_at, ft) })
                          : t('live.noLocation')}
                      </Pill>
                    ) : (
                      <Pill tone="green">{t('live.active')}</Pill>
                    )}
                  </button>
                );
              })}
            </div>
          )}
        </section>
      </div>
    </Page>
  );
}
