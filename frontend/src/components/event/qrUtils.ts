/**
 * Extracts the check-in code and optional event ID from a scanned QR string.
 * Handles:
 * - Full URLs: https://domain/events/:id/checkin?code=ABC
 * - Relative URLs: /events/:id/checkin?code=ABC
 * - GRPASS formats: GRPASS:10:code or GRPASS::code
 * - Bare codes: ABC
 */
export function extractCheckinDetails(input: string): { code: string; eventId?: number } {
  const trimmed = input.trim();
  if (!trimmed) {
    return { code: '' };
  }

  // 1. Try URL parsing
  try {
    let url: URL | null = null;
    if (trimmed.startsWith('http://') || trimmed.startsWith('https://')) {
      url = new URL(trimmed);
    } else if (trimmed.startsWith('/') || trimmed.includes('events/') || trimmed.includes('code=')) {
      url = new URL(trimmed.startsWith('/') ? trimmed : `/${trimmed}`, window.location.origin);
    }

    if (url) {
      const code = url.searchParams.get('code');
      const match = url.pathname.match(/\/events\/(\d+)/);
      const eventId = match ? parseInt(match[1], 10) : undefined;
      if (code) {
        return { code: code.trim(), eventId: isNaN(eventId!) ? undefined : eventId };
      }
      if (eventId) {
        return { code: '', eventId };
      }
    }
  } catch {
    // not a valid URL
  }

  // 2. Try GRPASS pass code: GRPASS:<event_id>:<pass_code>
  if (trimmed.startsWith('GRPASS:')) {
    const parts = trimmed.split(':');
    if (parts.length === 3) {
      const eventId = parseInt(parts[1], 10);
      return {
        code: parts[2].trim(),
        eventId: isNaN(eventId) ? undefined : eventId,
      };
    }
    if (parts.length === 2) {
      return { code: parts[1].trim() };
    }
  }

  // 3. Fallback to bare code
  return { code: trimmed };
}
