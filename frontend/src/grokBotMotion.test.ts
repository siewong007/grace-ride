import { describe, expect, it } from 'vitest';
import { GROKBOT_MOVES, GROKBOT_SHAPES, coordsToBezier64Path, SHAPE_COORDS } from './grokBotStudio';
import { GROKBOT_MOVE_DURATIONS, GrokBotAnimator, notificationAnchor, staticGrokBotFrame } from './grokBotMotion';

const input = { shape: 'droplet', face: 'neutral', color: 'turquoise', uid: 'gb-t', pointer: null, ambient: true } as const;

describe('Grok Bot motion engine', () => {
  it('places the notification badge on each outline at the reference angle', () => {
    const [x, y] = notificationAnchor('circle');
    expect(x).toBeCloseTo(74.3, 0);
    expect(y).toBeCloseTo(-67.1, 0);
    const [dx, dy] = notificationAnchor('droplet');
    expect(Math.hypot(dx - 41.4, dy + 36.8)).toBeLessThan(6);
  });

  it('plays every move once and settles back to the resting outline', () => {
    for (const move of GROKBOT_MOVES) {
      const a = new GrokBotAnimator(`seed-${move}`);
      a.frame(0, input);
      a.play(move, 0);
      const mid = a.frame(GROKBOT_MOVE_DURATIONS[move] * 500, input);
      expect(mid.eyes).toHaveLength(2);
      a.frame(GROKBOT_MOVE_DURATIONS[move] * 1000 + 50, input);
      expect(a.busy).toBe(false);
      const after = a.frame(GROKBOT_MOVE_DURATIONS[move] * 1000 + 100, input);
      expect(after.back).toBe('');
      expect(after.front).toBe('');
      expect(after.eyeOpacity).toBe(1);
    }
  });

  it('only rebuilds the body path while it changes', () => {
    const a = new GrokBotAnimator('quiet');
    expect(a.frame(0, input).bodyD).toBe(coordsToBezier64Path(SHAPE_COORDS.droplet));
    expect(a.frame(16, input).bodyD).toBeNull();
    a.play('thinking', 32);
    expect(a.frame(200, input).bodyD).not.toBeNull();
  });

  it('builds a still pose for every Move tile', () => {
    for (const shape of GROKBOT_SHAPES) {
      for (const move of GROKBOT_MOVES) {
        const f = staticGrokBotFrame(shape, 'happy', 'blue', move, 'gb-tile');
        expect(f.bodyD).toMatch(/^M.*Z$/);
        expect(f.back).toBe('');
      }
    }
    expect(staticGrokBotFrame('circle', 'neutral', 'blue', 'thinking', 'u').front).toContain('<circle');
    expect(staticGrokBotFrame('circle', 'neutral', 'blue', 'notification', 'u').badge).not.toBeNull();
  });
});
