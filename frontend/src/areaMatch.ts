/**
 * Infers the best matching active pickup area ID from a street address or autocomplete label
 * by matching normalized active area names (longest match wins).
 */
export function inferAreaId(
  address: string,
  areas: { id: number; name: string }[],
): number | null {
  const normAddr = address.trim().toLowerCase().replace(/\s+/g, ' ');
  if (!normAddr) return null;

  const sorted = [...areas]
    .map((a) => ({ id: a.id, norm: a.name.trim().toLowerCase().replace(/\s+/g, ' ') }))
    .filter((a) => a.norm.length >= 2)
    .sort((a, b) => b.norm.length - a.norm.length);

  for (const area of sorted) {
    if (/^[a-z0-9\s-]+$/i.test(area.norm)) {
      const escaped = area.norm.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      const re = new RegExp(`(^|[^a-z0-9])${escaped}($|[^a-z0-9])`, 'i');
      if (re.test(normAddr)) return area.id;
    } else if (normAddr.includes(area.norm)) {
      return area.id;
    }
  }
  return null;
}
