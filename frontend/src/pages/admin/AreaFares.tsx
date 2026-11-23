import { FormEvent, useMemo, useState } from 'react';
import { Download, FileSpreadsheet, Upload } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { del, put, useApi } from '../../api';
import { useAuth } from '../../auth';
import { Button, Card, CardSkeleton, EmptyState, ErrorBox, Pill, useAction, useConfirm } from '../../components/ui';
import { buildFaresCsv, buildFaresXlsx, triggerBlobDownload } from '../../fareSheet';
import FareImportModal from './FareImportModal';

export interface FareArea {
  id: number;
  name: string;
  active: boolean;
}

export interface AreaFare {
  id: number;
  area_a: { id: number; name: string };
  area_b: { id: number; name: string };
  amount_cents: number;
  updated_at: string;
}

export interface FaresResponse {
  fares: AreaFare[];
}

/** Fares are stored once per pair, smaller area id first. */
export const pairKey = (a: number, b: number) => (a <= b ? `${a}-${b}` : `${b}-${a}`);

/** Typed in ringgit, stored in cents. */
export const toRm = (cents: number) => (cents / 100).toFixed(2);
const parseRm = (s: string): number | null => {
  const v = s.trim();
  if (v === '' || !Number.isFinite(Number(v)) || Number(v) < 0 || Number(v) > 500) return null;
  return Math.round(Number(v) * 100);
};

/** How many routes between open areas (including within one area) have a fare. */
export function fareCoverage(areas: FareArea[], fares: AreaFare[]) {
  const open = areas.filter((a) => a.active);
  const ids = new Set(open.map((a) => a.id));
  const total = (open.length * (open.length + 1)) / 2;
  const set = fares.filter((f) => ids.has(f.area_a.id) && ids.has(f.area_b.id)).length;
  return { set, total };
}

/**
 * Fixed fares between service areas, the same in both directions. Pick an area, then
 * set its fare to every other open area. Unpriced routes are flagged so the traffic
 * lead can see what is still missing.
 */
export default function AreaFares() {
  const { t } = useTranslation('admin');
  const { can } = useAuth();
  const canUpdate = can('admin_fares', 'update');
  const canCreateAreas = can('admin_areas', 'create');
  const fares = useApi<FaresResponse>('/admin/settings/fares');
  const areasApi = useApi<FareArea[]>('/areas');
  const open = useMemo(
    () => (areasApi.data ?? []).filter((a) => a.active).sort((x, y) => x.name.localeCompare(y.name)),
    [areasApi.data],
  );
  const byPair = useMemo(() => {
    const m = new Map<string, AreaFare>();
    for (const f of fares.data?.fares ?? []) m.set(pairKey(f.area_a.id, f.area_b.id), f);
    return m;
  }, [fares.data]);
  const [picked, setPicked] = useState<number | null>(null);
  const [viewMode, setViewMode] = useState<'list' | 'matrix'>('list');
  const [matrixPair, setMatrixPair] = useState<{ from: FareArea; to: FareArea } | null>(null);
  const [importOpen, setImportOpen] = useState(false);
  const from = open.find((a) => a.id === picked) ?? open[0];

  const unsetFor = (a: FareArea) => open.filter((b) => !byPair.has(pairKey(a.id, b.id))).length;
  const { set, total } = fareCoverage(open, fares.data?.fares ?? []);
  const coveragePercent = total > 0 ? Math.round((set / total) * 100) : 0;

  const handleExportXlsx = () => {
    const bytes = buildFaresXlsx(areasApi.data ?? [], fares.data?.fares ?? [], {
      sheetName: t('fares.sheetNameExport'),
      originLabel: t('fares.matrixOrigin'),
    });
    const copy = new Uint8Array(bytes.byteLength);
    copy.set(bytes);
    triggerBlobDownload(
      new Blob([copy], {
        type: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
      }),
      'grace-ride-area-fares.xlsx',
    );
  };

  const handleExportCsv = () => {
    const csv = buildFaresCsv(
      areasApi.data ?? [],
      fares.data?.fares ?? [],
      t('fares.matrixOrigin'),
    );
    triggerBlobDownload(
      new Blob([csv], { type: 'text/csv;charset=utf-8;' }),
      'grace-ride-area-fares.csv',
    );
  };

  const showIoBar =
    !fares.loading && !areasApi.loading && (open.length > 0 || (canUpdate && canCreateAreas));

  return (
    <Card className="stack">
      <div className="row between wrap gap-sm">
        <div>
          <h3>{t('fares.title')}</h3>
          <p className="muted small">{t('fares.desc')}</p>
        </div>
        {open.length > 0 && (
          <div className="chips" role="tablist" aria-label="View mode">
            <button
              type="button"
              role="tab"
              aria-selected={viewMode === 'list'}
              className={`chip ${viewMode === 'list' ? 'on' : ''}`}
              onClick={() => setViewMode('list')}
            >
              {t('fares.viewList')}
            </button>
            <button
              type="button"
              role="tab"
              aria-selected={viewMode === 'matrix'}
              className={`chip ${viewMode === 'matrix' ? 'on' : ''}`}
              onClick={() => {
                setViewMode('matrix');
                if (!matrixPair && open.length > 0) {
                  setMatrixPair({ from: open[0], to: open[0] });
                }
              }}
            >
              {t('fares.viewMatrix')}
            </button>
          </div>
        )}
      </div>

      {showIoBar && (
        <div className="fare-io-bar">
          {open.length > 0 && (
            <>
              <Button variant="secondary" size="sm" onClick={handleExportXlsx}>
                <FileSpreadsheet size={16} aria-hidden="true" />
                {t('fares.exportXlsx')}
              </Button>
              <Button variant="secondary" size="sm" onClick={handleExportCsv}>
                <Download size={16} aria-hidden="true" />
                {t('fares.exportCsv')}
              </Button>
            </>
          )}
          {canUpdate && (open.length > 0 || canCreateAreas) && (
            <Button variant="primary" size="sm" onClick={() => setImportOpen(true)}>
              <Upload size={16} aria-hidden="true" />
              {t('fares.importAction')}
            </Button>
          )}
        </div>
      )}

      {fares.loading || areasApi.loading ? (
        <CardSkeleton count={2} />
      ) : open.length === 0 ? (
        <EmptyState title={t('fares.noAreas')} />
      ) : (
        <>
          <div className="row wrap gap-sm" aria-live="polite">
            <strong>{t('fares.coverage', { set, total })}</strong>
            <span className="muted small">({coveragePercent}%)</span>
            {total - set > 0 ? (
              <Pill tone="warning">{t('fares.unsetCount', { count: total - set })}</Pill>
            ) : (
              <Pill tone="success">{t('fares.allSet')}</Pill>
            )}
          </div>

          {viewMode === 'list' ? (
            <>
              <div className="stack-sm">
                <div className="label small muted">{t('fares.pickArea')}</div>
                <div className="chips fare-area-chips" role="group" aria-label={t('fares.pickArea')}>
                  {open.map((a) => {
                    const unset = unsetFor(a);
                    return (
                      <button
                        key={a.id}
                        type="button"
                        className={`chip${from?.id === a.id ? ' on' : ''}`}
                        aria-pressed={from?.id === a.id}
                        onClick={() => setPicked(a.id)}
                      >
                        {a.name}
                        {unset > 0 && (
                          <span className="fare-chip-count" aria-label={t('fares.unsetCount', { count: unset })}>
                            {unset}
                          </span>
                        )}
                      </button>
                    );
                  })}
                </div>
              </div>

              {from && (
                <ul className="fare-list" aria-label={t('fares.routesFrom', { area: from.name })}>
                  {[from, ...open.filter((b) => b.id !== from.id)].map((b) => (
                    <FareRow
                      key={`${from.id}-${b.id}`}
                      from={from}
                      to={b}
                      fare={byPair.get(pairKey(from.id, b.id))}
                      canUpdate={canUpdate}
                      onChanged={fares.reload}
                    />
                  ))}
                </ul>
              )}
            </>
          ) : (
            <div className="stack-sm">
              <p className="muted small">{t('fares.matrixSelectHint')}</p>
              <div className="fare-matrix-wrap" role="region" aria-label={t('fares.viewMatrix')}>
                <table className="fare-matrix-table">
                  <thead>
                    <tr>
                      <th className="origin-header">{t('fares.matrixOrigin')}</th>
                      {open.map((toArea) => (
                        <th key={toArea.id}>{toArea.name}</th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {open.map((fromArea) => (
                      <tr key={fromArea.id}>
                        <th className="origin-header">{fromArea.name}</th>
                        {open.map((toArea) => {
                          const key = pairKey(fromArea.id, toArea.id);
                          const fare = byPair.get(key);
                          const isSelected =
                            matrixPair && pairKey(matrixPair.from.id, matrixPair.to.id) === key;
                          const isDiagonal = fromArea.id === toArea.id;
                          return (
                            <td
                              key={toArea.id}
                              className={`fare-matrix-cell ${isSelected ? 'active-cell' : ''} ${isDiagonal ? 'diagonal' : ''}`}
                              onClick={() => setMatrixPair({ from: fromArea, to: toArea })}
                              title={`${fromArea.name} ↔ ${toArea.name}`}
                              role="button"
                              tabIndex={0}
                              onKeyDown={(e) => {
                                if (e.key === 'Enter' || e.key === ' ') {
                                  e.preventDefault();
                                  setMatrixPair({ from: fromArea, to: toArea });
                                }
                              }}
                            >
                              {fare ? (
                                <span className="fare-matrix-val">RM {toRm(fare.amount_cents)}</span>
                              ) : (
                                <span className="fare-matrix-unset">{t('fares.notSet')}</span>
                              )}
                            </td>
                          );
                        })}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>

              {matrixPair && (
                <div className="fare-matrix-editor">
                  <FareRow
                    from={matrixPair.from}
                    to={matrixPair.to}
                    fare={byPair.get(pairKey(matrixPair.from.id, matrixPair.to.id))}
                    canUpdate={canUpdate}
                    onChanged={fares.reload}
                  />
                </div>
              )}
            </div>
          )}
        </>
      )}

      {importOpen && (
        <FareImportModal
          allAreas={areasApi.data ?? []}
          openAreas={open}
          existingFares={fares.data?.fares ?? []}
          canCreateAreas={canCreateAreas}
          onClose={() => setImportOpen(false)}
          onImported={(createdAreas) => {
            fares.reload();
            if (createdAreas) areasApi.reload();
          }}
          onDownloadXlsx={handleExportXlsx}
          onDownloadCsv={handleExportCsv}
        />
      )}
    </Card>
  );
}

function FareRow({
  from,
  to,
  fare,
  canUpdate,
  onChanged,
}: {
  from: FareArea;
  to: FareArea;
  fare: AreaFare | undefined;
  canUpdate: boolean;
  onChanged: () => void;
}) {
  const { t } = useTranslation('admin');
  const confirm = useConfirm();
  const { busy, error, run } = useAction();
  const current = fare ? toRm(fare.amount_cents) : '';
  const [draft, setDraft] = useState<string | null>(null);
  const value = draft ?? current;
  const cents = parseRm(value);
  const invalid = value.trim() !== '' && cents === null;
  const dirty = draft !== null && value.trim() !== current;
  const label = from.id === to.id ? t('fares.within', { area: from.name }) : `${from.name} ↔\u00a0${to.name}`;

  const save = (e: FormEvent) => {
    e.preventDefault();
    if (cents === null || !dirty) return;
    run(
      () =>
        put('/admin/settings/fares', { area_a_id: from.id, area_b_id: to.id, amount_cents: cents }).then(() => {
          setDraft(null);
          onChanged();
        }),
      t('fares.saved', { route: label }),
    );
  };

  const clear = async () => {
    if (!fare) return;
    if (!(await confirm({ title: t('fares.clearConfirm', { route: label }), body: t('fares.clearBody') }))) return;
    run(
      () =>
        del(`/admin/settings/fares/${fare.id}`).then(() => {
          setDraft(null);
          onChanged();
        }),
      t('fares.cleared'),
    );
  };

  return (
    <li className="fare-row">
      <div className="fare-route">
        <span className="fare-route-name">{label}</span>
        {fare ? (
          !canUpdate && <strong className="nowrap">RM {toRm(fare.amount_cents)}</strong>
        ) : (
          <Pill tone="warning">{t('fares.notSet')}</Pill>
        )}
      </div>
      {canUpdate && (
        <form className="fare-edit" onSubmit={save}>
          <div className="input-affix-wrap">
            <span className="input-affix">RM</span>
            <input
              className="input"
              type="number"
              inputMode="decimal"
              min="0"
              max="500"
              step="0.50"
              placeholder="—"
              aria-label={t('fares.amountFor', { route: label })}
              aria-invalid={invalid || undefined}
              value={value}
              disabled={busy}
              onChange={(e) => setDraft(e.target.value)}
            />
          </div>
          {dirty || !fare ? (
            <Button type="submit" size="sm" disabled={!dirty || cents === null || busy}>
              {t('fares.save')}
            </Button>
          ) : (
            <Button type="button" variant="ghost" size="sm" onClick={clear} disabled={busy}>
              {t('fares.clear')}
            </Button>
          )}
        </form>
      )}
      {invalid && <p className="fare-row-error small">{t('fares.amountError')}</p>}
      <ErrorBox error={error} />
    </li>
  );
}
