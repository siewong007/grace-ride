import { beforeEach, describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import { guideStatus, markDone, readDone, stepIds, writeDone } from './guideProgress';
import StepText, { parseStepText } from './components/StepText';
import type { GuideStep } from './pages/HelpGuides';

const step = (title: string, link = ''): GuideStep => ({ title_en: title, title_zh: '', body_en: '', body_zh: '', link });
const A = step('Find the event', '/events');
const B = step('Open the board');
const C = step('Watch it live', '/admin/live');

beforeEach(() => localStorage.clear());

describe('guide progress follows the step, not its position', () => {
  it('keeps ticks on the right steps when steps are reordered or inserted', () => {
    markDone('g', [A, B, C], 0);
    markDone('g', [A, B, C], 2);
    expect(readDone('g', [C, A, B])).toEqual([0, 1]);
    expect(readDone('g', [step('New first step'), A, B, C])).toEqual([1, 3]);
  });

  it('drops ticks for removed steps and never exceeds the step count', () => {
    writeDone('g', [A, B, C], [0, 1, 2]);
    expect(readDone('g', [A, C])).toEqual([0, 1]);
    expect(guideStatus('g', 2)).toMatchObject({ done: 2, total: 2, state: 'finished' });
  });

  it('gives identical steps different ids', () => {
    const ids = stepIds([A, A, A]);
    expect(new Set(ids).size).toBe(3);
  });

  it('converts progress saved by position in an older version', () => {
    localStorage.setItem('graceride_guide_old_done', JSON.stringify([1, 9]));
    expect(readDone('old', [A, B, C])).toEqual([1]);
    expect(localStorage.getItem('graceride_guide_old_ids')).not.toBeNull();
  });
});

describe('step text', () => {
  it('turns Note/Tip/Warning lines into callouts, in English and Chinese', () => {
    expect(parseStepText('Open Events.\nTip: use search\n\u8b66\u544a\uff1a\u522b\u5173\u95ed\u9875\u9762')).toEqual([
      { kind: null, text: 'Open Events.' },
      { kind: 'tip', text: 'use search' },
      { kind: 'warning', text: '\u522b\u5173\u95ed\u9875\u9762' },
    ]);
  });

  it('renders **bold** as bold text and never as HTML', () => {
    const { container } = render(<StepText text={'Press **Save** now <img src=x onerror=alert(1)>'} />);
    expect(screen.getByText('Save').tagName).toBe('STRONG');
    expect(container.querySelector('img')).toBeNull();
  });
});
