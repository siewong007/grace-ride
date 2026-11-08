//! First-run data. In dev mode we load demo members so every screen has something to show;
//! in production we only create the first administrator and print a one-time setup link.

use chrono::{Datelike, Duration, Local, Weekday};
use sqlx::PgPool;

use crate::routes::auth::hash_password;
use crate::state::Config;
use crate::util::{in_days, now, random_token};

pub const DEMO_PASSWORD: &str = "GraceRide2026!";

pub async fn run(db: &PgPool, cfg: &Config) -> anyhow::Result<()> {
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(db)
        .await?;
    if n > 0 {
        return Ok(());
    }
    for a in [
        "八打灵再也 PJ",
        "SS2",
        "新市",
        "梳邦再也",
        "白沙罗",
        "蒲种",
        "吉隆坡市中心",
    ] {
        sqlx::query("INSERT INTO areas (name) VALUES ($1)")
            .bind(a)
            .execute(db)
            .await?;
    }
    sqlx::query(
        "INSERT INTO groups (name, description, is_all) VALUES ('全体会员', '所有获批准的会员', 1)",
    )
    .execute(db)
    .await?;
    sqlx::query(
        "INSERT INTO groups (name, description) VALUES ('青年团契', 'PJ State CMC 青年团契')",
    )
    .execute(db)
    .await?;
    sqlx::query("INSERT INTO groups (name, description) VALUES ('运动兴趣小组', '羽球、保龄球、皮划艇与散步')").execute(db).await?;

    if !cfg.dev_mode {
        let email = std::env::var("ADMIN_EMAIL").unwrap_or_else(|_| "admin@example.org".into());
        let id = sqlx::query_as::<_, (i64,)>("INSERT INTO users (name, email, roles, webauthn_uuid) VALUES ('管理员', $1, 'admin', $2) RETURNING id")
            .bind(&email)
            .bind(uuid::Uuid::new_v4().to_string())
            .fetch_one(db)
            .await?
            .0;
        let token = random_token(32);
        sqlx::query("INSERT INTO invites (token, kind, email, user_id, created_by, expires_at) VALUES ($1, 'recovery', $2, $3, $4, $5)")
            .bind(&token)
            .bind(&email)
            .bind(id)
            .bind(id)
            .bind(in_days(2))
            .execute(db)
            .await?;
        tracing::warn!("First administrator created for {email}. Open this one-time link to register a passkey: {}/invite/{token}", cfg.public_url);
        return Ok(());
    }

    let pw = hash_password(DEMO_PASSWORD).map_err(|e| anyhow::anyhow!("{e}"))?;
    let users = [
        ("管理员 Grace", "admin@example.org", "012-300 0001", "admin"),
        (
            "黄美仪",
            "transport@example.org",
            "012-300 0002",
            "transport,driver",
        ),
        ("林慧君", "finance@example.org", "012-300 0003", "finance"),
        (
            "李德明",
            "driver@example.org",
            "012-300 0004",
            "driver,leader",
        ),
        ("张家乐", "driver2@example.org", "012-300 0005", "driver"),
        ("陈法拉", "member@example.org", "012-300 0006", ""),
        ("阿米娜", "amina@example.org", "012-300 0007", ""),
        ("林苏菲", "sophie@example.org", "012-300 0008", ""),
    ];
    let mut ids = vec![];
    for (name, email, phone, roles) in users {
        let id = sqlx::query_as::<_, (i64,)>("INSERT INTO users (name, email, phone, roles, password_hash, webauthn_uuid) VALUES ($1,$2,$3,$4,$5,$6) RETURNING id")
            .bind(name)
            .bind(email)
            .bind(phone)
            .bind(roles)
            .bind(&pw)
            .bind(uuid::Uuid::new_v4().to_string())
            .fetch_one(db)
            .await?
            .0;
        ids.push(id);
    }
    let (admin, transport, _finance, deming, jiale, fala, amina, sophie) = (
        ids[0], ids[1], ids[2], ids[3], ids[4], ids[5], ids[6], ids[7],
    );

    for (uid, g) in [
        (deming, 2),
        (deming, 3),
        (fala, 2),
        (fala, 3),
        (amina, 3),
        (sophie, 2),
        (jiale, 3),
        (transport, 2),
    ] {
        sqlx::query("INSERT INTO group_members (group_id, user_id) VALUES ($1,$2)")
            .bind(g)
            .bind(uid)
            .execute(db)
            .await?;
    }
    sqlx::query("INSERT INTO vehicles (user_id, model, color, plate, seats) VALUES ($1, 'Perodua Myvi', '白色', 'VXX 1234', 5)").bind(deming).execute(db).await?;
    sqlx::query("INSERT INTO vehicles (user_id, model, color, plate, seats) VALUES ($1, 'Honda City', '银色', 'WQA 8821', 5)").bind(jiale).execute(db).await?;
    sqlx::query("INSERT INTO vehicles (user_id, model, color, plate, seats) VALUES ($1, 'Proton X50', '灰色', 'BPM 3302', 5)").bind(transport).execute(db).await?;
    sqlx::query("UPDATE users SET carpool_enabled=1, spare_seats=2 WHERE id=$1")
        .bind(deming)
        .execute(db)
        .await?;
    sqlx::query("UPDATE users SET carpool_enabled=0, spare_seats=1 WHERE id=$1")
        .bind(jiale)
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
    ] {
        sqlx::query("INSERT INTO driver_areas (user_id, area_id) VALUES ($1,$2)")
            .bind(uid)
            .bind(area)
            .execute(db)
            .await?;
    }

    let today = Local::now().date_naive();
    let next = |wd: Weekday| {
        let mut d = today + Duration::days(1);
        while d.weekday() != wd {
            d += Duration::days(1);
        }
        d
    };
    let tomorrow = today + Duration::days(1);
    let fmt = |d: chrono::NaiveDate, t: &str| format!("{}T{}", d.format("%Y-%m-%d"), t);

    // Events
    let events = [
        (
            "周四羽毛球局",
            "badminton",
            "轻松双打交流，欢迎初学者。",
            fmt(next(Weekday::Thu), "19:30"),
            "PJ 室内羽球馆",
            12,
            "场租及羽球活动后平均分摊",
            deming,
            vec![2, 3],
        ),
        (
            "周末保龄球",
            "bowling",
            "周末保龄球活动。",
            fmt(next(Weekday::Sat), "14:00"),
            "Sunway Pyramid 保龄球馆",
            8,
            "每局约 RM12，自付",
            deming,
            vec![3],
        ),
        (
            "皮划艇体验",
            "kayak",
            "户外划艇体验，请穿运动服。",
            fmt(next(Weekday::Sat), "08:00"),
            "Putrajaya 湖畔",
            10,
            "器材租借费活动后分摊",
            deming,
            vec![2],
        ),
        (
            "公园散步",
            "walk",
            "轻松结伴散步，一起享受早晨。",
            fmt(next(Weekday::Sun), "07:00"),
            "Taman Jaya 公园入口",
            0,
            "免费",
            deming,
            vec![1],
        ),
    ];
    for (title, kind, desc, at, loc, cap, cost, by, groups) in events {
        let eid = sqlx::query_as::<_, (i64,)>("INSERT INTO events (title, kind, description, starts_at, location, capacity, cost_note, created_by, signup_deadline) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING id")
            .bind(title).bind(kind).bind(desc).bind(&at).bind(loc).bind(cap).bind(cost).bind(by)
            .bind(format!("{}T12:00", &at[..10]))
            .fetch_one(db).await?.0;
        for g in groups {
            sqlx::query("INSERT INTO event_groups (event_id, group_id) VALUES ($1,$2)")
                .bind(eid)
                .bind(g)
                .execute(db)
                .await?;
        }
    }
    sqlx::query("INSERT INTO event_signups (event_id, user_id, status, need_ride) VALUES (1, $1, 'going', 1)").bind(fala).execute(db).await?;
    sqlx::query("INSERT INTO event_signups (event_id, user_id, status, offer_drive, offer_seats) VALUES (1, $1, 'going', 1, 2)").bind(deming).execute(db).await?;

    // A finished past badminton game with attendance, ready for 费用分摊.
    let past = fmt(today - Duration::days(5), "19:30");
    let past_id = sqlx::query_as::<_, (i64,)>("INSERT INTO events (title, kind, description, starts_at, location, capacity, cost_note, created_by, status) VALUES ('上周羽毛球局','badminton','双打交流',$1, 'PJ 室内羽球馆', 12, '场租 RM120 + 羽球 RM48', $2, 'finished') RETURNING id")
        .bind(&past).bind(deming).fetch_one(db).await?.0;
    sqlx::query("INSERT INTO event_groups (event_id, group_id) VALUES ($1, 3)")
        .bind(past_id)
        .execute(db)
        .await?;
    for uid in [deming, fala, amina, jiale, transport, sophie] {
        sqlx::query("INSERT INTO event_signups (event_id, user_id, status, attended) VALUES ($1, $2, 'going', 1)").bind(past_id).bind(uid).execute(db).await?;
    }

    // Rides in different states
    let rides = [
        (
            fala,
            None::<i64>,
            fmt(tomorrow, "08:30"),
            2,
            "SS2, Jalan 21/1 门牌 12",
            "社区中心 · 北门入口",
            1,
            "pending_driver",
        ),
        (
            sophie,
            None,
            fmt(tomorrow, "09:15"),
            1,
            "PJ Section 14 公寓 B 座",
            "PJ State CMC 教会",
            2,
            "pending_driver",
        ),
        (
            amina,
            Some(deming),
            fmt(tomorrow, "10:10"),
            4,
            "梳邦再也 SS15 门牌 8",
            "诊所入口",
            1,
            "pending_approval",
        ),
        (
            fala,
            Some(deming),
            fmt(today, "20:00"),
            2,
            "SS2, Jalan 21/1 门牌 12",
            "PJ State CMC 教会",
            1,
            "confirmed",
        ),
        (
            sophie,
            Some(jiale),
            fmt(today - Duration::days(3), "09:05"),
            1,
            "PJ Section 14 公寓 B 座",
            "图书馆入口",
            1,
            "completed",
        ),
        (
            amina,
            Some(deming),
            fmt(today - Duration::days(10), "22:00"),
            4,
            "梳邦再也 SS15 门牌 8",
            "宵夜团契",
            1,
            "completed",
        ),
    ];
    for (p, d, at, area, addr, dest, seats, status) in rides {
        let rid = sqlx::query_as::<_, (i64,)>("INSERT INTO rides (passenger_id, driver_id, pickup_at, pickup_area_id, pickup_address, destination, seats, status, created_at, updated_at, completed_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING id")
            .bind(p).bind(d).bind(&at).bind(area).bind(addr).bind(dest).bind(seats).bind(status).bind(now()).bind(now())
            .bind(if status == "completed" { Some(at.clone()) } else { None })
            .fetch_one(db).await?.0;
        if matches!(status, "confirmed" | "completed") {
            sqlx::query("UPDATE rides SET trip_id=id, approved_by=$1 WHERE id=$2")
                .bind(transport)
                .bind(rid)
                .execute(db)
                .await?;
            sqlx::query("INSERT INTO ride_messages (trip_id, user_id, body) VALUES ($1, NULL, '行程已获批准，群聊已开启。')").bind(rid).execute(db).await?;
        }
        if status == "confirmed" {
            sqlx::query("INSERT INTO ride_messages (trip_id, user_id, body) VALUES ($1, $2, '是在北门入口集合吗？')").bind(rid).bind(p).execute(db).await?;
            sqlx::query(
                "INSERT INTO ride_messages (trip_id, user_id, body) VALUES ($1, $2, '对，8 点见！')",
            )
            .bind(rid)
            .bind(d)
            .execute(db)
            .await?;
        }
    }

    sqlx::query("INSERT INTO announcements (title, body, pinned, created_by) VALUES ('欢迎使用恩典同行', '接送、活动与重要消息，集中在同一个平台。如有问题，请到帮助中心提问。', 1, $1)")
        .bind(admin).execute(db).await?;
    sqlx::query("INSERT INTO announcements (title, body, created_by) VALUES ('本周日接送安排', '请于周五前提交周日崇拜的接送报名，交通组会在周六确认。', $1)")
        .bind(transport).execute(db).await?;

    tracing::warn!(
        "Demo data loaded. All demo accounts use password {DEMO_PASSWORD} (dev mode only)."
    );
    Ok(())
}
