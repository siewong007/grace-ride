// Writes public/third-party-licenses.txt: the licence text of every production
// dependency that ends up in the browser bundle. MIT, ISC, BSD, OFL (Jost font) and
// Hippocratic (react-leaflet) all require their notice to travel with copies of the code,
// and serving the bundle to members' phones is distribution. Runs as `npm run prebuild`.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const out = path.join(root, 'public', 'third-party-licenses.txt');

const dirs = execFileSync('npm', ['ls', '--omit=dev', '--all', '--parseable'], {
  cwd: root,
  encoding: 'utf8',
})
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
  const licenseFile = fs
    .readdirSync(dir)
    .find((f) => /^(licen[cs]e|copying|notice)(\.|$)/i.test(f));
  const license = typeof pkg.license === 'string' ? pkg.license : pkg.license?.type ?? 'UNKNOWN';
  const text = licenseFile ? fs.readFileSync(path.join(dir, licenseFile), 'utf8').trim() : `(no licence file shipped; package declares ${license})`;
  seen.set(id, { license, text });
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
console.log(`third-party-licenses.txt: ${seen.size} packages`);
