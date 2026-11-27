//! Server-sent events: push "something changed for you" to open tabs.
//!
//! Polling stays as the fallback. This only removes the wait between a change
//! and the next poll tick, so a dropped stream degrades quietly.
//!
//! Each signed-in user gets an independent bounded broadcast channel so a
//! church-wide announcement never overflows a single global ring buffer or wakes
//! unrelated users' open tabs.

use std::{
    collections::HashMap,
    convert::Infallible,
    sync::{Arc, Mutex as StdMutex, OnceLock, RwLock as StdRwLock},
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

#[derive(Clone, Debug)]
pub struct StreamEvent {
    pub kind: &'static str,
    pub unread: Option<i64>,
}

const USER_CHANNEL_CAPACITY: usize = 64;
const MAX_STREAMS_PER_USER: usize = 5;

#[derive(Clone, Default)]
pub struct EventHub {
    channels: Arc<StdRwLock<HashMap<i64, broadcast::Sender<StreamEvent>>>>,
}

impl EventHub {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&self, user_id: i64) -> broadcast::Receiver<StreamEvent> {
        {
            let map = self.channels.read().unwrap_or_else(|e| e.into_inner());
            if let Some(tx) = map.get(&user_id) {
                return tx.subscribe();
            }
        }
        let mut map = self.channels.write().unwrap_or_else(|e| e.into_inner());
        let tx = map
            .entry(user_id)
            .or_insert_with(|| broadcast::channel(USER_CHANNEL_CAPACITY).0);
        tx.subscribe()
    }

    pub fn send(&self, push: Push) -> Result<usize, Infallible> {
        let sender = {
            let map = self.channels.read().unwrap_or_else(|e| e.into_inner());
            map.get(&push.user_id).cloned()
        };
        match sender {
            Some(tx) => Ok(tx
                .send(StreamEvent {
                    kind: push.kind,
                    unread: None,
                })
                .unwrap_or(0)),
            None => Ok(0),
        }
    }

    pub fn publish_unread(&self, user_id: i64, unread: i64) {
        let sender = {
            let map = self.channels.read().unwrap_or_else(|e| e.into_inner());
            map.get(&user_id).cloned()
        };
        if let Some(tx) = sender {
            let _ = tx.send(StreamEvent {
                kind: "unread",
                unread: Some(unread),
            });
        }
    }

    fn remove_if_empty(&self, user_id: i64) {
        let mut map = self.channels.write().unwrap_or_else(|e| e.into_inner());
        if let Some(tx) = map.get(&user_id) {
            if tx.receiver_count() == 0 {
                map.remove(&user_id);
            }
        }
    }
}

struct StreamEntry {
    stream_id: u64,
    session_hash: String,
    revoke_tx: tokio::sync::watch::Sender<bool>,
}

static EVENTS: OnceLock<EventHub> = OnceLock::new();
static ACTIVE_STREAMS: OnceLock<StdMutex<HashMap<i64, Vec<StreamEntry>>>> = OnceLock::new();
static NEXT_STREAM_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn stream_map() -> &'static StdMutex<HashMap<i64, Vec<StreamEntry>>> {
    ACTIVE_STREAMS.get_or_init(|| StdMutex::new(HashMap::new()))
}

struct StreamGuard {
    user_id: i64,
    stream_id: u64,
}
impl Drop for StreamGuard {
    fn drop(&mut self) {
        let mut became_empty = false;
        if let Ok(mut map) = stream_map().lock() {
            if let Some(entries) = map.get_mut(&self.user_id) {
                entries.retain(|e| e.stream_id != self.stream_id);
                if entries.is_empty() {
                    map.remove(&self.user_id);
                    became_empty = true;
                }
            }
        }
        if became_empty {
            if let Some(hub) = EVENTS.get() {
                hub.remove_if_empty(self.user_id);
            }
        }
    }
}

pub fn channel() -> EventHub {
    EVENTS.get_or_init(EventHub::new).clone()
}

/// Wake this user's open tabs. Dropped silently when nobody is listening.
pub fn publish(user_id: i64, kind: &'static str) {
    if let Some(hub) = EVENTS.get() {
        let _ = hub.send(Push { user_id, kind });
    }
}

/// Push an updated unread notification badge count to this user's open tabs.
pub fn publish_unread(user_id: i64, unread: i64) {
    if let Some(hub) = EVENTS.get() {
        hub.publish_unread(user_id, unread);
    }
}

/// True when the user currently has at least one open live SSE stream.
pub fn has_active_stream(user_id: i64) -> bool {
    stream_map()
        .lock()
        .ok()
        .is_some_and(|map| map.get(&user_id).is_some_and(|entries| !entries.is_empty()))
}

/// Immediately terminate any active SSE streams for a specific revoked session hash.
pub fn revoke_session(session_hash: &str) {
    if let Ok(map) = stream_map().lock() {
        for entries in map.values() {
            for entry in entries {
                if entry.session_hash == session_hash {
                    let _ = entry.revoke_tx.send(true);
                }
            }
        }
    }
}

/// Immediately terminate any active SSE streams for all other sessions of a user.
pub fn revoke_other_sessions(user_id: i64, keep_session_hash: &str) {
    if let Ok(map) = stream_map().lock() {
        if let Some(entries) = map.get(&user_id) {
            for entry in entries {
                if entry.session_hash != keep_session_hash {
                    let _ = entry.revoke_tx.send(true);
                }
            }
        }
    }
}

/// Immediately terminate all active SSE streams owned by `user_id`.
pub fn revoke_user(user_id: i64) {
    if let Ok(map) = stream_map().lock() {
        if let Some(entries) = map.get(&user_id) {
            for entry in entries {
                let _ = entry.revoke_tx.send(true);
            }
        }
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
    // Fail closed when session validity cannot be verified.
    .is_ok_and(|n| n > 0)
}

pub async fn handler(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    let stream_id = NEXT_STREAM_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let (revoke_tx, revoke_rx) = tokio::sync::watch::channel(false);
    {
        let mut map = stream_map().lock().unwrap_or_else(|e| e.into_inner());
        let entries = map.entry(u.id).or_default();
        if entries.len() >= MAX_STREAMS_PER_USER {
            return Err(bad(u.lang, ErrorKey::RateLimited));
        }
        entries.push(StreamEntry {
            stream_id,
            session_hash: u.session_id.clone(),
            revoke_tx,
        });
    }
    let guard = Arc::new(StreamGuard {
        user_id: u.id,
        stream_id,
    });
    let rx = st.events.subscribe(u.id);
    let pushes = BroadcastStream::new(rx).filter_map(move |msg| {
        // A lagged receiver just misses a tick; the next poll catches up.
        let ev = msg.ok()?;
        let sse = match ev.unread {
            Some(n) => Event::default().event("unread").data(n.to_string()),
            None => Event::default().data(ev.kind),
        };
        Some(Tick::Push(sse))
    });
    // Signing out, a session running out, or an admin disabling the member closes the
    // stream within a minute instead of leaving it open until the tab closes.
    let state = st.clone();
    let session_id = u.session_id.clone();
    let mut checks =
        tokio::time::interval_at(tokio::time::Instant::now() + SESSION_CHECK, SESSION_CHECK);
    checks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let err_ticks = Arc::new(std::sync::atomic::AtomicU8::new(0));
    let checks = IntervalStream::new(checks).then(move |_| {
        let state = state.clone();
        let session_id = session_id.clone();
        let err_ticks = err_ticks.clone();
        async move {
            let tick =
                match crate::security::evaluate_session_alive_and_lock_checked(&state, &session_id)
                    .await
                {
                    Ok(true) => {
                        err_ticks.store(0, std::sync::atomic::Ordering::Relaxed);
                        true
                    }
                    Ok(false) => false,
                    Err(_) => {
                        // Fail closed after 3 consecutive DB errors (~3 min) instead of staying open forever.
                        let n = err_ticks.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                        n < 3
                    }
                };
            Tick::Session(tick)
        }
    });
    let revoked =
        WatchStream::new(revoke_rx).filter_map(|revoked| revoked.then_some(Tick::Session(false)));
    let shutdown = WatchStream::new(st.shutdown.subscribe())
        .filter_map(|stop| stop.then_some(Tick::Session(false)));
    let stream = pushes
        .merge(checks)
        .merge(revoked)
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
