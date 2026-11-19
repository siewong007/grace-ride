// Writes public/third-party-licenses.txt: the licence text of every production
// dependency that ends up in the browser bundle. MIT, ISC, BSD, OFL (Jost font) and
// Hippocratic (react-leaflet) all require their notice to travel with copies of the code,
// and serving the bundle to members' phones is distribution. Runs as `npm run prebuild`.
// Also writes the same data as public/third-party-licenses.json, which the /licences page
// reads to show it as a styled list instead of a wall of text.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const out = path.join(root, 'public', 'third-party-licenses.txt');
const outJson = path.join(root, 'public', 'third-party-licenses.json');

// `npm ls` exits non-zero when node_modules is stale or has peer conflicts, but it still
// prints the tree on stdout, so fall back to the error's stdout instead of failing the build.
let tree;
try {
  tree = execFileSync('npm', ['ls', '--omit=dev', '--all', '--parseable'], {
    cwd: root,
    encoding: 'utf8',
  });
} catch (err) {
  tree = err.stdout ?? '';
  if (!tree) throw err;
}

const dirs = tree
  .split('\n')
  .map((s) => s.trim())
  .filter((d) => d && d !== root);

const seen = new Map();
for (const dir of dirs) {
  const pkgFile = path.join(dir, 'package.json');
  if (!fs.existsSync(pkgFile)) continue;
  const pkg = JSON.parse(fs.readFileSync(pkgFile, 'utf8'));
  const id = `${pkg.name}@${pkg.version}`;
  if (seen.has(id)) continue;
  const license = typeof pkg.license === 'string' ? pkg.license : pkg.license?.type ?? 'UNKNOWN';
  // Packages name the file many ways: LICENSE, LICENCE.md, LICENSE-MIT.txt, COPYING.LESSER,
  // NOTICE. Match the stem plus any suffix and read every variant, because a dual-licensed
  // package ships one file per licence.
  const licenseFiles = fs
    .readdirSync(dir)
    .filter((f) => /^(licen[cs]e|copying|notice)([-._].*)?$/i.test(f))
    .filter((f) => fs.statSync(path.join(dir, f)).isFile())
    .sort();
  const texts = [
    ...new Set(licenseFiles.map((f) => fs.readFileSync(path.join(dir, f), 'utf8').trim())),
  ];
  const text = texts.length ? texts.join('\n\n') : `(no licence file shipped; package declares ${license})`;
  seen.set(id, { name: pkg.name, version: pkg.version, license, text });
}

const header = `Grace Ride (恩典同行) — third-party software notices
Grace Ride itself is proprietary; see LICENSE in the source repository.
The web app includes the open-source packages below. Map data © OpenStreetMap contributors,
available under the Open Database License (https://www.openstreetmap.org/copyright).
`;
const body = [...seen]
  .sort(([a], [b]) => a.localeCompare(b))
  .map(([id, { license, text }]) => `${'='.repeat(78)}\n${id} — ${license}\n${'='.repeat(78)}\n${text}\n`)
  .join('\n');
fs.writeFileSync(out, `${header}\n${body}`);

const packages = [...seen]
  .sort(([a], [b]) => a.localeCompare(b))
  .map(([, { name, version, license, text }]) => ({ name, version, license, text }));
fs.writeFileSync(outJson, `${JSON.stringify(packages, null, 2)}\n`);

console.log(`third-party-licenses.txt / .json: ${packages.length} packages`);
