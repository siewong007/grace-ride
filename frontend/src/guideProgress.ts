/** Which steps of a guide a person has ticked off. Kept in this browser only. */
const doneKey = (slug: string) => `graceride_guide_${slug}_done`;
const feedbackKey = (slug: string) => `graceride_guide_${slug}_fb`;

import type { GuideStep } from './pages/HelpGuides';

const idsKey = (slug: string) => `graceride_guide_${slug}_ids`;

const hash = (str: string) => {
  let h = 5381;
  for (let i = 0; i < str.length; i++) h = ((h * 33) ^ str.charCodeAt(i)) >>> 0;
  return h.toString(36);
};

/**
 * A stable id for each step, from what the step says (titles and link) rather than where it sits.
 * Reordering or inserting steps keeps ticks on the right steps; two identical steps get a suffix.
 */
export function stepIds(steps: GuideStep[]): string[] {
  const seen = new Map<string, number>();
  return steps.map((s) => {
    const explicit = s.id?.trim();
    const base = explicit || hash(`${s.title_en}|${s.title_zh}|${s.link}`);
    const n = seen.get(base) ?? 0;
    seen.set(base, n + 1);
    return n ? `${base}.${n}` : base;
  });
}

function readIds(slug: string): string[] | null {
  try {
    const raw = localStorage.getItem(idsKey(slug));
    if (raw === null) return null;
    const arr: unknown = JSON.parse(raw);
    return Array.isArray(arr) ? arr.filter((x): x is string => typeof x === 'string') : [];
  } catch {
    return null;
  }
}

function writeIds(slug: string, ids: string[]) {
  try {
    localStorage.setItem(idsKey(slug), JSON.stringify(ids));
  } catch {
    // storage unavailable: progress simply is not remembered
  }
}

/** Older versions saved ticked positions. Convert them once, dropping any past the end. */
function legacyPositions(slug: string, total: number): number[] {
  try {
    const raw = localStorage.getItem(doneKey(slug));
    const arr: unknown = raw ? JSON.parse(raw) : [];
    return Array.isArray(arr) ? arr.filter((i): i is number => Number.isInteger(i) && i >= 0 && i < total) : [];
  } catch {
    return [];
  }
}

/** Positions of the steps ticked off, found by step id so edits to the guide do not shift them. */
export function readDone(slug: string, steps: GuideStep[]): number[] {
  const ids = stepIds(steps);
  let stored = readIds(slug);
  if (stored === null) {
    stored = legacyPositions(slug, steps.length).map((i) => ids[i]);
    if (stored.length) writeIds(slug, stored);
  }
  const set = new Set(stored);
  return ids.flatMap((id, i) => (set.has(id) ? [i] : []));
}

export function writeDone(slug: string, steps: GuideStep[], done: number[]) {
  const ids = stepIds(steps);
  writeIds(slug, [...new Set(done)].filter((i) => i >= 0 && i < ids.length).map((i) => ids[i]));
}

export function markDone(slug: string, steps: GuideStep[], idx: number) {
  const done = readDone(slug, steps);
  if (!done.includes(idx)) writeDone(slug, steps, [...done, idx]);
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

/** Where someone is in a guide. Lists have no steps, so this counts saved ids, capped at the step count. */
export function guideStatus(slug: string, total: number): { done: number; total: number; state: 'new' | 'started' | 'finished' } {
  const t = Math.max(total, 1);
  const stored = readIds(slug);
  const done = Math.min(stored ? stored.length : legacyPositions(slug, t).length, t);
  return { done, total: t, state: done === 0 ? 'new' : done >= t ? 'finished' : 'started' };
}
