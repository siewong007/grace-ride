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
use rand::seq::{IndexedRandom, SliceRandom};
use rand::{RngExt as _, SeedableRng};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashSet;
use uuid::Uuid;

use crate::routes::auth::hash_password;
use crate::state::Config;
use crate::util::{audit, in_days, now, random_token, today_local};

pub const DEMO_PASSWORD: &str = "GraceRide2026!";
pub const DEMO_BATCH: &str = "demo-2026";
pub const EDGE_BATCH: &str = "edge-2026";
const PRNG_SEED: u64 = 0x6772_6163_655f_7365;
const GENERATED_MEMBER_ID_BASE: i64 = 10001;
const GENERATED_EVENT_ID_BASE: i64 = 1001;
const GENERATED_RIDE_ID_BASE: i64 = 30001;
pub const SIMULATED_TOTP_SECRET: &str = "MFRGGZDF73DGXYP3";
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
    "president@example.org",
    "vicepresident@example.org",
    "secretary@example.org",
    "traffichead@example.org",
    "cellleader@example.org",
    "auditor@example.org",
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
        self.inner.random_bool(p)
    }
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        self.inner.random_range(lo..hi)
    }
    fn shuffle<T>(&mut self, vals: &mut [T]) {
        vals.shuffle(&mut self.inner);
    }
    fn unique_plate(&mut self, used: &mut HashSet<String>) -> String {
        loop {
            let letters: String = (&mut self.inner)
                .sample_iter(&rand::distr::Alphanumeric)
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
    // The demo_batch foreign keys require demo_batches rows before any demo_batch-stamped seed insert.
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
    // Visitors who come with a member join events as this group (visitor passes). Created
    // after the groups above: demo memberships refer to those by id (2 = 青年团契, 3 = 运动兴趣小组).
    sqlx::query("INSERT INTO groups (name, description, is_visitor) VALUES ('访客', '会员带来的访客', 1) ON CONFLICT (name) DO UPDATE SET is_visitor = 1").execute(db).await?;

    // Starter admin workflow guides — reference content, editable in-app by
    // help-centre staff. ON CONFLICT keeps operator edits on re-deploys.
    let guides: Vec<(&str, &str, &str, &str, &str, &str, serde_json::Value)> = vec![
        (
            "plan-transport",
            "为活动安排交通",
            "Plan transport for an event",
            "从报名到当天实况，完整走一遍交通安排流程。",
            "Walk through arranging rides for an event, from sign-ups to the live map.",
            "staff",
            json!([
                {"title_en": "Find the event", "title_zh": "找到活动",
                 "body_en": "Open Events and pick the service or activity that needs transport.",
                 "body_zh": "打开「活动」页，选择需要安排交通的聚会或活动。",
                 "link": "/events/*"},
                {"title_en": "Open the planning board", "title_zh": "打开交通规划板",
                 "body_en": "From the event, open Plan transport. Review demand by area, auto-arrange or assign drivers manually, then save.",
                 "body_zh": "在活动里进入「规划交通」。按区域查看需求，自动编排或手动指派司机后保存。",
                 "link": "/admin/events/*/plan"},
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
            "staff",
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
            "staff",
            json!([
                {"title_en": "Compose it", "title_zh": "撰写通知",
                 "body_en": "Open Announcements, write a bilingual title and body, and choose the audience.",
                 "body_zh": "打开「通知」，填写中英文标题与内容，并选择接收群体。",
                 "link": "/admin/announcements"},
                {"title_en": "Members get notified", "title_zh": "会员收到通知",
                 "body_en": "Members see it under Notifications with a badge; urgent notices can also trigger reminders.",
                 "body_zh": "会员会在「通知」页看到并显示角标；重要通知还会触发提醒。",
                 "link": "/admin/announcements"}
            ]),
        ),
        (
            "manage-members",
            "管理会员",
            "Manage members",
            "审批新会员、设置小组、停用或恢复账号。",
            "Approve members, set groups, and disable or restore accounts.",
            "staff",
            json!([
                {"title_en": "The directory", "title_zh": "会员名录",
                 "body_en": "Open Members to see everyone with contact details. Approve pending applicants, disable or restore accounts, and set groups.",
                 "body_zh": "打开「会员」查看名录与联系方式。审批待定申请人、停用或恢复账号、调整群体。",
                 "link": "/admin/members"}
            ]),
        ),
        (
            "work-help-desk",
            "处理帮助中心求助",
            "Work the help desk",
            "跟进会员在帮助中心提出的问题。",
            "Follow up on questions members raise in the help centre.",
            "staff",
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
            "查看审计日志",
            "Read the audit log",
            "查看谁做了什么，可筛选并导出。",
            "See who did what, filter it and export it.",
            "staff",
            json!([
                {"title_en": "The audit log", "title_zh": "审计日志",
                 "body_en": "Open Audit to see who did what, with filters and CSV export. Retention follows the configured window.",
                 "body_zh": "打开「审计」查看操作记录，可筛选并导出 CSV。保存期限按配置执行。",
                 "link": "/admin/audit"}
            ]),
        ),
        (
            "getting-started",
            "开始使用",
            "Getting started",
            "从邀请注册到保护账号与找回账号。",
            "Accept an invitation, set up your account and recover access.",
            "member",
            json!([
                {"title_en":"Accept your invite","title_zh":"接受邀请","body_en":"Open your invitation, register a passkey, sign in and accept the terms.","body_zh":"打开邀请链接、登记通行密钥、登录并接受条款。","link":""},
                {"title_en":"Set up and recover your account","title_zh":"设置及找回账号","body_en":"Check your profile and language, add a passkey and configure 2FA. If you lose access, request recovery and wait for staff approval.","body_zh":"检查个人资料和语言，添加通行密钥并设置双重验证；若无法登录，请申请找回并等待职员审批。","link":"/me"}
            ]),
        ),
        (
            "request-a-ride",
            "申请接送",
            "Request a ride",
            "为自己或代他人申请接送，也可关联活动。",
            "Request a ride for yourself or someone else, and link it to an event.",
            "member",
            json!([
                {"title_en":"Create a ride request","title_zh":"新增接送申请","body_en":"Choose pickup, destination and time. Enter the proxy rider's details when requesting for someone else.","body_zh":"填写上车点、目的地和时间；代他人申请时填写代乘者资料。","link":"/rides/new"},
                {"title_en":"Link an event","title_zh":"关联活动","body_en":"Select the related event so organisers can coordinate transport.","body_zh":"选择相关活动，方便负责人统筹交通。","link":"/rides/new"}
            ]),
        ),
        (
            "track-your-ride",
            "跟进行程",
            "Track your ride",
            "回应司机安排、查看准备状态、分享位置、聊天或取消。",
            "Respond to a driver, check readiness, share location, chat or cancel.",
            "member",
            json!([
                {"title_en":"Respond and get ready","title_zh":"回应并准备","body_en":"Open your ride to accept or decline the driver and confirm your readiness.","body_zh":"打开行程接受或拒绝司机安排，并确认已准备好。","link":"/rides"},
                {"title_en":"Coordinate or cancel","title_zh":"联系或取消","body_en":"Share your location when asked, use ride chat for updates and cancel promptly if plans change.","body_zh":"按需要分享位置，用行程聊天更新情况；计划有变时尽早取消。","link":"/rides/*"}
            ]),
        ),
        (
            "volunteer-driving",
            "义务驾驶",
            "Volunteer driving",
            "登记司机资料、接单并完成行程。",
            "Set up your driver profile, claim rides and complete trips.",
            "member",
            json!([
                {"title_en":"Set up and claim","title_zh":"设置资料并接单","body_en":"Add your vehicle and available seats in Profile, then choose a ride in Drive. Confirm any reassignment before setting off.","body_zh":"在个人资料填写车辆和座位，再到「驾驶」选择行程；重新指派后须先确认。","link":"/drive"},
                {"title_en":"Complete or withdraw","title_zh":"完成或退出","body_en":"Start the trip, mark pickup and complete it. Report delays, gate issues or no-shows in chat; withdraw early if unable to drive.","body_zh":"开始行程、标记接载并完成；延误、门禁或乘客未到请在聊天说明，无法驾驶时尽早退出。","link":"/drive"}
            ]),
        ),
        (
            "join-events",
            "参加活动",
            "Join events",
            "报名活动、提供驾驶协助、候补或退出。",
            "Sign up for events, offer to drive, join a waitlist or withdraw.",
            "member",
            json!([
                {"title_en":"Browse and sign up","title_zh":"浏览并报名","body_en":"Browse upcoming events and open one to sign up or offer to drive.","body_zh":"浏览即将举行的活动，打开活动报名或表示愿意驾驶。","link":"/events"},
                {"title_en":"Manage your place","title_zh":"管理报名","body_en":"Check signup or waitlist status and withdraw if plans change.","body_zh":"查看报名或候补状态；计划有变时退出活动。","link":"/events/*"}
            ]),
        ),
        (
            "organise-an-event",
            "筹办活动",
            "Organise an event",
            "负责人管理活动、报名、出席和结束流程。",
            "Create and manage events, signups, attendance and completion.",
            "member",
            json!([
                {"title_en":"Create and manage signups","title_zh":"新增活动并管理报名","body_en":"Organisers create or edit events, review signups and driver offers.","body_zh":"活动负责人新增或编辑活动，并查看报名及驾驶意向。","link":"/events/new"},
                {"title_en":"Record attendance and finish","title_zh":"记录出席并结束","body_en":"Record attendance, manage signups, cancel when needed or mark the event finished.","body_zh":"记录出席、管理报名；需要时取消活动或标记完成。","link":"/events/*"}
            ]),
        ),
        (
            "split-activity-costs",
            "分摊活动费用",
            "Split activity costs",
            "负责人建立账单、发布、核实付款并结账。",
            "Create, publish and close an activity bill.",
            "member",
            json!([
                {"title_en":"Create the bill","title_zh":"建立账单","body_en":"From an event, list expenses, choose participants and calculate each share.","body_zh":"从活动建立账单、列出费用、选择参与者并计算分摊。","link":"/events/*/settle"},
                {"title_en":"Publish and close","title_zh":"发布并结账","body_en":"Review and publish the split, verify submitted payments and close the bill when settled.","body_zh":"核对并发布分摊，确认付款资料并在结清后关闭账单。","link":"/settlements"}
            ]),
        ),
        (
            "pay-your-share",
            "缴付分摊费用",
            "Pay your share",
            "查看应缴费用并提交付款证明。",
            "Review what you owe and submit payment proof.",
            "member",
            json!([
                {"title_en":"Review your costs","title_zh":"查看应缴费用","body_en":"Open My costs to see published bills and your amount due.","body_zh":"打开「我的费用」查看已发布账单和应缴金额。","link":"/costs"},
                {"title_en":"Submit payment proof","title_zh":"提交付款证明","body_en":"Open the bill, upload transfer proof and wait for the organiser to verify it.","body_zh":"打开账单并上传转账证明，等待负责人核实。","link":"/costs"}
            ]),
        ),
        (
            "claim-reimbursement",
            "申请报销",
            "Claim reimbursement",
            "提交 Grab 车费申请并按要求补交资料。",
            "Submit a Grab fare claim and resubmit returned details.",
            "member",
            json!([
                {"title_en":"Submit a claim","title_zh":"提交申请","body_en":"Choose an eligible ride, enter payee details and attach the receipt.","body_zh":"选择符合条件的行程，填写收款资料并附上收据。","link":"/reimbursements/new"},
                {"title_en":"Resubmit a returned claim","title_zh":"补资料重新提交","body_en":"Read the staff note, correct the details and submit the returned claim again.","body_zh":"阅读职员备注、修正资料后重新提交退回的申请。","link":"/reimbursements/*"}
            ]),
        ),
        (
            "answer-a-driver-offer",
            "回复司机的接送邀请",
            "Answer a driver's offer",
            "义工司机愿意接送您时，查看后接受或拒绝。",
            "When a volunteer offers to drive you, check the details and accept or decline.",
            "member",
            json!([
                {"title_en":"Open the ride","title_zh":"打开接送","body_en":"Home shows “… offered to drive you”, and the ride says “Your reply needed”. Open it to see the driver and their car.","body_zh":"首页会显示「… 愿意接送您」，接送状态为「待您回复」。打开即可查看司机与车辆。","link":"/rides"},
                {"title_en":"Accept or decline","title_zh":"接受或拒绝","body_en":"Accept and the transport team confirms the ride. Decline and the request goes back to other drivers.","body_zh":"接受后由交通组确认接送；拒绝则重新开放给其他司机。","link":"/rides/*"}
            ]),
        ),
        (
            "cant-make-it",
            "当天无法出行",
            "Can't make it on the day",
            "告诉司机您准备好了、需要稍等或无法出行。",
            "Tell your driver you're ready, running late, or can't come.",
            "member",
            json!([
                {"title_en":"Open today's ride","title_zh":"打开今天的接送","body_en":"On the day, your ride shows a readiness check right under the driver.","body_zh":"接送当天，司机资料下方会显示出行确认。","link":"/rides"},
                {"title_en":"Ready or running late","title_zh":"准时或稍慢","body_en":"Tap “Ready on time”, or “Need 5-10 mins” so your driver can plan.","body_zh":"点「准时出发」，或点「稍慢 5-10 分钟」方便司机安排。","link":"/rides/*"},
                {"title_en":"Can't join today","title_zh":"今天无法出行","body_en":"Tap “Can't join today” and add a reason if you like. Your driver skips your stop at once; the reason is seen by the driver and transport team. Days before, use “Cancel ride” instead.","body_zh":"点「今天无法出行」，可选填原因。司机会立即跳过您的站点，原因只有司机与交通组可见。提前几天请改用「取消接送」。","link":"/rides/*"}
            ]),
        ),
        (
            "join-as-a-group",
            "以群体报名活动",
            "Sign up for an event as your group",
            "活动面向群体及其子群体；报名时选择以哪个群体参加。",
            "Events are for groups and their sub-groups; choose which group you come as.",
            "member",
            json!([
                {"title_en":"Find events for you","title_zh":"找到适合您的活动","body_en":"Events lists what's open to your groups, including events for a group yours belongs to.","body_zh":"「活动」列出对您群体开放的活动，包括您所属上级群体的活动。","link":"/events"},
                {"title_en":"Choose who you're joining as","title_zh":"选择以哪个群体参加","body_en":"If more than one of your groups fits, pick one before signing up. It decides which drivers can take your ride.","body_zh":"若您有多个群体适用，报名前请选择其一；这决定哪些司机可以接送您。","link":"/events/*"},
                {"title_en":"Ask for a ride","title_zh":"申请接送","body_en":"Once you're going, “I need a ride to the event” appears on the event page.","body_zh":"报名成功后，活动页面会出现「我需要接送」。","link":"/events/*"}
            ]),
        ),
        (
            "bring-a-visitor",
            "带访客参加活动",
            "Bring a visitor to an event",
            "为非会员朋友预约接送，并发送一次性链接。",
            "Book a ride for a friend without an account and send them a one-time link.",
            "member",
            json!([
                {"title_en":"Invite them","title_zh":"邀请访客","body_en":"Sign up for the event, then under “Bringing a visitor” tap “Invite a visitor” and enter their name and pickup.","body_zh":"先报名活动，然后在「带访客同行」中点「邀请访客」，填写姓名与上车地点。","link":"/events"},
                {"title_en":"Send the link","title_zh":"发送链接","body_en":"Share the link on WhatsApp. It opens 24 hours before pickup and works on the one phone that opens it first.","body_zh":"通过 WhatsApp 分享链接。链接于接送前 24 小时开放，只能在第一部打开它的手机上使用。","link":"/events/*"},
                {"title_en":"If something changes","title_zh":"如有变动","body_en":"Send a new link if they change phone, or turn the link off. The link stops by itself when the journey ends; cancel their ride from the ride page if they aren't coming.","body_zh":"对方换手机可发送新链接，也可停用链接。行程结束后链接会自动失效；若对方不来，请在接送页面取消。","link":"/events/*"}
            ]),
        ),
        (
            "ask-for-help",
            "寻求协助",
            "Ask for help",
            "提交问题、回复职员并在解决后结案。",
            "Open a help ticket, reply to staff and close it when resolved.",
            "member",
            json!([
                {"title_en":"Open a request","title_zh":"提交求助","body_en":"Describe your question clearly and include the related ride when relevant.","body_zh":"清楚说明问题；若与行程有关，请关联行程。","link":"/help"},
                {"title_en":"Follow up and close","title_zh":"跟进并结案","body_en":"Reply in the ticket thread and mark it resolved once you have the answer.","body_zh":"在求助对话中回复；问题解决后标记为已解决。","link":"/help"}
            ]),
        ),
        (
            "notifications-and-report",
            "通知与报告",
            "Notifications and reports",
            "查看通知、首页动态和每月报告。",
            "Read notifications, the home feed and monthly reports.",
            "member",
            json!([
                {"title_en":"Read notifications","title_zh":"查看通知","body_en":"Open Notifications to read announcements and updates addressed to you.","body_zh":"打开「通知」阅读发给你的公告和更新。","link":"/notifications"},
                {"title_en":"Check the home feed and report","title_zh":"查看首页动态与报告","body_en":"The home page shows current activity; reports summarise your monthly rides and participation.","body_zh":"首页显示近期动态；报告汇总每月行程和参与情况。","link":"/report"}
            ]),
        ),
        (
            "admin-dashboard",
            "管理仪表板",
            "Admin dashboard",
            "查看待处理工作并进入管理功能。",
            "Review pending work and open staff tools.",
            "staff",
            json!([
                {"title_en":"Review the dashboard","title_zh":"查看仪表板","body_en":"The dashboard highlights queues and recent activity. Open each section to take action.","body_zh":"仪表板显示待处理队列和近期动态；进入各区处理事项。","link":"/admin"}
            ]),
        ),
        (
            "manage-rides",
            "管理接送",
            "Manage rides",
            "审核申请、退回、重新指派及处理特殊情况。",
            "Review requests, reassign drivers and handle exceptions.",
            "staff",
            json!([
                {"title_en":"Review and assign","title_zh":"审核与指派","body_en":"Approve or return requests, assign or reassign drivers and arrange alternative transport when needed.","body_zh":"批准或退回申请，指派或重新指派司机；需要时安排替代交通。","link":"/admin/rides"},
                {"title_en":"Resolve changes","title_zh":"处理变更","body_en":"Record no-shows and cancellations on the ride detail page.","body_zh":"在行程详情记录乘客未到和取消。","link":"/admin/rides"}
            ]),
        ),
        (
            "live-map",
            "实时地图",
            "Live map",
            "监看正在进行的接送并处理延误。",
            "Monitor active rides and respond to delays.",
            "staff",
            json!([
                {"title_en":"Monitor active drivers","title_zh":"监看行驶中的司机","body_en":"See active drivers sharing their location and their current trips.","body_zh":"查看正在分享位置的司机及其行程。","link":"/admin/live"}
            ]),
        ),
        (
            "manage-any-event",
            "管理活动",
            "Manage any event",
            "职员查看和处理任何活动。",
            "Review and manage events across the organisation.",
            "staff",
            json!([
                {"title_en":"Review events","title_zh":"查看活动","body_en":"With event permissions, review details, signups and attendance across events.","body_zh":"拥有活动权限的职员可查看各活动详情、报名和出席。","link":"/events"},
                {"title_en":"Edit or finish","title_zh":"编辑或结束","body_en":"Update or cancel an event, record attendance or mark it finished.","body_zh":"更新或取消活动、记录出席或标记完成。","link":"/events/*"}
            ]),
        ),
        (
            "view-all-bills",
            "查看所有账单",
            "View all bills",
            "财务查看活动账单和付款状态。",
            "Review activity bills and payment status.",
            "staff",
            json!([
                {"title_en":"Review bills","title_zh":"查看账单","body_en":"Finance staff with bill access can review published settlements and outstanding payment status.","body_zh":"拥有账单权限的财务职员可查看已发布分摊及未付款状态。","link":"/settlements"}
            ]),
        ),
        (
            "staff-reports",
            "职员报告",
            "Staff reports",
            "查看管理报告并按需筛选。",
            "Review staff reports and available filters.",
            "staff",
            json!([
                {"title_en":"Open reports","title_zh":"打开报告","body_en":"Review organisation-wide activity and export available summaries.","body_zh":"查看全体活动，并导出可用摘要。","link":"/admin/reports"}
            ]),
        ),
        (
            "invites",
            "邀请会员",
            "Invites",
            "创建、分享和撤销会员邀请。",
            "Create, share and revoke member invitations.",
            "staff",
            json!([
                {"title_en":"Create an invite","title_zh":"创建邀请","body_en":"Set invite details and share the generated link with the intended member.","body_zh":"填写邀请资料后，把生成链接交给指定会员。","link":"/admin/members"},
                {"title_en":"Revoke an invite","title_zh":"撤销邀请","body_en":"Revoke unused invites when they should no longer be accepted.","body_zh":"不再使用的邀请应及时撤销。","link":"/admin/members"}
            ]),
        ),
        (
            "account-recovery",
            "账号找回",
            "Account recovery",
            "审核账号找回请求并遵守双管理员规则。",
            "Review recovery requests under the two-admin rule.",
            "staff",
            json!([
                {"title_en":"Review the request","title_zh":"审核申请","body_en":"Verify the member's identity before approving or rejecting a recovery request.","body_zh":"批准或拒绝找回申请前，先核实会员身份。","link":"/admin/members"},
                {"title_en":"Get a second approval","title_zh":"取得第二位管理员批准","body_en":"Two different admins must approve the same request. Approval supersedes the member's other pending requests.","body_zh":"同一申请须由两位不同管理员批准；批准后该会员其他待处理申请会被取代。","link":"/admin/members"}
            ]),
        ),
        (
            "roles-and-permissions",
            "角色与权限",
            "Roles and permissions",
            "管理角色并了解超级管理员保护规则。",
            "Manage roles and understand superadmin safeguards.",
            "staff",
            json!([
                {"title_en":"Edit roles","title_zh":"编辑角色","body_en":"Grant only necessary permissions. Some system-role permissions are locked.","body_zh":"仅授予必要权限；部分系统角色权限不可修改。","link":"/admin/roles"},
                {"title_en":"Leadership roles","title_zh":"领导职务","body_en":"President, Vice-President, Secretary, Head of Traffic and Cell Group Leader hold admin tools but never system settings. Only the super admin, President and Vice-President may appoint or demote an administrator. A Cell Group Leader's traffic tools (ride list, live map and per-event plan) cover only events they created, and claims need a separate finance role granted by the President or Vice-President.","body_zh":"会长、副会长、秘书、交通组组长和小组组长拥有管理功能，但不能进入系统设置。只有超级管理员、会长和副会长可以委任或卸任管理员。小组组长的交通功能（接送管理、实时地图与活动交通编排）只限其本人创建的活动；如需报销权限，须由会长或副会长另行授予财务职务。","link":"/admin/members"},
                {"title_en":"Superadmin safeguards","title_zh":"超级管理员保护规则","body_en":"Only a superadmin can grant or remove superadmin, and the super admin keeps ultimate power. The last superadmin cannot be removed or disabled, and role changes require MFA. A Vice-President promoting an ordinary member to administrator must confirm the President has acknowledged it first.","body_zh":"只有超级管理员可授予或移除超级管理员权限，并保留最终权力；最后一位超级管理员不可被移除或停用，角色变更须通过双重验证。副会长把普通会员升为管理员前，必须先确认会长已知悉并同意。","link":"/admin/roles"}
            ]),
        ),
        (
            "groups-areas-rates",
            "群组、区域与费率",
            "Groups, areas and rates",
            "在设置中维护群组、服务区域和报销费率。",
            "Maintain groups, service areas and reimbursement rates.",
            "staff",
            json!([
                {"title_en":"Maintain groups and areas","title_zh":"维护群组和区域","body_en":"Manage the groups and areas used by member access and ride requests.","body_zh":"管理会员权限和接送申请所用的群组与区域。","link":"/admin/settings"},
                {"title_en":"Update rates","title_zh":"更新费率","body_en":"Review reimbursement rates before saving; changes affect future claims.","body_zh":"保存前核对报销费率；变更会影响之后的申请。","link":"/admin/settings"}
            ]),
        ),
        (
            "edit-help-guides",
            "编辑帮助指南",
            "Edit help guides",
            "编辑指南并选择会员或职员可见范围。",
            "Edit guides and choose whether members or staff can see them.",
            "staff",
            json!([
                {"title_en":"Open Guides","title_zh":"打开指南","body_en":"In Help, choose Guides. Help editors can add, edit, publish or delete workflows.","body_zh":"在「帮助中心」选择「指南」；帮助编辑者可新增、编辑、发布或删除指南。","link":"/help?tab=guides"},
                {"title_en":"Set audience and links","title_zh":"设置对象与链接","body_en":"Choose Members or Staff. Members see published member guides; staff can see both audiences. Step links must be internal app routes.","body_zh":"选择「会员」或「职员」。会员只看已发布的会员指南；职员可查看两类指南。步骤链接须使用应用内路径。","link":"/help?tab=guides"}
            ]),
        ),
    ];
    for (i, (slug, tzh, ten, szh, sen, audience, steps)) in guides.iter().enumerate() {
        sqlx::query(
            "INSERT INTO guide_workflows (slug, title_zh, title_en, summary_zh, summary_en, audience, steps, sort_order, published, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7::jsonb,$8,TRUE,$9) ON CONFLICT (slug) DO NOTHING",
        )
        .bind(slug)
        .bind(tzh)
        .bind(ten)
        .bind(szh)
        .bind(sen)
        .bind(audience)
        .bind(steps.to_string())
        .bind(i as i32)
        .bind(now())
        .execute(db)
        .await?;
    }
    Ok(())
}

/// Production path: first administrator + one-time recovery link, no demo data.
/// Existing members are left alone. A second insert would fail the unique email
/// and stop the process on every restart.
async fn seed_admin_only(db: &PgPool, cfg: &Config) -> anyhow::Result<()> {
    let email = std::env::var("ADMIN_EMAIL").unwrap_or_else(|_| "admin@example.org".into());
    let user: Option<(i64, Option<String>)> =
        sqlx::query_as("SELECT id, password_hash FROM users WHERE email = $1")
            .bind(&email)
            .fetch_optional(db)
            .await?;

    let id = match user {
        None => {
            let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
                .fetch_one(db)
                .await?;
            if total > 0 {
                return Ok(());
            }
            sqlx::query_as::<_, (i64,)>("INSERT INTO users (name, email, roles, webauthn_uuid, demo_batch) VALUES ('管理员', $1, 'admin', $2, NULL) RETURNING id")
                .bind(&email).bind(stable_uuid(0)).fetch_one(db).await?.0
        }
        Some((id, pw)) => {
            // Only an administrator who has never finished setup gets a new link. Anyone
            // who has signed in before (accepted the terms) and later lost their keys
            // goes through recovery like any member: otherwise a restart after a
            // recovery approval would write a working sign-in link into the logs.
            let (passkeys, set_up): (i64, bool) = sqlx::query_as(
                "SELECT (SELECT COUNT(*) FROM passkeys WHERE user_id = $1),
                        (SELECT terms_accepted_at IS NOT NULL OR status <> 'active' FROM users WHERE id = $1)",
            )
            .bind(id)
            .fetch_one(db)
            .await?;
            if passkeys > 0 || pw.is_some() || set_up {
                return Ok(());
            }
            // First admin exists but has no passkey and no password set up.
            // Check if there is already an unexpired recovery link.
            let (unexpired,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM invites WHERE user_id = $1 AND kind = 'recovery' AND used_at IS NULL AND expires_at > $2",
            )
            .bind(id)
            .bind(now())
            .fetch_one(db)
            .await?;
            if unexpired > 0 {
                return Ok(());
            }
            // Previous invite expired without being claimed; issue a fresh one.
            sqlx::query("UPDATE invites SET expires_at = $1 WHERE user_id = $2 AND kind = 'recovery' AND used_at IS NULL")
                .bind(now())
                .bind(id)
                .execute(db)
                .await?;
            id
        }
    };

    let token = random_token(32);
    sqlx::query("INSERT INTO invites (token, kind, email, name, roles, group_ids, user_id, created_by, expires_at, demo_batch) VALUES ($1, 'recovery', $2, '管理员', 'admin', '[]', $3, $3, $4, NULL)")
        .bind(crate::auth::hash_token(&token)).bind(&email).bind(id).bind(in_days(2)).execute(db).await?;
    tracing::warn!("Administrator setup link for {email}. Open this one-time link to register a passkey: {}/invite/{token}", cfg.public_url);
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
    let terms_at = if active { Some(now()) } else { None };
    let terms_v = if active {
        crate::routes::auth::TERMS_VERSION
    } else {
        1
    };
    sqlx::query_as::<_, (i64,)>(
        "INSERT INTO users (id, name, email, phone, roles, status, password_hash, webauthn_uuid, totp_secret, totp_enabled, demo_batch, created_at, terms_accepted_at, terms_version)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14) RETURNING id",
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
    .bind(now())
    .bind(terms_at)
    .bind(terms_v)
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
            .bind(now())
            .execute(db)
            .await
            .unwrap();
    }
    id
}

/// Original core demo fixtures (demo accounts 1..=20, events 1..=7, rides 20001..=20009).
/// Reproduces the pre-existing seed content so current integration tests keep
/// passing, now stamped with demo_batch and given stable ids.
async fn seed_core(db: &PgPool) -> anyhow::Result<()> {
    let pw = hash_password(DEMO_PASSWORD).map_err(|e| anyhow::anyhow!("{e}"))?;
    let today = today_local();

    // --- core demo users (ids 1..=20) ---
    let core = [
        // Holds every permission. Kept apart from 管理员 Grace so the demo also shows
        // what a plain admin cannot do (finance stays separate).
        (
            14,
            "超级管理员",
            "superadmin@example.org",
            "012-300 0016",
            "superadmin",
            "active",
        ),
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
        // --- church leadership (ids 15..=20) ---
        // Admin tools without the system configuration page. The President and
        // Vice-President can appoint or demote administrators.
        (
            15,
            "会长 陈志远",
            "president@example.org",
            "012-300 0017",
            "president",
            "active",
        ),
        (
            16,
            "副会长 林美珍",
            "vicepresident@example.org",
            "012-300 0018",
            "vice_president",
            "active",
        ),
        (
            17,
            "秘书 黄晓婷",
            "secretary@example.org",
            "012-300 0019",
            "secretary",
            "active",
        ),
        (
            18,
            "交通组组长 张伟康",
            "traffichead@example.org",
            "012-300 0020",
            "traffic_head",
            "active",
        ),
        (
            19,
            "小组组长 李秀兰",
            "cellleader@example.org",
            "012-300 0021",
            "cell_leader",
            "active",
        ),
        // 查账: reads the finance records and member reports without paying or editing.
        (
            20,
            "查账 周文彬",
            "auditor@example.org",
            "012-300 0022",
            "auditor",
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
    let president = get("会长 陈志远", &id_of);
    let vice_president = get("副会长 林美珍", &id_of);
    let secretary = get("秘书 黄晓婷", &id_of);
    let traffic_head = get("交通组组长 张伟康", &id_of);
    let cell_leader = get("小组组长 李秀兰", &id_of);
    let auditor = get("查账 周文彬", &id_of);

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
        (president, 2),
        (vice_president, 2),
        (secretary, 2),
        (traffic_head, 3),
        (cell_leader, 2),
        (cell_leader, 3),
        (auditor, 2),
    ] {
        sqlx::query("INSERT INTO group_members (group_id, user_id, demo_batch) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING")
            .bind(g).bind(uid).bind(DEMO_BATCH).execute(db).await?;
    }

    // --- vehicles + driver preferences ---
    sqlx::query("INSERT INTO vehicles (user_id, model, color, plate, seats, demo_batch, is_active) VALUES ($1,'Perodua Myvi','白色','VXX 1234',5,$2,1) ON CONFLICT(user_id, plate) DO UPDATE SET model=excluded.model, color=excluded.color, seats=excluded.seats, is_active=excluded.is_active, demo_batch=excluded.demo_batch")
        .bind(deming).bind(DEMO_BATCH).execute(db).await?;
    sqlx::query("INSERT INTO vehicles (user_id, model, color, plate, seats, demo_batch, is_active) VALUES ($1,'Toyota Innova','银色','WCC 1118',7,$2,0) ON CONFLICT(user_id, plate) DO UPDATE SET model=excluded.model, color=excluded.color, seats=excluded.seats, is_active=excluded.is_active, demo_batch=excluded.demo_batch")
        .bind(deming).bind(DEMO_BATCH).execute(db).await?;
    sqlx::query("INSERT INTO vehicles (user_id, model, color, plate, seats, demo_batch, is_active) VALUES ($1,'Honda City','银色','WQA 8821',5,$2,1) ON CONFLICT(user_id, plate) DO UPDATE SET model=excluded.model, color=excluded.color, plate=excluded.plate, seats=excluded.seats, is_active=excluded.is_active, demo_batch=excluded.demo_batch")
        .bind(jiale).bind(DEMO_BATCH).execute(db).await?;
    sqlx::query("INSERT INTO vehicles (user_id, model, color, plate, seats, demo_batch, is_active) VALUES ($1,'Proton X50','灰色','BPM 3302',5,$2,1) ON CONFLICT(user_id, plate) DO UPDATE SET model=excluded.model, color=excluded.color, plate=excluded.plate, seats=excluded.seats, is_active=excluded.is_active, demo_batch=excluded.demo_batch")
        .bind(transport).bind(DEMO_BATCH).execute(db).await?;
    sqlx::query("INSERT INTO vehicles (user_id, model, color, plate, seats, demo_batch, is_active) VALUES ($1,'Toyota Vios','蓝色','BKS 4410',5,$2,1) ON CONFLICT(user_id, plate) DO UPDATE SET model=excluded.model, color=excluded.color, plate=excluded.plate, seats=excluded.seats, is_active=excluded.is_active, demo_batch=excluded.demo_batch")
        .bind(zhiqiang).bind(DEMO_BATCH).execute(db).await?;
    sqlx::query("INSERT INTO vehicles (user_id, model, color, plate, seats, demo_batch, is_active) VALUES ($1,'Mazda 3','红色','VBE 7788',5,$2,1) ON CONFLICT(user_id, plate) DO UPDATE SET model=excluded.model, color=excluded.color, plate=excluded.plate, seats=excluded.seats, is_active=excluded.is_active, demo_batch=excluded.demo_batch")
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
        (
            20010i64,
            yawen,
            Some(zhiqiang),
            kl_hm(tomorrow, 14, 0),
            2,
            "SS2, Jalan 21/1 门牌 12",
            "社区中心礼堂",
            2,
            "",
            "pending_driver_confirm",
        ),
    ];
    for (id, p, d, at, area, addr, dest, seats, assistance, status) in rides {
        let completed = if status == "completed" {
            Some(at.clone())
        } else {
            None
        };
        sqlx::query_as::<_, (i64,)>("INSERT INTO rides (id, passenger_id, driver_id, pickup_at, pickup_area_id, pickup_address, destination, seats, assistance, status, created_at, updated_at, completed_at, demo_batch) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14) RETURNING id")
            .bind(id).bind(p).bind(d).bind(&at).bind(area).bind(addr).bind(dest).bind(seats).bind(assistance).bind(status).bind(now()).bind(now()).bind(completed).bind(DEMO_BATCH).fetch_one(db).await?;
        if matches!(status, "confirmed" | "completed") {
            sqlx::query("UPDATE rides SET trip_id=id, approved_by=$1 WHERE id=$2")
                .bind(transport)
                .bind(id)
                .execute(db)
                .await?;
            sqlx::query("INSERT INTO ride_messages (id, trip_id, user_id, body, demo_batch, created_at) VALUES ($1,$2,NULL,'行程已获批准，群聊已开启。',$3,$4) ON CONFLICT DO NOTHING")
                .bind(id).bind(id).bind(DEMO_BATCH).bind(now()).execute(db).await?;
        }
        if status == "confirmed" {
            sqlx::query("INSERT INTO ride_messages (id, trip_id, user_id, body, demo_batch, created_at) VALUES ($1,$2,$3,'是在北门入口集合吗？',$4,$5) ON CONFLICT DO NOTHING")
                .bind(id + 80000).bind(id).bind(p).bind(DEMO_BATCH).bind(now()).execute(db).await?;
            sqlx::query("INSERT INTO ride_messages (id, trip_id, user_id, body, demo_batch, created_at) VALUES ($1,$2,$3,'对，8 点见！',$4,$5) ON CONFLICT DO NOTHING")
                .bind(id + 80001).bind(id).bind(d).bind(DEMO_BATCH).bind(now()).execute(db).await?;
        }
    }
    // Staff assigned zhiqiang to ride 20010; he must confirm the assignment.
    // The ride needs transport-staff attention until the driver confirms.
    sqlx::query(
        "UPDATE rides SET needs_attention=1, attention_reason='等待新司机确认 / awaiting new driver confirmation' WHERE id=20010",
    )
    .execute(db)
    .await?;

    // --- announcements ---
    sqlx::query("INSERT INTO announcements (id, title, body, pinned, created_by, demo_batch, created_at) VALUES ($1,'欢迎使用恩典同行','接送、活动与重要消息，集中在同一个平台。如有问题，请到帮助中心提问。',1,$2,$3,$4) ON CONFLICT DO NOTHING")
        .bind(101i64).bind(admin).bind(DEMO_BATCH).bind(now()).execute(db).await?;
    sqlx::query("INSERT INTO announcements (id, title, body, pinned, created_by, demo_batch, created_at) VALUES ($2,'本周日接送安排','请于周五前提交周日崇拜的接送报名，交通组会在周六确认。',0,$2,$3,$4) ON CONFLICT DO NOTHING")
        .bind(102i64).bind(transport).bind(DEMO_BATCH).bind(now()).execute(db).await?;

    // --- notifications ---
    sqlx::query("INSERT INTO notifications (id, user_id, title, body, link, demo_batch, created_at) VALUES ($1,$2,'本周有新的接送报名','蒲种有一位会员等待司机。','/admin/rides',$3,$4) ON CONFLICT DO NOTHING")
        .bind(7001i64).bind(transport).bind(DEMO_BATCH).bind(now()).execute(db).await?;
    sqlx::query("INSERT INTO notifications (id, user_id, title, body, link, demo_batch, created_at) VALUES ($1,$2,'费用分摊已发布','上周羽毛球费用，每人 RM28.00。','/settlements',$3,$4) ON CONFLICT DO NOTHING")
        .bind(7002).bind(fala).bind(DEMO_BATCH).bind(now()).execute(db).await?;
    sqlx::query("INSERT INTO notifications (id, user_id, title, body, link, demo_batch, created_at) VALUES ($1,$2,'委任管理员需会长确认','把普通会员升为管理员前，请先取得会长同意，并在确认提示中勾选确认。','/admin/members',$3,$4) ON CONFLICT DO NOTHING")
        .bind(7003i64).bind(vice_president).bind(DEMO_BATCH).bind(now()).execute(db).await?;

    // --- help ticket + reply ---
    sqlx::query("INSERT INTO help_tickets (id, user_id, subject, status, demo_batch, created_at, updated_at) VALUES ($1,$2,'周日接送可以改到北门吗？','answered',$3,$4,$4) ON CONFLICT DO NOTHING")
        .bind(501i64).bind(yawen).bind(DEMO_BATCH).bind(now()).execute(db).await?;
    sqlx::query("INSERT INTO help_replies (id, ticket_id, user_id, body, demo_batch, created_at) VALUES ($1,$2,$3,'可以，请在报名里把上车点改成北门入口。',$4,$5) ON CONFLICT DO NOTHING")
        .bind(901i64).bind(501).bind(transport).bind(DEMO_BATCH).bind(now()).execute(db).await?;

    // --- settlement: 上周羽毛球费用 (event 5, equal split RM28 each, reconciles to RM168.00) ---
    let due = kl(today + Duration::days(7), 23, 59);
    let settle = sqlx::query_as::<_, (i64,)>("INSERT INTO settlements (id, event_id, created_by, title, split_mode, subsidy_cents, payee_name, payee_account, due_date, status, version, demo_batch, created_at, updated_at) VALUES ($1,$2,$3,'上周羽毛球费用','equal',0,'李德明','012-300 0004',$4,'published',1,$5,$6,$6) RETURNING id")
        .bind(40001i64).bind(5).bind(deming).bind(&due).bind(DEMO_BATCH).bind(now()).fetch_one(db).await?.0;
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
            .bind(42000i64 + uid).bind(settle).bind(uid).bind(status).bind(now()).bind(DEMO_BATCH).execute(db).await?;
    }
    sqlx::query("INSERT INTO settlement_revisions (id, settlement_id, version, snapshot, reason, created_by, demo_batch, created_at) VALUES ($1,$2,1,$3::jsonb,'首次发布',$4,$5,$6) ON CONFLICT DO NOTHING")
        .bind(43001i64).bind(settle).bind(r#"{"title":"上周羽毛球费用","total_cents":16800}"#).bind(deming).bind(DEMO_BATCH).bind(now()).execute(db).await?;

    audit(
        db,
        Some(admin),
        "seed.core",
        "users",
        Some(20),
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

/// Edge / edge-case fixtures stamped with `EDGE_BATCH`.
///
/// Idempotent and safe to call alongside `seed_core` (or on its own). Stable
/// ids use reserved ranges that never collide with the core demo
/// (users 1..=20, events 1..=7, rides 20001..=20009) or the generated scale
/// range (users 10001..=60000).
///
/// Look up edge users in tests by these stable emails:
///   edgestaff@example.org   (admin,transport,finance)
///   edgeadmin@example.org   (admin)
///   edgetransport@example.org (transport,driver)
///   edgefinance@example.org (finance, TOTP)
///   edgedriver@example.org  (driver, no vehicle)
///   edgecarpool@example.org (driver, carpool, spare_seats=8)
///   edgemember@example.org  (plain active member)
///   edgenopw@example.org    (active, no password_hash - passkey/OAuth only)
///   edgedisabled@example.org (disabled member)
///   edgeen@example.org      (lang='en', terms not accepted)
pub async fn seed_edge(db: &PgPool) -> anyhow::Result<()> {
    insert_reference_data(db).await?;
    ensure_demo_batches(db).await?;
    let pw = hash_password(DEMO_PASSWORD).map_err(|e| anyhow::anyhow!("{e}"))?;
    let now = now();
    let today = today_local();
    let kl = |d: NaiveDate, h: i32, m: i32| format!("{}T{:02}:{:02}", d, h, m);
    let future = kl(today + Duration::days(2), 9, 0);
    let past = kl(today - Duration::days(5), 9, 0);
    let sat = next_weekday(today, Weekday::Sat);
    let sun = next_weekday(today, Weekday::Sun);
    let fri = next_weekday(today, Weekday::Fri);

    // --- edge users (ids 90001..=90010) ---
    // (id, name, email, phone, roles, status, lang, terms_accepted_at, spare_seats, carpool_enabled, totp, passkey)
    let edge_users = [
        (
            90001i64,
            "Edge Staff",
            "edgestaff@example.org",
            "012-300 9001",
            "admin,transport,finance",
            "active",
            "zh",
            Some(&now),
            8,
            1,
            false,
            false,
        ),
        (
            90002,
            "Edge Admin",
            "edgeadmin@example.org",
            "012-300 9002",
            "admin",
            "active",
            "zh",
            Some(&now),
            1,
            0,
            false,
            false,
        ),
        (
            90003,
            "Edge Transport",
            "edgetransport@example.org",
            "012-300 9003",
            "transport,driver",
            "active",
            "zh",
            Some(&now),
            2,
            1,
            false,
            false,
        ),
        (
            90004,
            "Edge Finance",
            "edgefinance@example.org",
            "012-300 9004",
            "finance",
            "active",
            "zh",
            Some(&now),
            1,
            0,
            true,
            false,
        ),
        (
            90005,
            "Edge Driver",
            "edgedriver@example.org",
            "012-300 9005",
            "driver",
            "active",
            "zh",
            Some(&now),
            1,
            0,
            false,
            false,
        ),
        (
            90006,
            "Edge Carpool",
            "edgecarpool@example.org",
            "012-300 9006",
            "driver",
            "active",
            "zh",
            Some(&now),
            8,
            1,
            false,
            false,
        ),
        (
            90007,
            "Edge Member",
            "edgemember@example.org",
            "012-300 9007",
            "",
            "active",
            "zh",
            Some(&now),
            1,
            0,
            false,
            false,
        ),
        (
            90008,
            "Edge NoPw",
            "edgenopw@example.org",
            "012-300 9008",
            "",
            "active",
            "zh",
            None,
            1,
            0,
            false,
            false,
        ),
        (
            90009,
            "Edge Disabled",
            "edgedisabled@example.org",
            "012-300 9009",
            "",
            "disabled",
            "zh",
            Some(&now),
            1,
            0,
            false,
            false,
        ),
        (
            90010,
            "Edge EN",
            "edgeen@example.org",
            "012-3456789012345678",
            "",
            "active",
            "en",
            None,
            1,
            0,
            false,
            false,
        ),
    ];
    for (id, name, email, phone, roles, status, lang, terms, spare, carpool, totp, _passkey) in
        edge_users
    {
        insert_member(
            db,
            &mut Rng::new(),
            id,
            name,
            email,
            phone,
            roles,
            status,
            EDGE_BATCH,
            &pw,
            totp,
            false,
        )
        .await;
        let terms_sql = match terms {
            Some(t) => format!("terms_accepted_at = '{t}'"),
            None => "terms_accepted_at = NULL".to_string(),
        };
        let sql = format!(
            "UPDATE users SET spare_seats={spare}, carpool_enabled={carpool}, lang='{lang}', {terms_sql} WHERE id={id}"
        );
        sqlx::query(sqlx::AssertSqlSafe(&*sql)).execute(db).await?;
        if totp {
            sqlx::query("UPDATE users SET totp_secret=$1, totp_enabled=1 WHERE id=$2")
                .bind(SIMULATED_TOTP_SECRET)
                .bind(id)
                .execute(db)
                .await?;
        }
    }
    // edgenopw: active user with no password hash (passkey / OAuth-only edge case).
    sqlx::query("UPDATE users SET password_hash = NULL WHERE id = 90008")
        .execute(db)
        .await?;

    // --- edge vehicles ---
    let edge_vehicles = [
        (90003i64, "Toyota Innova", "白色", "EDG 1001", 7),
        (90006, "Honda Odyssey", "灰色", "EDG 1003", 7),
    ];
    for (uid, model, color, plate, seats) in edge_vehicles {
        sqlx::query(
            "INSERT INTO vehicles (user_id, model, color, plate, seats, demo_batch, vehicle_type, is_active)
             VALUES ($1,$2,$3,$4,$5,$6,'standard',1)
             ON CONFLICT(user_id, plate) DO UPDATE SET model=excluded.model, color=excluded.color,
             seats=excluded.seats, vehicle_type=excluded.vehicle_type, is_active=excluded.is_active,
             demo_batch=excluded.demo_batch",
        )
        .bind(uid)
        .bind(model)
        .bind(color)
        .bind(plate)
        .bind(seats)
        .bind(EDGE_BATCH)
        .execute(db)
        .await?;
    }

    // --- edge driver areas ---
    for (uid, area) in [
        (90003i64, 1i64),
        (90003i64, 5i64),
        (90006i64, 2i64),
        (90006i64, 7i64),
    ] {
        let _ = sqlx::query_as::<_, (i64,)>(
            "INSERT INTO driver_areas (user_id, area_id, demo_batch) VALUES ($1,$2,$3) RETURNING user_id",
        )
        .bind(uid)
        .bind(area)
        .bind(EDGE_BATCH)
        .fetch_optional(db)
        .await?;
    }

    // --- edge group memberships ---
    for (uid, gid) in [
        (90007i64, 1i64),
        (90007i64, 2i64),
        (90009i64, 3i64),
        (90010i64, 1i64),
    ] {
        let _ = sqlx::query_as::<_, (i64,)>(
            "INSERT INTO group_members (group_id, user_id, demo_batch) VALUES ($1,$2,$3) ON CONFLICT (group_id, user_id) DO NOTHING RETURNING group_id",
        )
        .bind(gid)
        .bind(uid)
        .bind(EDGE_BATCH)
        .fetch_optional(db)
        .await?;
    }

    // --- edge events (ids 9001..=9006) ---
    let edge_events = [
        (
            9001i64,
            "过去羽毛球",
            "badminton",
            "三十天前的已结束活动",
            kl(today - Duration::days(30), 19, 30),
            "PJ 室内羽球馆",
            12i64,
            "已结算",
            "finished",
            format!("{}T12:00", today - Duration::days(30)),
        ),
        (
            9002,
            "已取消保龄球",
            "bowling",
            "组织者取消的活动",
            kl(sat, 14, 0),
            "Sunway Pyramid 保龄球馆",
            8,
            "已退款",
            "cancelled",
            format!("{}T12:00", sat),
        ),
        (
            9003,
            "满员徒步",
            "walk",
            "名额已满，用于候补测试",
            kl(sun, 8, 0),
            "Taman Jaya 公园入口",
            2,
            "免费",
            "published",
            format!("{}T12:00", sun),
        ),
        (
            9004,
            "周末羽球",
            "badminton",
            "即将举行的标准活动",
            kl(fri, 19, 30),
            "PJ 室内羽球馆",
            12,
            "场租平均分摊",
            "published",
            format!("{}T12:00", fri),
        ),
        (
            9005,
            "免费公园散步",
            "walk",
            "零费用活动，不限名额",
            kl(sun, 7, 0),
            "社区中心礼堂",
            0,
            "免费",
            "published",
            format!("{}T12:00", sun),
        ),
        (
            9006,
            "无群体活动",
            "other",
            "不关联任何群体",
            kl(fri, 18, 30),
            "PJ State CMC 教会",
            20,
            "自备",
            "published",
            format!("{}T12:00", fri),
        ),
    ];
    for (eid, title, kind, desc, at, loc, cap, cost, status, deadline) in edge_events {
        sqlx::query_as::<_, (i64,)>(
            "INSERT INTO events (id, title, kind, description, starts_at, location, capacity, cost_note, created_by, signup_deadline, status, demo_batch)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
             ON CONFLICT (id) DO UPDATE SET title=excluded.title, kind=excluded.kind,
             description=excluded.description, starts_at=excluded.starts_at, location=excluded.location,
             capacity=excluded.capacity, cost_note=excluded.cost_note, status=excluded.status,
             signup_deadline=excluded.signup_deadline, demo_batch=excluded.demo_batch
             RETURNING id",
        )
        .bind(eid)
        .bind(title)
        .bind(kind)
        .bind(desc)
        .bind(&at)
        .bind(loc)
        .bind(cap)
        .bind(cost)
        .bind(90001i64)
        .bind(&deadline)
        .bind(status)
        .bind(EDGE_BATCH)
        .fetch_one(db)
        .await?;
    }
    for (eid, gid) in [(9001i64, 2i64), (9002, 3), (9003, 1), (9004, 2), (9004, 3)] {
        // 9006 deliberately has no groups (edge case).
        sqlx::query(
            "INSERT INTO event_groups (event_id, group_id, demo_batch) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING",
        )
        .bind(eid)
        .bind(gid)
        .bind(EDGE_BATCH)
        .execute(db)
        .await?;
    }

    // --- edge event signups ---
    let edge_signups = [
        // 9003 full: two going, one waitlist.
        (9003i64, 90007i64, "going", 0i64, 0, 0, None::<i64>),
        (9003, 90001, "going", 0, 0, 0, None),
        (9003, 90005, "waitlist", 0, 0, 0, None),
        // 9004 upcoming: member needs ride, driver offers drive, one cancelled.
        (9004, 90007, "going", 1, 0, 0, None),
        (9004, 90005, "going", 0, 1, 2, None),
        (9004, 90010, "cancelled", 0, 0, 0, None),
        // 9001 finished: attended mix, including a disabled user.
        (9001, 90007, "going", 0, 0, 0, Some(1)),
        (9001, 90009, "going", 0, 0, 0, Some(0)),
        (9005, 90010, "going", 0, 0, 0, None),
    ];
    for (eid, uid, status, need_ride, offer_drive, offer_seats, attended) in edge_signups {
        sqlx::query(
            "INSERT INTO event_signups (event_id, user_id, status, need_ride, offer_drive, offer_seats, attended, demo_batch)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
             ON CONFLICT(event_id,user_id) DO UPDATE SET status=excluded.status,
             need_ride=excluded.need_ride, offer_drive=excluded.offer_drive,
             offer_seats=excluded.offer_seats, attended=excluded.attended,
             demo_batch=excluded.demo_batch",
        )
        .bind(eid)
        .bind(uid)
        .bind(status)
        .bind(need_ride)
        .bind(offer_drive)
        .bind(offer_seats)
        .bind(attended)
        .bind(EDGE_BATCH)
        .execute(db)
        .await?;
    }

    // --- edge rides (ids 80001..=80011), every status + pending_driver_confirm ---
    let edge_rides = [
        (
            80001i64,
            90007i64,
            None::<i64>,
            None::<i64>,
            &future,
            1i64,
            "12 Jalan Demo",
            "PJ 室内羽球馆",
            1i64,
            "",
            "",
            "pending_driver",
            0i64,
            "",
            None::<i64>,
            None::<i64>,
            None::<String>,
            None::<String>,
            None::<String>,
            0i64,
            None::<i64>,
            None::<i64>,
            0i64,
            "",
            "",
            "",
        ),
        (
            80002,
            90007,
            Some(90005i64),
            None,
            &future,
            1,
            "No 45, Jalan PJ Selamat 1",
            "Sunway Pyramid 保龄球馆",
            2,
            "wheelchair",
            "",
            "pending_passenger",
            0,
            "",
            None,
            None,
            None,
            None,
            None,
            0,
            None,
            None,
            0,
            "",
            "",
            "",
        ),
        (
            80003,
            90010,
            Some(90006i64),
            Some(9004i64),
            &future,
            1,
            "Taman Demo, Jalan SS2/38, SS2",
            "社区中心礼堂",
            3,
            "",
            "proxy booking for elder",
            "pending_approval",
            0,
            "",
            None,
            None,
            None,
            None,
            None,
            0,
            None,
            None,
            1i64,
            "Edge Elder",
            "012-300 9011",
            "parent",
        ),
        (
            80004,
            90007,
            Some(90003i64),
            Some(9004i64),
            &future,
            1,
            "Lot 123, Jalan Kelana Jaya 1",
            "社区中心礼堂",
            1,
            "",
            "",
            "confirmed",
            0,
            "",
            Some(90001i64),
            Some(5001i64),
            None,
            None,
            None,
            0,
            None,
            None,
            0i64,
            "",
            "",
            "",
        ),
        (
            80005,
            90007,
            Some(90003i64),
            None,
            &future,
            1,
            "Pangsiran Demo, Kelana Jaya",
            "图书馆入口",
            1,
            "",
            "",
            "in_progress",
            0,
            "",
            Some(90001i64),
            Some(5001i64),
            Some(future.clone()),
            None,
            None,
            0,
            None,
            None,
            0,
            "",
            "",
            "",
        ),
        (
            80006,
            90007,
            Some(90003i64),
            None,
            &past,
            1,
            "No 88, Jalan SS2/72, SS2",
            "宵夜团契",
            1,
            "",
            "",
            "completed",
            0,
            "",
            Some(90001i64),
            Some(5002i64),
            Some(past.clone()),
            Some(past.clone()),
            Some(past.clone()),
            0,
            Some(10i64),
            Some(3000i64),
            0,
            "",
            "",
            "",
        ),
        (
            80007,
            90009,
            None::<i64>,
            None,
            &future,
            1,
            "18 Jalan Demo Utara, Kelana Jaya",
            "诊所入口",
            1,
            "",
            "",
            "cancelled",
            0,
            "",
            None,
            None,
            None,
            None,
            None,
            0,
            None,
            None,
            0,
            "",
            "",
            "",
        ),
        (
            80008,
            90007,
            Some(90005i64),
            None,
            &future,
            1,
            "4 Jalan SS2/80, SS2",
            "北门入口",
            1,
            "",
            "driver waited 15 minutes",
            "no_show",
            0,
            "driver waited 15 min, passenger absent",
            None,
            None,
            None,
            None,
            None,
            0,
            None,
            None,
            0,
            "",
            "",
            "",
        ),
        (
            80009,
            90007,
            Some(90006i64),
            None,
            &future,
            1,
            "No 15, Jalan PJ Selamat 3",
            "图书馆入口",
            2,
            "",
            "took alternative transport",
            "alt_transport",
            0,
            "passenger took Grab instead",
            None,
            None,
            None,
            None,
            None,
            0,
            None,
            None,
            0,
            "",
            "",
            "",
        ),
        (
            80010,
            90010,
            Some(90003i64),
            None,
            &future,
            1,
            "No 201, Jalan SS2/102, SS2",
            "PJ State CMC 教会",
            1,
            "",
            "",
            "pending_driver",
            1,
            "needs attention",
            None,
            None,
            None,
            None,
            None,
            0,
            None,
            None,
            0,
            "",
            "",
            "",
        ),
        (
            80011,
            90007i64,
            Some(90005i64),
            None,
            &future,
            1,
            "12 Jalan Demo",
            "PJ 室内羽球馆",
            1,
            "",
            "",
            "pending_driver_confirm",
            1,
            "awaiting driver confirmation",
            None,
            None,
            None,
            None,
            None,
            0,
            None,
            None,
            0,
            "",
            "",
            "",
        ),
    ];
    for (
        id,
        passenger_id,
        driver_id,
        event_id,
        pickup_at,
        pickup_area_id,
        pickup_address,
        destination,
        seats,
        assistance,
        notes,
        status,
        needs_attention,
        attention_reason,
        approved_by,
        trip_id,
        started_at,
        picked_up_at,
        completed_at,
        reminder_sent,
        distance_km,
        reimbursement_amount_cents,
        is_proxy,
        rider_name,
        rider_phone,
        rider_relationship,
    ) in edge_rides
    {
        sqlx::query(
            "INSERT INTO rides (id, passenger_id, driver_id, event_id, pickup_at, pickup_area_id,
             pickup_address, destination, seats, assistance, notes, status, needs_attention,
             attention_reason, approved_by, trip_id, started_at, picked_up_at, completed_at,
             reminder_sent, driver_reminder_sent, demo_batch, created_at, updated_at, distance_km,
             reimbursement_amount_cents, is_proxy, rider_name, rider_phone, rider_relationship)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$20,$21,$22,$23,$24,$25,$26,$27,$28,$29)
             ON CONFLICT (id) DO UPDATE SET passenger_id=excluded.passenger_id,
             driver_id=excluded.driver_id, status=excluded.status, needs_attention=excluded.needs_attention,
             demo_batch=excluded.demo_batch, updated_at=excluded.updated_at",
        )
        .bind(id)
        .bind(passenger_id)
        .bind(driver_id)
        .bind(event_id)
        .bind(pickup_at)
        .bind(pickup_area_id)
        .bind(pickup_address)
        .bind(destination)
        .bind(seats)
        .bind(assistance)
        .bind(notes)
        .bind(status)
        .bind(needs_attention)
        .bind(attention_reason)
        .bind(approved_by)
        .bind(trip_id)
        .bind(started_at)
        .bind(picked_up_at)
        .bind(completed_at)
        .bind(reminder_sent)
        .bind(EDGE_BATCH)
        .bind(&now)
        .bind(&now)
        .bind(distance_km)
        .bind(reimbursement_amount_cents.unwrap_or(0))
        .bind(is_proxy)
        .bind(rider_name)
        .bind(rider_phone)
        .bind(rider_relationship)
        .execute(db)
        .await?;
    }

    // --- live locations and shared-trip chat ---
    for (driver_id, lat, lng) in [
        (90003i64, 3.1390f64, 101.6133f64),
        (90006, 3.1405, 101.6100),
    ] {
        sqlx::query(
            "INSERT INTO driver_locations (driver_id, lat, lng, heading, demo_batch, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT(driver_id) DO UPDATE SET lat=excluded.lat,
             lng=excluded.lng, heading=excluded.heading, updated_at=excluded.updated_at",
        )
        .bind(driver_id)
        .bind(lat)
        .bind(lng)
        .bind(None::<f64>)
        .bind(EDGE_BATCH)
        .bind(&now)
        .execute(db)
        .await?;
    }
    // passenger location for the in-progress ride.
    sqlx::query(
        "INSERT INTO passenger_locations (ride_id, lat, lng, updated_at)
         VALUES ($1,$2,$3,$4) ON CONFLICT(ride_id) DO UPDATE SET lat=excluded.lat,
         lng=excluded.lng, updated_at=excluded.updated_at",
    )
    .bind(80005i64)
    .bind(3.1395f64)
    .bind(101.6130f64)
    .bind(&now)
    .execute(db)
    .await?;
    for (trip_id, user_id, body) in [
        (5001i64, Some(90003i64), "On the way, be there in 5 min."),
        (5001, Some(90007i64), "Thanks, see you at the gate."),
    ] {
        sqlx::query(
            "INSERT INTO ride_messages (trip_id, user_id, body, demo_batch, created_at)
             VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(trip_id)
        .bind(user_id)
        .bind(body)
        .bind(EDGE_BATCH)
        .bind(&now)
        .execute(db)
        .await?;
    }

    // --- files used as reimbursement / settlement receipts ---
    let edge_files = [
        (
            20001i64,
            90007i64,
            "receipt_jan.png",
            "image/png",
            "January receipt",
            12345i64,
        ),
        (
            20002,
            90007,
            "receipt_feb.png",
            "image/png",
            "February receipt",
            23456,
        ),
    ];
    for (id, owner_id, stored_name, mime, original_name, size) in edge_files {
        sqlx::query(
            "INSERT INTO files (id, owner_id, stored_name, mime, original_name, size, demo_batch, created_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT (id) DO NOTHING",
        )
        .bind(id)
        .bind(owner_id)
        .bind(stored_name)
        .bind(mime)
        .bind(original_name)
        .bind(size)
        .bind(EDGE_BATCH)
        .bind(&now)
        .execute(db)
        .await?;
    }

    // --- reimbursements (ids 70001..=70006), all five statuses ---
    let edge_reimbursements = [
        (
            70001i64,
            90007i64,
            Some(80006i64),
            1000i64,
            "Grab fare January",
            20001i64,
            "Edge Member",
            "012-300 9007",
            "pending",
            "",
            None::<i64>,
            0i64,
            "",
            None::<i64>,
            None::<&str>,
        ),
        (
            70002,
            90007,
            None,
            3000,
            "Grab fare February",
            20001,
            "Edge Member",
            "012-300 9007",
            "approved",
            "Looks good",
            Some(90001i64),
            0,
            "",
            None,
            None,
        ),
        (
            70003,
            90007,
            None,
            2000,
            "Grab fare March",
            20002,
            "Edge Member",
            "012-300 9007",
            "returned",
            "Wrong payee name",
            None,
            0,
            "",
            None,
            None,
        ),
        (
            70004,
            90007,
            None,
            1500,
            "Grab fare April",
            20002,
            "Edge Member",
            "012-300 9007",
            "rejected",
            "Ineligible route",
            None,
            0,
            "",
            None,
            None,
        ),
        (
            70005,
            90007,
            None,
            5000,
            "Grab fare May",
            20001,
            "Edge Member",
            "012-300 9007",
            "paid",
            "Paid via DuitNow",
            Some(90001i64),
            1,
            "DNT-2026-05",
            Some(90001i64),
            Some("2026-05-01T10:00"),
        ),
        (
            70006,
            90005,
            None,
            800,
            "Short trip",
            20001,
            "Edge Driver",
            "012-300 9005",
            "pending",
            "",
            None,
            0,
            "",
            None,
            None,
        ),
    ];
    for (
        id,
        user_id,
        ride_id,
        amount_cents,
        description,
        receipt_file_id,
        payee_name,
        payee_phone,
        status,
        review_note,
        reviewed_by,
        payee_verified,
        paid_ref,
        paid_by,
        paid_at,
    ) in edge_reimbursements
    {
        sqlx::query(
            "INSERT INTO reimbursements (id, user_id, ride_id, amount_cents, description, receipt_file_id,
             payee_name, payee_phone, status, review_note, reviewed_by, payee_verified, paid_ref, paid_by,
             paid_at, demo_batch, created_at, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)
             ON CONFLICT (id) DO UPDATE SET status=excluded.status, amount_cents=excluded.amount_cents,
             demo_batch=excluded.demo_batch, updated_at=excluded.updated_at",
        )
        .bind(id)
        .bind(user_id)
        .bind(ride_id)
        .bind(amount_cents)
        .bind(description)
        .bind(receipt_file_id)
        .bind(payee_name)
        .bind(payee_phone)
        .bind(status)
        .bind(review_note)
        .bind(reviewed_by)
        .bind(payee_verified)
        .bind(paid_ref)
        .bind(paid_by)
        .bind(paid_at)
        .bind(EDGE_BATCH)
        .bind(&now)
        .bind(&now)
        .execute(db)
        .await?;
    }

    // --- settlements (ids 60001..=60003) + items + shares + a revision ---
    let edge_settlements = [
        (
            60001i64,
            9004i64,
            90001i64,
            "Badminton split",
            "equal",
            0i64,
            "",
            "",
            "2026-12-10",
            "draft",
            1i64,
        ),
        (
            60002,
            9004,
            90001,
            "Badminton split published",
            "equal",
            0,
            "",
            "",
            "2026-12-10",
            "published",
            1,
        ),
        (
            60003,
            9001i64,
            90001,
            "Past event settlement",
            "custom",
            500,
            "Organiser",
            "DNT-PAST",
            "2026-11-01",
            "closed",
            1,
        ),
    ];
    for (
        id,
        event_id,
        created_by,
        title,
        split_mode,
        subsidy_cents,
        payee_name,
        payee_account,
        due_date,
        status,
        version,
    ) in edge_settlements
    {
        sqlx::query(
            "INSERT INTO settlements (id, event_id, created_by, title, split_mode, subsidy_cents,
             payee_name, payee_account, due_date, status, version, demo_batch, created_at, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
             ON CONFLICT (id) DO UPDATE SET status=excluded.status, version=excluded.version,
             demo_batch=excluded.demo_batch, updated_at=excluded.updated_at",
        )
        .bind(id)
        .bind(event_id)
        .bind(created_by)
        .bind(title)
        .bind(split_mode)
        .bind(subsidy_cents)
        .bind(payee_name)
        .bind(payee_account)
        .bind(due_date)
        .bind(status)
        .bind(version)
        .bind(EDGE_BATCH)
        .bind(&now)
        .bind(&now)
        .execute(db)
        .await?;
    }
    let edge_items = [
        (61001i64, 60001i64, "Court fee", 1200i64, None::<i64>),
        (61002, 60001, "Shuttlecocks", 180, None),
        (61003, 60002, "Court fee", 1200, None),
        (61004, 60003, "Court fee", 1200, None),
        (61005, 60003, "Drinks", 200, None),
    ];
    for (id, settlement_id, label, amount_cents, receipt_file_id) in edge_items {
        sqlx::query(
            "INSERT INTO settlement_items (id, settlement_id, label, amount_cents, receipt_file_id, demo_batch)
             VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (id) DO NOTHING",
        )
        .bind(id)
        .bind(settlement_id)
        .bind(label)
        .bind(amount_cents)
        .bind(receipt_file_id)
        .bind(EDGE_BATCH)
        .execute(db)
        .await?;
    }
    let edge_shares = [
        (
            62001i64,
            60001i64,
            90007i64,
            600i64,
            "unpaid",
            None::<i64>,
            "",
            None::<&str>,
        ),
        (62002, 60001, 90001, 600, "confirmed", None, "", None),
        (62003, 60002, 90007, 600, "pending", None, "", None),
        (62004, 60003, 90007, 800, "needs_info", None, "", None),
        (62005, 60003, 90001, 150, "unpaid", None, "", None),
    ];
    for (id, settlement_id, user_id, amount_cents, status, proof_file_id, txn_ref, paid_date) in
        edge_shares
    {
        sqlx::query(
            "INSERT INTO settlement_shares (id, settlement_id, user_id, amount_cents, status, proof_file_id, txn_ref, paid_date, demo_batch, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) ON CONFLICT (id) DO NOTHING",
        )
        .bind(id)
        .bind(settlement_id)
        .bind(user_id)
        .bind(amount_cents)
        .bind(status)
        .bind(proof_file_id)
        .bind(txn_ref)
        .bind(paid_date)
        .bind(EDGE_BATCH)
        .bind(&now)
        .execute(db)
        .await?;
    }
    sqlx::query(
        "INSERT INTO settlement_revisions (id, settlement_id, version, snapshot, reason, created_by, demo_batch, created_at)
         VALUES ($1,$2,$3,$4::jsonb,$5,$6,$7,$8) ON CONFLICT (id) DO NOTHING",
    )
    .bind(63001i64)
    .bind(60001i64)
    .bind(1i64)
    .bind(r#"{"title":"initial draft"}"#)
    .bind("created by staff")
    .bind(90001i64)
    .bind(EDGE_BATCH)
    .bind(&now)
    .execute(db)
    .await?;

    // --- help tickets (ids 50001..=50004) + replies ---
    let edge_tickets = [
        (
            50001i64,
            90007i64,
            None::<i64>,
            "Cannot book a ride",
            "open",
            None::<i64>,
        ),
        (
            50002,
            90007,
            None,
            "Payment not reflected",
            "answered",
            Some(90001i64),
        ),
        (
            50003,
            90005,
            Some(80001i64),
            "Driver never arrived",
            "closed",
            Some(90001i64),
        ),
        (
            50004,
            90010,
            None,
            "Language preference not saved",
            "open",
            None,
        ),
    ];
    for (id, user_id, ride_id, subject, status, assignee_id) in edge_tickets {
        let inserted: (i64,) = sqlx::query_as(
            "INSERT INTO help_tickets (id, user_id, ride_id, subject, status, assignee_id, demo_batch, created_at, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING id",
        )
        .bind(id)
        .bind(user_id)
        .bind(ride_id)
        .bind(subject)
        .bind(status)
        .bind(assignee_id)
        .bind(EDGE_BATCH)
        .bind(&now)
        .bind(&now)
        .fetch_one(db)
        .await?;
        tracing::debug!("inserted help_ticket id={} requested={}", inserted.0, id);
    }
    let edge_replies = [
        (50001i64, 50001i64, "We are looking into the booking issue."),
        (50002, 50002, "Please upload the receipt again."),
    ];
    for (id, ticket_id, body) in edge_replies {
        sqlx::query(
            "INSERT INTO help_replies (id, ticket_id, user_id, body, demo_batch, created_at)
             VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (id) DO NOTHING",
        )
        .bind(id)
        .bind(ticket_id)
        .bind(90001i64)
        .bind(body)
        .bind(EDGE_BATCH)
        .bind(&now)
        .execute(db)
        .await?;
    }

    // --- notifications (read / unread edge) ---
    let edge_notifications = [
        (
            40001i64,
            90007i64,
            "Ride confirmed",
            "Your ride has been confirmed.",
            "/rides/80004",
            Some(&now),
        ),
        (
            40002,
            90007,
            "Settlement published",
            "Please review your share.",
            "/settlements/60002",
            None::<&String>,
        ),
        (
            40003,
            90001,
            "New help ticket",
            "A member needs assistance.",
            "/admin/help",
            None,
        ),
        (
            40004,
            90010,
            "Welcome to Grace Ride",
            "Explore upcoming events and request a ride.",
            "/",
            None,
        ),
    ];
    for (id, user_id, title, body, link, read_at) in edge_notifications {
        sqlx::query(
            "INSERT INTO notifications (id, user_id, title, body, link, read_at, demo_batch, created_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT (id) DO NOTHING",
        )
        .bind(id)
        .bind(user_id)
        .bind(title)
        .bind(body)
        .bind(link)
        .bind(read_at)
        .bind(EDGE_BATCH)
        .bind(&now)
        .execute(db)
        .await?;
    }

    // --- announcements ---
    let edge_announcements = [
        (
            30001i64,
            "System maintenance",
            "Maintenance on Sunday night.",
            1i64,
            90001i64,
        ),
        (
            30002,
            "New dashboard",
            "Check out the new ride dashboard.",
            0i64,
            90001i64,
        ),
    ];
    for (id, title, body, pinned, created_by) in edge_announcements {
        sqlx::query(
            "INSERT INTO announcements (id, title, body, pinned, created_by, demo_batch, created_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT (id) DO NOTHING",
        )
        .bind(id)
        .bind(title)
        .bind(body)
        .bind(pinned)
        .bind(created_by)
        .bind(EDGE_BATCH)
        .bind(&now)
        .execute(db)
        .await?;
    }

    // --- audit logs ---
    let edge_audit = [
        (
            10001i64,
            Some(90001i64),
            "role.update",
            "user",
            Some(90007i64),
            "granted driver role",
        ),
        (
            10002,
            Some(90001i64),
            "settlement.publish",
            "settlement",
            Some(60002i64),
            "published badminton split",
        ),
        (
            10003,
            None::<i64>,
            "ride.cancel",
            "ride",
            Some(80007i64),
            "cancelled by system",
        ),
    ];
    for (id, actor_id, action, entity, entity_id, detail) in edge_audit {
        sqlx::query(
            "INSERT INTO audit_logs (id, actor_id, action, entity, entity_id, detail, demo_batch, created_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT (id) DO NOTHING",
        )
        .bind(id)
        .bind(actor_id)
        .bind(action)
        .bind(entity)
        .bind(entity_id)
        .bind(detail)
        .bind(EDGE_BATCH)
        .bind(&now)
        .execute(db)
        .await?;
    }

    // --- proxy riders ---
    sqlx::query(
        "INSERT INTO proxy_riders (id, user_id, name, phone, relationship, notes, created_at, updated_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT (id) DO NOTHING",
    )
    .bind(11001i64)
    .bind(90007i64)
    .bind("Edge Elder")
    .bind("012-300 9011")
    .bind("parent")
    .bind("Elderly parent, needs wheelchair assistance")
    .bind(&now)
    .bind(&now)
    .execute(db)
    .await?;

    // --- an edge guide workflow + feedback ---
    sqlx::query(
        "INSERT INTO guide_workflows (id, slug, title_zh, title_en, summary_zh, summary_en, audience, steps, sort_order, published, updated_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8::jsonb,$9,TRUE,$10) ON CONFLICT (id) DO NOTHING",
    )
    .bind(90001i64)
    .bind("edge-guide-member")
    .bind("边缘指南")
    .bind("Edge guide")
    .bind("用于测试的边缘用例指南")
    .bind("Edge-case guide used for testing")
    .bind("member")
    .bind("[]")
    .bind(1i32)
    .bind(&now)
    .execute(db)
    .await?;
    sqlx::query(
        "INSERT INTO guide_feedback (guide_id, user_id, helpful, created_at)
         VALUES ($1,$2,$3,$4) ON CONFLICT (guide_id, user_id) DO NOTHING",
    )
    .bind(90001i64)
    .bind(90007i64)
    .bind(true)
    .bind(&now)
    .execute(db)
    .await?;

    sync_identity_sequences(db).await?;
    tracing::info!("Edge-case fixtures seeded ({} batches)", EDGE_BATCH);
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
        sqlx::query(sqlx::AssertSqlSafe(&*sql)).execute(db).await?;
    }
    Ok(())
}
/// Scalable fictional membership: `scale` additional members with stable ids
/// starting at GENERATED_MEMBER_ID_BASE, plus vehicles, service areas and invites.
async fn seed_scale(db: &PgPool, scale: i64) -> anyhow::Result<()> {
    let n = scale.clamp(1, 50_000);
    let pw = hash_password(DEMO_PASSWORD).map_err(|e| anyhow::anyhow!("{e}"))?;
    let today = today_local();
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
            sqlx::query("INSERT INTO vehicles (user_id, model, color, plate, seats, demo_batch, is_active) VALUES ($1,$2,$3,$4,5,$5,1) ON CONFLICT(user_id, plate) DO UPDATE SET model=excluded.model, color=excluded.color, seats=excluded.seats, is_active=excluded.is_active, demo_batch=excluded.demo_batch")
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
