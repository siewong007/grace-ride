import { useTranslation } from 'react-i18next'
import i18n from './i18n'

/** Translator for format keys; takes interpolation values like `{ n: 5 }`. */
export type FormatT = (key: string, values?: Record<string, unknown>) => string

const globalT: FormatT = (key, values) => String(i18n.t(key, values))

/** Translate a format label key (e.g. 'format:rideStatus.pending_driver'). */
export function useFormatT(): FormatT {
  const { t } = useTranslation()
  return (key, values) => String(t(key, values))
}

export type Tone =
  | 'green'
  | 'amber'
  | 'blue'
  | 'red'
  | 'gray'
  | 'teal'
  | 'neutral'
  | 'success'
  | 'warning'
  | 'danger'
  | 'info'
  | 'primary';

export interface StatusMap {
  label: string // i18n key
  tone: Tone
  emoji?: string
}

export const RIDE_STATUS: Record<string, StatusMap> = {
  pending_driver: { label: 'format:rideStatus.pending_driver', tone: 'amber' },
  pending_passenger: { label: 'format:rideStatus.pending_passenger', tone: 'amber' },
  pending_approval: { label: 'format:rideStatus.pending_approval', tone: 'blue' },
  pending_driver_confirm: { label: 'format:rideStatus.pending_driver_confirm', tone: 'amber' },
  confirmed: { label: 'format:rideStatus.confirmed', tone: 'green' },
  in_progress: { label: 'format:rideStatus.in_progress', tone: 'teal' },
  completed: { label: 'format:rideStatus.completed', tone: 'gray' },
  cancelled: { label: 'format:rideStatus.cancelled', tone: 'gray' },
  no_show: { label: 'format:rideStatus.no_show', tone: 'red' },
  alt_transport: { label: 'format:rideStatus.alt_transport', tone: 'blue' },
};

export const RIDE_FLOW = ['pending_driver', 'pending_passenger', 'pending_approval', 'completed'];

export const EVENT_KIND: Record<string, { label: string; emoji: string }> = {
  badminton: { label: 'format:eventKind.badminton', emoji: '🏸' },
  bowling: { label: 'format:eventKind.bowling', emoji: '🎳' },
  kayak: { label: 'format:eventKind.kayak', emoji: '🛶' },
  walk: { label: 'format:eventKind.walk', emoji: '🚶' },
  other: { label: 'format:eventKind.other', emoji: '✨' },
};

export const SHARE_STATUS: Record<string, StatusMap> = {
  unpaid: { label: 'format:shareStatus.unpaid', tone: 'amber' },
  pending: { label: 'format:shareStatus.pending', tone: 'blue' },
  confirmed: { label: 'format:shareStatus.confirmed', tone: 'green' },
  needs_info: { label: 'format:shareStatus.needs_info', tone: 'red' },
};

export const REIMB_STATUS: Record<string, StatusMap> = {
  pending: { label: 'format:reimbStatus.pending', tone: 'blue' },
  returned: { label: 'format:reimbStatus.returned', tone: 'amber' },
  rejected: { label: 'format:reimbStatus.rejected', tone: 'red' },
  approved: { label: 'format:reimbStatus.approved', tone: 'teal' },
  paid: { label: 'format:reimbStatus.paid', tone: 'green' },
};

export const SETTLEMENT_STATUS: Record<string, StatusMap> = {
  draft: { label: 'format:settlementStatus.draft', tone: 'gray' },
  published: { label: 'format:settlementStatus.published', tone: 'blue' },
  closed: { label: 'format:settlementStatus.closed', tone: 'green' },
};

export const HELP_STATUS: Record<string, StatusMap> = {
  open: { label: 'format:helpStatus.open', tone: 'amber' },
  answered: { label: 'format:helpStatus.answered', tone: 'blue' },
  closed: { label: 'format:helpStatus.closed', tone: 'gray' },
};

export const ROLE_LABEL: Record<string, string> = {
  member: 'format:role.member',
  admin: 'format:role.admin',
  transport: 'format:role.transport',
  finance: 'format:role.finance',
  leader: 'format:role.leader',
  driver: 'format:role.driver',
};

/** Built-in roles are named by the app in the reader's language; custom roles carry
 *  the name their administrator typed (`names`: role key → name). */
export function roleLabel(key: string, ft: (k: string) => string, names?: Record<string, string>): string {
  return ROLE_LABEL[key] ? ft(ROLE_LABEL[key]) : names?.[key] || key;
}

const WEEKDAY = ['format:weekday.0', 'format:weekday.1', 'format:weekday.2', 'format:weekday.3', 'format:weekday.4', 'format:weekday.5', 'format:weekday.6'];

export function parseLocal(s: string): Date {
  // "YYYY-MM-DDTHH:MM[:SS]" in local time
  // Accepts both "T" and space separators.
  const [d, t = '00:00'] = s.trim().split(/[T ]/);
  const [y, m, day] = d.split('-').map(Number);
  const [hh, mm, ss = 0] = t.split(':').map(Number);
  return new Date(y, m - 1, day, hh, mm, ss);
}

export function fmtTime(s: string, tt?: FormatT) {
  const d = parseLocal(s);
  const h = d.getHours();
  const t = tt ?? globalT;
  const period = h < 12 ? t('format:period.am') : h < 18 ? t('format:period.pm') : t('format:period.evening');
  const h12 = h % 12 === 0 ? 12 : h % 12;
  return t('format:clock', { period, time: `${h12}:${String(d.getMinutes()).padStart(2, '0')}` });
}

export function fmtDay(s: string, tt?: FormatT) {
  const d = parseLocal(s);
  const today = new Date();
  const tomorrow = new Date();
  tomorrow.setDate(today.getDate() + 1);
  const same = (a: Date, b: Date) => a.toDateString() === b.toDateString();
  const t = tt ?? globalT;
  if (same(d, today)) return `${t('format:today')} ${t(WEEKDAY[d.getDay()])}`;
  if (same(d, tomorrow)) return `${t('format:tomorrow')} ${t(WEEKDAY[d.getDay()])}`;
  return `${t('format:monthDay', { m: d.getMonth() + 1, d: d.getDate() })} ${t(WEEKDAY[d.getDay()])}`;
}

export const fmtDateTime = (s: string, t?: FormatT) => `${fmtDay(s, t)} · ${fmtTime(s, t)}`;

export function fmtAgo(s: string, tt?: FormatT) {
  const diff = (Date.now() - parseLocal(s).getTime()) / 1000;
  const t = tt ?? globalT;
  if (diff < 60) return t('format:justNow');
  if (diff < 3600) return t('format:minutesAgo', { n: Math.floor(diff / 60) });
  if (diff < 86400) return t('format:hoursAgo', { count: Math.floor(diff / 3600) });
  return fmtDay(s, t);
}

/** Road distance: "850 m", "3.2 km", "12 km". */
export function fmtDistance(m: number, tt?: FormatT) {
  const t = tt ?? globalT;
  if (m < 950) return t('format:meters', { n: Math.max(10, Math.round(m / 10) * 10) });
  return t('format:km', { n: (m / 1000).toFixed(m < 9950 ? 1 : 0) });
}

/** Drive time: "1 min", "25 min", "1 h 5 min". */
export function fmtDuration(s: number, tt?: FormatT) {
  const t = tt ?? globalT;
  const mins = Math.max(1, Math.round(s / 60));
  if (mins < 60) return t('format:minutes', { n: mins });
  return t('format:hoursMinutes', { h: Math.floor(mins / 60), m: mins % 60 });
}

export const rm = (cents: number) => `RM ${(cents / 100).toFixed(2)}`;

/** Parse "28.50" → 2850 cents. Returns NaN for invalid input. */
export function toCents(v: string): number {
  if (!/^\d+(\.\d{0,2})?$/.test(v.trim())) return NaN;
  return Math.round(parseFloat(v) * 100);
}

export const todayStr = () => localDate(new Date());
export function localDate(d: Date) {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}
export const greeting = (t: (k: string) => string) => {
  const h = new Date().getHours();
  return h < 12 ? t('format:morning') : h < 18 ? t('format:afternoon') : t('format:evening');
};

/** Formats tabular data as CSV and triggers a client-side download. */
export function exportToCsv(
  filename: string,
  headers: string[],
  rows: (string | number | null | undefined)[][],
) {
  const escapeCell = (val: unknown): string => {
    if (val === null || val === undefined) return '""';
    const str = String(val).replace(/"/g, '""');
    return `"${str}"`;
  };

  const csvContent = [
    headers.map(escapeCell).join(','),
    ...rows.map((row) => row.map(escapeCell).join(',')),
  ].join('\r\n');

  const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.setAttribute('href', url);
  link.setAttribute('download', `${filename}.csv`);
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}
