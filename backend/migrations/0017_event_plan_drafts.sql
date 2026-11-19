-- Transport arrangement drafts: traffic group members can save a draft and ask
-- the traffic lead (traffic_head) to approve the transport arrangement.

CREATE TABLE IF NOT EXISTS event_plan_drafts (
    event_id        BIGINT PRIMARY KEY REFERENCES events(id) ON DELETE CASCADE,
    status          TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'pending_approval', 'approved', 'returned')),
    assignments     TEXT NOT NULL DEFAULT '[]',
    updated_by      BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    submitted_by    BIGINT REFERENCES users(id) ON DELETE SET NULL,
    submitted_at    TEXT,
    reviewed_by     BIGINT REFERENCES users(id) ON DELETE SET NULL,
    reviewed_at     TEXT,
    review_notes    TEXT NOT NULL DEFAULT '',
    created_at      TEXT NOT NULL DEFAULT (app_now()),
    updated_at      TEXT NOT NULL DEFAULT (app_now())
);

CREATE INDEX IF NOT EXISTS idx_event_plan_drafts_status ON event_plan_drafts(status);
