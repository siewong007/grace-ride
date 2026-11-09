# 恩典同行 Grace Ride

An invite-only ride and fellowship web app for PJ State CMC members, built from `Grace_Ride_Chinese.pdf`.

- **Backend:** Rust (Axum 0.8, SQLx with PostgreSQL, webauthn-rs passkeys, TOTP 2FA)
- **Frontend:** React 18, TypeScript and Vite. One responsive UI:
  - **Phone (under 768px):** bottom tab bar, single column, tables shown as cards
  - **Tablet (768–1199px):** icon rail, grids, one column of content up to 1024px
  - **Desktop (1200px and up):** full sidebar with 会员 / 服事 / 管理 sections and multi-column dashboards
- **Map:** Leaflet with OpenStreetMap tiles (free, so no Google Maps bill)

## Run locally

```bash
# 1. Backend (uses the Rust toolchain on the external SSD)
cd backend
cp .env.example .env            # DEV_MODE=true loads demo data
../dev-env.sh cargo run         # API on http://127.0.0.1:8080
```

Requires a local PostgreSQL server. Create the database first (day one):

```bash
createdb grace_ride_dev         # tables + demo data are created on first start
```

```bash
# 2. Frontend (in a second terminal)
cd frontend
npm install
npm run dev                     # open http://localhost:5173
```

Demo accounts (dev mode only). The password for all of them is `GraceRide2026!`.

| Email | Role |
|---|---|
| admin@example.org | 管理员 |
| transport@example.org | 交通组 + 志愿司机 |
| finance@example.org | 财务 |
| driver@example.org | 志愿司机 + 活动负责人 (李德明) |
| driver2@example.org | 志愿司机 |
| member@example.org / amina@example.org / sophie@example.org | 会员 (乘客) |

To reset the demo data, drop and recreate the `grace_ride_dev` database and restart the API.

## Spec coverage

| Spec (page) | Where it lives |
|---|---|
| 三种角色、只看相关资料 (3, 13) | Roles `admin/transport/finance/leader/driver` in `backend/src/auth.rs`. Every route projects data by viewer (`ride_json`, `reimb_json`, settlement views). |
| 邀请制、通行密钥、2FA、账号恢复 (12, 27) | `routes/auth.rs`: invite links, WebAuthn passkeys, TOTP, admin-reviewed one-time recovery link. Privileged roles must use a passkey or 2FA (`require_any`). |
| 乘客报名 → 司机选择 → 乘客接受 → 交通组批准 (4, 16, 28, 36) | `routes/rides.rs`. The atomic claim stops two drivers taking the same request. Seat capacity and carpool rules are enforced. The address is hidden until approval. |
| 司机地区、共乘、空余座位 (17) | Driver settings (`/drive`, 司机设置) and `driver_load` |
| 临时变动 (8, 29) | Driver withdraw → 交通组 reassign → new driver confirms → passenger notified. Also cancel, no-show, 退回, and the 没人接单 → Grab 替代 path. |
| 实时地图 (9, 10, 37) | Location is shared only while a ride is `in_progress`. At most `MAX_LIVE_DRIVERS` (10) drivers share at once. Updates every 15 s. Shown on the passenger's ride page and on 交通组 `/admin/live`. |
| 每趟临时群聊 (18, 37) | Opens on approval. Rides with the same driver, destination and ±60 min share one trip chat. It becomes read-only after completion. |
| Grab 报销 + TNG 人工付款 (19, 20) | `/reimbursements`: claimant → 交通组 approve / return / reject → 财务 verifies payee and records the transaction ID |
| 感谢徽章 (21) | 宵夜王 / 常驻司机 / 爱心同行者 with public criteria. Members can opt out. |
| 活动、指定群体、报名、候补、接送连结、签到 (22–24, 30, 35) | `routes/events.rs`. Visibility comes from groups. Waitlist auto-promotes. Withdrawing cancels linked rides. The signup list is visible only to the organiser. |
| 费用分摊 平分 / 个别 (31–33, 38, 39) | `routes/settlements.rs`. Cent-exact remainder handling. Custom amounts must equal the total. Only attended members can be charged. Payment proof needs payee confirmation. Duplicate references are blocked. Revisions are kept and members notified. |
| 个人月报 (25, 40) | `/report`: attendance by type, rides driven / received, settled costs, badges |
| 公告、帮助中心、通知、操作记录 (11, 15) | Announcements, help tickets, in-app notifications (pushed live over SSE, polled as fallback), audit log at `/admin/audit` |

## Production

One command on a small VM, with automatic HTTPS through Caddy:

```bash
cp deploy/.env.production.example .env.production   # set DOMAIN, PUBLIC_URL, ADMIN_EMAIL
docker compose --env-file .env.production up -d --build
```

| Document | What it covers |
|---|---|
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | System shape, design decisions and when to revisit them, ride state machine, roadmap |
| [docs/SECURITY.md](docs/SECURITY.md) | Threat model, every control and where it lives, accepted risks, pre-launch checklist |
| [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md) | First deploy, updates, backups and restore drill, monitoring |

Quality gates (also run in CI on every push): `make lint test audit`.

Schema lives in a **single** file, `backend/migrations/0001_init.sql`. Edit that file while data is disposable (then reset the DB); once production has real members, switch to additive `0002+` migrations — see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

Before launch, the church still needs a PDPA privacy notice, approved retention periods, and decisions on the open questions on page 42: who collects, absentee rules, subsidy approval, refund handling.
