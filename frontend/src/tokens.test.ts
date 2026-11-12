import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';

describe('Design Tokens & Visual Language (Phase 3)', () => {
  const cssPath = path.resolve(__dirname, 'styles.css');
  const css = fs.readFileSync(cssPath, 'utf-8');

  it('defines all required design tokens in :root', () => {
    const requiredTokens = [
      '--bg',
      '--surface',
      '--surface-2',
      '--border',
      '--text',
      '--muted',
      '--primary',
      '--primary-hover',
      '--primary-ink',
      '--primary-soft',
      '--accent',
      '--accent-ink',
      '--accent-soft',
      '--success',
      '--success-soft',
      '--warning',
      '--warning-soft',
      '--danger',
      '--danger-soft',
      '--info',
      '--info-soft',
    ];
    for (const token of requiredTokens) {
      expect(css).toContain(`${token}:`);
    }
  });

  it('aliases legacy variable names for backwards compatibility', () => {
    expect(css).toMatch(/--blue:\s*var\(--info\)/);
    expect(css).toMatch(/--blue-soft:\s*var\(--info-soft\)/);
    expect(css).toMatch(/--teal:\s*var\(--muted\)/);
    expect(css).toMatch(/--gray-soft:\s*var\(--surface-2\)/);
  });

  it('defines spacing, radius, elevation, motion, and typography scales', () => {
    // Spacing
    for (const s of ['1', '2', '3', '4', '5', '6', '8', '10']) {
      expect(css).toContain(`--space-${s}:`);
    }
    // Radius
    for (const r of ['sm', 'lg', 'pill']) {
      expect(css).toContain(`--radius-${r}:`);
    }
    expect(css).toContain('--radius:');
    // Elevation
    expect(css).toContain('--shadow-sm:');
    expect(css).toContain('--shadow:');
    expect(css).toContain('--shadow-lg:');
    // Motion & Tap
    expect(css).toContain('--dur-fast:');
    expect(css).toContain('--dur:');
    expect(css).toContain('--ease:');
    expect(css).toContain('--tap:');
    // Font scale
    for (const fsSize of ['xs', 'sm', 'md', 'lg', 'xl', '2xl', '3xl']) {
      expect(css).toContain(`--fs-${fsSize}:`);
    }
  });

  it('sets base font-size to 106.25% and configures text scaling', () => {
    expect(css).toContain('font-size: 106.25%');
    expect(css).toContain("html[data-text='large']");
    expect(css).toContain("html[data-text='xlarge']");
    expect(css).toContain('body {');
    expect(css).toMatch(/body\s*\{[^}]*font-size:\s*var\(--fs-md\)/);
  });

  it('contains zero hex colors in any .tsx file in src/', () => {
    const hexPattern = /#[0-9a-fA-F]{3,8}\b/;
    const hits: string[] = [];
    const srcDir = path.resolve(__dirname);

    const scan = (dir: string) => {
      for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
        const full = path.join(dir, entry.name);
        if (entry.isDirectory()) {
          scan(full);
        } else if (entry.isFile() && entry.name.endsWith('.tsx')) {
          const content = fs.readFileSync(full, 'utf-8');
          const lines = content.split('\n');
          for (let i = 0; i < lines.length; i++) {
            if (hexPattern.test(lines[i])) {
              hits.push(`${entry.name}:${i + 1}: ${lines[i].trim()}`);
            }
          }
        }
      }
    };

    scan(srcDir);
    expect(hits).toEqual([]);
  });

  it('uses the logo kit app icon, brand colours and a white launch background', () => {
    const iconSvg = fs.readFileSync(path.resolve(__dirname, '../public/icon.svg'), 'utf-8');
    const kitIcon = fs.readFileSync(
      path.resolve(__dirname, '../../docs/brand/grace-ride-logo-kit/svg/grace-ride-app-icon.svg'),
      'utf-8',
    );
    expect(iconSvg).toBe(kitIcon);

    const indexHtml = fs.readFileSync(path.resolve(__dirname, '../index.html'), 'utf-8');
    expect(indexHtml).toContain('<meta name="theme-color" content="#1F5DB4" />');
    expect(indexHtml).toContain('<div id="splash"');

    const manifest = fs.readFileSync(path.resolve(__dirname, '../public/manifest.webmanifest'), 'utf-8');
    expect(manifest).toContain('"theme_color": "#1F5DB4"');
    expect(manifest).toContain('"background_color": "#FFFFFF"'); // matches the white loading screen

    expect(css).toContain('--brand-red: #E8462F;');
    expect(css).toContain('--brand-blue: #1F5DB4;');
    // The theme follows the logo: Ride blue primary, Grace red accent.
    expect(css).toMatch(/:root\s*\{[^}]*--primary:\s*#1F5DB4;/);
    expect(css).toMatch(/:root\s*\{[^}]*--accent:\s*#D23C25;/);
  });

  it('draws logos with the inline mark, not the app-icon image', () => {
    const scan = (dir: string): string[] =>
      fs.readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
        const full = path.join(dir, e.name);
        if (e.isDirectory()) return scan(full);
        return e.name.endsWith('.tsx') && fs.readFileSync(full, 'utf-8').includes('src="/icon.svg"') ? [e.name] : [];
      });
    expect(scan(path.resolve(__dirname))).toEqual([]);
  });
});
