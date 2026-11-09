//! Server-sent events: push "something changed for you" to open tabs.
//!
//! Polling stays as the fallback. This only removes the wait between a change
//! and the next poll tick, so a dropped stream degrades quietly.
//!
//! ponytail: one in-process broadcast channel. Fine for the single instance
//! this app is designed around; with more than one, fan out via Postgres
//! `LISTEN/NOTIFY` instead.

use std::{convert::Infallible, sync::OnceLock, time::Duration};

use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use tokio::sync::broadcast;
use tokio_stream::{wrappers::BroadcastStream, Stream, StreamExt};

use crate::{auth::CurrentUser, state::AppState};

/// `kind` is only a hint for the client: "notify" (ride/event/settlement
/// updates) or "chat" (a trip message).
#[derive(Clone, Debug)]
pub struct Push {
    pub user_id: i64,
    pub kind: &'static str,
}

static EVENTS: OnceLock<broadcast::Sender<Push>> = OnceLock::new();

pub fn channel() -> broadcast::Sender<Push> {
    EVENTS.get_or_init(|| broadcast::channel(256).0).clone()
}

/// Wake this user's open tabs. Dropped silently when nobody is listening.
pub fn publish(user_id: i64, kind: &'static str) {
    if let Some(tx) = EVENTS.get() {
        let _ = tx.send(Push { user_id, kind });
    }
}

pub async fn handler(
    State(st): State<AppState>,
    u: CurrentUser,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let uid = u.id;
    let stream = BroadcastStream::new(st.events.subscribe()).filter_map(move |msg| {
        // A lagged receiver just misses a tick; the next poll catches up.
        let ev = msg.ok()?;
        (ev.user_id == uid).then(|| Ok(Event::default().data(ev.kind)))
    });
    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}
