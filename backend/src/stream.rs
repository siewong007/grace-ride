//! Server-sent events: push "something changed for you" to open tabs.
//!
//! Polling stays as the fallback. This only removes the wait between a change
//! and the next poll tick, so a dropped stream degrades quietly.
//!
//! ponytail: one in-process broadcast channel. Fine for the single instance
//! this app is designed around; with more than one, fan out via Postgres
//! `LISTEN/NOTIFY` instead.

use std::{
    collections::HashMap, convert::Infallible, sync::Mutex as StdMutex, sync::OnceLock,
    time::Duration,
};

use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use tokio::sync::broadcast;
use tokio_stream::{
    wrappers::{BroadcastStream, IntervalStream, WatchStream},
    Stream, StreamExt,
};

use crate::{
    auth::CurrentUser,
    error::{bad, AppResult},
    i18n::ErrorKey,
    state::AppState,
};

/// `kind` is only a hint for the client: "notify" (ride/event/settlement
/// updates) or "chat" (a trip message).
#[derive(Clone, Debug)]
pub struct Push {
    pub user_id: i64,
    pub kind: &'static str,
}

static EVENTS: OnceLock<broadcast::Sender<Push>> = OnceLock::new();
static ACTIVE_STREAMS: OnceLock<StdMutex<HashMap<i64, usize>>> = OnceLock::new();
const MAX_STREAMS_PER_USER: usize = 5;

fn stream_map() -> &'static StdMutex<HashMap<i64, usize>> {
    ACTIVE_STREAMS.get_or_init(|| StdMutex::new(HashMap::new()))
}

struct StreamGuard(i64);
impl Drop for StreamGuard {
    fn drop(&mut self) {
        if let Ok(mut map) = stream_map().lock() {
            if let Some(count) = map.get_mut(&self.0) {
                if *count <= 1 {
                    map.remove(&self.0);
                } else {
                    *count -= 1;
                }
            }
        }
    }
}

pub fn channel() -> broadcast::Sender<Push> {
    EVENTS.get_or_init(|| broadcast::channel(256).0).clone()
}

/// Wake this user's open tabs. Dropped silently when nobody is listening.
pub fn publish(user_id: i64, kind: &'static str) {
    if let Some(tx) = EVENTS.get() {
        let _ = tx.send(Push { user_id, kind });
    }
}

/// How often an open stream checks that its session is still valid.
const SESSION_CHECK: Duration = Duration::from_secs(60);

enum Tick {
    Push(Event),
    /// Result of the periodic session check.
    Session(bool),
}

/// Is this session still one the app accepts (not signed out, expired or disabled)?
pub async fn session_alive(db: &sqlx::PgPool, session_id: &str) -> bool {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM sessions s JOIN users u ON u.id = s.user_id
         WHERE s.token = $1 AND s.expires_at > $2 AND s.locked_at IS NULL AND u.status IN ('active', 'pending')",
    )
    .bind(session_id)
    .bind(crate::util::now())
    .fetch_one(db)
    .await
    // A database hiccup keeps the stream; only a clear "gone" ends it.
    .map_or(true, |n| n > 0)
}

/// Checks session liveness including server-enforced idle auto-lock.
async fn session_alive_checked(st: &AppState, session_id: &str) -> bool {
    if !session_alive(&st.db, session_id).await {
        return false;
    }
    crate::security::evaluate_session_lock(st, session_id, false)
        .await
        .is_none()
}

pub async fn handler(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    {
        let mut map = stream_map().lock().unwrap_or_else(|e| e.into_inner());
        let count = map.entry(u.id).or_insert(0);
        if *count >= MAX_STREAMS_PER_USER {
            return Err(bad(u.lang, ErrorKey::RateLimited));
        }
        *count += 1;
    }
    let guard = std::sync::Arc::new(StreamGuard(u.id));
    let uid = u.id;
    let pushes = BroadcastStream::new(st.events.subscribe()).filter_map(move |msg| {
        // A lagged receiver just misses a tick; the next poll catches up.
        let ev = msg.ok()?;
        (ev.user_id == uid).then(|| Tick::Push(Event::default().data(ev.kind)))
    });
    // Signing out, a session running out, or an admin disabling the member closes the
    // stream within a minute instead of leaving it open until the tab closes.
    let state = st.clone();
    let session_id = u.session_id.clone();
    let mut checks =
        tokio::time::interval_at(tokio::time::Instant::now() + SESSION_CHECK, SESSION_CHECK);
    checks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let checks = IntervalStream::new(checks).then(move |_| {
        let state = state.clone();
        let session_id = session_id.clone();
        async move { Tick::Session(session_alive_checked(&state, &session_id).await) }
    });
    let shutdown = WatchStream::new(st.shutdown.subscribe())
        .filter_map(|stop| stop.then_some(Tick::Session(false)));
    let stream = pushes
        .merge(checks)
        .merge(shutdown)
        .take_while(|t| !matches!(t, Tick::Session(false)))
        .filter_map(move |t| {
            let _hold = &guard;
            match t {
                Tick::Push(ev) => Some(Ok(ev)),
                Tick::Session(_) => None,
            }
        });
    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    ))
}
