-- 恩典同行 Grace Ride schema
-- Times are stored as local (Asia/Kuala_Lumpur) ISO strings "YYYY-MM-DDTHH:MM[:SS]".
-- Money is stored as integer cents (RM 0.01).

CREATE TABLE users (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT NOT NULL,
    email           TEXT NOT NULL UNIQUE COLLATE NOCASE,
    phone           TEXT NOT NULL DEFAULT '',
    photo_file_id   INTEGER,
    roles           TEXT NOT NULL DEFAULT '',          -- comma list: admin,transport,finance,leader,driver
    status          TEXT NOT NULL DEFAULT 'active',    -- active | disabled
    password_hash   TEXT,
    totp_secret     TEXT,
    totp_enabled    INTEGER NOT NULL DEFAULT 0,
    webauthn_uuid   TEXT NOT NULL,
    badges_opt_out  INTEGER NOT NULL DEFAULT 0,
    carpool_enabled INTEGER NOT NULL DEFAULT 0,
    spare_seats     INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE vehicles (
    user_id INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    model   TEXT NOT NULL,
    color   TEXT NOT NULL,
    plate   TEXT NOT NULL,
    seats   INTEGER NOT NULL DEFAULT 4
);

CREATE TABLE invites (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    token       TEXT NOT NULL UNIQUE,
    kind        TEXT NOT NULL DEFAULT 'invite',        -- invite | recovery
    email       TEXT NOT NULL,
    name        TEXT NOT NULL DEFAULT '',
    roles       TEXT NOT NULL DEFAULT '',
    group_ids   TEXT NOT NULL DEFAULT '[]',
    user_id     INTEGER REFERENCES users(id),          -- for recovery
    created_by  INTEGER NOT NULL REFERENCES users(id),
    expires_at  TEXT NOT NULL,
    used_at     TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE passkeys (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    cred_id     TEXT NOT NULL UNIQUE,
    passkey     TEXT NOT NULL,
    label       TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL DEFAULT (datetime('now','localtime')),
    last_used   TEXT
);

CREATE TABLE sessions (
    token       TEXT PRIMARY KEY,
    user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    mfa_ok      INTEGER NOT NULL DEFAULT 0,
    expires_at  TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE auth_challenges (
    id          TEXT PRIMARY KEY,
    user_id     INTEGER NOT NULL,
    kind        TEXT NOT NULL,      -- reg | auth
    state       TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE recovery_requests (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    email       TEXT NOT NULL,
    reason      TEXT NOT NULL DEFAULT '',
    status      TEXT NOT NULL DEFAULT 'pending',    -- pending | approved | rejected
    resolved_by INTEGER REFERENCES users(id),
    created_at  TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE areas (
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    name    TEXT NOT NULL UNIQUE,
    active  INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE driver_areas (
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    area_id INTEGER NOT NULL REFERENCES areas(id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, area_id)
);

CREATE TABLE groups (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    is_all      INTEGER NOT NULL DEFAULT 0       -- "全体会员": every active member is implicitly in it
);

CREATE TABLE group_members (
    group_id INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    user_id  INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    PRIMARY KEY (group_id, user_id)
);

CREATE TABLE files (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    owner_id      INTEGER NOT NULL REFERENCES users(id),
    stored_name   TEXT NOT NULL,
    mime          TEXT NOT NULL,
    original_name TEXT NOT NULL DEFAULT '',
    size          INTEGER NOT NULL DEFAULT 0,
    created_at    TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE events (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    title            TEXT NOT NULL,
    kind             TEXT NOT NULL DEFAULT 'other',    -- badminton | bowling | kayak | walk | other
    description      TEXT NOT NULL DEFAULT '',
    starts_at        TEXT NOT NULL,
    location         TEXT NOT NULL DEFAULT '',
    capacity         INTEGER NOT NULL DEFAULT 0,       -- 0 = unlimited
    cost_note        TEXT NOT NULL DEFAULT '',
    signup_deadline  TEXT,
    status           TEXT NOT NULL DEFAULT 'published',-- published | cancelled | finished
    created_by       INTEGER NOT NULL REFERENCES users(id),
    created_at       TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE event_groups (
    event_id INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    group_id INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    PRIMARY KEY (event_id, group_id)
);

CREATE TABLE event_signups (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id     INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    user_id      INTEGER NOT NULL REFERENCES users(id),
    status       TEXT NOT NULL DEFAULT 'going',    -- going | waitlist | cancelled
    need_ride    INTEGER NOT NULL DEFAULT 0,
    offer_drive  INTEGER NOT NULL DEFAULT 0,
    offer_seats  INTEGER NOT NULL DEFAULT 0,
    attended     INTEGER,                          -- NULL unknown, 0 absent, 1 attended
    created_at   TEXT NOT NULL DEFAULT (datetime('now','localtime')),
    UNIQUE (event_id, user_id)
);

CREATE TABLE rides (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    passenger_id      INTEGER NOT NULL REFERENCES users(id),
    driver_id         INTEGER REFERENCES users(id),
    event_id          INTEGER REFERENCES events(id),
    pickup_at         TEXT NOT NULL,
    pickup_area_id    INTEGER NOT NULL REFERENCES areas(id),
    pickup_address    TEXT NOT NULL DEFAULT '',     -- private until approved
    destination       TEXT NOT NULL DEFAULT '',
    seats             INTEGER NOT NULL DEFAULT 1,
    assistance        TEXT NOT NULL DEFAULT '',
    notes             TEXT NOT NULL DEFAULT '',
    status            TEXT NOT NULL DEFAULT 'pending_driver',
    -- pending_driver → pending_passenger → pending_approval → confirmed → in_progress → completed
    -- side states: pending_driver_confirm (after reassignment), cancelled, no_show,
    --              alt_transport (交通组批准 Grab 替代交通)
    trip_id           INTEGER,                      -- rides sharing a driver/time form one trip (shared chat)
    needs_attention   INTEGER NOT NULL DEFAULT 0,
    attention_reason  TEXT NOT NULL DEFAULT '',
    approved_by       INTEGER REFERENCES users(id),
    started_at        TEXT,
    picked_up_at      TEXT,
    completed_at      TEXT,
    created_at        TEXT NOT NULL DEFAULT (datetime('now','localtime')),
    updated_at        TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);
CREATE INDEX idx_rides_status ON rides(status);
CREATE INDEX idx_rides_driver ON rides(driver_id);
CREATE INDEX idx_rides_passenger ON rides(passenger_id);

CREATE TABLE ride_messages (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    trip_id    INTEGER NOT NULL,                 -- = rides.trip_id
    user_id    INTEGER REFERENCES users(id),     -- NULL = system message
    body       TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE driver_locations (
    driver_id  INTEGER PRIMARY KEY REFERENCES users(id),
    lat        REAL NOT NULL,
    lng        REAL NOT NULL,
    heading    REAL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE notifications (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title      TEXT NOT NULL,
    body       TEXT NOT NULL DEFAULT '',
    link       TEXT NOT NULL DEFAULT '',
    read_at    TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);
CREATE INDEX idx_notif_user ON notifications(user_id, read_at);

CREATE TABLE announcements (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    title      TEXT NOT NULL,
    body       TEXT NOT NULL,
    pinned     INTEGER NOT NULL DEFAULT 0,
    created_by INTEGER NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE help_tickets (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id    INTEGER NOT NULL REFERENCES users(id),
    ride_id    INTEGER REFERENCES rides(id),
    subject    TEXT NOT NULL,
    status     TEXT NOT NULL DEFAULT 'open',    -- open | answered | closed
    created_at TEXT NOT NULL DEFAULT (datetime('now','localtime')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE help_replies (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    ticket_id  INTEGER NOT NULL REFERENCES help_tickets(id) ON DELETE CASCADE,
    user_id    INTEGER NOT NULL REFERENCES users(id),
    body       TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE audit_logs (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    actor_id   INTEGER REFERENCES users(id),
    action     TEXT NOT NULL,
    entity     TEXT NOT NULL,
    entity_id  INTEGER,
    detail     TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE reimbursements (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id         INTEGER NOT NULL REFERENCES users(id),
    ride_id         INTEGER REFERENCES rides(id),
    amount_cents    INTEGER NOT NULL,
    description     TEXT NOT NULL DEFAULT '',
    receipt_file_id INTEGER NOT NULL REFERENCES files(id),
    payee_name      TEXT NOT NULL DEFAULT '',
    payee_phone     TEXT NOT NULL DEFAULT '',
    status          TEXT NOT NULL DEFAULT 'pending',  -- pending | returned | rejected | approved | paid
    review_note     TEXT NOT NULL DEFAULT '',
    reviewed_by     INTEGER REFERENCES users(id),
    payee_verified  INTEGER NOT NULL DEFAULT 0,
    paid_ref        TEXT NOT NULL DEFAULT '',
    paid_by         INTEGER REFERENCES users(id),
    paid_at         TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now','localtime')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE settlements (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id       INTEGER NOT NULL REFERENCES events(id),
    created_by     INTEGER NOT NULL REFERENCES users(id),
    title          TEXT NOT NULL,
    split_mode     TEXT NOT NULL DEFAULT 'equal',   -- equal | custom
    subsidy_cents  INTEGER NOT NULL DEFAULT 0,
    payee_name     TEXT NOT NULL DEFAULT '',
    payee_account  TEXT NOT NULL DEFAULT '',       -- TNG phone / DuitNow / bank details
    due_date       TEXT,
    status         TEXT NOT NULL DEFAULT 'draft',  -- draft | published | closed
    version        INTEGER NOT NULL DEFAULT 1,
    created_at     TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE settlement_items (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    settlement_id   INTEGER NOT NULL REFERENCES settlements(id) ON DELETE CASCADE,
    label           TEXT NOT NULL,
    amount_cents    INTEGER NOT NULL,
    receipt_file_id INTEGER REFERENCES files(id)
);

CREATE TABLE settlement_shares (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    settlement_id  INTEGER NOT NULL REFERENCES settlements(id) ON DELETE CASCADE,
    user_id        INTEGER NOT NULL REFERENCES users(id),
    amount_cents   INTEGER NOT NULL,
    status         TEXT NOT NULL DEFAULT 'unpaid',  -- unpaid | pending | confirmed | needs_info
    proof_file_id  INTEGER REFERENCES files(id),
    txn_ref        TEXT NOT NULL DEFAULT '',
    paid_date      TEXT,
    note           TEXT NOT NULL DEFAULT '',
    updated_at     TEXT NOT NULL DEFAULT (datetime('now','localtime')),
    UNIQUE (settlement_id, user_id)
);

CREATE TABLE settlement_revisions (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    settlement_id  INTEGER NOT NULL REFERENCES settlements(id) ON DELETE CASCADE,
    version        INTEGER NOT NULL,
    snapshot       TEXT NOT NULL,
    reason         TEXT NOT NULL DEFAULT '',
    created_by     INTEGER NOT NULL REFERENCES users(id),
    created_at     TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);
