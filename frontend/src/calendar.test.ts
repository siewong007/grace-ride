import { describe, expect, it } from 'vitest';
import { formatIcsDate, generateIcs } from './calendar';

describe('Calendar (.ics) generation', () => {
  it('formats dates into iCalendar format', () => {
    const formatted = formatIcsDate('2026-11-15T09:30:00');
    expect(formatted).toMatch(/^20261115T\d{6}$/);
  });

  it('generates a valid VCALENDAR with escaped text', () => {
    const ics = generateIcs({
      title: 'Sunday Worship, Fellowship; & Ride',
      description: 'Pick up at gate.\nBring Bible, notebook.',
      location: 'PJ State CMC, Petaling Jaya',
      startsAt: '2026-11-15T09:30:00',
      durationMinutes: 90,
    });

    expect(ics).toContain('BEGIN:VCALENDAR');
    expect(ics).toContain('VERSION:2.0');
    expect(ics).toContain('BEGIN:VEVENT');
    expect(ics).toContain('SUMMARY:Sunday Worship\\, Fellowship\\; & Ride');
    expect(ics).toContain('DESCRIPTION:Pick up at gate.\\nBring Bible\\, notebook.');
    expect(ics).toContain('LOCATION:PJ State CMC\\, Petaling Jaya');
    expect(ics).toContain('STATUS:CONFIRMED');
    expect(ics).toContain('END:VEVENT');
    expect(ics).toContain('END:VCALENDAR');
  });
});
