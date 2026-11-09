//! Deterministic demo / edge-case data generation for Grace Ride (恩典同行).
//!
//! Server start (dev mode): `seed::run` loads the core demo fixtures once so
//! every screen has something to show. The `seed` CLI subcommand (see `main`)
//! offers `demo [scale]`, `edge`, `reset` and `verify`.
//!
//! Design rules:
//! - Dev mode only (localhost). Production creates the first administrator only.
//! - Every seeded row carries `demo_batch = DEMO_BATCH` / `EDGE_BATCH`, so
//!   `seed reset` deletes only demo rows and never touches real member data.
//! - Generation is deterministic: fixed PRNG seed, stable IDs, fixed synthetic
//!   name lists, KL-time (UTC+8) timestamps. Running twice gives identical data.
//! - All names, phones, emails and addresses are obviously synthetic
//!   (@example.org is reserved for examples, RFC 2606); no real church member
//!   data is ever used.
//! - No real passkey private keys, SMS, email, GPS streaming or payment transfers.

use chrono::{Datelike, Duration, FixedOffset, NaiveDate, Timelike, Weekday};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashSet;
use uuid::Uuid;

use crate::routes::auth::{hash_password, totp_for};
use crate::state::Config;
use crate::util::{audit, in_days, now, now_kl, random_token, today_kl};

pub const DEMO_PASSWORD: &str = "GraceRide2026!";
pub const DEMO_BATCH: &str = "demo-2026";
pub const EDGE_BATCH: &str = "edge-2026";
const PRNG_SEED: u64 = 0x67_72_61_63_65_5f_73_65_65_64;
const KL: FixedOffset = FixedOffset::east(28800);
const GENERATED_MEMBER_ID_BASE: i64 = 10001;
const GENERATED_EVENT_ID_BASE: i64 = 1001;
const GENERATED_RIDE_ID_BASE: i64 = 30001;
const SIMULATED_TOTP_SECRET: &str = "MFRGGZDF73DGXYP3";
const LEGACY_DEMO_EMAILS: &[&str] = &[
    "admin@example.org", "transport@example.org", "finance@example.org",
    "driver@example.org", "driver2@example.org", "member@example.org",
    "amina@example.org", "sophie@example.org", "yawen@example.org",
    "zhiqiang@example.org", "meiling@example.org", "enci@example.org",
    "guorong@example.org",
];
const GIVEN_NAMES: &[&str] = &[
    "Ah Mei", "Demo", "Grace", "Kumar", "Rahman", "Siti", "Wei", "Ling",
    "Sample", "Fictional", "PJ", "Ride", "Member", "Volunteer", "Trial",
    "Demo Ana", "Grace Ridzuan", "Daniel Sample",
];
const FAMILY_NAMES: &[&str] = &[
    "Tan", "Lim", "Member", "Rider", "Driver", "Kumar", "Rahman", "Ali",
    "Demo", "Sample", "Test", "Fictional", "Raja", "Nair",
];
const ADDRESSES: &[&str] = &[
    "12 Jalan Demo, SS2/24, 47400 Petaling Jaya (SS2)",
    "No 45, Jalan PJ Selamat 1, 八打灵再也 PJ",
    "Lot 123, Jalan Kelana Jaya 1, Kelana Jaya",
    "No 88, Jalan SS2/72, SS2",
    "9 Jalan SS15/4, Subang Jaya (梳邦再也)",
    "Taman Demo, Jalan SS2/38, SS2",
    "No 201, Jalan SS2/102, SS2",
    "Pangsiran Demo, Kelana Jaya",
    "4 Jalan SS2/80, SS2",
    "No 15, Jalan PJ Selamat 3, 八打灵再也 PJ",
    "18 Jalan Demo Utara, Kelana Jaya",
    "No 77, Jalan SS2/55, SS2",
];
const VEHICLE_MODELS: &[&str] = &[
    "Perodua Myvi", "Honda City", "Toyota Vios", "Mazda 3", "Proton X50",
    "Nissan Almera", "Perodua Axia", "Toyota Innova",
];
const VEHICLE_COLORS: &[&str] = &["白色", "银色", "灰色", "蓝色", "红色", "黑色"];
const VEHICLE_PLATE_PREFIXES: &[&str] = &["W", "B", "V", "F", "S"];
const DESTINATIONS: &[&str] = &[
    "PJ State CMC 教会", "社区中心礼堂", "诊所入口", "PJ 室内羽球馆",
    "图书馆入口", "宵夜团契", "Sunway Pyramid 保龄球馆", "Putrajaya 湖畔",
    "Taman Jaya 公园入口", "北门入口",
];
const AREA_NAMES: &[&str] = &[
    "八打灵再也 PJ", "SS2", "新市", "梳邦再也", "白沙罗", "蒲种",
    "吉隆坡市中心", "Kelana Jaya",
];

fn kl(dt: NaiveDate, h: i32, m: i32) -> String {
    format!("{}T{:02}:{:02}", dt, h, m)
}

fn next_weekday(start: NaiveDate, wd: Weekday) -> NaiveDate {
    let mut d = start + Duration::days(1);
    while d.weekday() != wd {
        d += Duration::days(1);
    }
    d
}

struct Rng {
    inner: StdRng,
}

impl Rng {
    fn new() -> Self {
        Self { inner: StdRng::seed_from_u64(PRNG_SEED) }
    }
    fn pick<T: Clone>(&mut self, vals: &[T]) -> T {
        let idx = (self.inner.gen::<u32>() as usize) % vals.len();
        vals[idx].clone()
    }
    fn bool(&mut self, p: f64) -> bool {
        self.inner.gen_bool(p)
    }
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.inner.gen::<u32>() as i64 % ((hi - lo) as u32)) as i64
    }
    fn shuffle<T>(&mut self, vals: &mut [T]) {
        let n = vals.len();
        for i in (0..n).rev() {
            let j = (self.inner.gen::<u32>() as usize) % (i + 1);
            vals.swap(i, j);
        }
    }
    fn unique_plate(&mut self, used: &mut HashSet<String>) -> String {
        loop {
            let letters = (0..3)
                .map(|_| (b'A' + (self.inner.gen::<u32>() % 26) as u8) as char)
                .collect();
            let digits = 1000 + (self.inner.gen::<u32>() % 9000);
            let plate = format!("{} {} {}", self.pick(VEHICLE_PLATE_PREFIXES), letters, digits);
            if used.insert(plate.clone()) {
                return plate;
            }
        }
    }
}
fn stable_uuid(id: i64) -> String {
    Uuid::from_u128(0x67_72_61_63_65_5f_64_65_6d_6f_00_00_00_00 + id as u128).to_string()
}

struct Counts {
    pub users: i64,
    pub invites: i64,
    pub vehicles: i64,
    pub events: i64,
    pub signups: i64,
    pub rides: i64,
    pub messages: i64,
    pub locations: i64,
    pub notifications: i64,
    pub reimbursements: i64,
    pub settlements: i64,
}

pub async fn run(db: &PgPool, cfg: &Config) -> anyhow::Result<()> {
    insert_reference_data(db).await?;
    if !cfg.dev_mode {
        return seed_admin_only(db, cfg).await;
    }
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE demo_batch = $1")
        .bind(DEMO_BATCH)
        .fetch_one(db)
        .await?;
    if n == 0 {
        seed_core(db).await?;
    }
    Ok(())
}


/// Canonical reference data (areas + groups). Safe to re-run: uses ON CONFLICT.
async fn insert_reference_data(db: &PgPool) -> anyhow::Result<()> {
    for name in AREA_NAMES {
        sqlx::query("INSERT INTO areas (name, demo_batch) VALUES ($1, NULL) ON CONFLICT (name) DO NOTHING")
            .bind(name)
            .execute(db)
            .await?;
    }
    sqlx::query("INSERT INTO groups (name, description, is_all, demo_batch) VALUES ('全体会员', '所有获批准的会员', 1, NULL) ON CONFLICT (name) DO NOTHING").execute(db).await?;
    sqlx::query("INSERT INTO groups (name, description, demo_batch) VALUES ('青年团契', 'PJ State CMC 青年团契', NULL) ON CONFLICT (name) DO NOTHING").execute(db).await?;
    sqlx::query("INSERT INTO groups (name, description, demo_batch) VALUES ('运动兴趣小组', '羽球、保龄球、皮划艇与散步', NULL) ON CONFLICT (name) DO NOTHING").execute(db).await?;
    Ok(())
}

/// Production path: first administrator + one-time recovery link, no demo data.
/// Existing members are left alone. A second insert would fail the unique email
/// and stop the process on every restart.
async fn seed_admin_only(db: &PgPool, cfg: &Config) -> anyhow::Result<()> {
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users").fetch_one(db).await?;
    if n > 0 {
        return Ok(());
    }
    let email = std::env::var("ADMIN_EMAIL").unwrap_or_else(|_| "admin@example.org".into());
    let id = sqlx::query_as::<_, (i64,)>("INSERT INTO users (name, email, roles, webauthn_uuid, demo_batch) VALUES ('管理员', $1, 'admin', $2, NULL) RETURNING id")
        .bind(&email).bind(stable_uuid(0)).fetch_one(db).await?.0;
    let token = random_token(32);
    sqlx::query("INSERT INTO invites (token, kind, email, name, roles, group_ids, user_id, created_by, expires_at, demo_batch) VALUES ($1, 'recovery', $2, '管理员', 'admin', '[]', $3, $3, $4, NULL)")
        .bind(&token).bind(&email).bind(id).bind(in_days(2)).execute(db).await?;
    tracing::warn!("First administrator created for {email}. Open this one-time link to register a passkey: {}/invite/{token}", cfg.public_url);
    Ok(())
}

/// Insert one demo user with a stable id; returns the id.
fn demo_user<'a>(
    _rng: &mut Rng,
    id: i64,
    name: &'a str,
    email: &'a str,
    phone: &'a str,
    roles: &'a str,
    status: &'a str,
    batch: &'a str,
    pw: &str,
) -> sqlx::query::Query<'a, sqlx::Postgres, (i64,)> {
    let active = status == "active";
    let password_hash = if active { Some(pw.to_string()) } else { None };
    sqlx::query_as::<_, (i64,)>(
        "INSERT INTO users (id, name, email, phone, roles, status, password_hash, webauthn_uuid, totp_secret, totp_enabled, demo_batch, created_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) RETURNING id",
    )
    .bind(id)
    .bind(name)
    .bind(email)
    .bind(phone)
    .bind(roles)
    .bind(status)
    .bind(password_hash)
    .bind(stable_uuid(id))
    .bind(Option::<String>::None)
    .bind(0i64)
    .bind(batch)
    .bind(now_kl())
}

/// Seed one `users` row and optional TOTP/passkey simulation; returns id.
async fn insert_member(
    db: &PgPool,
    rng: &mut Rng,
    id: i64,
    name: &str,
    email: &str,
    phone: &str,
    roles: &str,
    status: &str,
    batch: &str,
    pw: &str,
    totp: bool,
    passkey: bool,
) -> i64 {
    let (id,): (i64,) = demo_user(rng, id, name, email, phone, roles, status, batch, pw)
        .fetch_one(db)
        .await
        .unwrap();
    if totp {
        sqlx::query("UPDATE users SET totp_secret=$1, totp_enabled=1 WHERE id=$2")
            .bind(SIMULATED_TOTP_SECRET)
            .bind(id)
            .execute(db)
            .await
            .unwrap();
    }
    if passkey {
        // SIMULATED passkey record: cred_id and payload are fabricated for demo
        // display only. They cannot complete a real WebAuthn ceremony.
        sqlx::query("INSERT INTO passkeys (id, user_id, cred_id, passkey, label, demo_batch, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7)")
            .bind(id)
            .bind(id)
            .bind(format!("sim-cred-{id:05}"))
            .bind(json!({"demo": true, "note": "SIMULATED passkey record - not a real credential", "id": id}).to_string())
            .bind(format!("SIMULATED - device {id}"))
            .bind(batch)
            .bind(now_kl())
            .execute(db)
            .await
            .unwrap();
    }
    id
}

/// Original core demo fixtures (demo accounts 1..=13, events 1..=7, rides 20001..=20009).
/// Reproduces the pre-existing seed content so current integration tests keep
/// passing, now stamped with demo_batch and given stable ids.
async fn seed_core(db: &PgPool) -> anyhow::Result<()> {
    let pw = hash_password(DEMO_PASSWORD).map_err(|e| anyhow::anyhow!("{e}"))?;
    let today = today_kl();

    // --- core demo users (ids 1..=13) ---
    let core = [
        (1, "管理员 Grace", "admin@example.org", "012-300 0001", "admin", "active"),
        (2, "黄美仪", "transport@example.org", "012-300 0002", "transport,driver", "active"),
        (3, "林慧君", "finance@example.org", "012-300 0003", "finance", "active"),
        (4, "李德明", "driver@example.org", "012-300 0004", "driver,leader", "active"),
        (5, "张家乐", "driver2@example.org", "012-300 0005", "driver", "active"),
        (6, "陈法拉", "member@example.org", "012-300 0006", "", "active"),
        (7, "阿米娜", "amina@example.org", "012-300 0007", "", "active"),
        (8, "林苏菲", "sophie@example.org", "012-300 0008", "", "active"),
        (9, "周雅文", "yawen@example.org", "012-300 0011", "", "active"),
        (10, "吴志强", "zhiqiang@example.org", "012-300 0012", "driver", "active"),
        (11, "郑美玲", "meiling@example.org", "012-300 0013", "", "active"),
        (12, "许恩慈", "enci@example.org", "012-300 0014", "leader", "active"),
        (13, "陈国荣", "guorong@example.org", "012-300 0015", "driver,transport", "active"),
    ];
    let mut id_of = std::collections::HashMap::<&str, i64>::new();
    for (id, name, email, phone, roles, status) in core {
        let uid = insert_member(db, &mut Rng::new(), id, name, email, phone, roles, status, DEMO_BATCH, &pw, false, false).await;
        id_of.insert(name, uid);
    }
    let get = |dbg: &str, map: &std::collections::HashMap<&str, i64>| *map.get(dbg).unwrap_or_else(|| panic!("unknown core member {dbg}"));
    let admin = get("管理员 Grace", &id_of);
    let transport = get("黄美仪", &id_of);
    let finance = get("林慧君", &id_of);
    let deming = get("李德明", &id_of);
    let jiale = get("张家乐", &id_of);
    let fala = get("陈法拉", &id_of);
    let amina = get("阿米娜", &id_of);
    let sophie = get("林苏菲", &id_of);
    let yawen = get("周雅文", &id_of);
    let zhiqiang = get("吴志强", &id_of);
    let meiling = get("郑美玲", &id_of);
    let enci = get("许恩慈", &id_of);
    let guorong = get("陈国荣", &id_of);

    // --- group memberships ---
    for (uid, g) in [
        (deming, 2), (deming, 3), (fala, 2), (fala, 3), (amina, 3),
        (sophie, 2), (jiale, 3), (transport, 2), (yawen, 2),
        (meiling, 2), (meiling, 3), (enci, 2), (zhiqiang, 3), (guorong, 3),
    ] {
        sqlx::query("INSERT INTO group_members (group_id, user_id, demo_batch) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING")
            .bind(g).bind(uid).bind(DEMO_BATCH).execute(db).await?;
    }

    // --- vehicles + driver preferences ---
    sqlx::query("INSERT INTO vehicles (user_id, model, color, plate, seats, demo_batch) VALUES ($1,'Perodua Myvi','白色','VXX 1234',5,$2) ON CONFLICT(user_id) DO UPDATE SET model=excluded.model, color=excluded.color, plate=excluded.plate, seats=excluded.seats, demo_batch=excluded.demo_batch")
        .bind(deming).bind(DEMO_BATCH).execute(db).await?;
    sqlx::query("INSERT INTO vehicles (user_id, model, color, plate, seats, demo_batch) VALUES ($1,'Honda City','银色','WQA 8821',5,$2) ON CONFLICT(user_id) DO UPDATE SET model=excluded.model, color=excluded.color, plate=excluded.plate, seats=excluded.seats, demo_batch=excluded.demo_batch")
        .bind(jiale).bind(DEMO_BATCH).execute(db).await?;
    sqlx::query("INSERT INTO vehicles (user_id, model, color, plate, seats, demo_batch) VALUES ($1,'Proton X50','灰色','BPM 3302',5,$2) ON CONFLICT(user_id) DO UPDATE SET model=excluded.model, color=excluded.color, plate=excluded.plate, seats=excluded.seats, demo_batch=excluded.demo_batch")
        .bind(transport).bind(DEMO_BATCH).execute(db).await?;
    sqlx::query("INSERT INTO vehicles (user_id, model, color, plate, seats, demo_batch) VALUES ($1,'Toyota Vios','蓝色','BKS 4410',5,$2) ON CONFLICT(user_id) DO UPDATE SET model=excluded.model, color=excluded.color, plate=excluded.plate, seats=excluded.seats, demo_batch=excluded.demo_batch")
        .bind(zhiqiang).bind(DEMO_BATCH).execute(db).await?;
    sqlx::query("INSERT INTO vehicles (user_id, model, color, plate, seats, demo_batch) VALUES ($1,'Mazda 3','红色','VBE 7788',5,$2) ON CONFLICT(user_id) DO UPDATE SET model=excluded.model, color=excluded.color, plate=excluded.plate, seats=excluded.seats, demo_batch=excluded.demo_batch")
        .bind(guorong).bind(DEMO_BATCH).execute(db).await?;
    sqlx::query("UPDATE users SET carpool_enabled=1, spare_seats=2 WHERE id=$1").bind(deming).execute(db).await?;
    sqlx::query("UPDATE users SET carpool_enabled=0, spare_seats=1 WHERE id=$1").bind(jiale).execute(db).await?;
    sqlx::query("UPDATE users SET carpool_enabled=1, spare_seats=3 WHERE id=$1").bind(zhiqiang).execute(db).await?;
    for (uid, area) in [
        (deming, 1), (deming, 2), (deming, 4), (jiale, 3), (jiale, 6),
        (transport, 1), (transport, 5), (zhiqiang, 1), (zhiqiang, 5),
        (guorong, 2), (guorong, 7),
    ] {
        sqlx::query("INSERT INTO driver_areas (user_id, area_id, demo_batch) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING")
            .bind(uid).bind(area).bind(DEMO_BATCH).execute(db).await?;
    }
    Ok(())
    let tomorrow = today + Duration::days(1);
    let kl_hm = |d: NaiveDate, h: i32, m: i32| kl(d, h, m);

    // --- events (ids 1..=7) ---
    let events = [
        (1i64, "周四羽毛球局", "badminton", "轻松双打交流，欢迎初学者。", kl_hm(next_weekday(today, Weekday::Thu), 19, 30), "PJ 室内羽球馆", 12, "场租及羽球活动后平均分摊", deming, vec![2i64, 3]),
        (2, "周末保龄球", "bowling", "周末保龄球活动。", kl_hm(next_weekday(today, Weekday::Sat), 14, 0), "Sunway Pyramid 保龄球馆", 8, "每局约 RM12，自付", deming, vec![3]),
        (3, "皮划艇体验", "kayak", "户外划艇体验，请穿运动服。", kl_hm(next_weekday(today, Weekday::Sat), 8, 0), "Putrajaya 湖畔", 10, "器材租借费活动后分摊", deming, vec![2]),
        (4, "公园散步", "walk", "轻松结伴散步，一起享受早晨。", kl_hm(next_weekday(today, Weekday::Sun), 7, 0), "Taman Jaya 公园入口", 0, "免费", deming, vec![1]),
        (5, "上周羽毛球局", "badminton", "双打交流（已结束，用于费用分摊示例）", kl_hm(today - Duration::days(5), 19, 30), "PJ 室内羽球馆", 12, "场租 RM120 + 羽球 RM48", deming, vec![3]),
        (6, "主日崇拜接送", "other", "主日崇拜前后的接送安排。", kl_hm(next_weekday(today, Weekday::Sun), 9, 0), "PJ State CMC 教会", 0, "免费", transport, vec![1]),
        (7, "团契聚餐", "other", "每人带一道菜，交通组协助接送。", kl_hm(next_weekday(today, Weekday::Fri), 18, 30), "社区中心礼堂", 20, "自备食物", enci, vec![2]),
    ];
    for (eid, title, kind, desc, at, loc, cap, cost, by, groups) in events {
        let deadline = kl(at.date_naive(), 12, 0);
        let status = if eid == 5 { "finished" } else { "published" };
        sqlx::query_as::<_, (i64,)>("INSERT INTO events (id, title, kind, description, starts_at, location, capacity, cost_note, created_by, signup_deadline, status, demo_batch) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) RETURNING id")
            .bind(eid).bind(title).bind(kind).bind(desc).bind(&at).bind(loc).bind(cap).bind(cost).bind(by).bind(&deadline).bind(status).bind(DEMO_BATCH).fetch_one(db).await?;
        for g in groups {
            sqlx::query("INSERT INTO event_groups (event_id, group_id, demo_batch) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING")
                .bind(eid).bind(g).bind(DEMO_BATCH).execute(db).await?;
        }
    }
    // --- event signups ---
    sqlx::query("INSERT INTO event_signups (event_id, user_id, status, need_ride, offer_drive, offer_seats, demo_batch) VALUES (1,$1,'going',1,0,0,$2) ON CONFLICT(event_id,user_id) DO UPDATE SET status='going', need_ride=1, offer_drive=0, offer_seats=0, demo_batch=excluded.demo_batch")
        .bind(fala).bind(DEMO_BATCH).execute(db).await?;
    sqlx::query("INSERT INTO event_signups (event_id, user_id, status, need_ride, offer_drive, offer_seats, demo_batch) VALUES (1,$1,'going',0,1,2,$2) ON CONFLICT(event_id,user_id) DO UPDATE SET status='going', need_ride=0, offer_drive=1, offer_seats=2, demo_batch=excluded.demo_batch")
        .bind(deming).bind(DEMO_BATCH).execute(db).await?;
    for uid in [deming, fala, amina, jiale, transport, sophie] {
        sqlx::query("INSERT INTO event_signups (event_id, user_id, status, need_ride, offer_drive, offer_seats, attended, demo_batch) VALUES (5,$1,'going',0,0,0,1,$2) ON CONFLICT(event_id,user_id) DO UPDATE SET status='going', attended=1, demo_batch=excluded.demo_batch")
            .bind(uid).bind(DEMO_BATCH).execute(db).await?;
    }
    sqlx::query("INSERT INTO event_signups (event_id, user_id, status, need_ride, offer_drive, offer_seats, demo_batch) VALUES (6,$1,'going',1,0,0,$2) ON CONFLICT(event_id,user_id) DO NOTHING")
        .bind(yawen).bind(DEMO_BATCH).execute(db).await?;
    sqlx::query("INSERT INTO event_signups (event_id, user_id, status, need_ride, offer_drive, offer_seats, demo_batch) VALUES (6,$1,'going',0,1,3,$2) ON CONFLICT(event_id,user_id) DO NOTHING")
        .bind(zhiqiang).bind(DEMO_BATCH).execute(db).await?;
    for uid in [fala, yawen, meiling, enci] {
        sqlx::query("INSERT INTO event_signups (event_id, user_id, status, need_ride, offer_drive, offer_seats, demo_batch) VALUES (7,$1,'going',1,0,0,$2) ON CONFLICT(event_id,user_id) DO NOTHING")
            .bind(uid).bind(DEMO_BATCH).execute(db).await?;
    }
    // --- rides (ids 20001..=20009) ---
    let rides = [
        (20001i64, fala, None::<i64>, kl_hm(tomorrow, 8, 30), 2, "SS2, Jalan 21/1 门牌 12", "社区中心 · 北门入口", 1, "", "pending_driver"),
        (20002, sophie, None, kl_hm(tomorrow, 9, 15), 1, "PJ Section 14 公寓 B 座", "PJ State CMC 教会", 2, "", "pending_driver"),
        (20003, amina, Some(deming), kl_hm(tomorrow, 10, 10), 4, "梳邦再也 SS15 门牌 8", "诊所入口", 1, "", "pending_approval"),
        (20004, fala, Some(deming), kl_hm(today, 20, 0), 2, "SS2, Jalan 21/1 门牌 12", "PJ State CMC 教会", 1, "", "confirmed"),
        (20005, sophie, Some(jiale), kl_hm(today - Duration::days(3), 9, 5), 1, "PJ Section 14 公寓 B 座", "图书馆入口", 1, "", "completed"),
        (20006, amina, Some(deming), kl_hm(today - Duration::days(10), 22, 0), 4, "梳邦再也 SS15 门牌 8", "宵夜团契", 1, "", "completed"),
        (20007, yawen, Some(zhiqiang), kl_hm(tomorrow, 18, 40), 6, "白沙罗 SS2 附近", "社区中心礼堂", 2, "", "pending_passenger"),
        (20008, enci, Some(guorong), kl_hm(today - Duration::days(1), 9, 0), 7, "吉隆坡市中心公寓", "PJ State CMC 教会", 1, "", "cancelled"),
        (20009, yawen, Some(deming), kl_hm(today - Duration::days(8), 19, 20), 2, "SS2, Jalan 21/1 门牌 12", "PJ 室内羽球馆", 1, "", "no_show"),
    ];
    for (id, p, d, at, area, addr, dest, seats, assistance, status) in rides {
        let completed = if status == "completed" { Some(at.clone()) } else { None };
        sqlx::query_as::<_, (i64,)>("INSERT INTO rides (id, passenger_id, driver_id, pickup_at, pickup_area_id, pickup_address, destination, seats, assistance, status, created_at, updated_at, completed_at, demo_batch) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14) RETURNING id")
            .bind(id).bind(p).bind(d).bind(&at).bind(area).bind(addr).bind(dest).bind(seats).bind(assistance).bind(status).bind(now_kl()).bind(now_kl()).bind(completed).bind(DEMO_BATCH).fetch_one(db).await?;
        if matches!(status, "confirmed" | "completed") {
            sqlx::query("UPDATE rides SET trip_id=id, approved_by=$1 WHERE id=$2").bind(transport).bind(id).execute(db).await?;
            sqlx::query("INSERT INTO ride_messages (id, trip_id, user_id, body, demo_batch, created_at) VALUES ($1,$2,NULL,'行程已获批准，群聊已开启。',$3,$4) ON CONFLICT DO NOTHING")
                .bind(id).bind(id).bind(DEMO_BATCH).bind(now_kl()).execute(db).await?;
        }
        if status == "confirmed" {
            sqlx::query("INSERT INTO ride_messages (id, trip_id, user_id, body, demo_batch, created_at) VALUES ($1,$2,$3,'是在北门入口集合吗？',$4,$5) ON CONFLICT DO NOTHING")
                .bind(id + 80000).bind(id).bind(p).bind(DEMO_BATCH).bind(now_kl()).execute(db).await?;
            sqlx::query("INSERT INTO ride_messages (id, trip_id, user_id, body, demo_batch, created_at) VALUES ($1,$2,$3,'对，8 点见！',$4,$5) ON CONFLICT DO NOTHING")
                .bind(id + 80001).bind(id).bind(d).bind(DEMO_BATCH).bind(now_kl()).execute(db).await?;
        }
    }
    // --- announcements ---
    sqlx::query("INSERT INTO announcements (id, title, body, pinned, created_by, demo_batch, created_at) VALUES ($1,'欢迎使用恩典同行','接送、活动与重要消息，集中在同一个平台。如有问题，请到帮助中心提问。',1,$2,$3,$4) ON CONFLICT DO NOTHING")
        .bind(101i64).bind(admin).bind(DEMO_BATCH).bind(now_kl()).execute(db).await?;
    sqlx::query("INSERT INTO announcements (id, title, body, pinned, created_by, demo_batch, created_at) VALUES ($2,'本周日接送安排','请于周五前提交周日崇拜的接送报名，交通组会在周六确认。',0,$2,$3,$4) ON CONFLICT DO NOTHING")
        .bind(102i64).bind(transport).bind(DEMO_BATCH).bind(now_kl()).execute(db).await?;

    // --- notifications ---
    sqlx::query("INSERT INTO notifications (id, user_id, title, body, link, demo_batch, created_at) VALUES ($1,$2,'本周有新的接送报名','蒲种有一位会员等待司机。','/admin/rides',$3,$4) ON CONFLICT DO NOTHING")
        .bind(7001i64).bind(transport).bind(DEMO_BATCH).bind(now_kl()).execute(db).await?;
    sqlx::query("INSERT INTO notifications (id, user_id, title, body, link, demo_batch, created_at) VALUES ($1,$2,'费用分摊已发布','上周羽毛球费用，每人 RM28.00。','/settlements',$3,$4) ON CONFLICT DO NOTHING")
        .bind(7002).bind(fala).bind(DEMO_BATCH).bind(now_kl()).execute(db).await?;

    // --- help ticket + reply ---
    sqlx::query("INSERT INTO help_tickets (id, user_id, subject, status, demo_batch, created_at, updated_at) VALUES ($1,$2,'周日接送可以改到北门吗？','answered',$3,$4,$4) ON CONFLICT DO NOTHING")
        .bind(501i64).bind(yawen).bind(DEMO_BATCH).bind(now_kl()).execute(db).await?;
    sqlx::query("INSERT INTO help_replies (id, ticket_id, user_id, body, demo_batch, created_at) VALUES ($1,$2,$3,'可以，请在报名里把上车点改成北门入口。',$4,$5) ON CONFLICT DO NOTHING")
        .bind(901i64).bind(501).bind(transport).bind(DEMO_BATCH).bind(now_kl()).execute(db).await?;

    // --- settlement: 上周羽毛球费用 (event 5, equal split RM28 each, reconciles to RM168.00) ---
    let due = kl(today + Duration::days(7), 23, 59);
    let settle = sqlx::query_as::<_, (i64,)>("INSERT INTO settlements (id, event_id, created_by, title, split_mode, subsidy_cents, payee_name, payee_account, due_date, status, version, demo_batch, created_at, updated_at) VALUES ($1,$2,$3,'上周羽毛球费用','equal',0,'李德明','012-300 0004',$4,'published',1,$5,$6,$6) RETURNING id")
        .bind(40001i64).bind(5).bind(deming).bind(&due).bind(DEMO_BATCH).bind(now_kl()).fetch_one(db).await?.0;
    sqlx::query("INSERT INTO settlement_items (id, settlement_id, label, amount_cents, receipt_file_id, demo_batch) VALUES ($1,$2,'场租',12000,NULL,$3) ON CONFLICT DO NOTHING").bind(41001i64).bind(settle).bind(DEMO_BATCH).execute(db).await?;
    sqlx::query("INSERT INTO settlement_items (id, settlement_id, label, amount_cents, receipt_file_id, demo_batch) VALUES ($1,$2,'羽球',4800,NULL,$3) ON CONFLICT DO NOTHING").bind(41002).bind(settle).bind(DEMO_BATCH).execute(db).await?;
    for (uid, status) in [(deming, "confirmed"), (fala, "unpaid"), (amina, "pending"), (jiale, "unpaid"), (transport, "confirmed"), (sophie, "unpaid")] {
        sqlx::query("INSERT INTO settlement_shares (id, settlement_id, user_id, amount_cents, status, proof_file_id, txn_ref, paid_date, note, updated_at, demo_batch) VALUES ($1,$2,$3,2800,$4,NULL,'',NULL,'',$5,$6) ON CONFLICT DO NOTHING")
            .bind(42000i64 + uid).bind(settle).bind(uid).bind(status).bind(now_kl()).bind(DEMO_BATCH).execute(db).await?;
    }
    sqlx::query("INSERT INTO settlement_revisions (id, settlement_id, version, snapshot, reason, created_by, demo_batch, created_at) VALUES ($1,$2,1,$3,'首次发布',$4,$5,$6) ON CONFLICT DO NOTHING")
        .bind(43001i64).bind(settle).bind(r#"{"title":"上周羽毛球费用","total_cents":16800}"#).bind(deming).bind(DEMO_BATCH).bind(now_kl()).execute(db).await?;

    audit(db, Some(admin), "seed.core", "users", Some(13), &format!("core demo users {}-{}", core.first().unwrap().0, core.last().unwrap().0)).await;
    tracing::info!("Core demo fixtures seeded ({} users)", id_of.len());
    Ok(())
}
/// Scalable fictional membership: `scale` additional members with stable ids
/// starting at GENERATED_MEMBER_ID_BASE, plus vehicles, service areas and invites.
async fn id_by_email(db: &PgPool, email: &str) -> anyhow::Result<i64> {
    let rows: Vec<(i64,)> = sqlx::query_as("SELECT id FROM users WHERE email=$1 AND demo_batch=$2")
        .bind(email).bind(DEMO_BATCH).fetch_all(db).await.unwrap_or_default();
    match rows.as_slice() {
        [r] => Ok(r.0),
        _ => Err(anyhow::anyhow!("core demo user {email} missing")),
    }
}

async fn seed_scale(db: &PgPool, scale: i64) -> anyhow::Result<()> {
    let n = scale.clamp(1, 50_000);
    let pw = hash_password(DEMO_PASSWORD).map_err(|e| anyhow::anyhow!("{e}"))?;
    let deming = id_by_email(db, "driver@example.org").await?;
    let jiale = id_by_email(db, "driver2@example.org").await?;
    let transport = id_by_email(db, "transport@example.org").await?;
    let finance = id_by_email(db, "finance@example.org").await?;
    let admin = id_by_email(db, "admin@example.org").await?;
    let fala = id_by_email(db, "member@example.org").await?;
    let amina = id_by_email(db, "amina@example.org").await?;
    let sophie = id_by_email(db, "sophie@example.org").await?;
    let yawen = id_by_email(db, "yawen@example.org").await?;
    let zhiqiang = id_by_email(db, "zhiqiang@example.org").await?;
    let meiling = id_by_email(db, "meiling@example.org").await?;
    let enci = id_by_email(db, "enci@example.org").await?;
    let guorong = id_by_email(db, "guorong@example.org").await?;
    let created_by = admin;
    let today = today_kl();
    let mut rng = Rng::new();
    let mut plate_pool = HashSet::new();
    for p in ["VXX 1234", "WQA 8821", "BPM 3302", "BKS 4410", "VBE 7788"] { plate_pool.insert(p.to_string()); }

    // --- generated members ---
    for i in 0..n {
        let id = GENERATED_MEMBER_ID_BASE + i;
        let seq = i + 1;
        let name = format!("{} {}", rng.pick(GIVEN_NAMES), rng.pick(FAMILY_NAMES));
        let email = format!("grdemo{:05}@example.org", id);
        let phone = format!("012-300 {:04}", seq);
        let address = rng.pick(ADDRESSES);
        let (roles, status) = if rng.bool(0.03) {
            ("".to_string(), "disabled")
        } else if rng.bool(0.55) {
            ("".to_string(), "active")
        } else if rng.bool(0.18 / 0.45) {
            ("driver".to_string(), "active")
        } else if rng.bool(0.06 / 0.27) {
            ("driver,leader".to_string(), "active")
        } else if rng.bool(0.08 / 0.21) {
            ("leader".to_string(), "active")
        } else if rng.bool(0.04 / 0.13) {
            ("transport,driver".to_string(), "active")
        } else if rng.bool(0.03 / 0.09) {
            ("transport".to_string(), "active")
        } else if rng.bool(0.02 / 0.06) {
            ("finance".to_string(), "active")
        } else {
            ("admin".to_string(), "active")
        };
        let totp = i < 20;
        let passkey = i < 15;
        insert_member(db, &mut rng, id, &name, &email, &phone, &roles, &status, DEMO_BATCH, &pw, totp, passkey).await;

        if roles.contains("driver") && rng.bool(0.90) {
            let carpool = if rng.bool(0.40) { 1 } else { 0 };
            let spare = if carpool == 1 { rng.range(2, 4) } else { 1 };
            let plate = rng.unique_plate(&mut plate_pool);
            sqlx::query("INSERT INTO vehicles (user_id, model, color, plate, seats, demo_batch) VALUES ($1,$2,$3,$4,5,$5) ON CONFLICT(user_id) DO UPDATE SET model=excluded.model, color=excluded.color, plate=excluded.plate, seats=excluded.seats, demo_batch=excluded.demo_batch")
                .bind(id).bind(rng.pick(VEHICLE_MODELS)).bind(rng.pick(VEHICLE_COLORS)).bind(plate).bind(DEMO_BATCH).execute(db).await?;
            sqlx::query("UPDATE users SET carpool_enabled=$1, spare_seats=$2 WHERE id=$3").bind(carpool).bind(spare).bind(id).execute(db).await?;
            let mut areas: Vec<i64> = vec![1, 2, 4, 5, 7, 8];
            rng.shuffle(&mut areas);
            for area in areas.into_iter().take(rng.range(1, 4)) {
                sqlx::query("INSERT INTO driver_areas (user_id, area_id, demo_batch) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING")
                    .bind(id).bind(area).bind(DEMO_BATCH).execute(db).await?;
            }
        }
    }
    drop(plate_pool);
    // --- simulated invitees: pending / expired / duplicate / accepted invites ---
    let created_by = sqlx::query_as::<_, (i64,)>("SELECT id FROM users WHERE demo_batch=$1 ORDER BY id LIMIT 1")
        .bind(DEMO_BATCH).fetch_one(db).await.unwrap_or((1,)).0;
    for i in 0..30 {
        let id = 90001 + i;
        let email = format!("grdemo-invite{:05}@example.org", id);
        let roles = if i % 5 == 0 { "driver" } else if i % 7 == 0 { "leader" } else { "" };
        let expires = kl(today + Duration::days(7 + (i % 20)), 10, 0);
        sqlx::query("INSERT INTO invites (id, token, kind, email, name, roles, group_ids, created_by, expires_at, demo_batch, created_at) VALUES ($1,$2,'invite',$3,$4,$5,'[]',$6,$7,$8,$9)")
            .bind(id).bind(format!("inv-{id:05}")).bind(&email).bind(format!("Invitee {id}")).bind(roles).bind(created_by).bind(&expires).bind(DEMO_BATCH).bind(now_kl()).execute(db).await?;
    }
    for i in 0..15 {
        let id = 90031 + i;
        let email = format!("grdemo-expired{:05}@example.org", id);
        sqlx::query("INSERT INTO invites (id, token, kind, email, name, roles, group_ids, created_by, expires_at, demo_batch, created_at) VALUES ($1,$2,'invite',$3,$4,'','[]',$6,$7,$8,$9)")
            .bind(id).bind(format!("exp-{id:05}")).bind(&email).bind(format!("Expired {id}")).bind("").bind(created_by).bind(kl(today - Duration::days(1 + (i % 30)), 10, 0)).bind(DEMO_BATCH).bind(now_kl()).execute(db).await?;
    }
    // Duplicate open invites for the same email (double-invite scenario).
    for (i, roles) in ["driver", "member"].into_iter().enumerate() {
        let id = 90051 + i as i64;
        sqlx::query("INSERT INTO invites (id, token, kind, email, name, roles, group_ids, created_by, expires_at, demo_batch, created_at) VALUES ($1,$2,'invite',$3,$4,$5,'[]',$6,$7,$8,$9)")
            .bind(id).bind(format!("dup-{id:05}")).bind("grdemo-dupe@example.org").bind(format!("Duplicate {id}")).bind(roles).bind(created_by).bind(kl(today + Duration::days(14), 10, 0)).bind(DEMO_BATCH).bind(now_kl()).execute(db).await?;
    }
    // Accepted invites that produced members (invitation-only onboarding evidence).
    for i in 0..5 {
        let id = 90061 + i;
        let uid = GENERATED_MEMBER_ID_BASE + i;
        sqlx::query("INSERT INTO invites (id, token, kind, email, name, roles, group_ids, user_id, created_by, expires_at, used_at, demo_batch, created_at) VALUES ($1,$2,'invite',$3,$4,$5,'[]',$6,$7,$8,$9,$10,$11)")
            .bind(id).bind(format!("acc-{id:05}")).bind(format!("grdemo-accepted{:05}@example.org", uid)).bind(format!("Accepted {id}")).bind("").bind(uid).bind(created_by).bind(kl(today - Duration::days(60), 10, 0)).bind(now_kl()).bind(now_kl()).bind(DEMO_BATCH).execute(db).await?;
    }
    // --- generated events (ids 1001..) with group-scoped visibility ---
    let gen_events = [
        (1001i64, "生技羽球局", "badminton", "业余双打，新手欢迎。", next_weekday(today, Weekday::Tue), 19, 30, "PJ 室内羽球馆", 10, deming, vec![2i64, 3], Some(7)),
        (1002, "周末保龄球局", "bowling", "轻松保龄球聚会。", next_weekday(today, Weekday::Sat), 10, 0, "Sunway Pyramid 保龄球馆", 6, zhiqiang, vec![3], Some(3)),
        (1003, "满员皮划艇", "kayak", "名额已满，后续报名进入候补。", next_weekday(today, Weekday::Sun), 8, 0, "Putrajaya 湖畔", 8, deming, vec![2], None),
        (1004, "社区公园散步", "walk", "早晨散步，人数不限。", next_weekday(today, Weekday::Sat), 7, 0, "Taman Jaya 公园入口", 0, enci, vec![1], Some(1)),
        (1005, "青年团契聚餐", "other", "每人带一道菜。", next_weekday(today, Weekday::Fri), 18, 30, "社区中心礼堂", 15, enci, vec![2], Some(2)),
        (1006, "小组羽球赛", "badminton", "小组内比赛，名额紧张。", next_weekday(today, Weekday::Thu), 20, 0, "PJ 室内羽球馆", 4, meiling, vec![3], None),
        (1007, "已取消的羽毛球活动", "badminton", "因天气取消，供参考。", today - Duration::days(3), 19, 30, "PJ 室内羽球馆", 10, deming, vec![2], Some(1)),
        (1008, "上周保龄球局", "bowling", "已结束，考勤示例。", today - Duration::days(12), 14, 0, "Sunway Pyramid 保龄球馆", 6, zhiqiang, vec![3], Some(1)),
        (1009, "报名已截止的活动", "other", "活动未开始但报名已截止。", today + Duration::days(20), 10, 0, "社区中心礼堂", 20, transport, vec![1], None),
        (1010, "小团体散步", "walk", "小团体早晨散步，名额3个。", next_weekday(today, Weekday::Sun), 7, 0, "Taman Jaya 公园入口", 3, yawen, vec![1], Some(4)),
        (1011, "周末划艇", "kayak", "周末划艇体验。", next_weekday(today, Weekday::Sat), 8, 0, "Putrajaya 湖畔", 5, enci, vec![2], Some(2)),
        (1012, "周二羽球局", "badminton", "常规双打交流。", next_weekday(today, Weekday::Tue), 19, 30, "PJ 室内羽球馆", 12, deming, vec![2, 3], Some(5)),
    ];
    let members: Vec<i64> = sqlx::query_as("SELECT id FROM users WHERE demo_batch=$1 AND status='active' ORDER BY id")
        .bind(DEMO_BATCH).fetch_all(db).await.unwrap_or_default();
    let gm: Vec<(i64, i64)> = sqlx::query_as("SELECT group_id, user_id FROM group_members WHERE demo_batch=$1")
        .bind(DEMO_BATCH).fetch_all(db).await.unwrap_or_default();
    let mut in_group: std::collections::HashMap<i64, Vec<i64>> = std::collections::HashMap::new();
    for (g, u) in gm { in_group.entry(g).or_default().push(u); }
    let mut event_rng = rng;
    for (eid, title, kind, desc, wd, h, m, loc, cap, by, groups, force_full) in gen_events {
        let at = kl_hm(wd, h, m);
        let deadline = if force_full.is_none() { Some(kl(today + Duration::days(3), 12, 0)) } else { None };
        let status = if eid == 1007 { "cancelled" } else if eid == 1008 { "finished" } else { "published" };
        sqlx::query_as::<_, (i64,)>("INSERT INTO events (id, title, kind, description, starts_at, location, capacity, cost_note, created_by, signup_deadline, status, demo_batch, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13) RETURNING id")
            .bind(eid).bind(title).bind(kind).bind(desc).bind(&at).bind(loc).bind(cap).bind("活动费用活动后分摊").bind(by).bind(deadline).bind(status).bind(DEMO_BATCH).bind(now_kl()).fetch_one(db).await?;
        counts.events += 1;
        for g in groups {
            sqlx::query("INSERT INTO event_groups (event_id, group_id, demo_batch) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING")
                .bind(eid).bind(g).bind(DEMO_BATCH).execute(db).await?;
        }
        let is_all = groups.contains(&1);
        let mut eligible: Vec<i64> = members.iter().filter(|u| is_all || in_group.get(1).map_or(false, |v| v.contains(u)) || groups.iter().any(|g| in_group.get(*g).map_or(false, |v| v.contains(u)))).copied().collect();
        event_rng.shuffle(&mut eligible);
        let mut want = event_rng.range(0, cap + 2);
        if let Some(f) = force_full { want = f.max(cap); }
        let going = eligible.len().min(want as usize).min(cap.max(0) as usize);
        for (idx, uid) in eligible.iter().enumerate().take(want as usize) {
            let stat = if idx < going { "going" } else { "waitlist" };
            let need_ride = event_rng.bool(0.5) as i64;
            let (offer_drive, offer_seats) = if eligible[idx] != by && role_of(&member_rows, *uid).map_or(false, |r| has_role(&r, "driver")) && event_rng.bool(0.35) { (1, event_rng.range(1, 4)) } else { (0, 0) };
            sqlx::query("INSERT INTO event_signups (event_id, user_id, status, need_ride, offer_drive, offer_seats, demo_batch, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(event_id,user_id) DO NOTHING")
                .bind(eid).bind(uid).bind(stat).bind(need_ride).bind(offer_drive).bind(offer_seats).bind(DEMO_BATCH).bind(now_kl()).execute(db).await?;
            counts.signups += 1;
        }
        // finished event: record attendance
        if eid == 1008 {
            for uid in eligible.iter().take(going) {
                sqlx::query("UPDATE event_signups SET attended=$1 WHERE event_id=1008 AND user_id=$2")
                    .bind(if event_rng.bool(0.8) { 1i64 } else { 0 }).bind(uid).execute(db).await?;
            }
        }
    }
fn has_role(roles: &str, want: &str) -> bool {
    roles.split(',').any(|r| r == want)
}
fn role_of(rows: &[(i64, String)], uid: i64) -> Option<String> {
    rows.iter().find(|(u, _)| *u == uid).map(|(_, r)| r.clone())
}
fn pick_status(rng: &mut Rng, table: &[(&str, i64)]) -> &str {
    let roll = rng.range(0, 100);
    let mut acc = 0i64;
    table.iter().find(|(_, w)| { acc += w; acc > roll }).unwrap().0
}
async fn seed_rides(
    db: &PgPool,
    n: i64,
    rng: &mut Rng,
    staff: Option<i64>,
    counts: &mut Counts,
) -> anyhow::Result<()> {
    let today = today_kl();
    let rides_n = (n as f64 * 0.5) as i64;
    let member_ids: Vec<i64> = sqlx::query_as(
        "SELECT id FROM users WHERE demo_batch=$1 AND status='active' ORDER BY id",
    ).bind(DEMO_BATCH).fetch_all(db).await.unwrap_or_default();
    let driver_rows: Vec<(i64, i64, i64, Option<i64>)> = sqlx::query_as(
        "SELECT u.id, u.carpool_enabled, u.spare_seats, v.seats FROM users u LEFT JOIN vehicles v ON v.user_id=u.id
         WHERE u.demo_batch=$1 AND u.roles LIKE '%driver%' AND u.status='active'",
    ).bind(DEMO_BATCH).fetch_all(db).await.unwrap_or_default();
    let mut cap_map: HashMap<i64, i64> = HashMap::new();
    for (id, carpool, spare, vseats) in driver_rows {
        let vehicle_cap = vseats.map(|s| (s - 1).max(1)).unwrap_or(0);
        cap_map.insert(id, spare.max(1).min(vehicle_cap));
    }
    let da: Vec<(i64, i64)> = sqlx::query_as("SELECT user_id, area_id FROM driver_areas WHERE demo_batch=$1")
        .bind(DEMO_BATCH).fetch_all(db).await.unwrap_or_default();
    let mut area_drivers: HashMap<i64, Vec<i64>> = HashMap::new();
    for (u, a) in da { area_drivers.entry(a).or_default().push(u); }
    let mut load: HashMap<(i64, NaiveDate), i64> = HashMap::new();
    let statuses: &[(&str, i64)] = &[
        ("pending_driver", 20), ("pending_passenger", 10), ("pending_approval", 8),
        ("pending_driver_confirm", 4), ("confirmed", 15), ("in_progress", 8),
        ("completed", 20), ("cancelled", 7), ("no_show", 4), ("alt_transport", 4),
    ];
    let mut gps_drivers: Vec<i64> = vec![];
    let mut stale_drivers: Vec<i64> = vec![];
    let mut messages = 0i64;

    for i in 0..rides_n {
        let id = GENERATED_RIDE_ID_BASE + i;
        let status = pick_status(rng, statuses);
        let passenger = *rng.pick(&member_ids);
        let at_day = today + Duration::days(rng.range(-45, 46));
        let h = rng.range(7, 22);
        let m = [0, 15, 30, 45][rng.range(0, 4) as usize];
        let at = kl(at_day, h, m);
        let area = rng.range(1, 9);
        let seats = rng.range(1, 5);
        let assistance = if rng.bool(0.10) { "行动不便需协助上下车" } else { "" };
        let dest = rng.pick(DESTINATIONS);
        let event_id = if rng.bool(0.15) { Some(GENERATED_EVENT_ID_BASE + rng.range(0, 12)) } else { None };
        let needs_driver = matches!(
            status,
            "pending_passenger" | "pending_approval" | "pending_driver_confirm" | "confirmed" | "in_progress" | "completed"
        );
        let mut driver: Option<i64> = None;
        if needs_driver {
            if let Some(cands) = area_drivers.get(&area) {
                let mut ok: Vec<i64> = vec![];
                for &d in cands {
                    let cap = cap_map.get(&d).unwrap_or(&1);
                    let used = load.get(&(d, at_day)).unwrap_or(&0);
                    if used + seats <= *cap { ok.push(d); }
                }
                if !ok.is_empty() {
                    driver = Some(*rng.pick(&ok));
                    *load.entry((driver.unwrap(), at_day)).or_insert(0) += seats;
                }
            }
        }
        let (started_at, picked_up_at, completed_at) = match status {
            "in_progress" => (Some(at.clone()), None, None),
            "completed" => {
                let end = kl(at_day, (h + 1).rem_euclid(24), m);
                (Some(at.clone()), Some(at.clone()), Some(end))
            }
            _ => (None, None, None),
        };
        sqlx::query_as::<_, (i64,)>(
            "INSERT INTO rides (id, passenger_id, driver_id, event_id, pickup_at, pickup_area_id, pickup_address, destination, seats, assistance, status, created_at, updated_at, started_at, picked_up_at, completed_at, demo_batch)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$12,$13,$14,$15,$16) RETURNING id",
        )
        .bind(id).bind(passenger).bind(driver).bind(event_id).bind(&at).bind(area).bind(rng.pick(ADDRESSES)).bind(dest)
        .bind(seats).bind(assistance).bind(status).bind(now_kl()).bind(started_at).bind(picked_up_at).bind(completed_at)
        .bind(DEMO_BATCH).fetch_one(db).await?;
        counts.rides += 1;
        if let Some(d) = driver {
            if matches!(status, "confirmed" | "in_progress" | "completed") {
                sqlx::query("UPDATE rides SET trip_id=id, approved_by=$1 WHERE id=$2")
                    .bind(staff.unwrap_or(d)).bind(id).execute(db).await?;
            }
        }
        if matches!(status, "confirmed" | "in_progress" | "completed") {
            sqlx::query("INSERT INTO ride_messages (id, trip_id, user_id, body, demo_batch, created_at) VALUES ($1,$2,NULL,'行程已获批准，群聊已开启。',$3,$4) ON CONFLICT DO NOTHING")
                .bind(id).bind(id).bind(DEMO_BATCH).bind(now_kl()).execute(db).await?;
            messages += 1;
            if status == "confirmed" {
                sqlx::query("INSERT INTO ride_messages (id, trip_id, user_id, body, demo_batch, created_at) VALUES ($1,$2,$3,'司机能在七点前到吗？',$4,$5) ON CONFLICT DO NOTHING")
                    .bind(id + 1).bind(id).bind(passenger).bind(DEMO_BATCH).bind(now_kl()).execute(db).await?;
                messages += 1;
            }
        }
    }
    // Live location tracks: fresh for up to 8 drivers, stale for 2, none for the rest.
    for (i, &d) in gps_drivers.iter().enumerate() {
        let lat = 3.1300 + (d % 100) as f64 * 0.0005 + (i as f64 * 0.0001);
        let lng = 101.6000 + (d % 100) as f64 * 0.0008 + (i as f64 * 0.0001);
        sqlx::query("INSERT INTO driver_locations (driver_id, lat, lng, heading, updated_at, demo_batch) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT(driver_id) DO UPDATE SET lat=excluded.lat, lng=excluded.lng, heading=excluded.heading, updated_at=excluded.updated_at, demo_batch=excluded.demo_batch")
            .bind(d).bind(lat).bind(lng).bind(Some((d * 37) as f64 % 360.0)).bind(now_kl()).bind(DEMO_BATCH).execute(db).await?;
        counts.locations += 1;
    }
    for &d in stale_drivers.iter() {
        let lat = 3.1300 + (d % 100) as f64 * 0.0005;
        let lng = 101.6000 + (d % 100) as f64 * 0.0008;
        sqlx::query("INSERT INTO driver_locations (driver_id, lat, lng, heading, updated_at, demo_batch) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT(driver_id) DO UPDATE SET lat=excluded.lat, lng=excluded.lng, heading=excluded.heading, updated_at=excluded.updated_at, demo_batch=excluded.demo_batch")
            .bind(d).bind(lat).bind(lng).bind(Some((d * 37) as f64 % 360.0)).bind(kl(today - Duration::days(1), 8, 0)).bind(DEMO_BATCH).execute(db).await?;
        counts.locations += 1;
    }
    counts.messages += messages;
    Ok(())
}

/// Notifications across roles: unread / read, reminders, delayed, mixed links.
async fn seed_notifications(db: &PgPool, rng: &mut Rng, counts: &mut Counts) -> anyhow::Result<()> {
    let today = today_kl();
    let pool: Vec<(i64, &str, &str, &str)> = vec![
        (transport, "新的接送报名", "蒲种有一位会员等待司机。", "/admin/rides"),
        (finance, "费用分摊已发布", "上周羽毛球费用，每人 RM28.00。", "/settlements"),
        (deming, "接送已获批准", "周四羽球局接送已由交通组批准。", "/drive"),
        (fala, "有司机愿意接送您", "吴志强 邀请接送您（明天 08:30），请查看并确认。", "/rides/20001"),
        (amina, "行程已开始", "您的接送行程已开始，司机正在前往上车点。", "/rides/20004"),
        (zhiqiang, "接送提醒", "明天 08:30 接载陈法拉（SS2），出发时请按「开始行程」。", "/drive"),
        (admin, "账号恢复申请", "一位会员提交了账号恢复申请，请审阅。", "/admin/recovery"),
        (guorong, "费用需要跟进", "团契聚餐费用分摊，凭证资料不全，请补交。", "/settlements"),
        (sophie, "活动候补成功", "您已获得「小组羽球赛」的名额。", "/events/1006"),
        (yawen, "司机无法出车", "交通组正在为您安排其他司机，请留意通知。", "/rides/20007"),
    ];
    for (i, (uid, title, body, link)) in pool.iter().enumerate() {
        let read = if rng.bool(0.4) { Some(kl(today - Duration::days(1 + (i % 5)), 9, 0)) } else { None };
        sqlx::query("INSERT INTO notifications (id, user_id, title, body, link, read_at, demo_batch, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT DO NOTHING")
            .bind(7003i64 + i as i64).bind(uid).bind(title).bind(body).bind(link).bind(read).bind(DEMO_BATCH).bind(now_kl()).execute(db).await?;
        counts.notifications += 1;
    }
    // Ride-reminder style notifications (reminder_sent style history) for confirmed rides.
    for i in 0..8i64 {
        let uid = *rng.pick(&[deming, fala, amina, sophie, yawen, meiling]);
        let read = if rng.bool(0.5) { Some(now_kl()) } else { None };
        sqlx::query("INSERT INTO notifications (id, user_id, title, body, link, read_at, demo_batch, created_at) VALUES ($1,$2,'接送提醒',$3,'/rides/20004',$4,$5,$6) ON CONFLICT DO NOTHING")
            .bind(7100i64 + i).bind(uid).bind("司机将于今天 20:00 到 SS2 接您。").bind(read).bind(DEMO_BATCH).bind(now_kl()).execute(db).await?;
        counts.notifications += 1;
    }
    Ok(())
}

/// Cost-sharing settlements: equal/custom splits, rounding remainder, subsidy,
/// zero amounts, cancelled events, partial payments, outstanding balances.
/// Every settlement reconciles: sum(shares) + subsidy == sum(items).
async fn seed_settlements(db: &PgPool, rng: &mut Rng, counts: &mut Counts) -> anyhow::Result<()> {
    let today = today_kl();
    let specs: Vec<(i64, i64, &str, &str, i64, i64, Vec<(&str, i64)>, Vec<(i64, i64, &str)>)> = vec![
        (40002, 1003, "皮划艇器材费", "equal", 0, 5, vec![("皮划艇租借", 6000), ("救生衣", 2000)], vec![(deming, 2000, "confirmed"), (fala, 2000, "unpaid"), (amina, 2000, "pending"), (zhiqiang, 2000, "confirmed")]),
        (40003, 1005, "团契聚餐食材", "custom", 0, 7, vec![("食材采购", 9000)], vec![(enci, 3000, "confirmed"), (meiling, 3000, "unpaid"), (fala, 3000, "needs_info")]),
        (40004, 1004, "公园散步茶点", "equal", 5000, 3, vec![("饮料与点心", 5000)], vec![(yawen, 0, "confirmed"), (fala, 0, "unpaid"), (enci, 0, "confirmed"), (meiling, 0, "unpaid")]),
        (40005, 1007, "已取消活动场租", "equal", 0, 1, vec![("场租（活动取消）", 0)], vec![(deming, 0, "unpaid"), (fala, 0, "unpaid")]),
        (40006, 1002, "保龄球局费用", "equal", 0, 4, vec![("球道租借", 10050)], vec![(zhiqiang, 3350, "confirmed"), (deming, 3350, "pending"), (jiale, 3350, "unpaid")]),
        (40007, 1001, "羽毛球场租", "custom", 2000, 10, vec![("场租", 10000), ("羽球", 2000)], vec![(meiling, 3000, "confirmed"), (zhiqiang, 4000, "unpaid"), (fala, 3000, "confirmed")]),
        (40008, 1012, "周二羽球局", "equal", 0, 14, vec![("场租", 9999)], vec![(deming, 2000, "unpaid"), (fala, 2000, "unpaid"), (amina, 1999, "unpaid"), (sophie, 2000, "unpaid"), (yawen, 2000, "unpaid")]),
    ];
    let mut sid = 42001i64;
    for (id, eid, title, mode, subsidy, due, items, shares) in specs {
        let due_date = kl(today + Duration::days(due), 23, 59);
        sqlx::query_as::<_, (i64,)>("INSERT INTO settlements (id, event_id, created_by, title, split_mode, subsidy_cents, payee_name, payee_account, due_date, status, version, demo_batch, created_at, updated_at) VALUES ($1,$2,$3,$4,$5,$6,'TNG 演示账户','012-300 9999',$7,'published',1,$8,$9,$9) RETURNING id")
            .bind(id).bind(eid).bind(deming).bind(title).bind(mode).bind(subsidy).bind(&due_date).bind(DEMO_BATCH).bind(now_kl()).fetch_one(db).await?;
        let mut gross = 0i64;
        for (i, (label, cents)) in items.into_iter().enumerate() {
            gross += cents;
            sqlx::query("INSERT INTO settlement_items (id, settlement_id, label, amount_cents, receipt_file_id, demo_batch) VALUES ($1,$2,$3,$4,NULL,$5) ON CONFLICT DO NOTHING")
                .bind(41100i64 + i).bind(id).bind(label).bind(cents).bind(DEMO_BATCH).execute(db).await?;
        }
        for (j, (uid, amt, status)) in shares.into_iter().enumerate() {
            sqlx::query("INSERT INTO settlement_shares (id, settlement_id, user_id, amount_cents, status, proof_file_id, txn_ref, paid_date, note, updated_at, demo_batch) VALUES ($1,$2,$3,$4,$5,NULL,'',NULL,'',$6,$7) ON CONFLICT DO NOTHING")
                .bind(sid).bind(id).bind(uid).bind(amt).bind(status).bind(now_kl()).bind(DEMO_BATCH).execute(db).await?;
            sid += 1;
        }
        sqlx::query("INSERT INTO settlement_revisions (id, settlement_id, version, snapshot, reason, created_by, demo_batch, created_at) VALUES ($1,$2,1,$3,'首次发布',$4,$5,$6) ON CONFLICT DO NOTHING")
            .bind(43002i64 + id - 40001).bind(id).bind(format!("{{\"title\":\"{title}\",\"total_cents\":{}}}", gross - subsidy).as_str()).bind(deming).bind(DEMO_BATCH).bind(now_kl()).execute(db).await?;
        counts.settlements += 1;
    }
    Ok(())
}

/// Reimbursement cases: pending / returned / rejected / approved / paid with
/// fictional receipts. Uploading a receipt never auto-confirms funds received.
async fn seed_reimbursements(db: &PgPool, rng: &mut Rng, counts: &mut Counts, upload_dir: &std::path::Path) -> anyhow::Result<()> {
    let today = today_kl();
    let payee_admin = sqlx::query_as::<_, (i64,)>("SELECT id FROM users WHERE roles LIKE '%finance%' AND demo_batch=$1 ORDER BY id LIMIT 1")
        .bind(DEMO_BATCH).fetch_optional(db).await.unwrap_or(None).map(|(i,)| i).unwrap_or(finance);
    let specs: Vec<(i64, i64, Option<i64>, i64, &str, &str, &str, bool, &str)> = vec![
        (30001, fala, Some(20001), 4500, "Grab 替代交通（蒲种→教会）", "pending", "陈法拉", true, ""),
        (30002, amina, Some(20006), 3800, "Grab 替代交通（宵夜团契）", "returned", "阿米娜", true, ""),
        (30003, sophie, Some(20005), 5200, "Grab 替代交通（图书馆）", "rejected", "林苏菲", true, ""),
        (30004, yawen, None, 6100, "TNG 代购车费（凭证模糊）", "approved", "周雅文", true, ""),
        (30005, zhiqiang, Some(20007), 2900, "Bank transfer receipt", "paid", "吴志强", true, "DUTNOW-DEMO-0001"),
        (30006, deming, None, 1500, "PJ 往返车费", "pending", "李德明", false, ""),
        (30007, meiling, Some(20003), 4400, "DuitNow 转账凭证", "paid", "郑美玲", true, "DUTNOW-DEMO-0002"),
        (30008, enci, None, 3300, "Grab 收据缺失日期", "returned", "许恩慈", false, ""),
    ];
    let mut fid = 9501i64;
    for (id, uid, ride, amount, desc, status, payee, exists, paid_ref) in specs {
        let stored = format!("reimb-{id}.png");
        if exists {
            let _ = tokio::fs::write(upload_dir.join(&stored), &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]).await;
        }
        sqlx::query("INSERT INTO files (id, owner_id, stored_name, mime, original_name, size, demo_batch, created_at) VALUES ($1,$2,$3,'image/png',$4,8,$5,$6) ON CONFLICT DO NOTHING")
            .bind(fid).bind(uid).bind(&stored).bind("receipt.png").bind(DEMO_BATCH).bind(now_kl()).execute(db).await?;
        let paid_by = if status == "paid" { Some(payee_admin) } else { None };
        let paid_at = if paid_by.is_some() { Some(now_kl()) } else { None };
        sqlx::query_as::<_, (i64,)>("INSERT INTO reimbursements (id, user_id, ride_id, amount_cents, description, receipt_file_id, payee_name, payee_phone, status, review_note, paid_ref, paid_by, paid_at, created_at, updated_at, demo_batch) VALUES ($1,$2,$3,$4,$5,$6,$7,'012-300 0000',$8,'',$9,$10,$11,$12,$12,$13) RETURNING id")
            .bind(id).bind(uid).bind(ride).bind(amount).bind(desc).bind(fid).bind(payee).bind(status).bind(paid_ref).bind(paid_by).bind(paid_at).bind(now_kl()).bind(DEMO_BATCH).fetch_one(db).await?;
        fid += 1;
        counts.reimbursements += 1;
    }
    Ok(())
}

/// Edge-case batch: deliberately difficult/edge scenarios for negative tests.
/// Every row carries demo_batch = EDGE_BATCH so `seed reset` can remove it.
async fn seed_edge(db: &PgPool) -> anyhow::Result<()> {
    let pw = hash_password(DEMO_PASSWORD).map_err(|e| anyhow::anyhow!("{e}"))?;
    let deming = id_by_email(db, "driver@example.org").await?;
    let transport = id_by_email(db, "transport@example.org").await?;
    let fala = id_by_email(db, "member@example.org").await?;
    let today = today_kl();

    for (id, name, email, phone, roles, status, totp, pk) in [
        (50001, "Disabled demo user", "disabled@example.org", "012-300 9001", "", "disabled", false, false),
        (50002, "Driver without vehicle", "novhicle@example.org", "012-300 9002", "driver", "active", false, false),
        (50003, "SS2-only driver", "s2driver@example.org", "012-300 9003", "driver", "active", false, false),
        (50004, "Finance demo user", "finance2@example.org", "012-300 9004", "finance", "active", false, false),
        (50005, "TOTP simulation", "totpdemo@example.org", "012-300 9005", "", "active", true, false),
        (50006, "Passkey simulation", "passkeydemo@example.org", "012-300 9006", "", "active", false, true),
    ] {
        let (id,): (i64,) = demo_user(&mut Rng::new(), id, name, email, phone, roles, status, EDGE_BATCH, &pw)
            .fetch_one(db).await.unwrap();
        if totp {
            sqlx::query("UPDATE users SET totp_secret=$1, totp_enabled=1 WHERE id=$2").bind(SIMULATED_TOTP_SECRET).bind(id).execute(db).await?;
        }
        if pk {
            sqlx::query("INSERT INTO passkeys (id, user_id, cred_id, passkey, label, demo_batch, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT DO NOTHING")
                .bind(id).bind(id).bind("sim-cred-{id}").bind(json!({"demo": true}).to_string()).bind("SIMULATED - test device").bind(EDGE_BATCH).bind(now_kl()).execute(db).await?;
        }
    }
    // Edge user 50003: SS2-only driver with vehicle + area.
    sqlx::query("INSERT INTO vehicles (user_id, model, color, plate, seats, demo_batch) VALUES (50003,'Toyota Vios','银色','S2D 1001',5,$1) ON CONFLICT DO NOTHING")
        .bind(EDGE_BATCH).execute(db).await?;
    sqlx::query("INSERT INTO driver_areas (user_id, area_id, demo_batch) VALUES (50003,2,$2) ON CONFLICT DO NOTHING")
        .bind(EDGE_BATCH).bind(EDGE_BATCH).execute(db).await?;

    for (id, email, name) in [
        (9101, "edge-expired@example.org", "Expired Demo"),
        (9102, "edge-revoked@example.org", "Revoked Demo"),
    ] {
        sqlx::query("INSERT INTO invites (id, token, kind, email, name, roles, group_ids, created_by, expires_at, demo_batch, created_at) VALUES ($1,$2,'invite',$3,$4,'',$5,$6,$7,$8,$9)")
            .bind(id).bind(format!("edge-{id}")).bind("invite").bind(email).bind(name).bind("").bind(transport).bind(if id == 9101 { kl(today - Duration::days(5), 10, 0) } else { kl(today - Duration::days(1), 10, 0) }).bind(EDGE_BATCH).bind(now_kl()).execute(db).await?;
    }
    for (i, roles) in ["driver", "member"].into_iter().enumerate() {
        let id = 9103 + i as i64;
        sqlx::query("INSERT INTO invites (id, token, kind, email, name, roles, group_ids, created_by, expires_at, demo_batch, created_at) VALUES ($1,$2,'invite','edge-dupe@example.org','Duplicate Demo',$3,'[]',$4,$5,$6,$7)")
            .bind(id).bind(format!("edge-dup-{id}")).bind("invite").bind("edge-dupe@example.org").bind(format!("Duplicate {i}")).bind(roles).bind(transport).bind(kl(today + Duration::days(14), 10, 0)).bind(EDGE_BATCH).bind(now_kl()).execute(db).await?;
    }

    // Full event (capacity 2, 2 signups going) and a cancelled event.
    sqlx::query_as::<_, (i64,)>("INSERT INTO events (id, title, kind, description, starts_at, location, capacity, created_by, signup_deadline, status, demo_batch, created_at) VALUES ($1,'满员活动','walk','满员测试 — 报名满后进入候补。',$2,'Taman Jaya 公园入口',2,$3,$4,$5,$6,$7,$8) RETURNING id")
        .bind(9001i64).bind(kl(today + Duration::days(4), 7, 0)).bind(transport).bind(kl(today + Duration::days(3), 12, 0)).bind("published").bind(EDGE_BATCH).bind(now_kl()).fetch_one(db).await?;
    sqlx::query("INSERT INTO event_signups (event_id, user_id, status, need_ride, offer_drive, offer_seats, demo_batch, created_at) VALUES (9001,$1,'going',0,0,0,$2,$3) ON CONFLICT DO NOTHING")
        .bind(fala).bind(EDGE_BATCH).bind(now_kl()).execute(db).await?;
    sqlx::query("INSERT INTO event_signups (event_id, user_id, status, need_ride, offer_drive, offer_seats, demo_batch, created_at) VALUES (9001,$1,'going',0,0,0,$2,$3) ON CONFLICT DO NOTHING")
        .bind(deming).bind(EDGE_BATCH).bind(now_kl()).execute(db).await?;
    sqlx::query("INSERT INTO events (id, title, kind, description, starts_at, location, capacity, created_by, signup_deadline, status, demo_batch, created_at) VALUES ($1,'已取消的散步活动','walk','已取消的活动供参考',$2,'Taman Jaya 公园入口',0,$3,$4,$5,$6,$7,$8) ON CONFLICT DO NOTHING")
        .bind(9002i64).bind(kl(today + Duration::days(10), 7, 0)).bind(transport).bind(kl(today + Duration::days(8), 12, 0)).bind("cancelled").bind(EDGE_BATCH).bind(now_kl()).execute(db).await?;
}

/// Delete every row stamped with a batch (dependency order), so a reseed starts
/// from a clean slate without touching real member data.
async fn clear_demo(db: &PgPool, batch: &str) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM ride_messages WHERE demo_batch=$1").bind(batch).execute(db).await?;
    sqlx::query("DELETE FROM driver_locations WHERE demo_batch=$1").bind(batch).execute(db).await?;
    sqlx::query("DELETE FROM rides WHERE demo_batch=$1").bind(batch).execute(db).await?;
    sqlx::query("DELETE FROM event_signups WHERE demo_batch=$1").bind(batch).execute(db).await?;
    sqlx::query("DELETE FROM event_groups WHERE demo_batch=$1").bind(batch).execute(db).await?;
    sqlx::query("DELETE FROM events WHERE demo_batch=$1").bind(batch).execute(db).await?;
    sqlx::query("DELETE FROM settlement_revisions WHERE demo_batch=$1").bind(batch).execute(db).await?;
    sqlx::query("DELETE FROM settlement_shares WHERE demo_batch=$1").bind(batch).execute(db).await?;
    sqlx::query("DELETE FROM settlement_items WHERE demo_batch=$1").bind(batch).execute(db).await?;
    sqlx::query("DELETE FROM settlements WHERE demo_batch=$1").bind(batch).execute(db).await?;
    sqlx::query("DELETE FROM reimbursements WHERE demo_batch=$1").bind(batch).execute(db).await?;
    sqlx::query("DELETE FROM files WHERE demo_batch=$1").bind(batch).execute(db).await?;
    sqlx::query("DELETE FROM notifications WHERE demo_batch=$1").bind(batch).execute(db).await?;
    sqlx::query("DELETE FROM announcements WHERE demo_batch=$1").bind(batch).execute(db).await?;
    sqlx::query("DELETE FROM help_replies WHERE demo_batch=$1").bind(batch).execute(db).await?;
    sqlx::query("DELETE FROM help_tickets WHERE demo_batch=$1").bind(batch).execute(db).await?;
    sqlx::query("DELETE FROM invites WHERE demo_batch=$1").bind(batch).execute(db).await?;
    sqlx::query("DELETE FROM passkeys WHERE demo_batch=$1").bind(batch).execute(db).await?;
    sqlx::query("DELETE FROM vehicles WHERE demo_batch=$1").bind(batch).execute(db).await?;
    sqlx::query("DELETE FROM driver_areas WHERE demo_batch=$1").bind(batch).execute(db).await?;
    sqlx::query("DELETE FROM group_members WHERE demo_batch=$1").bind(batch).execute(db).await?;
    sqlx::query("DELETE FROM groups WHERE demo_batch=$1").bind(batch).execute(db).await?;
    sqlx::query("DELETE FROM areas WHERE demo_batch=$1").bind(batch).execute(db).await?;
    sqlx::query("DELETE FROM audit_logs WHERE demo_batch=$1").bind(batch).execute(db).await?;
    sqlx::query("DELETE FROM users WHERE demo_batch=$1").bind(batch).execute(db).await?;
    sqlx::query("DELETE FROM demo_batches WHERE batch=$1").bind(batch).execute(db).await?;
    Ok(())
}

/// Delete the pre-batch legacy fixture (the original seed content): the legacy
/// demo user accounts (ids 1..=13) plus their cascade dependents and the unbatched
/// business-table rows they own. Real production data is kept untouched.
async fn clear_legacy(db: &PgPool) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM passkeys WHERE user_id IN (1,2,3,4,5,6,7,8,9,10,11,12,13)").execute(db).await?;
    sqlx::query("DELETE FROM vehicles WHERE user_id IN (1,2,3,4,5,6,7,8,9,10,11,12,13)").execute(db).await?;
    sqlx::query("DELETE FROM driver_areas WHERE user_id IN (1,2,3,4,5,6,7,8,9,10,11,12,13)").execute(db).await?;
    sqlx::query("DELETE FROM group_members WHERE user_id IN (1,2,3,4,5,6,7,8,9,10,11,12,13)").execute(db).await?;
    sqlx::query("DELETE FROM notifications WHERE user_id IN (1,2,3,4,5,6,7,8,9,10,11,12,13)").execute(db).await?;
    sqlx::query("DELETE FROM help_tickets WHERE user_id IN (1,2,3,4,5,6,7,8,9,10,11,12,13)").execute(db).await?;
    sqlx::query("DELETE FROM help_replies WHERE user_id IN (1,2,3,4,5,6,7,8,9,10,11,12,13)").execute(db).await?;
    sqlx::query("DELETE FROM files WHERE owner_id IN (1,2,3,4,5,6,7,8,9,10,11,12,13)").execute(db).await?;
    sqlx::query("DELETE FROM users WHERE id IN (1,2,3,4,5,6,7,8,9,10,11,12,13,50001,50002,50003,50004,50005,50006)").execute(db).await?;
    sqlx::query("DELETE FROM rides WHERE demo_batch IS NULL AND (passenger_id IN (1,2,3,4,5,6,7,8,9,10,11,12,13) OR driver_id IN (1,2,3,4,5,6,7,8,9,10,11,12,13) OR approved_by IN (1,2,3,4,5,6,7,8,9,10,11,12,13))").execute(db).await?;
    sqlx::query("DELETE FROM event_signups WHERE demo_batch IS NULL AND user_id IN (1,2,3,4,5,6,7,8,9,10,11,12,13)").execute(db).await?;
    sqlx::query("DELETE FROM event_groups WHERE demo_batch IS NULL AND event_id IN (SELECT id FROM events WHERE demo_batch IS NULL)").execute(db).await?;
    sqlx::query("DELETE FROM events WHERE demo_batch IS NULL AND created_by IN (1,2,3,4,5,6,7,8,9,10,11,12,13)").execute(db).await?;
    sqlx::query("DELETE FROM settlement_revisions WHERE demo_batch IS NULL AND settlement_id IN (SELECT id FROM settlements WHERE demo_batch IS NULL)").execute(db).await?;
    sqlx::query("DELETE FROM settlement_shares WHERE demo_batch IS NULL AND (settlement_id IN (SELECT id FROM settlements WHERE demo_batch IS NULL) OR user_id IN (1,2,3,4,5,6,7,8,9,10,11,12,13))").execute(db).await?;
    sqlx::query("DELETE FROM settlement_items WHERE demo_batch IS NULL AND settlement_id IN (SELECT id FROM settlements WHERE demo_batch IS NULL)").execute(db).await?;
    sqlx::query("DELETE FROM settlements WHERE demo_batch IS NULL AND (created_by IN (1,2,3,4,5,6,7,8,9,10,11,12,13) OR event_id IN (SELECT id FROM events WHERE demo_batch IS NULL))").execute(db).await?;
    sqlx::query("DELETE FROM reimbursements WHERE demo_batch IS NULL AND (user_id IN (1,2,3,4,5,6,7,8,9,10,11,12,13) OR ride_id IN (SELECT id FROM rides WHERE demo_batch IS NULL) OR reviewed_by IN (1,2,3,4,5,6,7,8,9,10,11,12,13) OR paid_by IN (1,2,3,4,5,6,7,8,9,10,11,12,13))").execute(db).await?;
    sqlx::query("DELETE FROM announcements WHERE demo_batch IS NULL AND created_by IN (1,2,3,4,5,6,7,8,9,10,11,12,13)").execute(db).await?;
    sqlx::query("DELETE FROM groups").execute(db).await?;
    sqlx::query("DELETE FROM areas").execute(db).await?;
    Ok(())
}


/// Reset sequences to the maximum inserted id so future real records keep stable
/// primary keys without colliding with generated demo data.
async fn setval_sequences(db: &PgPool) -> anyhow::Result<()> {
    let tables: &[(&str, &str)] = &[
        ("users", "id"), ("invites", "id"), ("passkeys", "id"), ("events", "id"),
        ("event_signups", "id"), ("rides", "id"), ("ride_messages", "id"),
        ("notifications", "id"), ("announcements", "id"), ("help_tickets", "id"),
        ("help_replies", "id"), ("files", "id"), ("reimbursements", "id"),
        ("settlements", "id"), ("settlement_items", "id"), ("settlement_shares", "id"),
        ("settlement_revisions", "id"), ("driver_locations", "driver_id"),
        ("driver_areas", "user_id"), ("group_members", "group_id"),
        ("event_groups", "event_id"), ("areas", "id"), ("groups", "id"),
        ("audit_logs", "id"), ("login_failures", "id"), ("recovery_requests", "id"),
        ("oauth_states", "id"), ("auth_challenges", "id"),
    ];
    for (table, col) in tables {
        sqlx::query(&format!(
            "SELECT setval(pg_get_serial_sequence($1, $2), COALESCE((SELECT MAX({col}) FROM {table}), 1), false)",
        )
        .bind(table)
        .bind(col)
        .execute(db)
        .await?;
    }
    Ok(())
}

/// Orchestrator: `seed demo [scale]` (10 | 100 | 1000 | stress=5000).
/// Scale = number of additional fictional members beyond the core demo (13 users).
pub async fn seed_demo(db: &PgPool, cfg: &Config, scale: i64) -> anyhow::Result<()> {
    clear_demo(db, DEMO_BATCH).await?;
    clear_legacy(db).await?;
    seed_core(db).await?;
    seed_scale(db, scale).await?;
    let mut rng = Rng::new();
    let mut counts = Counts { users: 0, invites: 0, vehicles: 0, events: 0, signups: 0, rides: 0, messages: 0, locations: 0, notifications: 0, reimbursements: 0, settlements: 0 };
    let transport = id_by_email(db, "transport@example.org").await?;
    let staff = id_by_email(db, "admin@example.org").await?;
    {
        let ids: Vec<i64> = sqlx::query_as("SELECT id FROM users WHERE demo_batch=$1 AND status='active'",)
            .bind(DEMO_BATCH).fetch_all(db).await.unwrap_or_default();
        counts.users = ids.len() as i64;
    }
    seed_rides(db, scale as i64, &mut rng, &mut counts, staff).await?;
    seed_notifications(db, &mut rng, &mut counts).await?;
    seed_settlements(db, &mut rng, &mut counts).await?;
    seed_reimbursements(db, &mut rng, &mut counts, &cfg.upload_dir).await?;
    if scale >= 10 {
        seed_edge(db).await?;
    }
    setval_sequences(db).await?;
    sqlx::query("INSERT INTO demo_batches (batch, scale, note, created_at) VALUES ($1,'demo', 'demo dataset', $2) ON CONFLICT (batch) DO UPDATE SET scale=excluded.scale, note=excluded.note, created_at=excluded.created_at")
        .bind(DEMO_BATCH).bind(now_kl()).execute(db).await?;
    tracing::info!("Demo dataset seeded (scale {}): {} users, {} rides", scale, counts.users, counts.rides);
    Ok(())
}
/// Reset the demo database: clear demo batches + legacy fixture, then re-seed
/// the demo dataset. Destructive by design - requires acknowledgement via the
/// GRACE_RIDE_RESET_DEMO=1 environment variable.
pub async fn seed_reset(db: &PgPool, cfg: &Config, acknowledge: bool) -> anyhow::Result<()> {
    if !acknowledge {
        anyhow::bail!(
            "seed reset is destructive. Set GRACE_RIDE_RESET_DEMO=1 to acknowledge and reset demo data in {}",
            cfg.database_url
        );
    }
    clear_demo(db, DEMO_BATCH).await?;
    clear_legacy(db).await?;
    let scale = std::env::var("GRACE_RIDE_RESET_SCALE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1000);
    seed_demo(db, cfg, scale).await
}

/// Verifies the seeded demo dataset. Checks volume per module, status coverage,
/// financial reconciliation (sum(shares) + subsidy == sum(items)), FK integrity,
/// demo-content sanity, simulated 2FA/passkeys, GPS concurrency cap and the
/// presence of an inaccessible receipt file. Returns Ok with a printed report on
/// success; returns Err on any failed check.
pub async fn seed_verify(db: &PgPool) -> anyhow::Result<()> {
    let batch = DEMO_BATCH;
    let mut checks: Vec<(bool, String)> = vec![];

    macro_rules! check {
        ($name:expr, $cond:expr) => {
            checks.push(($cond, format!($name)));
        };
    }

    // Data volume per module.
    check!("users (core+scale)", {
        let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE demo_batch=$1",)
            .bind(batch).fetch_one(db).await.unwrap_or(0);
        n >= 1013
    });
    check!("invites", {
        let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM invites WHERE demo_batch=$1",)
            .bind(batch).fetch_one(db).await.unwrap_or(0);
        n >= 52
    });
    check!("vehicles", {
        let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM vehicles WHERE demo_batch=$1",)
            .bind(batch).fetch_one(db).await.unwrap_or(0);
        n >= 15
    });
    check!("areas", { let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM areas").fetch_one(db).await.unwrap_or(0); n >= 8 });
    check!("groups", { let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM groups").fetch_one(db).await.unwrap_or(0); n >= 3 });
    check!("events", {
        let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM events WHERE demo_batch=$1",)
            .bind(batch).fetch_one(db).await.unwrap_or(0);
        n >= 19
    });
    check!("ride statuses covered", {
        let sql = "SELECT status FROM rides WHERE demo_batch=$1";
        let mut stmt = sqlx::query_scalar::<_, String>(sql).bind(batch);
        let statuses: Vec<String> = stmt.fetch_all(db).await.unwrap_or_default();
        for s in &[
            "pending_driver", "pending_passenger", "pending_approval", "pending_driver_confirm",
            "confirmed", "in_progress", "completed", "cancelled", "no_show", "alt_transport",
        ] {
            if !statuses.contains(&s.to_string()) { return false; }
        }
        true
    });
    check!("reimbursements statuses", {
        let sql = "SELECT status FROM reimbursements WHERE demo_batch=$1";
        let mut stmt = sqlx::query_scalar::<_, String>(sql).bind(batch);
        let statuses: Vec<String> = stmt.fetch_all(db).await.unwrap_or_default();
        for s in ["pending", "returned", "rejected", "approved", "paid"] {
            if !statuses.contains(&s.to_string()) { return false; }
        }
        true
    });
    check!("settlement share statuses", {
        let sql = "SELECT status FROM settlement_shares WHERE demo_batch=$1";
        let mut stmt = sqlx::query_scalar::<_, String>(sql).bind(batch);
        let statuses: Vec<String> = stmt.fetch_all(db).await.unwrap_or_default();
        for s in ["unpaid", "pending", "confirmed", "needs_info"] {
            if !statuses.contains(&s.to_string()) { return false; }
        }
        true
    });
    check!("event statuses", {
        let sql = "SELECT status FROM events WHERE demo_batch=$1";
        let mut stmt = sqlx::query_scalar::<_, String>(sql).bind(batch);
        let statuses: Vec<String> = stmt.fetch_all(db).await.unwrap_or_default();
        for s in ["published", "cancelled", "finished"] {
            if !statuses.contains(&s.to_string()) { return false; }
        }
        true
    });

    // Financial reconciliation: sum(shares) + subsidy == sum(items) for every settlement.
    check!("settlements reconcile", {
        let rows: Vec<(i64, i64, i64)> = sqlx::query_as(
            "SELECT s.id, COALESCE((SELECT SUM(sh.amount_cents) FROM settlement_shares sh WHERE sh.settlement_id=s.id),0), s.subsidy_cents
             FROM settlements s WHERE s.demo_batch=$1",

    // Demo content sanity.
    check!("invite statuses present", {
        let sql = "SELECT kind, used_at, expires_at < $1 FROM invites WHERE demo_batch=$1";
        let mut stmt = sqlx::query_as::<_, (String, Option<String>, bool)>(sql).bind(batch).bind(now_kl());
        let rows: Vec<(String, Option<String>, bool)> = stmt.fetch_all(db).await.unwrap_or_default();
        for n in &["invite", "recovery"] { if !rows.iter().any(|(k, _, _)| k == n) { return false; } }
        true
    });
    check!("sim-2fa users", {
        let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE demo_batch=$1 AND totp_enabled=1",)
            .bind(batch).fetch_one(db).await.unwrap_or(0);
        n >= 20
    });
    check!("sim-passkey records", {
        let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM passkeys WHERE demo_batch=$1",)
            .bind(batch).fetch_one(db).await.unwrap_or(0);
        n >= 15
    });
    check!("live drivers <= cap", {
        let (live,): (i64,) = sqlx::query_as("SELECT COUNT(DISTINCT driver_id) FROM rides WHERE status='in_progress' AND demo_batch=$1",)
            .bind(batch).fetch_one(db).await.unwrap_or(0);
        live <= 10
    });
    check!("missing receipt files", {
        let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM files WHERE demo_batch=$1",)
            .bind(batch).fetch_one(db).await.unwrap_or(0);
        n >= 1
    });
    check!("demo_batches tracked", {
        let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM demo_batches").fetch_one(db).await.unwrap_or(0);
        n >= 2
    });

    for (ok, name) in checks {
        if ok {
            tracing::info!("verify PASS: {name}");
        } else {
            tracing::error!("verify FAIL: {name}");
        }
    }
    if checks.iter().all(|(ok, _)| *ok) {
        tracing::info!("seed verify: ALL CHECKS PASSED");
        Ok(())
    } else {
        let failed = checks.iter().filter(|(ok, _)| !*ok).count();
        tracing::error!("seed verify: {failed} of {} checks failed", checks.len());
        Err(anyhow::anyhow!("seed verify failed: {failed} check(s)"));
    }
}
        )
        .bind(batch).fetch_all(db).await.unwrap_or_default();
        let items: Vec<(i64, i64)> = sqlx::query_as(
            "SELECT settlement_id, SUM(amount_cents) FROM settlement_items WHERE demo_batch=$1 GROUP BY settlement_id",
        )
        .bind(batch).fetch_all(db).await.unwrap_or_default();
        let im: std::collections::HashMap<i64, i64> = items.iter().cloned().collect();
        for (id, shares, subsidy) in rows {
            let expected = im.get(&id).copied().unwrap_or(0);
            if shares + subsidy != expected { return false; }
        }
        true
    });

    // FK integrity: demo ride passengers/drivers + event ids exist.
    check!("fk integrity", {
        let rides: Vec<(i64, i64, Option<i64>, Option<i64>)> = sqlx::query_as(
            "SELECT id, passenger_id, driver_id, event_id FROM rides WHERE demo_batch=$1",
        )
        .bind(batch).fetch_all(db).await.unwrap_or_default();
        let users: std::collections::HashSet<i64> = sqlx::query_scalar::<_, i64>("SELECT id FROM users WHERE demo_batch=$1",)
            .bind(batch).fetch_all(db).await.unwrap_or_default().into_iter().collect();
        let events: std::collections::HashSet<i64> = sqlx::query_scalar::<_, i64>("SELECT id FROM events WHERE demo_batch=$1",)
            .bind(batch).fetch_all(db).await.unwrap_or_default().into_iter().collect();
        for (id, p, d, e) in rides {
            if !users.contains(&p) { return false; }
            if let Some(x) = d { if !users.contains(&x) { return false; } }
            if let Some(x) = e { if !events.contains(&x) { return false; } }
        }
        true
    });

fn row_count(db: &PgPool, sql: &str, batch: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(sql).bind(batch).fetch_one(db).await.unwrap_or(0)
}

/// CLI: seed demo [scale] | seed edge | seed reset | seed verify
///   scale: 10 | 100 | 1000 | stress (5000, default 1000).
/// The seed command refuses to run unless DEV_MODE is on (dev only).
pub async fn cli(args: &[String], db: &PgPool, cfg: &Config) -> anyhow::Result<()> {
    if cfg.dev_mode {
        match args.first() {
            Some("demo") => {
                let scale = args.get(1).map_or(Ok(1000), |s| s.parse())
                    .map_err(|_| anyhow::anyhow!("scale must be a number"))?;
                let valid = [10, 100, 1000, 5000];
                if !valid.contains(&scale) {
                    anyhow::bail!("scale must be one of 10, 100, 1000, stress(5000)");
                }
                seed_demo(db, cfg, scale).await?;
                println!("seed demo: done (scale={scale})");
                Ok(())
            }
            Some("edge") => {
                if row_count(db, "SELECT COUNT(*) FROM users WHERE demo_batch=$1", EDGE_BATCH) == 0 {
                    seed_demo(db, cfg, 1000).await?;
                }
                seed_edge(db).await?;
                println!("seed edge: done");
                Ok(())
            }
            Some("reset") => {
                let ack = std::env::var("GRACE_RIDE_RESET_DEMO").unwrap_or_default() == "1";
                seed_reset(db, cfg, ack).await
            }
            Some("verify") => {
                seed_verify(db).await
            }
            _ => {
                eprintln!("usage: grace-ride seed demo [10|100|1000|stress] | seed edge | seed reset | seed verify");
                Err(anyhow::anyhow!("unknown seed subcommand"))
            }
        }
    } else {
        eprintln!("seed commands are dev-mode only (production always creates the first administrator only)");
        Err(anyhow::anyhow!("seed commands are dev-mode only"))
    }
}

fn batch(name: &str) -> String {
    name.to_string()
}

fn seed_count<'a>(f: impl FnOnce(&PgPool) -> sqlx::query::Query<'a, sqlx::Postgres, (i64,)>, db: &PgPool, batch: String) -> anyhow::Result<i64> {
    Ok(f(db).bind(&batch).fetch_one(db).await.unwrap_or((0,)).0)
}
