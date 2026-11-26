/**
 * RFC 5545 iCalendar (.ics) generation helper for events and rides.
 */

import { parseLocal } from './format';

export interface CalendarEvent {
  title: string;
  description?: string;
  location?: string;
  startsAt: string; // Local/ISO string e.g. "2026-11-15T09:30:00" or "2026-11-15 09:30"
  durationMinutes?: number;
}

function pad(n: number): string {
  return n < 10 ? `0${n}` : String(n);
}

function parseIcsDate(raw: string): Date {
  const s = raw.trim();
  if (/^\d{4}-\d{2}-\d{2}([T ]\d{2}:\d{2}(:\d{2}(\.\d+)?)?)?$/.test(s)) {
    return parseLocal(s);
  }
  return new Date(s);
}

function formatLocalIcsDate(d: Date): string {
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

function formatUtcIcsDate(d: Date): string {
  return (
    d.getUTCFullYear() +
    pad(d.getUTCMonth() + 1) +
    pad(d.getUTCDate()) +
    'T' +
    pad(d.getUTCHours()) +
    pad(d.getUTCMinutes()) +
    pad(d.getUTCSeconds()) +
    'Z'
  );
}

export function formatIcsDate(isoStr: string): string {
  const d = parseIcsDate(isoStr);
  if (isNaN(d.getTime())) {
    const digits = isoStr.replace(/\D/g, '');
    if (digits.length >= 8) {
      return `${digits.slice(0, 8)}T${digits.slice(8, 14).padEnd(6, '0')}`;
    }
    return isoStr.replace(/[^A-Za-z0-9]/g, '').slice(0, 15);
  }
  return formatLocalIcsDate(d);
}

function escapeIcsText(text: string): string {
  // Strip ASCII C0 control characters (except CR/LF/TAB) before normalizing line breaks.
  let cleaned = '';
  for (let i = 0; i < text.length; i += 1) {
    const code = text.charCodeAt(i);
    if ((code <= 0x08) || code === 0x0b || code === 0x0c || (code >= 0x0e && code <= 0x1f) || code === 0x7f) {
      continue;
    }
    cleaned += text[i];
  }
  return cleaned
    .replace(/\\/g, '\\\\')
    .replace(/;/g, '\\;')
    .replace(/,/g, '\\,')
    .replace(/\r\n|\r|\n/g, '\\n');
}

export function generateIcs(event: CalendarEvent): string {
  const startDate = parseIcsDate(event.startsAt);
  const dtStart = formatIcsDate(event.startsAt);
  const durationMs = (event.durationMinutes ?? 120) * 60 * 1000;
  const dtEnd = isNaN(startDate.getTime())
    ? dtStart
    : formatLocalIcsDate(new Date(startDate.getTime() + durationMs));

  const uid = `gr-${Date.now()}-${Math.random().toString(36).slice(2, 9)}@graceride.cmc`;

  const lines = [
    'BEGIN:VCALENDAR',
    'VERSION:2.0',
    'PRODID:-//Grace Ride PJ State CMC//EN',
    'CALSCALE:GREGORIAN',
    'METHOD:PUBLISH',
    'BEGIN:VEVENT',
    `UID:${uid}`,
    `DTSTAMP:${formatUtcIcsDate(new Date())}`,
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
