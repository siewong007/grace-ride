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

#![allow(dead_code, clippy::too_many_arguments, clippy::needless_borrow)]
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng as _, SeedableRng};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashSet;
use uuid::Uuid;

use crate::routes::auth::hash_password;
use crate::state::Config;
use crate::util::{audit, in_days, now_kl, random_token, today_kl};

pub const DEMO_PASSWORD: &str = "GraceRide2026!";
pub const DEMO_BATCH: &str = "demo-2026";
pub const EDGE_BATCH: &str = "edge-2026";
const PRNG_SEED: u64 = 0x6772_6163_655f_7365;
const GENERATED_MEMBER_ID_BASE: i64 = 10001;
const GENERATED_EVENT_ID_BASE: i64 = 1001;
const GENERATED_RIDE_ID_BASE: i64 = 30001;
const SIMULATED_TOTP_SECRET: &str = "MFRGGZDF73DGXYP3";
const LEGACY_DEMO_EMAILS: &[&str] = &[
    "admin@example.org",
    "transport@example.org",
    "finance@example.org",
    "driver@example.org",
    "driver2@example.org",
    "member@example.org",
    "amina@example.org",
    "sophie@example.org",
    "yawen@example.org",
    "zhiqiang@example.org",
    "meiling@example.org",
    "enci@example.org",
    "guorong@example.org",
];
const GIVEN_NAMES: &[&str] = &[
    "Ah Mei",
    "Demo",
    "Grace",
    "Kumar",
    "Rahman",
    "Siti",
    "Wei",
    "Ling",
    "Sample",
    "Fictional",
    "PJ",
    "Ride",
    "Member",
    "Volunteer",
    "Trial",
    "Demo Ana",
    "Grace Ridzuan",
    "Daniel Sample",
];
const FAMILY_NAMES: &[&str] = &[
    "Tan",
    "Lim",
    "Member",
    "Rider",
    "Driver",
    "Kumar",
    "Rahman",
    "Ali",
    "Demo",
    "Sample",
    "Test",
    "Fictional",
    "Raja",
    "Nair",
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
    "Perodua Myvi",
    "Honda City",
    "Toyota Vios",
    "Mazda 3",
    "Proton X50",
    "Nissan Almera",
    "Perodua Axia",
    "Toyota Innova",
];
const VEHICLE_COLORS: &[&str] = &["白色", "银色", "灰色", "蓝色", "红色", "黑色"];
const VEHICLE_PLATE_PREFIXES: &[&str] = &["W", "B", "V", "F", "S"];
const DESTINATIONS: &[&str] = &[
    "PJ State CMC 教会",
    "社区中心礼堂",
    "诊所入口",
    "PJ 室内羽球馆",
    "图书馆入口",
    "宵夜团契",
    "Sunway Pyramid 保龄球馆",
    "Putrajaya 湖畔",
    "Taman Jaya 公园入口",
    "北门入口",
];
const AREA_NAMES: &[&str] = &[
    "八打灵再也 PJ",
    "SS2",
    "新市",
    "梳邦再也",
    "白沙罗",
    "蒲种",
    "吉隆坡市中心",
    "Kelana Jaya",
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
        Self {
            inner: StdRng::seed_from_u64(PRNG_SEED),
        }
    }
    fn pick<T: Clone>(&mut self, vals: &[T]) -> T {
        vals.choose(&mut self.inner).unwrap().clone()
    }
    fn bool(&mut self, p: f64) -> bool {
        self.inner.gen_bool(p)
    }
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        self.inner.gen_range(lo..hi)
    }
    fn shuffle<T>(&mut self, vals: &mut [T]) {
        vals.shuffle(&mut self.inner);
    }
    fn unique_plate(&mut self, used: &mut HashSet<String>) -> String {
        loop {
            let letters: String = (&mut self.inner)
                .sample_iter(&rand::distributions::Alphanumeric)
                .take(3)
                .map(char::from)
                .collect();
            let plate = format!(
                "{} {} {}",
                self.pick(VEHICLE_PLATE_PREFIXES),
                letters,
                self.range(1000, 9999)
            );
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
    // 0005 FK requires demo_batches rows before any demo_batch-stamped seed insert.
    ensure_demo_batches(db).await?;
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE demo_batch = $1")
        .bind(DEMO_BATCH)
        .fetch_one(db)
        .await?;
    if n == 0 {
        seed_core(db).await?;
    }
    Ok(())
}

async fn ensure_demo_batches(db: &PgPool) -> anyhow::Result<()> {
    for (batch, note) in [
        (DEMO_BATCH, "Core demo fixtures"),
        (EDGE_BATCH, "Edge / scale fixtures"),
    ] {
        sqlx::query(
            "INSERT INTO demo_batches (batch, scale, note) VALUES ($1, 'core', $2) ON CONFLICT (batch) DO NOTHING",
        )
        .bind(batch)
        .bind(note)
        .execute(db)
        .await?;
    }
    Ok(())
}

/// Canonical reference data (areas + groups). Safe to re-run: uses ON CONFLICT.
async fn insert_reference_data(db: &PgPool) -> anyhow::Result<()> {
    for name in AREA_NAMES {
        sqlx::query(
            "INSERT INTO areas (name, demo_batch) VALUES ($1, NULL) ON CONFLICT (name) DO NOTHING",
        )
        .bind(name)
        .execute(db)
        .await?;
    }
    sqlx::query("INSERT INTO groups (name, description, is_all, demo_batch) VALUES ('全体会员', '所有获批准的会员', 1, NULL) ON CONFLICT (name) DO NOTHING").execute(db).await?;
    sqlx::query("INSERT INTO groups (name, description, demo_batch) VALUES ('青年团契', 'PJ State CMC 青年团契', NULL) ON CONFLICT (name) DO NOTHING").execute(db).await?;
    sqlx::query("INSERT INTO groups (name, description, demo_batch) VALUES ('运动兴趣小组', '羽球、保龄球、皮划艇与散步', NULL) ON CONFLICT (name) DO NOTHING").execute(db).await?;

    // Starter admin workflow guides — reference content, editable in-app by
    // help-centre staff. ON CONFLICT keeps operator edits on re-deploys.
    let guides: [(&str, &str, &str, &str, &str, serde_json::Value); 6] = [
        (
            "plan-transport",
            "为活动安排交通",
            "Plan transport for an event",
            "从报名到当天实况，完整走一遍交通安排流程。",
            "Walk through arranging rides for an event, from sign-ups to the live map.",
            json!([
                {"title_en": "Find the event", "title_zh": "找到活动",
                 "body_en": "Open Events and pick the service or activity that needs transport.",
                 "body_zh": "打开「活动」页，选择需要安排交通的聚会或活动。",
                 "link": "/events"},
                {"title_en": "Open the planning board", "title_zh": "打开交通规划板",
                 "body_en": "From the event, open Plan transport. Review demand by area, auto-arrange or assign drivers manually, then save.",
                 "body_zh": "在活动里进入「规划交通」。按区域查看需求，自动编排或手动指派司机后保存。",
                 "link": ""},
                {"title_en": "Watch it run on the day", "title_zh": "当天跟踪",
                 "body_en": "On the day, use the live map to follow drivers sharing GPS and react to delays.",
                 "body_zh": "活动当天通过实时地图查看共享位置的司机，及时处理延误。",
                 "link": "/admin/live"}
            ]),
        ),
        (
            "review-reimbursements",
            "审核与支付报销",
            "Review and pay reimbursements",
            "交通组先审核，财务再记录付款。",
            "Transport reviews claims first; finance records the payout.",
            json!([
                {"title_en": "Review the queue", "title_zh": "审核队列",
                 "body_en": "Open the reimbursement review page. Check the claim and receipt, then approve, return or reject.",
                 "body_zh": "打开报销审核页。核对申请与收据，然后批准、退回或拒绝。",
                 "link": "/admin/reimbursements"},
                {"title_en": "Record the payout", "title_zh": "记录付款",
                 "body_en": "Approved claims move to the payment queue. After transferring, record the payout with the reference.",
                 "body_zh": "已批准的申请进入付款队列。转账后记录付款及凭据编号。",
                 "link": "/admin/reimbursements"}
            ]),
        ),
        (
            "send-announcement",
            "发布通知",
            "Send an announcement",
            "向全体或指定群体发送重要通知。",
            "Send an important notice to everyone or a chosen group.",
            json!([
                {"title_en": "Compose it", "title_zh": "撰写通知",
                 "body_en": "Open Announcements, write a bilingual title and body, and choose the audience.",
                 "body_zh": "打开「通知」，填写中英文标题与内容，并选择接收群体。",
                 "link": "/admin/announcements"},
                {"title_en": "Members get notified", "title_zh": "会员收到通知",
                 "body_en": "Members see it under Notifications with a badge; urgent notices can also trigger reminders.",
                 "body_zh": "会员会在「通知」页看到并显示角标；重要通知还会触发提醒。",
                 "link": "/notifications"}
            ]),
        ),
        (
            "manage-members",
            "管理会员与邀请",
            "Manage members and invites",
            "审批新会员、发放邀请、处理账号找回。",
            "Approve members, hand out invites and handle account recovery.",
            json!([
                {"title_en": "The directory", "title_zh": "会员名录",
                 "body_en": "Open Members to see everyone with contact details. Approve pending applicants, disable or restore accounts, and set groups.",
                 "body_zh": "打开「会员」查看名录与联系方式。审批待定申请人、停用或恢复账号、调整群体。",
                 "link": "/admin/members"},
                {"title_en": "Invites and recovery", "title_zh": "邀请与找回",
                 "body_en": "Create single-use invite links for new members, and approve or reject recovery requests from the same area.",
                 "body_zh": "为新会员创建一次性邀请链接；账号找回申请也在同一处审批。",
                 "link": "/admin/members"}
            ]),
        ),
        (
            "work-help-desk",
            "处理帮助中心求助",
            "Work the help desk",
            "跟进会员在帮助中心提出的问题。",
            "Follow up on questions members raise in the help centre.",
            json!([
                {"title_en": "See every request", "title_zh": "查看全部求助",
                 "body_en": "Open the Help centre and switch to All requests. Open status means the member is waiting.",
                 "body_zh": "打开「帮助中心」并切换到「全部求助」。open 状态表示会员仍在等待。",
                 "link": "/help"},
                {"title_en": "Reply and close", "title_zh": "回复并结案",
                 "body_en": "Reply in the ticket thread, then close it once resolved. Closed tickets stay readable.",
                 "body_zh": "在对话中回复会员，问题解决后结案。已结案的求助仍可查看。",
                 "link": "/help"}
            ]),
        ),
        (
            "audit-settings",
            "审计与设置",
            "Audit and settings",
            "查看操作记录、调整系统设置。",
            "Review the action log and adjust system settings.",
            json!([
                {"title_en": "The audit log", "title_zh": "审计日志",
                 "body_en": "Open Audit to see who did what, with filters and CSV export. Retention follows the configured window.",
                 "body_zh": "打开「审计」查看操作记录，可筛选并导出 CSV。保存期限按配置执行。",
                 "link": "/admin/audit"},
                {"title_en": "Settings", "title_zh": "系统设置",
                 "body_en": "Settings holds groups, areas and app-wide configuration such as reimbursement rates.",
                 "body_zh": "「设置」管理群体、区域及全局配置（如报销费率）。",
                 "link": "/admin/settings"}
            ]),
        ),
    ];
    for (i, (slug, tzh, ten, szh, sen, steps)) in guides.iter().enumerate() {
        sqlx::query(
            "INSERT INTO guide_workflows (slug, title_zh, title_en, summary_zh, summary_en, steps, sort_order, published, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6::jsonb,$7,TRUE,$8) ON CONFLICT (slug) DO NOTHING",
        )
        .bind(slug)
        .bind(tzh)
        .bind(ten)
        .bind(szh)
        .bind(sen)
        .bind(steps.to_string())
        .bind(i as i32)
        .bind(now_kl())
        .execute(db)
        .await?;
    }
    Ok(())
}

/// Production path: first administrator + one-time recovery link, no demo data.
/// Existing members are left alone. A second insert would fail the unique email
/// and stop the process on every restart.
async fn seed_admin_only(db: &PgPool, cfg: &Config) -> anyhow::Result<()> {
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(db)
        .await?;
    if n > 0 {
        return Ok(());
    }
    let email = std::env::var("ADMIN_EMAIL").unwrap_or_else(|_| "admin@example.org".into());
    let id = sqlx::query_as::<_, (i64,)>("INSERT INTO users (name, email, roles, webauthn_uuid, demo_batch) VALUES ('管理员', $1, 'admin', $2, NULL) RETURNING id")
        .bind(&email).bind(stable_uuid(0)).fetch_one(db).await?.0;
    let token = random_token(32);
    sqlx::query("INSERT INTO invites (token, kind, email, name, roles, group_ids, user_id, created_by, expires_at, demo_batch) VALUES ($1, 'recovery', $2, '管理员', 'admin', '[]', $3, $3, $4, NULL)")
        .bind(crate::auth::hash_token(&token)).bind(&email).bind(id).bind(in_days(2)).execute(db).await?;
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
) -> sqlx::query::QueryAs<'a, sqlx::Postgres, (i64,), sqlx::postgres::PgArguments> {
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
        (
            1,
            "管理员 Grace",
            "admin@example.org",
            "012-300 0001",
            "admin",
            "active",
        ),
        (
            2,
            "黄美仪",
            "transport@example.org",
            "012-300 0002",
            "transport,driver",
            "active",
        ),
        (
            3,
            "林慧君",
            "finance@example.org",
            "012-300 0003",
            "finance",
            "active",
        ),
        (
            4,
            "李德明",
            "driver@example.org",
            "012-300 0004",
            "driver,leader",
            "active",
        ),
        (
            5,
            "张家乐",
            "driver2@example.org",
            "012-300 0005",
            "driver",
            "active",
        ),
        (
            6,
            "陈法拉",
            "member@example.org",
            "012-300 0006",
            "",
            "active",
        ),
        (
            7,
            "阿米娜",
            "amina@example.org",
            "012-300 0007",
            "",
            "active",
        ),
        (
            8,
            "林苏菲",
            "sophie@example.org",
            "012-300 0008",
            "",
            "active",
        ),
        (
            9,
            "周雅文",
            "yawen@example.org",
            "012-300 0011",
            "",
            "active",
        ),
        (
            10,
            "吴志强",
            "zhiqiang@example.org",
            "012-300 0012",
            "driver",
            "active",
        ),
        (
            11,
            "郑美玲",
            "meiling@example.org",
            "012-300 0013",
            "",
            "active",
        ),
        (
            12,
            "许恩慈",
            "enci@example.org",
            "012-300 0014",
            "leader",
            "active",
        ),
        (
            13,
            "陈国荣",
            "guorong@example.org",
            "012-300 0015",
            "driver,transport",
            "active",
        ),
    ];
    let mut id_of = std::collections::HashMap::<&str, i64>::new();
    for (id, name, email, phone, roles, status) in core {
        let uid = insert_member(
            db,
            &mut Rng::new(),
            id,
            name,
            email,
            phone,
            roles,
            status,
            DEMO_BATCH,
            &pw,
            false,
            false,
        )
        .await;
        id_of.insert(name, uid);
    }
    let get = |dbg: &str, map: &std::collections::HashMap<&str, i64>| {
        *map.get(dbg)
            .unwrap_or_else(|| panic!("unknown core member {dbg}"))
    };
    let admin = get("管理员 Grace", &id_of);
    let transport = get("黄美仪", &id_of);
    let _finance = get("林慧君", &id_of);
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
        (deming, 2),
        (deming, 3),
        (fala, 2),
        (fala, 3),
        (amina, 3),
        (sophie, 2),
        (jiale, 3),
        (transport, 2),
        (yawen, 2),
        (meiling, 2),
        (meiling, 3),
        (enci, 2),
        (zhiqiang, 3),
        (guorong, 3),
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
    sqlx::query("UPDATE users SET carpool_enabled=1, spare_seats=2 WHERE id=$1")
        .bind(deming)
        .execute(db)
        .await?;
    sqlx::query("UPDATE users SET carpool_enabled=0, spare_seats=1 WHERE id=$1")
        .bind(jiale)
        .execute(db)
        .await?;
    sqlx::query("UPDATE users SET carpool_enabled=1, spare_seats=3 WHERE id=$1")
        .bind(zhiqiang)
        .execute(db)
        .await?;
    for (uid, area) in [
        (deming, 1),
        (deming, 2),
        (deming, 4),
        (jiale, 3),
        (jiale, 6),
        (transport, 1),
        (transport, 5),
        (zhiqiang, 1),
        (zhiqiang, 5),
        (guorong, 2),
        (guorong, 7),
    ] {
        sqlx::query("INSERT INTO driver_areas (user_id, area_id, demo_batch) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING")
            .bind(uid).bind(area).bind(DEMO_BATCH).execute(db).await?;
    }
    let tomorrow = today + Duration::days(1);
    let kl_hm = |d: NaiveDate, h: i32, m: i32| kl(d, h, m);

    // --- events (ids 1..=7) ---
    let events = [
        (
            1i64,
            "周四羽毛球局",
            "badminton",
            "轻松双打交流，欢迎初学者。",
            kl_hm(next_weekday(today, Weekday::Thu), 19, 30),
            "PJ 室内羽球馆",
            12,
            "场租及羽球活动后平均分摊",
            deming,
            vec![2i64, 3],
        ),
        (
            2,
            "周末保龄球",
            "bowling",
            "周末保龄球活动。",
            kl_hm(next_weekday(today, Weekday::Sat), 14, 0),
            "Sunway Pyramid 保龄球馆",
            8,
            "每局约 RM12，自付",
            deming,
            vec![3],
        ),
        (
            3,
            "皮划艇体验",
            "kayak",
            "户外划艇体验，请穿运动服。",
            kl_hm(next_weekday(today, Weekday::Sat), 8, 0),
            "Putrajaya 湖畔",
            10,
            "器材租借费活动后分摊",
            deming,
            vec![2],
        ),
        (
            4,
            "公园散步",
            "walk",
            "轻松结伴散步，一起享受早晨。",
            kl_hm(next_weekday(today, Weekday::Sun), 7, 0),
            "Taman Jaya 公园入口",
            0,
            "免费",
            deming,
            vec![1],
        ),
        (
            5,
            "上周羽毛球局",
            "badminton",
            "双打交流（已结束，用于费用分摊示例）",
            kl_hm(today - Duration::days(5), 19, 30),
            "PJ 室内羽球馆",
            12,
            "场租 RM120 + 羽球 RM48",
            deming,
            vec![3],
        ),
        (
            6,
            "主日崇拜接送",
            "other",
            "主日崇拜前后的接送安排。",
            kl_hm(next_weekday(today, Weekday::Sun), 9, 0),
            "PJ State CMC 教会",
            0,
            "免费",
            transport,
            vec![1],
        ),
        (
            7,
            "团契聚餐",
            "other",
            "每人带一道菜，交通组协助接送。",
            kl_hm(next_weekday(today, Weekday::Fri), 18, 30),
            "社区中心礼堂",
            20,
            "自备食物",
            enci,
            vec![2],
        ),
    ];
    for (eid, title, kind, desc, at, loc, cap, cost, by, groups) in events {
        let deadline = format!("{}T12:00", &at[..10]);
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
        (
            20001i64,
            fala,
            None::<i64>,
            kl_hm(tomorrow, 8, 30),
            2,
            "SS2, Jalan 21/1 门牌 12",
            "社区中心 · 北门入口",
            1,
            "",
            "pending_driver",
        ),
        (
            20002,
            sophie,
            None,
            kl_hm(tomorrow, 9, 15),
            1,
            "PJ Section 14 公寓 B 座",
            "PJ State CMC 教会",
            2,
            "",
            "pending_driver",
        ),
        (
            20003,
            amina,
            Some(deming),
            kl_hm(tomorrow, 10, 10),
            4,
            "梳邦再也 SS15 门牌 8",
            "诊所入口",
            1,
            "",
            "pending_approval",
        ),
        (
            20004,
            fala,
            Some(deming),
            kl_hm(today, 20, 0),
            2,
            "SS2, Jalan 21/1 门牌 12",
            "PJ State CMC 教会",
            1,
            "",
            "confirmed",
        ),
        (
            20005,
            sophie,
            Some(jiale),
            kl_hm(today - Duration::days(3), 9, 5),
            1,
            "PJ Section 14 公寓 B 座",
            "图书馆入口",
            1,
            "",
            "completed",
        ),
        (
            20006,
            amina,
            Some(deming),
            kl_hm(today - Duration::days(10), 22, 0),
            4,
            "梳邦再也 SS15 门牌 8",
            "宵夜团契",
            1,
            "",
            "completed",
        ),
        (
            20007,
            yawen,
            Some(zhiqiang),
            kl_hm(tomorrow, 18, 40),
            6,
            "白沙罗 SS2 附近",
            "社区中心礼堂",
            2,
            "",
            "pending_passenger",
        ),
        (
            20008,
            enci,
            Some(guorong),
            kl_hm(today - Duration::days(1), 9, 0),
            7,
            "吉隆坡市中心公寓",
            "PJ State CMC 教会",
            1,
            "",
            "cancelled",
        ),
        (
            20009,
            yawen,
            Some(deming),
            kl_hm(today - Duration::days(8), 19, 20),
            2,
            "SS2, Jalan 21/1 门牌 12",
            "PJ 室内羽球馆",
            1,
            "",
            "no_show",
        ),
    ];
    for (id, p, d, at, area, addr, dest, seats, assistance, status) in rides {
        let completed = if status == "completed" {
            Some(at.clone())
        } else {
            None
        };
        sqlx::query_as::<_, (i64,)>("INSERT INTO rides (id, passenger_id, driver_id, pickup_at, pickup_area_id, pickup_address, destination, seats, assistance, status, created_at, updated_at, completed_at, demo_batch) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14) RETURNING id")
            .bind(id).bind(p).bind(d).bind(&at).bind(area).bind(addr).bind(dest).bind(seats).bind(assistance).bind(status).bind(now_kl()).bind(now_kl()).bind(completed).bind(DEMO_BATCH).fetch_one(db).await?;
        if matches!(status, "confirmed" | "completed") {
            sqlx::query("UPDATE rides SET trip_id=id, approved_by=$1 WHERE id=$2")
                .bind(transport)
                .bind(id)
                .execute(db)
                .await?;
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
    for (uid, status) in [
        (deming, "confirmed"),
        (fala, "unpaid"),
        (amina, "pending"),
        (jiale, "unpaid"),
        (transport, "confirmed"),
        (sophie, "unpaid"),
    ] {
        sqlx::query("INSERT INTO settlement_shares (id, settlement_id, user_id, amount_cents, status, proof_file_id, txn_ref, paid_date, note, updated_at, demo_batch) VALUES ($1,$2,$3,2800,$4,NULL,'',NULL,'',$5,$6) ON CONFLICT DO NOTHING")
            .bind(42000i64 + uid).bind(settle).bind(uid).bind(status).bind(now_kl()).bind(DEMO_BATCH).execute(db).await?;
    }
    sqlx::query("INSERT INTO settlement_revisions (id, settlement_id, version, snapshot, reason, created_by, demo_batch, created_at) VALUES ($1,$2,1,$3,'首次发布',$4,$5,$6) ON CONFLICT DO NOTHING")
        .bind(43001i64).bind(settle).bind(r#"{"title":"上周羽毛球费用","total_cents":16800}"#).bind(deming).bind(DEMO_BATCH).bind(now_kl()).execute(db).await?;

    audit(
        db,
        Some(admin),
        "seed.core",
        "users",
        Some(13),
        &format!(
            "core demo users {}-{}",
            core.first().unwrap().0,
            core.last().unwrap().0
        ),
    )
    .await;
    // Explicit ids above do not advance IDENTITY sequences; bump them so later
    // inserts (tests, capacity seed, invites) do not collide on primary keys.
    sync_identity_sequences(db).await?;
    tracing::info!("Core demo fixtures seeded ({} users)", id_of.len());
    Ok(())
}

/// Align each IDENTITY sequence with MAX(id) after seed inserts with fixed ids.
async fn sync_identity_sequences(db: &PgPool) -> anyhow::Result<()> {
    for table in [
        "users",
        "invites",
        "passkeys",
        "recovery_requests",
        "areas",
        "groups",
        "files",
        "events",
        "event_signups",
        "rides",
        "ride_messages",
        "notifications",
        "announcements",
        "help_tickets",
        "help_replies",
        "audit_logs",
        "login_failures",
        "oauth_accounts",
        "reimbursements",
        "settlements",
        "settlement_items",
        "settlement_shares",
        "settlement_revisions",
    ] {
        let sql = format!(
            "SELECT setval(pg_get_serial_sequence('{table}', 'id'), COALESCE((SELECT MAX(id) FROM {table}), 1))"
        );
        sqlx::query(&sql).execute(db).await?;
    }
    Ok(())
}
/// Scalable fictional membership: `scale` additional members with stable ids
/// starting at GENERATED_MEMBER_ID_BASE, plus vehicles, service areas and invites.
async fn seed_scale(db: &PgPool, scale: i64) -> anyhow::Result<()> {
    let n = scale.clamp(1, 50_000);
    let pw = hash_password(DEMO_PASSWORD).map_err(|e| anyhow::anyhow!("{e}"))?;
    let today = today_kl();
    let mut rng = Rng::new();
    let mut plate_pool = HashSet::new();
    for p in ["VXX 1234", "WQA 8821", "BPM 3302", "BKS 4410", "VBE 7788"] {
        plate_pool.insert(p.to_string());
    }

    // --- generated members ---
    for i in 0..n {
        let id = GENERATED_MEMBER_ID_BASE + i;
        let seq = i + 1;
        let name = format!("{} {}", rng.pick(GIVEN_NAMES), rng.pick(FAMILY_NAMES));
        let email = format!("grdemo{:05}@example.org", id);
        let phone = format!("012-300 {:04}", seq);
        let _address = rng.pick(ADDRESSES);
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
        insert_member(
            db, &mut rng, id, &name, &email, &phone, &roles, &status, DEMO_BATCH, &pw, totp,
            passkey,
        )
        .await;

        if roles.contains("driver") && rng.bool(0.90) {
            let carpool = if rng.bool(0.40) { 1 } else { 0 };
            let spare = if carpool == 1 { rng.range(2, 4) } else { 1 };
            let plate = rng.unique_plate(&mut plate_pool);
            sqlx::query("INSERT INTO vehicles (user_id, model, color, plate, seats, demo_batch) VALUES ($1,$2,$3,$4,5,$5) ON CONFLICT(user_id) DO UPDATE SET model=excluded.model, color=excluded.color, plate=excluded.plate, seats=excluded.seats, demo_batch=excluded.demo_batch")
                .bind(id).bind(rng.pick(VEHICLE_MODELS)).bind(rng.pick(VEHICLE_COLORS)).bind(plate).bind(DEMO_BATCH).execute(db).await?;
            sqlx::query("UPDATE users SET carpool_enabled=$1, spare_seats=$2 WHERE id=$3")
                .bind(carpool)
                .bind(spare)
                .bind(id)
                .execute(db)
                .await?;
            let mut areas: Vec<i64> = vec![1, 2, 4, 5, 7, 8];
            rng.shuffle(&mut areas);
            for area in areas.into_iter().take(rng.range(1, 4) as usize) {
                sqlx::query("INSERT INTO driver_areas (user_id, area_id, demo_batch) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING")
                    .bind(id).bind(area).bind(DEMO_BATCH).execute(db).await?;
            }
        }
    }
    drop(plate_pool);
    let _ = today;
    Ok(())
}
