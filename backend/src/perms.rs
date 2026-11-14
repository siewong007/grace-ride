//! Permission model: what a member may do is decided per page and per action.
//!
//! - A **page** is one screen of the app (`Page`), e.g. 接送管理 or 成员.
//! - An **action** is view / create / update / delete (`Act`). Each page lists the
//!   actions that mean something on it.
//! - A **permission** is one action on one page, written `admin_rides.update`.
//! - A **role** is a named set of permissions stored in the database. Administrators
//!   edit the built-in roles and add their own; nothing in the handlers names a role.
//!
//! Every active member holds the built-in `member` role on top of the roles listed in
//! `users.roles`. Handlers ask `u.can(Page::X, Act::Y)` or `require(&u, Page::X, Act::Y)`.
//!
//! Three rules sit above the stored matrix and cannot be configured away:
//! - Staff pages show other members' data, so they need a passkey or 2FA session.
//! - Finance pages (receipts and payouts) can never be self-granted.
//! - The `admin` role always keeps the permissions needed to manage roles and members,
//!   and nobody can remove their own right to manage roles.
//!
//! Permissions open a page and its actions. Records that are a member's own (their ride,
//! their bill, their help request) additionally follow ownership checks in the handlers.

use std::collections::HashSet;

use sqlx::PgPool;

/// Held by every active member without being listed in `users.roles`.
pub const MEMBER_ROLE: &str = "member";
pub const ADMIN_ROLE: &str = "admin";
/// Holds every permission, always. It is the one role that cannot be edited, so
/// whatever else an administrator configures, somebody can still put it right.
pub const SUPERADMIN_ROLE: &str = "superadmin";
/// Built-in roles, created on first start. They can be edited (all but `superadmin`)
/// and never deleted.
pub const SYSTEM_ROLES: &[&str] = &[
    MEMBER_ROLE,
    SUPERADMIN_ROLE,
    ADMIN_ROLE,
    "transport",
    "finance",
    "leader",
    "driver",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Act {
    View,
    Create,
    Update,
    Delete,
}

impl Act {
    pub fn key(self) -> &'static str {
        match self {
            Act::View => "view",
            Act::Create => "create",
            Act::Update => "update",
            Act::Delete => "delete",
        }
    }

    fn from_key(s: &str) -> Option<Act> {
        match s {
            "view" => Some(Act::View),
            "create" => Some(Act::Create),
            "update" => Some(Act::Update),
            "delete" => Some(Act::Delete),
            _ => None,
        }
    }
}

/// Which part of the app a page belongs to. Everything outside `Member` shows other
/// members' data and therefore needs a passkey or 2FA session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Area {
    /// A member's own rides, events, costs and requests.
    Member,
    /// 交通组: rides, drivers, events and member support.
    Transport,
    /// 财务: receipts and payouts. Never self-granted.
    Finance,
    /// 管理员: members, roles and settings.
    Admin,
}

impl Area {
    pub fn key(self) -> &'static str {
        match self {
            Area::Member => "member",
            Area::Transport => "transport",
            Area::Finance => "finance",
            Area::Admin => "admin",
        }
    }
}

pub struct PageDef {
    pub page: Page,
    pub key: &'static str,
    pub area: Area,
    pub actions: &'static [Act],
}

macro_rules! catalog {
    ($($variant:ident: $key:literal, $area:ident, [$($act:ident),+];)+) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Page { $($variant),+ }

        /// Every page and the actions that exist on it, in display order.
        pub const CATALOG: &[PageDef] = &[
            $(PageDef {
                page: Page::$variant,
                key: $key,
                area: Area::$area,
                actions: &[$(Act::$act),+],
            }),+
        ];

        impl Page {
            pub fn def(self) -> &'static PageDef {
                match self { $(Page::$variant => &CATALOG[Page::$variant as usize]),+ }
            }
        }
    };
}

// What each action means on a page is noted on the right; the handlers are the source
// of truth. Keys are stored in the database: rename one only with a migration.
catalog! {
    // ---- Member pages ----
    Rides:          "rides",          Member, [View, Create, Update, Delete]; // own rides: list · request · answer a driver · cancel
    Drive:          "drive",          Member, [View, Create, Update, Delete]; // open requests · take a ride / offer to drive · vehicle & availability · withdraw
    Events:         "events",         Member, [View, Create, Update, Delete]; // browse & sign up · organise · edit own, attendance · cancel own
    Settlements:    "settlements",    Member, [View, Create, Update, Delete]; // bills I organise: list · start · edit, publish, confirm payments · delete draft
    Costs:          "costs",          Member, [View, Update];                 // my shares · submit payment proof
    Reimbursements: "reimbursements", Member, [View, Create, Update];         // my claims · submit · resubmit
    Help:           "help",           Member, [View, Create, Update];         // my requests · ask · reply, close
    Report:         "report",         Member, [View];                         // my monthly report

    // ---- 交通组 ----
    AdminDashboard:     "admin_dashboard",     Transport, [View];
    AdminRides:         "admin_rides",         Transport, [View, Update, Delete]; // every ride with contact details · approve, return, reassign, no-show · cancel
    AdminLive:          "admin_live",          Transport, [View];
    AdminPlan:          "admin_plan",          Transport, [View, Update];         // event transport board · save, auto-arrange
    AdminEvents:        "admin_events",        Transport, [View, Update, Delete]; // any event regardless of group or organiser
    AdminReimbReview:   "admin_reimb_review",  Transport, [View, Update];         // claims queue with receipts · approve, return, reject
    AdminAnnouncements: "admin_announcements", Transport, [View, Create, Delete];
    AdminHelp:          "admin_help",          Transport, [View, Update];         // every help request · reply, close
    AdminReports:       "admin_reports",       Transport, [View];                 // any member's monthly report

    // ---- 财务 ----
    AdminReimbPay: "admin_reimb_pay", Finance, [View, Update]; // approved claims with payee details · record payout
    AdminBills:    "admin_bills",     Finance, [View];         // every bill, its receipts and payment proofs

    // ---- 管理员 ----
    AdminMembers:  "admin_members",  Admin, [View, Update];                 // directory with contact details · approve, disable, set groups
    AdminInvites:  "admin_invites",  Admin, [View, Create, Delete];
    AdminRecovery: "admin_recovery", Admin, [View, Update];                 // recovery requests · approve, reject
    AdminGroups:   "admin_groups",   Admin, [View, Create, Update, Delete];
    AdminAreas:    "admin_areas",    Admin, [View, Create, Update];
    AdminRoles:    "admin_roles",    Admin, [View, Create, Update, Delete]; // update also covers giving roles to members
    AdminAudit:    "admin_audit",    Admin, [View];
    AdminSettings: "admin_settings", Admin, [View, Update]; // app-wide configuration, e.g. reimbursement rates
}

impl Page {
    pub fn key(self) -> &'static str {
        self.def().key
    }

    pub fn area(self) -> Area {
        self.def().area
    }

    /// Shows other members' data: only usable in a passkey / 2FA session.
    pub fn sensitive(self) -> bool {
        self.area() != Area::Member
    }

    fn from_key(s: &str) -> Option<Page> {
        CATALOG.iter().find(|d| d.key == s).map(|d| d.page)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Perm(pub Page, pub Act);

impl Perm {
    pub fn key(self) -> String {
        format!("{}.{}", self.0.key(), self.1.key())
    }

    /// `None` for anything that is not an action offered by a known page.
    pub fn parse(s: &str) -> Option<Perm> {
        let (page, act) = s.split_once('.')?;
        let (page, act) = (Page::from_key(page)?, Act::from_key(act)?);
        page.def().actions.contains(&act).then_some(Perm(page, act))
    }
}

/// Every permission the app knows, in display order.
pub fn all() -> impl Iterator<Item = Perm> {
    CATALOG
        .iter()
        .flat_map(|d| d.actions.iter().map(|a| Perm(d.page, *a)))
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PermSet(HashSet<Perm>);

impl PermSet {
    /// Unknown entries are dropped: a permission removed from the app grants nothing.
    pub fn from_keys<'a>(keys: impl IntoIterator<Item = &'a str>) -> PermSet {
        PermSet(keys.into_iter().filter_map(Perm::parse).collect())
    }

    pub fn has(&self, page: Page, act: Act) -> bool {
        self.0.contains(&Perm(page, act))
    }

    pub fn contains(&self, perm: Perm) -> bool {
        self.0.contains(&perm)
    }

    /// Holds a permission on a staff page, so must sign in with a passkey or 2FA.
    pub fn privileged(&self) -> bool {
        self.0.iter().any(|p| p.0.sensitive())
    }

    /// Finance permissions in this set that `before` does not have.
    pub fn new_finance(&self, before: &PermSet) -> bool {
        self.0
            .iter()
            .any(|p| p.0.area() == Area::Finance && !before.0.contains(p))
    }

    pub fn union(&mut self, other: &PermSet) {
        self.0.extend(other.0.iter().copied());
    }

    /// Keys in catalog order.
    pub fn keys(&self) -> Vec<String> {
        all()
            .filter(|p| self.0.contains(p))
            .map(Perm::key)
            .collect()
    }

    /// As `keys`, without staff permissions when the session has no passkey / 2FA.
    pub fn usable_keys(&self, mfa_ok: bool) -> Vec<String> {
        all()
            .filter(|p| self.0.contains(p) && (mfa_ok || !p.0.sensitive()))
            .map(Perm::key)
            .collect()
    }
}

impl FromIterator<Perm> for PermSet {
    fn from_iter<I: IntoIterator<Item = Perm>>(iter: I) -> Self {
        PermSet(iter.into_iter().collect())
    }
}

fn grid(page: Page, acts: &[Act]) -> impl Iterator<Item = Perm> + '_ {
    acts.iter().map(move |a| Perm(page, *a))
}

/// What each built-in role is given the first time a permission exists. These mirror
/// the fixed roles the app had before roles became editable.
pub fn defaults(role: &str) -> Vec<Perm> {
    use Act::*;
    use Page::*;
    let pages: &[(Page, &[Act])] = match role {
        SUPERADMIN_ROLE => return all().collect(),
        "member" => &[
            (Rides, &[View, Create, Update, Delete]),
            (Events, &[View]),
            (Costs, &[View, Update]),
            (Reimbursements, &[View, Create, Update]),
            (Help, &[View, Create, Update]),
            (Report, &[View]),
        ],
        "driver" => &[(Drive, &[View, Create, Update, Delete])],
        "leader" => &[
            (Events, &[Create, Update, Delete]),
            (Settlements, &[View, Create, Update, Delete]),
        ],
        "transport" => &[
            (AdminDashboard, &[View]),
            (AdminRides, &[View, Update, Delete]),
            (AdminLive, &[View]),
            (AdminPlan, &[View, Update]),
            (AdminReimbReview, &[View, Update]),
            (AdminAnnouncements, &[View, Create, Delete]),
            (AdminHelp, &[View, Update]),
            (AdminMembers, &[View]),
            (AdminAreas, &[View, Create, Update]),
            (AdminAudit, &[View]),
        ],
        "finance" => &[(AdminReimbPay, &[View, Update]), (AdminBills, &[View])],
        // 管理员 also acts as 交通组 and 活动负责人, but not as finance or driver:
        // receipts and payment records stay with finance (按职务需要查看).
        "admin" => {
            let mut out = defaults("transport");
            out.extend(defaults("leader"));
            let own: &[(Page, &[Act])] = &[
                (AdminEvents, &[View, Update, Delete]),
                (AdminReports, &[View]),
                (AdminSettings, &[View, Update]),
                (AdminMembers, &[View, Update]),
                (AdminInvites, &[View, Create, Delete]),
                (AdminRecovery, &[View, Update]),
                (AdminGroups, &[View, Create, Update, Delete]),
                (AdminRoles, &[View, Create, Update, Delete]),
            ];
            out.extend(own.iter().flat_map(|(p, a)| grid(*p, a)));
            out.sort_by_key(|p| (p.0 as usize, p.1 as usize));
            out.dedup();
            return out;
        }
        _ => &[],
    };
    pages.iter().flat_map(|(p, a)| grid(*p, a)).collect()
}

/// Permissions a role can never lose. Only `superadmin` has any: it keeps all of them,
/// including ones added by later releases (start-up grants every locked permission),
/// so the app cannot be left with nobody able to manage roles and members.
pub fn locked(role: &str) -> Vec<Perm> {
    if role == SUPERADMIN_ROLE {
        all().collect()
    } else {
        Vec::new()
    }
}

/// Whether these role keys include the super admin role.
pub fn is_superadmin(roles: &[String]) -> bool {
    roles.iter().any(|r| r == SUPERADMIN_ROLE)
}

/// Start-up, after seeding: there is always one active super admin. If nobody holds
/// the role (a database from before it existed, or a first run), it goes to the
/// original administrator: `preferred_email` when that account is an active admin,
/// otherwise the longest-standing active admin. Returns who was promoted.
pub async fn ensure_superadmin(
    db: &PgPool,
    preferred_email: Option<&str>,
) -> sqlx::Result<Option<(i64, String)>> {
    let holds = |role: &str| format!("'{role}' = ANY(string_to_array(roles, ','))");
    let existing: Option<(i64,)> = sqlx::query_as(&format!(
        "SELECT id FROM users WHERE status = 'active' AND {} LIMIT 1",
        holds(SUPERADMIN_ROLE)
    ))
    .fetch_optional(db)
    .await?;
    if existing.is_some() {
        return Ok(None);
    }
    let promoted: Option<(i64, String)> = sqlx::query_as(&format!(
        "UPDATE users SET roles = '{SUPERADMIN_ROLE},' || roles
         WHERE id = (SELECT id FROM users WHERE status = 'active' AND {}
                     ORDER BY (email = $1) DESC, id LIMIT 1)
         RETURNING id, email",
        holds(ADMIN_ROLE)
    ))
    .bind(preferred_email.unwrap_or_default())
    .fetch_optional(db)
    .await?;
    Ok(promoted)
}

/// Role keys as stored in `users.roles`.
pub fn parse_roles(s: &str) -> Vec<String> {
    s.split(',')
        .map(|r| r.trim().to_string())
        .filter(|r| !r.is_empty() && r != MEMBER_ROLE)
        .collect()
}

/// SQL condition: the `users` row aliased `alias` is active and holds `perm`.
/// `perm` comes from the catalog, never from input.
pub fn sql_holds(alias: &str, perm: Perm) -> String {
    format!(
        "({alias}.status = 'active' AND EXISTS (SELECT 1 FROM role_permissions rp WHERE rp.permission = '{}' \
         AND (rp.role_key = '{MEMBER_ROLE}' OR rp.role_key = ANY(string_to_array({alias}.roles, ',')))))",
        perm.key()
    )
}

/// Permissions granted by the given roles, plus the `member` role.
pub async fn for_roles(db: &PgPool, roles: &[String]) -> sqlx::Result<PermSet> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT permission FROM role_permissions WHERE role_key = $1 OR role_key = ANY($2)",
    )
    .bind(MEMBER_ROLE)
    .bind(roles)
    .fetch_all(db)
    .await?;
    Ok(PermSet::from_keys(rows.iter().map(|r| r.0.as_str())))
}

/// Whether an account with these roles (as stored in `users.roles`) reaches staff pages.
pub async fn is_privileged(db: &PgPool, roles: &str) -> sqlx::Result<bool> {
    Ok(for_roles(db, &parse_roles(roles)).await?.privileged())
}

/// Start-up: make sure the built-in roles exist, give them the defaults for permissions
/// this database has not seen before, and restore the locked ones.
pub async fn sync(db: &PgPool) -> sqlx::Result<()> {
    let mut tx = db.begin().await?;
    let system: Vec<String> = SYSTEM_ROLES.iter().map(|r| r.to_string()).collect();
    sqlx::query(
        "INSERT INTO roles (key, is_system) SELECT k, 1 FROM UNNEST($1::text[]) AS k
         ON CONFLICT (key) DO UPDATE SET is_system = 1",
    )
    .bind(&system)
    .execute(&mut *tx)
    .await?;

    let known: HashSet<String> =
        sqlx::query_as::<_, (String,)>("SELECT permission FROM permission_catalog")
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .map(|r| r.0)
            .collect();
    let fresh: HashSet<Perm> = all().filter(|p| !known.contains(&p.key())).collect();

    let (mut grant_roles, mut grant_perms) = (Vec::new(), Vec::new());
    for role in SYSTEM_ROLES {
        let wanted = defaults(role)
            .into_iter()
            .filter(|p| fresh.contains(p))
            .chain(locked(role));
        for p in wanted {
            grant_roles.push(role.to_string());
            grant_perms.push(p.key());
        }
    }
    sqlx::query(
        "INSERT INTO role_permissions (role_key, permission)
         SELECT * FROM UNNEST($1::text[], $2::text[]) ON CONFLICT DO NOTHING",
    )
    .bind(&grant_roles)
    .bind(&grant_perms)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO permission_catalog (permission) SELECT * FROM UNNEST($1::text[]) ON CONFLICT DO NOTHING",
    )
    .bind(fresh.iter().map(|p| p.key()).collect::<Vec<_>>())
    .execute(&mut *tx)
    .await?;
    tx.commit().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_keys_are_unique_and_round_trip() {
        let mut seen = HashSet::new();
        for (i, d) in CATALOG.iter().enumerate() {
            assert_eq!(d.page as usize, i, "{} is out of order", d.key);
            assert!(seen.insert(d.key), "duplicate page key {}", d.key);
            assert_eq!(d.page.def().key, d.key);
            for a in d.actions {
                let p = Perm(d.page, *a);
                assert_eq!(Perm::parse(&p.key()), Some(p));
            }
        }
    }

    #[test]
    fn parse_rejects_actions_a_page_does_not_offer() {
        assert_eq!(Perm::parse("report.delete"), None);
        assert_eq!(Perm::parse("nowhere.view"), None);
        assert_eq!(Perm::parse("rides"), None);
    }

    #[test]
    fn staff_pages_are_sensitive_and_member_pages_are_not() {
        for d in CATALOG {
            assert_eq!(
                d.page.sensitive(),
                d.key.starts_with("admin_"),
                "{} should be named admin_* exactly when it is a staff page",
                d.key
            );
        }
    }

    #[test]
    fn defaults_keep_finance_away_from_admin_and_member_unprivileged() {
        let set = |role| PermSet(defaults(role).into_iter().collect());
        let admin = set("admin");
        assert!(admin.has(Page::AdminRoles, Act::Update));
        assert!(admin.has(Page::AdminRides, Act::Update));
        assert!(admin.has(Page::Events, Act::Create));
        assert!(!admin.has(Page::AdminReimbPay, Act::View));
        assert!(!admin.has(Page::AdminBills, Act::View));
        assert!(!admin.has(Page::Drive, Act::Create));
        assert!(!set("member").privileged());
        assert!(!set("driver").privileged());
        assert!(!set("leader").privileged());
        assert!(set("transport").privileged());
        assert!(set("finance").privileged());
        for p in locked("admin") {
            assert!(admin.has(p.0, p.1));
        }
    }

    #[test]
    fn usable_keys_hide_staff_permissions_without_mfa() {
        let set = PermSet(defaults("admin").into_iter().collect());
        assert!(set
            .usable_keys(true)
            .contains(&"admin_roles.update".to_string()));
        let weak = set.usable_keys(false);
        assert!(weak.contains(&"events.create".to_string()));
        assert!(weak.iter().all(|k| !k.starts_with("admin_")));
    }
}
