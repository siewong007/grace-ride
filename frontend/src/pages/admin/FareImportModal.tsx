import { ChangeEvent, DragEvent, useMemo, useRef, useState } from 'react';
import { AlertTriangle, Download, FileSpreadsheet, Upload } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { put } from '../../api';
import {
  Button,
  ErrorBox,
  Field,
  Modal,
  Notice,
  Pill,
  useAction,
  useToast,
  type PillTone,
} from '../../components/ui';
import {
  readFareFile,
  type FareParseIssue,
  type FareWorkbookParseResult,
  type ParsedFareRow,
} from '../../fareSheet';
import { XlsxError } from '../../xlsxCodec';
import { toRm, type AreaFare, type FareArea } from './AreaFares';

export interface FareImportModalProps {
  allAreas: FareArea[];
  openAreas: FareArea[];
  existingFares: AreaFare[];
  canCreateAreas: boolean;
  onClose: () => void;
  onImported: (createdAreas: boolean) => void;
  onDownloadXlsx: () => void;
  onDownloadCsv: () => void;
}

interface BulkFaresResult {
  created: number;
  updated: number;
  unchanged: number;
  total: number;
  areas_created: string[];
}

type EffectiveStatus = 'new' | 'update' | 'unchanged' | 'skipped';

const STATUS_ORDER: Record<EffectiveStatus, number> = {
  new: 0,
  update: 1,
  skipped: 2,
  unchanged: 3,
};

const STATUS_TONE: Record<EffectiveStatus, PillTone> = {
  new: 'success',
  update: 'info',
  skipped: 'warning',
  unchanged: 'neutral',
};

export default function FareImportModal({
  allAreas,
  openAreas,
  existingFares,
  canCreateAreas,
  onClose,
  onImported,
  onDownloadXlsx,
  onDownloadCsv,
}: FareImportModalProps) {
  const { t } = useTranslation('admin');
  const toast = useToast();
  const { busy, error, setError, run } = useAction();
  const fileInputRef = useRef<HTMLInputElement>(null);

  const [workbook, setWorkbook] = useState<FareWorkbookParseResult | null>(null);
  const [sheetIndex, setSheetIndex] = useState(0);
  const [parsing, setParsing] = useState(false);
  const [parseError, setParseError] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);
  const [createMissingAreas, setCreateMissingAreas] = useState(false);

  const activeSheet = workbook?.sheets[sheetIndex] ?? workbook?.sheets[0] ?? null;
  const activeParsed = activeSheet?.parsed ?? null;

  const handleFile = async (file: File) => {
    setParsing(true);
    setParseError(null);
    setError(null);
    try {
      const result = await readFareFile(file, allAreas, existingFares);
      const firstPopulated = result.sheets.findIndex(
        (s) => s.parsed.rows.length > 0 || s.parsed.issues.length > 0,
      );
      setWorkbook(result);
      setSheetIndex(firstPopulated >= 0 ? firstPopulated : 0);
      setCreateMissingAreas(false);
    } catch (err) {
      setWorkbook(null);
      if (err instanceof XlsxError) {
        if (err.code === 'LEGACY_XLS') setParseError(t('fares.importErrorLegacyXls'));
        else if (err.code === 'TOO_LARGE') setParseError(t('fares.importErrorTooLarge'));
        else setParseError(t('fares.importErrorInvalidXlsx'));
      } else {
        setParseError(t('fares.importErrorInvalidXlsx'));
      }
    } finally {
      setParsing(false);
    }
  };

  const onFileInputChange = (e: ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    e.target.value = '';
    if (file) void handleFile(file);
  };

  const onDrop = (e: DragEvent<HTMLLabelElement>) => {
    e.preventDefault();
    setDragging(false);
    const file = e.dataTransfer.files?.[0];
    if (file) void handleFile(file);
  };

  const summary = useMemo(() => {
    if (!activeParsed) {
      return {
        newCount: 0,
        updateCount: 0,
        unchangedCount: 0,
        skippedCount: 0,
        sortedRows: [] as { row: ParsedFareRow; effective: EffectiveStatus }[],
        applicablePayload: [] as { area_a: string; area_b: string; amount_cents: number }[],
      };
    }

    let newCount = 0;
    let updateCount = 0;
    let unchangedCount = 0;
    let skippedCount = 0;
    const decorated: { row: ParsedFareRow; effective: EffectiveStatus; idx: number }[] = [];
    const applicablePayload: { area_a: string; area_b: string; amount_cents: number }[] = [];

    activeParsed.rows.forEach((row, idx) => {
      const effective: EffectiveStatus =
        row.status === 'unknown_area'
          ? canCreateAreas && createMissingAreas
            ? 'new'
            : 'skipped'
          : row.status;
      if (effective === 'new') {
        newCount++;
        applicablePayload.push({
          area_a: row.areaAName,
          area_b: row.areaBName,
          amount_cents: row.amountCents,
        });
      } else if (effective === 'update') {
        updateCount++;
        applicablePayload.push({
          area_a: row.areaAName,
          area_b: row.areaBName,
          amount_cents: row.amountCents,
        });
      } else if (effective === 'unchanged') {
        unchangedCount++;
      } else {
        skippedCount++;
      }
      decorated.push({ row, effective, idx });
    });

    decorated.sort((a, b) => {
      const diff = STATUS_ORDER[a.effective] - STATUS_ORDER[b.effective];
      return diff !== 0 ? diff : a.idx - b.idx;
    });

    return {
      newCount,
      updateCount,
      unchangedCount,
      skippedCount,
      sortedRows: decorated.map(({ row, effective }) => ({ row, effective })),
      applicablePayload,
    };
  }, [activeParsed, canCreateAreas, createMissingAreas]);

  const formatIssue = (issue: FareParseIssue): string => {
    if (issue.kind === 'invalid_amount') {
      return t('fares.issueInvalidAmount', {
        row: issue.row,
        col: issue.col,
        route: issue.route ?? '',
        raw: issue.raw ?? '',
      });
    }
    if (issue.kind === 'conflict') {
      return t('fares.issueConflict', {
        row: issue.row,
        col: issue.col,
        route: issue.route ?? '',
        first: issue.firstCents !== undefined ? `RM ${toRm(issue.firstCents)}` : '',
        second: issue.secondCents !== undefined ? `RM ${toRm(issue.secondCents)}` : '',
      });
    }
    if (issue.kind === 'invalid_area_name') {
      return t('fares.issueInvalidArea', {
        row: issue.row,
        col: issue.col,
        raw: issue.raw ?? '',
      });
    }
    return t('fares.issueMissingArea', { row: issue.row, col: issue.col });
  };

  const statusLabel = (s: EffectiveStatus): string => {
    if (s === 'new') return t('fares.statusNew');
    if (s === 'update') return t('fares.statusUpdate');
    if (s === 'skipped') return t('fares.statusSkipped');
    return t('fares.statusUnchanged');
  };

  const applyChanges = () => {
    if (!activeParsed || activeParsed.issues.length > 0 || summary.applicablePayload.length === 0) {
      return;
    }
    void run(async () => {
      const res = await put<BulkFaresResult>('/admin/settings/fares/bulk', {
        fares: summary.applicablePayload,
        create_missing_areas: canCreateAreas && createMissingAreas,
      });
      toast(t('fares.importDone', { created: res.created, updated: res.updated }), 'success');
      onImported((res.areas_created?.length ?? 0) > 0);
      onClose();
    });
  };

  const canApply =
    !busy &&
    !parsing &&
    Boolean(activeParsed) &&
    (activeParsed?.issues.length ?? 0) === 0 &&
    summary.applicablePayload.length > 0;

  return (
    <Modal
      size="lg"
      title={t('fares.importTitle')}
      onClose={onClose}
      footer={
        <div className="btn-row end">
          <Button variant="ghost" onClick={onClose} disabled={busy}>
            {t('common:cancel')}
          </Button>
          <Button variant="primary" onClick={applyChanges} disabled={!canApply} loading={busy}>
            {t('fares.importApply', { count: summary.applicablePayload.length })}
          </Button>
        </div>
      }
    >
      <p className="muted small">{t('fares.importDesc')}</p>

      {openAreas.length > 0 && (
        <div className="row wrap gap-xs">
          <span className="muted small">{t('fares.downloadTemplate')}</span>
          <Button variant="secondary" size="sm" onClick={onDownloadXlsx}>
            <FileSpreadsheet size={15} aria-hidden="true" />
            {t('fares.exportXlsx')}
          </Button>
          <Button variant="secondary" size="sm" onClick={onDownloadCsv}>
            <Download size={15} aria-hidden="true" />
            {t('fares.exportCsv')}
          </Button>
        </div>
      )}

      {!workbook ? (
        <label
          className={`fare-dropzone${dragging ? ' dragging' : ''}`}
          onDragOver={(e) => {
            e.preventDefault();
            setDragging(true);
          }}
          onDragLeave={() => setDragging(false)}
          onDrop={onDrop}
        >
          <input
            ref={fileInputRef}
            type="file"
            accept=".xlsx,.csv,.tsv,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet,text/csv"
            className="sr-only"
            onChange={onFileInputChange}
            disabled={parsing}
          />
          <Upload size={22} aria-hidden="true" />
          <strong>{t('fares.dropzoneLabel')}</strong>
          <span className="muted small">{t('fares.dropzoneHint')}</span>
        </label>
      ) : (
        <div className="stack-sm">
          <div className="fare-import-meta">
            <div className="row wrap gap-xs">
              <strong>{workbook.fileName}</strong>
              {activeParsed && (
                <Pill tone="info">
                  {activeParsed.layout === 'matrix'
                    ? t('fares.layoutMatrix')
                    : t('fares.layoutList')}
                </Pill>
              )}
            </div>
            <label className="btn sm secondary">
              <input
                ref={fileInputRef}
                type="file"
                accept=".xlsx,.csv,.tsv,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet,text/csv"
                className="sr-only"
                onChange={onFileInputChange}
                disabled={parsing || busy}
              />
              {t('fares.chooseOtherFile')}
            </label>
          </div>

          {workbook.sheets.length > 1 && (
            <Field label={t('fares.sheetLabel')}>
              <select
                className="input"
                value={sheetIndex}
                onChange={(e) => {
                  setSheetIndex(Number(e.target.value));
                  setCreateMissingAreas(false);
                }}
              >
                {workbook.sheets.map((s, idx) => (
                  <option key={`${s.name}-${idx}`} value={idx}>
                    {s.name}
                  </option>
                ))}
              </select>
            </Field>
          )}
        </div>
      )}

      <ErrorBox error={parseError ?? error} />

      {activeParsed && (
        <div className="stack-sm">
          <div className="row wrap gap-xs" aria-live="polite">
            <Pill tone="success">{t('fares.summaryNew', { count: summary.newCount })}</Pill>
            <Pill tone="info">{t('fares.summaryUpdate', { count: summary.updateCount })}</Pill>
            <Pill tone="neutral">
              {t('fares.summaryUnchanged', { count: summary.unchangedCount })}
            </Pill>
            {summary.skippedCount > 0 && (
              <Pill tone="warning">
                {t('fares.summarySkipped', { count: summary.skippedCount })}
              </Pill>
            )}
          </div>

          {activeParsed.unknownAreas.length > 0 && (
            <Notice tone="warning" icon={<AlertTriangle size={18} />}>
              <div className="stack-xs">
                <strong>
                  {t('fares.unknownAreasTitle', {
                    areas: activeParsed.unknownAreas.join(', '),
                  })}
                </strong>
                {canCreateAreas ? (
                  <label className={`check ${createMissingAreas ? 'on' : ''}`}>
                    <input
                      type="checkbox"
                      checked={createMissingAreas}
                      onChange={(e) => setCreateMissingAreas(e.target.checked)}
                    />
                    {t('fares.createMissingAreas', {
                      count: activeParsed.unknownAreas.length,
                    })}
                  </label>
                ) : (
                  <p className="small">{t('fares.unknownAreasSkippedHint')}</p>
                )}
              </div>
            </Notice>
          )}

          {activeParsed.issues.length > 0 && (
            <Notice tone="danger" icon={<AlertTriangle size={18} />}>
              <div className="stack-xs">
                <strong>{t('fares.issuesTitle')}</strong>
                <ul className="fare-import-issues small">
                  {activeParsed.issues.slice(0, 10).map((issue, idx) => (
                    <li key={`${issue.kind}-${issue.row}-${issue.col}-${idx}`}>
                      {formatIssue(issue)}
                    </li>
                  ))}
                </ul>
                {activeParsed.issues.length > 10 && (
                  <p className="small">
                    {t('fares.issuesMore', { count: activeParsed.issues.length - 10 })}
                  </p>
                )}
              </div>
            </Notice>
          )}

          {activeParsed.rows.length === 0 ? (
            activeParsed.issues.length === 0 && (
              <Notice tone="info">{t('fares.importEmpty')}</Notice>
            )
          ) : (
            <>
              {summary.applicablePayload.length === 0 && activeParsed.issues.length === 0 && (
                <Notice tone="info">{t('fares.importNothingToApply')}</Notice>
              )}
              <div className="fare-import-table-wrap">
                <table className="fare-import-table">
                  <thead>
                    <tr>
                      <th scope="col">{t('fares.colRoute')}</th>
                      <th scope="col" className="num">
                        {t('fares.colCurrent')}
                      </th>
                      <th scope="col" className="num">
                        {t('fares.colImported')}
                      </th>
                      <th scope="col">{t('fares.colStatus')}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {summary.sortedRows.map(({ row, effective }) => {
                      const routeLabel =
                        row.areaAName.toLowerCase() === row.areaBName.toLowerCase()
                          ? t('fares.within', { area: row.areaAName })
                          : `${row.areaAName} \u2194 ${row.areaBName}`;
                      return (
                        <tr key={`${row.areaAName}-${row.areaBName}-${row.sourceRow}-${row.sourceCol}`}>
                          <td>{routeLabel}</td>
                          <td className="num">
                            {row.currentCents !== null ? `RM ${toRm(row.currentCents)}` : '\u2014'}
                          </td>
                          <td className="num">{`RM ${toRm(row.amountCents)}`}</td>
                          <td>
                            <Pill tone={STATUS_TONE[effective]}>{statusLabel(effective)}</Pill>
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            </>
          )}
        </div>
      )}
    </Modal>
  );
}
