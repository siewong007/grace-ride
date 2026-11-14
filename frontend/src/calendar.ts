/**
 * RFC 5545 iCalendar (.ics) generation helper for events and rides.
 */

export interface CalendarEvent {
  title: string;
  description?: string;
  location?: string;
  startsAt: string; // ISO string e.g. "2026-11-15T09:30:00"
  durationMinutes?: number;
}

function pad(n: number): string {
  return n < 10 ? `0${n}` : String(n);
}

export function formatIcsDate(isoStr: string): string {
  const d = new Date(isoStr);
  if (isNaN(d.getTime())) {
    // If not parseable as Date, strip non-alphanumeric
    return isoStr.replace(/[-:]/g, '').slice(0, 15);
  }
  return (
    d.getFullYear() +
    pad(d.getMonth() + 1) +
    pad(d.getDate()) +
    'T' +
    pad(d.getHours()) +
    pad(d.getMinutes()) +
    pad(d.getSeconds())
  );
}

function escapeIcsText(text: string): string {
  return text
    .replace(/\\/g, '\\\\')
    .replace(/;/g, '\\;')
    .replace(/,/g, '\\,')
    .replace(/\n/g, '\\n');
}

export function generateIcs(event: CalendarEvent): string {
  const dtStart = formatIcsDate(event.startsAt);
  const durationMs = (event.durationMinutes ?? 120) * 60 * 1000;
  const startDate = new Date(event.startsAt);
  const endDate = isNaN(startDate.getTime())
    ? startDate
    : new Date(startDate.getTime() + durationMs);
  const dtEnd = formatIcsDate(endDate.toISOString());

  const uid = `gr-${Date.now()}-${Math.random().toString(36).slice(2, 9)}@graceride.cmc`;

  const lines = [
    'BEGIN:VCALENDAR',
    'VERSION:2.0',
    'PRODID:-//Grace Ride PJ State CMC//EN',
    'CALSCALE:GREGORIAN',
    'METHOD:PUBLISH',
    'BEGIN:VEVENT',
    `UID:${uid}`,
    `DTSTAMP:${formatIcsDate(new Date().toISOString())}`,
    `DTSTART:${dtStart}`,
    `DTEND:${dtEnd}`,
    `SUMMARY:${escapeIcsText(event.title)}`,
  ];

  if (event.description) {
    lines.push(`DESCRIPTION:${escapeIcsText(event.description)}`);
  }
  if (event.location) {
    lines.push(`LOCATION:${escapeIcsText(event.location)}`);
  }

  lines.push('STATUS:CONFIRMED', 'END:VEVENT', 'END:VCALENDAR');

  return lines.join('\r\n');
}

export function downloadIcs(event: CalendarEvent, filename = 'event.ics') {
  const content = generateIcs(event);
  const blob = new Blob([content], { type: 'text/calendar;charset=utf-8' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename.endsWith('.ics') ? filename : `${filename}.ics`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}
