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
        // Tiles are stills without strokes, like the reference picker.
        expect(f.back + f.front).not.toMatch(/stroke=/);
      }
    }
    expect(staticGrokBotFrame('circle', 'neutral', 'blue', 'thinking', 'u').front).toContain('<circle');
    expect(staticGrokBotFrame('circle', 'neutral', 'blue', 'notification', 'u').badge).not.toBeNull();
    // Burst particles sit behind the body, three of them mid-move, as on the reference tile.
    expect(staticGrokBotFrame('droplet', 'neutral', 'blue', 'burst', 'u').back.match(/<circle/g)).toHaveLength(3);
  });

  const at = (move: (typeof GROKBOT_MOVES)[number], ms: number, shape: 'droplet' | 'circle' = 'droplet') => {
    const a = new GrokBotAnimator('fx');
    const inp = { ...input, shape, ambient: false };
    a.frame(0, inp);
    a.play(move, 0);
    for (let t = 16; t <= ms; t += 16) a.frame(t, inp);
    return a.frame(ms, inp);
  };

  it('draws the orbit and swirl rings with their own gradients, behind and in front of the body', () => {
    const orbit = at('orbit', 1200);
    for (let i = 0; i < 6; i++) {
      expect(orbit.back).toContain(`url(#gb-t-rg${i})`);
      expect(orbit.front).toContain(`url(#gb-t-rg${i})`);
    }
    const swirl = at('swirl', 600);
    expect(swirl.back).toContain('url(#gb-t-rg2)');
    expect(swirl.back).not.toContain('url(#gb-t-rg3)');
  });

  it('sends the comet tail round once: behind the body on the way out, in front on the way back', () => {
    const out = at('comet', 950);
    expect(out.back).toContain('url(#gb-t-cm0)');
    expect(out.front).toBe('');
    const back = at('comet', 1550);
    expect(back.front).toContain('url(#gb-t-cm3)');
    expect(at('comet', 2100).back).toBe('');
  });

  it('keeps hidden eyes open while they slide together and fade', () => {
    const rest = at('alert', 0);
    const mid = at('alert', 160);
    expect(mid.eyeOpacity).toBeGreaterThan(0);
    expect(mid.eyeOpacity).toBeLessThan(1);
    const sy = (m: string) => Math.hypot(...m.slice(7, -1).split(',').slice(2, 4).map(Number));
    expect(sy(mid.eyes[0].transform)).toBeCloseTo(sy(rest.eyes[0].transform), 2);
    const tx = (m: string) => Number(m.slice(7, -1).split(',')[4]);
    expect(Math.abs(tx(mid.eyes[0].transform))).toBeLessThan(Math.abs(tx(rest.eyes[0].transform)));
  });

  it('builds the alert and exclamation from the exact reference glyphs on every shape', () => {
    expect(at('alert', 900, 'droplet').bodyD).toBe(at('alert', 900, 'circle').bodyD);
    expect(at('exclamation', 900, 'droplet').front).toContain('r="11.3"');
    expect(at('alert', 900).front).toContain('rotate(17.7) scale(100)');
  });
});
