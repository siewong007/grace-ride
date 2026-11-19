-- Trip chat: emoji reactions and delivered / read receipts.
--
-- `ride_message_reactions`: one reaction per member per message (picking another emoji
-- replaces it, picking the same one again removes it). Rows go with their message, so
-- the chat-retention purge in `jobs.rs` clears them too.
--
-- `ride_chat_reads`: per member and trip, the highest message id their app has fetched
-- (`last_delivered_id`) and the highest one they have seen on screen (`last_read_id`).
-- A sender's message shows as delivered / read once every other trip member is past it.
CREATE TABLE IF NOT EXISTS ride_message_reactions (
    message_id BIGINT NOT NULL REFERENCES ride_messages(id) ON DELETE CASCADE,
    user_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    emoji      TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (app_now()),
    updated_at TEXT NOT NULL DEFAULT (app_now()),
    PRIMARY KEY (message_id, user_id)
);

CREATE TABLE IF NOT EXISTS ride_chat_reads (
    trip_id           BIGINT NOT NULL REFERENCES trips(id) ON DELETE CASCADE,
    user_id           BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    last_delivered_id BIGINT NOT NULL DEFAULT 0,
    last_read_id      BIGINT NOT NULL DEFAULT 0,
    created_at        TEXT NOT NULL DEFAULT (app_now()),
    updated_at        TEXT NOT NULL DEFAULT (app_now()),
    PRIMARY KEY (trip_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_ride_chat_reads_user ON ride_chat_reads(user_id);
CREATE INDEX IF NOT EXISTS idx_ride_message_reactions_user ON ride_message_reactions(user_id);
