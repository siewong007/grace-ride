import { describe, expect, it } from 'vitest';
import { formatIcsDate, generateIcs } from './calendar';

describe('Calendar (.ics) generation', () => {
  it('formats dates into iCalendar format', () => {
    const formatted = formatIcsDate('2026-11-15T09:30:00');
    expect(formatted).toMatch(/^20261115T\d{6}$/);
  });

  it('generates a valid VCALENDAR with escaped text and UTC DTSTAMP', () => {
    const ics = generateIcs({
      title: 'Sunday Worship, Fellowship; & Ride',
      description: 'Pick up at gate.\r\nBring Bible, notebook.',
      location: 'PJ State CMC, Petaling Jaya',
      startsAt: '2026-11-15T09:30:00',
      durationMinutes: 90,
    });

    expect(ics).toContain('BEGIN:VCALENDAR');
    expect(ics).toContain('VERSION:2.0');
    expect(ics).toContain('BEGIN:VEVENT');
    expect(ics).toContain('DTSTART:20261115T093000');
    expect(ics).toContain('DTEND:20261115T110000');
    expect(ics).toMatch(/^DTSTAMP:\d{8}T\d{6}Z$/m);
    expect(ics).toContain('SUMMARY:Sunday Worship\\, Fellowship\\; & Ride');
    expect(ics).toContain('DESCRIPTION:Pick up at gate.\\nBring Bible\\, notebook.');
    expect(ics).toContain('LOCATION:PJ State CMC\\, Petaling Jaya');
    expect(ics).toContain('STATUS:CONFIRMED');
    expect(ics).toContain('END:VEVENT');
    expect(ics).toContain('END:VCALENDAR');
  });

  it('handles space-separated timestamps and invalid dates without throwing RangeError', () => {
    expect(formatIcsDate('2026-11-15 09:30')).toBe('20261115T093000');

    const ics = generateIcs({
      title: 'Evening Cell',
      startsAt: '2026-11-15 09:30',
      durationMinutes: 120,
    });
    expect(ics).toContain('DTSTART:20261115T093000');
    expect(ics).toContain('DTEND:20261115T113000');

    const invalid = generateIcs({
      title: 'Fallback Event',
      startsAt: 'not-a-date',
    });
    expect(invalid).toContain('SUMMARY:Fallback Event');
  });
});
