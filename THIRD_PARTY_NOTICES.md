# Third-party notices

Grace Ride itself is proprietary (see [LICENSE](LICENSE)). It is built on open-source components that keep their own licences.

## Web app (shipped to members' browsers)

Serving the JavaScript bundle and fonts to members' phones counts as distributing them, so their licence notices must go with them. `npm run build` runs `frontend/scripts/third-party-licenses.mjs` first. That script writes every production dependency's licence text to `/third-party-licenses.txt`. The file is generated, not committed, and the Terms page links to it.

Licence tally of the 77 production packages on 17 November 2026:

| Licence | Packages | Notes |
|---|---|---|
| MIT, ISC, BSD-2-Clause, Apache-2.0, MIT AND Zlib | 72 | Permissive. Keep the notice. |
| MPL-2.0 OR Apache-2.0 | 1 (dompurify) | Used under Apache-2.0. |
| OFL-1.1 | 1 (@fontsource/jost) | The font may be bundled but not sold on its own. The licence must accompany it. |
| Hippocratic-2.1 | 2 (react-leaflet, @react-leaflet/core) | **Not an OSI open-source licence.** Use must comply with human-rights laws and principles; it includes an indemnity and an arbitration clause. Acceptable for a church ride app, but if counsel objects, replace react-leaflet with plain `leaflet` (BSD-2-Clause). |

## Server (Rust)

The backend runs on the church's own server and is not distributed to members, so the notice duty applies only if the binary or Docker image is given to someone else. Of the 364 crates in `Cargo.lock`, all are permissive (MIT/Apache-2.0/BSD/ISC/Zlib/Unicode) except:

- `webauthn-rs` and related crates, which are MPL-2.0. This is file-level copyleft: if you modify those crates' files and distribute them, publish the modified files. Using them unmodified creates no duty beyond keeping the notice.
- `r-efi` (MIT OR Apache-2.0 OR LGPL-2.1+), used under MIT. It is a UEFI-only target dependency and is not built for Linux.

No GPL or AGPL dependency is present.

## Map data and services

- Map tiles, geocoding and routing come from OpenStreetMap data under the [Open Database License](https://www.openstreetmap.org/copyright). The map shows "© OpenStreetMap contributors" with a link, as the ODbL requires.
- See [Compliance](docs/COMPLIANCE.md#third-party-services) for the usage policies of the public tile, Nominatim, OSRM and Photon servers.

## Fonts and brand

- Jost (OFL-1.1) is bundled. Noto Sans SC is a system fallback and is not bundled.
- The Grace Ride mark is original work. Its blue arc is taken from the PJ State CMC church logo, so it needs the church's written permission. See [Compliance](docs/COMPLIANCE.md#copyright-and-brand).

## Re-checking

```bash
cd frontend && node scripts/third-party-licenses.mjs
```

Re-run this and update this file whenever dependencies change. Flag any new GPL, AGPL, SSPL or unknown licence before you merge.
