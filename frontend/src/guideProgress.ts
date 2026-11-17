/** Which steps of a guide a person has ticked off. Kept in this browser only. */
const doneKey = (slug: string) => `graceride_guide_${slug}_done`;
const feedbackKey = (slug: string) => `graceride_guide_${slug}_fb`;

/** Ticked step positions, ignoring any beyond `total` (a guide may have been shortened). */
export function readDone(slug: string, total = Infinity): number[] {
  try {
    const raw = localStorage.getItem(doneKey(slug));
    const arr: unknown = raw ? JSON.parse(raw) : [];
    if (!Array.isArray(arr)) return [];
    return [...new Set(arr.filter((i): i is number => Number.isInteger(i) && i >= 0 && i < total))];
  } catch {
    return [];
  }
}

export function writeDone(slug: string, done: number[]) {
  try {
    localStorage.setItem(doneKey(slug), JSON.stringify(done));
  } catch {
    // storage unavailable: progress simply is not remembered
  }
}

export function markDone(slug: string, idx: number) {
  const done = readDone(slug);
  if (!done.includes(idx)) writeDone(slug, [...done, idx]);
}

export function readFeedback(slug: string): 'yes' | 'no' | null {
  try {
    const v = localStorage.getItem(feedbackKey(slug));
    return v === 'yes' || v === 'no' ? v : null;
  } catch {
    return null;
  }
}

export function writeFeedback(slug: string, v: 'yes' | 'no') {
  try {
    localStorage.setItem(feedbackKey(slug), v);
  } catch {
    // not remembered
  }
}

/** Where someone is in a guide, safe against guides that have since gained or lost steps. */
export function guideStatus(slug: string, total: number): { done: number; total: number; state: 'new' | 'started' | 'finished' } {
  const t = Math.max(total, 1);
  const done = readDone(slug, t).length;
  return { done, total: t, state: done === 0 ? 'new' : done >= t ? 'finished' : 'started' };
}
