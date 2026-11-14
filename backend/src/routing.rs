//! Pickup routing for a driver's trip: the best order to collect several passengers,
//! the road route, and the distance and ETA to each stop.
//!
//! Uses free OpenStreetMap services (no API key, no billing):
//! - **Nominatim** turns pickup addresses and the destination into coordinates. Results
//!   are cached in `geocode_cache` and requests are spaced at least 1 s apart, as its
//!   usage policy asks. A passenger's shared GPS spot beats their geocoded address.
//! - **OSRM** gives the road travel-time matrix (to choose the order) and the route line.
//! - **Photon** (komoot) suggests places while someone types an address. Nominatim's usage
//!   policy forbids search-as-you-type, so autocomplete uses Photon, which is built for it.
//!   Suggestions are proxied (the browser never contacts Photon) and each one is written
//!   to `geocode_cache`, so a ride booked with a picked address routes to that exact point.
//!
//! Both base URLs are configurable so the services can be self-hosted. When routing is
//! turned off or a service fails, straight-line estimates are used and the route is
//! marked `approximate`. ETAs don't include live traffic: turn-by-turn navigation is
//! handed off to Google Maps or Waze in the driver's phone.

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::Local;
use serde::Serialize;
use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::error::AppResult;
use crate::state::AppState;
use crate::util::now;

// ---------------- Configuration and shared state ----------------

#[derive(Clone, Debug)]
pub struct RoutingConfig {
    /// Call the geocoder and router over the network. Off ⇒ only cached coordinates and
    /// passengers' shared GPS are used, with straight-line estimates (tests run this way).
    pub enabled: bool,
    /// OSRM base URL. The public demo server is fine for a small community; self-host for more.
    pub osrm_url: String,
    /// Nominatim base URL.
    pub nominatim_url: String,
    /// Photon base URL, for address autocomplete.
    pub photon_url: String,
    /// Sent with every request; Nominatim requires clients to identify themselves.
    pub user_agent: String,
    /// Nominatim `countrycodes` filter (comma-separated ISO codes); empty = worldwide.
    pub country: String,
}

impl Default for RoutingConfig {
    /// Network calls off. `from_env` turns them on unless `ROUTING_ENABLED=false`.
    fn default() -> Self {
        Self {
            enabled: false,
            osrm_url: "https://router.project-osrm.org".into(),
            nominatim_url: "https://nominatim.openstreetmap.org".into(),
            photon_url: "https://photon.komoot.io".into(),
            user_agent: concat!("GraceRide/", env!("CARGO_PKG_VERSION")).into(),
            country: "my".into(),
        }
    }
}

impl RoutingConfig {
    pub fn from_env() -> Self {
        let d = Self::default();
        let env = |k: &str, def: &str| {
            std::env::var(k)
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| def.to_string())
        };
        Self {
            enabled: env("ROUTING_ENABLED", "true") == "true",
            osrm_url: env("OSRM_URL", &d.osrm_url)
                .trim_end_matches('/')
                .to_string(),
            nominatim_url: env("NOMINATIM_URL", &d.nominatim_url)
                .trim_end_matches('/')
                .to_string(),
            photon_url: env("PHOTON_URL", &d.photon_url)
                .trim_end_matches('/')
                .to_string(),
            user_agent: env("GEOCODER_USER_AGENT", &d.user_agent),
            // Set but empty means "search worldwide".
            country: std::env::var("GEOCODER_COUNTRY").unwrap_or(d.country),
        }
    }
}

/// Per-process caches: computed routes per (driver, trip), the geocoder's request gate,
/// and an HTTP client for the routing services.
pub struct RoutingState {
    routes: Mutex<HashMap<(i64, i64), Cached>>,
    /// One computation at a time per trip, so the driver and every passenger refreshing
    /// together cost one set of routing calls.
    inflight: Mutex<HashMap<(i64, i64), Arc<Mutex<()>>>>,
    geocoder_gate: Mutex<Option<Instant>>,
    /// Recent autocomplete answers, so retyping or several members typing the same
    /// thing don't each cost a Photon request.
    places: Mutex<HashMap<String, (Instant, Vec<Place>)>>,
    http: reqwest::Client,
}

impl Default for RoutingState {
    fn default() -> Self {
        Self {
            routes: Mutex::default(),
            inflight: Mutex::default(),
            geocoder_gate: Mutex::default(),
            places: Mutex::default(),
            // rustls for TLS 1.3 everywhere (the public OSRM server refuses 1.2). Public
            // servers also drop idle keep-alive connections, so don't hold ours long.
            http: reqwest::Client::builder()
                .use_rustls_tls()
                .pool_idle_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(8))
                .build()
                .unwrap_or_default(),
        }
    }
}

struct Cached {
    key: String,
    at: Instant,
    route: TripRoute,
    /// Ride ids of located stops in visiting order, so the order stays stable.
    order: Vec<i64>,
}

impl RoutingState {
    pub async fn invalidate(&self, driver: i64, trip: i64) {
        self.routes.lock().await.remove(&(driver, trip));
    }

    /// Nominatim allows at most one request per second.
    async fn wait_geocoder_slot(&self) {
        let mut last = self.geocoder_gate.lock().await;
        if let Some(t) = *last {
            let wait = Duration::from_millis(1100).saturating_sub(t.elapsed());
            if !wait.is_zero() {
                tokio::time::sleep(wait).await;
            }
        }
        *last = Some(Instant::now());
    }
}

/// How long a computed route is reused while the driver is moving, and while only planning.
const LIVE_TTL: Duration = Duration::from_secs(30);
const PLAN_TTL: Duration = Duration::from_secs(300);
/// Boarding time added at each pickup before the car moves on.
const PICKUP_DWELL_S: f64 = 60.0;
/// The new best order must save this much before a driver's stops are reshuffled.
const KEEP_ORDER_MARGIN_S: f64 = 60.0;
/// Most pickups solved exactly (8! orders); larger trips use a heuristic.
const EXACT_MAX: usize = 8;

// ---------------- Geometry ----------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LatLng {
    pub lat: f64,
    pub lng: f64,
}

/// Great-circle distance in metres.
pub fn haversine_m(a: LatLng, b: LatLng) -> f64 {
    const R: f64 = 6_371_000.0;
    let (p1, p2) = (a.lat.to_radians(), b.lat.to_radians());
    let dp = p2 - p1;
    let dl = (b.lng - a.lng).to_radians();
    let h = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * R * h.sqrt().min(1.0).asin()
}

/// Straight-line fallback: city roads run ~35 % longer than the crow flies, at ~30 km/h.
const ROAD_FACTOR: f64 = 1.35;
const CITY_SPEED_MS: f64 = 30.0 / 3.6;

fn estimate(a: LatLng, b: LatLng) -> (f64, f64) {
    let d = haversine_m(a, b) * ROAD_FACTOR;
    (d, d / CITY_SPEED_MS)
}

fn straight_matrix(points: &[LatLng]) -> Vec<Vec<f64>> {
    points
        .iter()
        .map(|&a| points.iter().map(|&b| estimate(a, b).1).collect())
        .collect()
}

fn round5(x: f64) -> f64 {
    (x * 1e5).round() / 1e5
}

/// For each waypoint, the index of the closest point on `line`, searching forward so the
/// legs stay in order. The first and last waypoints are the line's ends.
pub fn split_points(line: &[[f64; 2]], waypoints: &[LatLng]) -> Vec<usize> {
    let d2 = |p: [f64; 2], w: &LatLng| (p[0] - w.lat).powi(2) + (p[1] - w.lng).powi(2);
    let mut out = Vec::with_capacity(waypoints.len());
    let mut from = 0;
    for (k, w) in waypoints.iter().enumerate() {
        let idx = if line.is_empty() || k == 0 {
            0
        } else if k == waypoints.len() - 1 {
            line.len() - 1
        } else {
            (from..line.len())
                .min_by(|&a, &b| d2(line[a], w).total_cmp(&d2(line[b], w)))
                .unwrap_or(from)
        };
        from = idx;
        out.push(idx);
    }
    out
}

// ---------------- Stop order ----------------

/// Total travel time of visiting `seq` (matrix indices) from `start`, finishing at `end`.
pub fn path_cost(m: &[Vec<f64>], start: Option<usize>, seq: &[usize], end: Option<usize>) -> f64 {
    let mut it = start.into_iter().chain(seq.iter().copied()).chain(end);
    let Some(mut prev) = it.next() else {
        return 0.0;
    };
    let mut total = 0.0;
    for next in it {
        total += m[prev][next];
        prev = next;
    }
    total
}

/// Order of `stops` (matrix indices) with the least total travel time, starting at
/// `start` (the driver) and ending at `end` (the destination) when given. Exact for up to
/// eight stops; nearest-neighbour plus 2-opt beyond that. Ties keep the given order.
pub fn best_order(
    m: &[Vec<f64>],
    start: Option<usize>,
    stops: &[usize],
    end: Option<usize>,
) -> Vec<usize> {
    if stops.len() <= 1 {
        return stops.to_vec();
    }
    if stops.len() <= EXACT_MAX {
        exact_order(m, start, stops, end)
    } else {
        heuristic_order(m, start, stops, end)
    }
}

/// Every permutation (Heap's algorithm), keeping the cheapest.
fn exact_order(
    m: &[Vec<f64>],
    start: Option<usize>,
    stops: &[usize],
    end: Option<usize>,
) -> Vec<usize> {
    let mut a = stops.to_vec();
    let mut best = a.clone();
    let mut best_cost = path_cost(m, start, &a, end);
    let n = a.len();
    let mut c = vec![0usize; n];
    let mut i = 1;
    while i < n {
        if c[i] < i {
            if i % 2 == 0 {
                a.swap(0, i);
            } else {
                a.swap(c[i], i);
            }
            let cost = path_cost(m, start, &a, end);
            if cost < best_cost - 1e-9 {
                best_cost = cost;
                best.clone_from(&a);
            }
            c[i] += 1;
            i = 1;
        } else {
            c[i] = 0;
            i += 1;
        }
    }
    best
}

fn heuristic_order(
    m: &[Vec<f64>],
    start: Option<usize>,
    stops: &[usize],
    end: Option<usize>,
) -> Vec<usize> {
    let mut left = stops.to_vec();
    let mut seq = Vec::with_capacity(left.len());
    let mut cur = match start {
        Some(s) => s,
        None => {
            let first = left.remove(0);
            seq.push(first);
            first
        }
    };
    while !left.is_empty() {
        let k = (0..left.len())
            .min_by(|&a, &b| m[cur][left[a]].total_cmp(&m[cur][left[b]]))
            .unwrap_or(0);
        cur = left.remove(k);
        seq.push(cur);
    }
    let mut cost = path_cost(m, start, &seq, end);
    let mut improved = true;
    while improved {
        improved = false;
        for i in 0..seq.len() {
            for j in i + 1..seq.len() {
                let mut cand = seq.clone();
                cand[i..=j].reverse();
                let c = path_cost(m, start, &cand, end);
                if c + 1e-9 < cost {
                    seq = cand;
                    cost = c;
                    improved = true;
                }
            }
        }
    }
    seq
}

/// Best order, but keep the driver's current order unless the new one saves a minute or
/// more, so stops don't reshuffle every refresh as the car moves.
pub fn choose_order(
    m: &[Vec<f64>],
    start: Option<usize>,
    stops: &[usize],
    end: Option<usize>,
    previous: Option<&[usize]>,
) -> Vec<usize> {
    let best = best_order(m, start, stops, end);
    if let Some(prev) = previous {
        let same_set = prev.len() == stops.len() && stops.iter().all(|s| prev.contains(s));
        if same_set
            && path_cost(m, start, prev, end)
                <= path_cost(m, start, &best, end) + KEEP_ORDER_MARGIN_S
        {
            return prev.to_vec();
        }
    }
    best
}

// ---------------- Geocoding ----------------

fn cache_key(addr: &str) -> String {
    addr.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Forms of an address to try, most specific first. Malaysian addresses often only match
/// once the unit or house number is dropped ("No 12, Jalan SS 2/24, …" → "Jalan SS 2/24, …").
pub fn address_variants(addr: &str) -> Vec<String> {
    let parts: Vec<&str> = addr
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    if parts.is_empty() {
        return Vec::new();
    }
    let mut out = vec![parts.join(", ")];
    let mut i = 1;
    while out.len() < 3 && parts.len() >= i + 2 {
        out.push(parts[i..].join(", "));
        i += 1;
    }
    out
}

/// Coordinates for an address, and whether only a shortened form matched. From the cache,
/// else from Nominatim when routing is enabled. Not-found results are cached for a day;
/// outages aren't cached. The housekeeping job purges entries after 30 days.
pub async fn geocode(st: &AppState, address: &str) -> Option<(LatLng, bool)> {
    let key = cache_key(address);
    if key.is_empty() {
        return None;
    }
    let cfg = &st.cfg.routing;
    let cached: Option<(Option<f64>, Option<f64>, bool, String)> =
        sqlx::query_as("SELECT lat, lng, approx, updated_at FROM geocode_cache WHERE query=$1")
            .bind(&key)
            .fetch_optional(&st.db)
            .await
            .ok()
            .flatten();
    if let Some((lat, lng, approx, at)) = cached {
        if let (Some(lat), Some(lng)) = (lat, lng) {
            return Some((LatLng { lat, lng }, approx));
        }
        if !cfg.enabled || at > ago(Duration::from_secs(24 * 3600)) {
            return None;
        }
    }
    if !cfg.enabled {
        return None;
    }
    let mut found = None;
    for (i, q) in address_variants(address).iter().enumerate() {
        match nominatim(st, q).await {
            Ok(Some(p)) => {
                found = Some((p, i > 0));
                break;
            }
            Ok(None) => continue,
            Err(e) => {
                // Never log the address itself: it's a member's home.
                tracing::warn!("geocoder unavailable: {e}");
                return None;
            }
        }
    }
    let (lat, lng, approx) = match found {
        Some((p, a)) => (Some(p.lat), Some(p.lng), a),
        None => (None, None, false),
    };
    let _ = sqlx::query(
        "INSERT INTO geocode_cache (query, lat, lng, approx, updated_at) VALUES ($1,$2,$3,$4,$5)
         ON CONFLICT(query) DO UPDATE SET lat=excluded.lat, lng=excluded.lng, approx=excluded.approx, updated_at=excluded.updated_at",
    )
    .bind(&key)
    .bind(lat)
    .bind(lng)
    .bind(approx)
    .bind(now())
    .execute(&st.db)
    .await;
    found
}

/// Coordinates already in `geocode_cache` for these addresses (no network calls).
pub async fn cached_points(st: &AppState, addresses: &[String]) -> HashMap<String, LatLng> {
    let keys: Vec<String> = addresses.iter().map(|a| cache_key(a)).collect();
    let rows: Vec<(String, f64, f64)> = sqlx::query_as(
        "SELECT query, lat, lng FROM geocode_cache WHERE query = ANY($1) AND lat IS NOT NULL AND lng IS NOT NULL",
    )
    .bind(&keys)
    .fetch_all(&st.db)
    .await
    .unwrap_or_default();
    let found: HashMap<String, LatLng> = rows
        .into_iter()
        .map(|(k, lat, lng)| (k, LatLng { lat, lng }))
        .collect();
    addresses
        .iter()
        .filter_map(|a| found.get(&cache_key(a)).map(|p| (a.clone(), *p)))
        .collect()
}

/// Look up uncached addresses in the background (rate-limited like any geocode), so the
/// next planning run has exact distances. No-op when routing is off.
pub fn warm_geocodes(st: &AppState, addresses: Vec<String>) {
    if !st.cfg.routing.enabled || addresses.is_empty() {
        return;
    }
    let st = st.clone();
    tokio::spawn(async move {
        for a in addresses {
            let _ = geocode(&st, &a).await;
        }
    });
}

fn ago(d: Duration) -> String {
    (Local::now() - chrono::Duration::from_std(d).unwrap_or_default())
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string()
}

/// Error text with its causes, without the request URL (it carries addresses).
fn http_err(e: reqwest::Error) -> String {
    let e = e.without_url();
    let mut msg = e.to_string();
    let mut src = std::error::Error::source(&e);
    while let Some(cause) = src {
        msg.push_str(": ");
        msg.push_str(&cause.to_string());
        src = cause.source();
    }
    msg
}

/// Send a GET, retrying once if it couldn't be sent (e.g. a pooled connection the server
/// had closed). Timeouts aren't retried.
async fn send(req: reqwest::RequestBuilder) -> Result<reqwest::Response, String> {
    let again = req.try_clone();
    match req.send().await {
        Ok(res) => Ok(res),
        Err(e) if e.is_timeout() => Err(http_err(e)),
        Err(e) => match again {
            Some(req) => req.send().await.map_err(http_err),
            None => Err(http_err(e)),
        },
    }
}

async fn nominatim(st: &AppState, q: &str) -> Result<Option<LatLng>, String> {
    st.routing.wait_geocoder_slot().await;
    let cfg = &st.cfg.routing;
    let mut req = st
        .routing
        .http
        .get(format!("{}/search", cfg.nominatim_url))
        .query(&[("q", q), ("format", "jsonv2"), ("limit", "1")])
        .header(reqwest::header::USER_AGENT, &cfg.user_agent);
    if !cfg.country.trim().is_empty() {
        req = req.query(&[("countrycodes", cfg.country.trim())]);
    }
    let res = send(req).await?;
    if !res.status().is_success() {
        return Err(format!("HTTP {}", res.status()));
    }
    let v: Value = res.json().await.map_err(http_err)?;
    Ok(v.get(0).and_then(|f| {
        let lat = f["lat"].as_str()?.parse().ok()?;
        let lng = f["lon"].as_str()?.parse().ok()?;
        Some(LatLng { lat, lng })
    }))
}

// ---------------- Address autocomplete (Photon) ----------------

/// One autocomplete suggestion.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Place {
    pub label: String,
    pub lat: f64,
    pub lng: f64,
}

const PLACES_TTL: Duration = Duration::from_secs(24 * 3600);
const PLACES_CACHE_MAX: usize = 2000;
const PLACES_LIMIT: usize = 6;
/// Results lean towards Petaling Jaya, where the church is.
const BIAS: LatLng = LatLng {
    lat: 3.1073,
    lng: 101.6067,
};

/// A readable one-line address from a Photon feature's properties, e.g.
/// "Wisma Damai, 12 Jalan SS 2/24, SS 2, 47300 Petaling Jaya, Selangor".
pub fn place_label(p: &Value) -> Option<String> {
    let s = |k: &str| {
        p[k].as_str()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(String::from)
    };
    let street = match (s("housenumber"), s("street")) {
        (Some(n), Some(st)) => Some(format!("{n} {st}")),
        (None, st) => st,
        (Some(_), None) => None,
    };
    let town = s("city").or_else(|| s("county"));
    let town = match (s("postcode"), town) {
        (Some(pc), Some(t)) => Some(format!("{pc} {t}")),
        (pc, t) => t.or(pc),
    };
    let mut parts: Vec<String> = Vec::new();
    for part in [
        s("name"),
        street,
        s("locality").or_else(|| s("district")),
        town,
        s("state"),
    ]
    .into_iter()
    .flatten()
    {
        // Skip repeats such as a place named after its street or town.
        if !parts
            .iter()
            .any(|q| q.eq_ignore_ascii_case(&part) || q.ends_with(&format!(" {part}")))
        {
            parts.push(part);
        }
    }
    (parts.len() >= 2).then(|| parts.join(", "))
}

/// Suggestions from a Photon response, limited to `country` (ISO code; empty = any).
pub fn parse_photon(v: &Value, country: &str) -> Vec<Place> {
    let want: Vec<String> = country
        .split(',')
        .map(|c| c.trim().to_ascii_uppercase())
        .filter(|c| !c.is_empty())
        .collect();
    let mut out: Vec<Place> = Vec::new();
    for f in v["features"].as_array().into_iter().flatten() {
        let p = &f["properties"];
        let cc = p["countrycode"].as_str().unwrap_or("").to_ascii_uppercase();
        if !want.is_empty() && !want.contains(&cc) {
            continue;
        }
        let (Some(lng), Some(lat)) = (
            f["geometry"]["coordinates"][0].as_f64(),
            f["geometry"]["coordinates"][1].as_f64(),
        ) else {
            continue;
        };
        let Some(label) = place_label(p) else {
            continue;
        };
        if out.iter().any(|o| o.label == label) {
            continue;
        }
        out.push(Place {
            label,
            lat: round5(lat),
            lng: round5(lng),
        });
        if out.len() == PLACES_LIMIT {
            break;
        }
    }
    out
}

/// Places matching what someone has typed so far. Empty when routing is off, the query is
/// too short, or Photon is unavailable: the field then works as plain text.
pub async fn search_places(st: &AppState, q: &str, lang: &str) -> Vec<Place> {
    let cfg = &st.cfg.routing;
    let q = q.split_whitespace().collect::<Vec<_>>().join(" ");
    if !cfg.enabled || q.chars().count() < 3 || q.chars().count() > 120 {
        return Vec::new();
    }
    let key = format!("{lang}|{}", q.to_lowercase());
    {
        let cache = st.routing.places.lock().await;
        if let Some((at, hits)) = cache.get(&key) {
            if at.elapsed() < PLACES_TTL {
                return hits.clone();
            }
        }
    }
    let req = st
        .routing
        .http
        .get(format!("{}/api/", cfg.photon_url))
        .query(&[
            ("q", q.as_str()),
            ("limit", "15"),
            ("lat", &BIAS.lat.to_string()),
            ("lon", &BIAS.lng.to_string()),
            // Photon labels in en/de/fr/it; Chinese falls back to local names.
            ("lang", if lang == "en" { "en" } else { "default" }),
        ])
        .header(reqwest::header::USER_AGENT, &cfg.user_agent);
    let hits = match send(req).await {
        Ok(res) if res.status().is_success() => match res.json::<Value>().await {
            Ok(v) => parse_photon(&v, &cfg.country),
            Err(e) => {
                tracing::warn!("place search unreadable: {}", http_err(e));
                return Vec::new();
            }
        },
        Ok(res) => {
            tracing::warn!("place search unavailable: HTTP {}", res.status());
            return Vec::new();
        }
        Err(e) => {
            // Never log the query: it's often a member's home.
            tracing::warn!("place search unavailable: {e}");
            return Vec::new();
        }
    };
    // Picking a suggestion books the ride with this exact label, so let routing find it.
    for h in &hits {
        let _ = sqlx::query(
            "INSERT INTO geocode_cache (query, lat, lng, approx, updated_at) VALUES ($1,$2,$3,$4,$5)
             ON CONFLICT(query) DO NOTHING",
        )
        .bind(cache_key(&h.label))
        .bind(h.lat)
        .bind(h.lng)
        .bind(false)
        .bind(now())
        .execute(&st.db)
        .await;
    }
    let mut cache = st.routing.places.lock().await;
    if cache.len() >= PLACES_CACHE_MAX {
        cache.retain(|_, (at, _)| at.elapsed() < PLACES_TTL);
        if cache.len() >= PLACES_CACHE_MAX {
            cache.clear();
        }
    }
    cache.insert(key, (Instant::now(), hits.clone()));
    hits
}

// ---------------- Road routing (OSRM) ----------------

fn coords_param(points: &[LatLng]) -> String {
    points
        .iter()
        .map(|p| format!("{:.6},{:.6}", p.lng, p.lat))
        .collect::<Vec<_>>()
        .join(";")
}

async fn osrm_get(st: &AppState, path: &str) -> Result<Value, String> {
    let cfg = &st.cfg.routing;
    let res = send(
        st.routing
            .http
            .get(format!("{}{}", cfg.osrm_url, path))
            .header(reqwest::header::USER_AGENT, &cfg.user_agent),
    )
    .await?;
    if !res.status().is_success() {
        return Err(format!("HTTP {}", res.status()));
    }
    let v: Value = res.json().await.map_err(http_err)?;
    if v["code"] != "Ok" {
        return Err(format!("OSRM {}", v["code"]));
    }
    Ok(v)
}

/// Road travel times in seconds between every pair of points.
async fn osrm_table(st: &AppState, points: &[LatLng]) -> Result<Vec<Vec<f64>>, String> {
    let v = osrm_get(
        st,
        &format!(
            "/table/v1/driving/{}?annotations=duration",
            coords_param(points)
        ),
    )
    .await?;
    let rows = v["durations"].as_array().ok_or("no durations")?;
    let m: Vec<Vec<f64>> = rows
        .iter()
        .map(|r| {
            r.as_array()
                .map(|c| {
                    c.iter()
                        .map(|x| x.as_f64().unwrap_or(f64::INFINITY))
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect();
    if m.len() != points.len() || m.iter().any(|r| r.len() != points.len()) {
        return Err("matrix size mismatch".into());
    }
    Ok(m)
}

/// Legs (metres, seconds) between consecutive points, the route line as [lat, lng], and
/// the index on that line where each point is reached.
struct RoadRoute {
    legs: Vec<(f64, f64)>,
    geometry: Vec<[f64; 2]>,
    splits: Vec<usize>,
}

async fn osrm_route(st: &AppState, points: &[LatLng]) -> Result<RoadRoute, String> {
    let v = osrm_get(
        st,
        &format!(
            "/route/v1/driving/{}?overview=full&geometries=geojson&steps=false",
            coords_param(points)
        ),
    )
    .await?;
    let route = &v["routes"][0];
    let legs: Vec<(f64, f64)> = route["legs"]
        .as_array()
        .ok_or("no legs")?
        .iter()
        .map(|l| {
            (
                l["distance"].as_f64().unwrap_or(0.0),
                l["duration"].as_f64().unwrap_or(0.0),
            )
        })
        .collect();
    if legs.len() + 1 != points.len() {
        return Err("leg count mismatch".into());
    }
    let geometry: Vec<[f64; 2]> = route["geometry"]["coordinates"]
        .as_array()
        .ok_or("no geometry")?
        .iter()
        .filter_map(|c| Some([round5(c[1].as_f64()?), round5(c[0].as_f64()?)]))
        .collect();
    let snapped: Vec<LatLng> = v["waypoints"]
        .as_array()
        .filter(|w| w.len() == points.len())
        .map(|w| {
            w.iter()
                .zip(points)
                .map(|(w, p)| LatLng {
                    lat: w["location"][1].as_f64().unwrap_or(p.lat),
                    lng: w["location"][0].as_f64().unwrap_or(p.lng),
                })
                .collect()
        })
        .unwrap_or_else(|| points.to_vec());
    let splits = split_points(&geometry, &snapped);
    Ok(RoadRoute {
        legs,
        geometry,
        splits,
    })
}

fn straight_route(points: &[LatLng]) -> RoadRoute {
    RoadRoute {
        legs: points.windows(2).map(|w| estimate(w[0], w[1])).collect(),
        geometry: points
            .iter()
            .map(|p| [round5(p.lat), round5(p.lng)])
            .collect(),
        splits: (0..points.len()).collect(),
    }
}

// ---------------- Trip route ----------------

#[derive(Clone, Debug, Serialize)]
pub struct Origin {
    pub lat: f64,
    pub lng: f64,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Stop {
    pub ride_id: i64,
    pub passenger: String,
    pub address: String,
    pub pickup_at: String,
    pub seats: i64,
    /// The ride is in progress (the passenger knows the driver is on the way).
    pub started: bool,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    /// "live" = the passenger's shared GPS; "address" = the geocoded pickup address;
    /// "approx_address" = only part of the address matched; null = couldn't place it.
    pub location: Option<&'static str>,
    pub leg_distance_m: Option<f64>,
    pub leg_duration_s: Option<f64>,
    /// From the driver (or the first stop when the driver's position is unknown).
    pub distance_m: Option<f64>,
    pub duration_s: Option<f64>,
    pub eta: Option<String>,
    #[serde(skip)]
    path_end: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Destination {
    pub name: String,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub approx: bool,
    pub distance_m: Option<f64>,
    pub duration_s: Option<f64>,
    pub eta: Option<String>,
    #[serde(skip)]
    path_end: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TripRoute {
    pub trip_id: i64,
    /// A ride in this trip is in progress.
    pub live: bool,
    /// The driver's last shared position (only while live).
    pub origin: Option<Origin>,
    /// Pickups still to make: located ones in the best order, then any that couldn't be placed.
    pub stops: Vec<Stop>,
    /// Passengers already in the car.
    pub onboard: i64,
    pub destination: Destination,
    pub distance_m: f64,
    pub duration_s: f64,
    /// Straight-line estimates (routing off or the routing service unreachable).
    pub approximate: bool,
    pub computed_at: String,
    /// Route line as [lat, lng] pairs.
    pub geometry: Vec<[f64; 2]>,
}

#[derive(sqlx::FromRow, Clone)]
struct TripRide {
    id: i64,
    p_name: String,
    pickup_address: String,
    destination: String,
    status: String,
    picked_up_at: Option<String>,
    pickup_at: String,
    seats: i64,
    plat: Option<f64>,
    plng: Option<f64>,
}

fn minutes_from_now(m: i64) -> String {
    (Local::now() + chrono::Duration::minutes(m))
        .format("%Y-%m-%dT%H:%M")
        .to_string()
}

/// The trip under way, else the next confirmed trip in the coming hours.
async fn current_trip(st: &AppState, driver: i64) -> AppResult<Option<i64>> {
    let live: Option<(i64,)> = sqlx::query_as(
        "SELECT COALESCE(trip_id, id) FROM rides WHERE driver_id=$1 AND status='in_progress' ORDER BY pickup_at, id LIMIT 1",
    )
    .bind(driver)
    .fetch_optional(&st.db)
    .await?;
    if let Some((t,)) = live {
        return Ok(Some(t));
    }
    let next: Option<(i64,)> = sqlx::query_as(
        "SELECT COALESCE(trip_id, id) FROM rides WHERE driver_id=$1 AND status='confirmed' AND pickup_at BETWEEN $2 AND $3
         ORDER BY pickup_at, id LIMIT 1",
    )
    .bind(driver)
    .bind(minutes_from_now(-180))
    .bind(minutes_from_now(18 * 60))
    .fetch_optional(&st.db)
    .await?;
    Ok(next.map(|t| t.0))
}

/// Route for a driver's trip (`trip` = None ⇒ the current or next one). `None` when the
/// driver has no confirmed or in-progress rides in it.
pub async fn driver_route(
    st: &AppState,
    driver: i64,
    trip: Option<i64>,
) -> AppResult<Option<TripRoute>> {
    let trip = match trip {
        Some(t) => t,
        None => match current_trip(st, driver).await? {
            Some(t) => t,
            None => return Ok(None),
        },
    };
    let rows: Vec<TripRide> = sqlx::query_as(
        "SELECT r.id, p.name AS p_name, r.pickup_address, r.destination, r.status, r.picked_up_at, r.pickup_at, r.seats,
                pl.lat AS plat, pl.lng AS plng
         FROM rides r JOIN users p ON p.id = r.passenger_id
         LEFT JOIN passenger_locations pl ON pl.ride_id = r.id
         WHERE r.driver_id=$1 AND COALESCE(r.trip_id, r.id)=$2 AND r.status IN ('confirmed','in_progress')
         ORDER BY r.pickup_at, r.id",
    )
    .bind(driver)
    .bind(trip)
    .fetch_all(&st.db)
    .await?;
    if rows.is_empty() {
        return Ok(None);
    }
    let live = rows.iter().any(|r| r.status == "in_progress");
    let origin = if live {
        let loc: Option<(f64, f64, String)> =
            sqlx::query_as("SELECT lat, lng, updated_at FROM driver_locations WHERE driver_id=$1")
                .bind(driver)
                .fetch_optional(&st.db)
                .await?;
        loc.map(|(lat, lng, updated_at)| Origin {
            lat,
            lng,
            updated_at,
        })
    } else {
        None
    };

    // Inputs that change the stops; the driver's own movement is covered by the TTL.
    let key = format!(
        "{}|{}|{}",
        origin.is_some(),
        rows[0].destination,
        rows.iter()
            .map(|r| format!(
                "{}:{}:{}:{}:{}",
                r.id,
                r.status,
                r.picked_up_at.is_some(),
                r.plat.is_some(),
                r.pickup_address
            ))
            .collect::<Vec<_>>()
            .join(",")
    );
    let ttl = if origin.is_some() { LIVE_TTL } else { PLAN_TTL };
    let slot = (driver, trip);
    let cached = || async {
        let cache = st.routing.routes.lock().await;
        match cache.get(&slot) {
            Some(c) if c.key == key && c.at.elapsed() < ttl => Ok(c.route.clone()),
            Some(c) => Err(Some(c.order.clone())),
            None => Err(None),
        }
    };
    if let Ok(route) = cached().await {
        return Ok(Some(route));
    }
    let gate = {
        let mut inflight = st.routing.inflight.lock().await;
        inflight.retain(|_, g| Arc::strong_count(g) > 1);
        inflight.entry(slot).or_default().clone()
    };
    let _turn = gate.lock().await;
    // Someone else may have just computed it while we waited.
    let previous = match cached().await {
        Ok(route) => return Ok(Some(route)),
        Err(order) => order,
    };

    let (route, order) = compute(st, trip, live, origin, &rows, previous.as_deref()).await;
    let mut cache = st.routing.routes.lock().await;
    cache.retain(|_, c| c.at.elapsed() < Duration::from_secs(3600));
    cache.insert(
        slot,
        Cached {
            key,
            at: Instant::now(),
            route: route.clone(),
            order,
        },
    );
    Ok(Some(route))
}

async fn compute(
    st: &AppState,
    trip: i64,
    live: bool,
    origin: Option<Origin>,
    rows: &[TripRide],
    previous: Option<&[i64]>,
) -> (TripRoute, Vec<i64>) {
    let enabled = st.cfg.routing.enabled;
    let onboard = rows.iter().filter(|r| r.picked_up_at.is_some()).count() as i64;

    // Place each pickup still to make: shared GPS first, else the geocoded address.
    let mut stops = Vec::new();
    for r in rows.iter().filter(|r| r.picked_up_at.is_none()) {
        let (pt, location) = match (r.plat, r.plng) {
            (Some(lat), Some(lng)) => (Some(LatLng { lat, lng }), Some("live")),
            _ => match geocode(st, &r.pickup_address).await {
                Some((p, approx)) => (
                    Some(p),
                    Some(if approx { "approx_address" } else { "address" }),
                ),
                None => (None, None),
            },
        };
        stops.push(Stop {
            ride_id: r.id,
            passenger: r.p_name.clone(),
            address: r.pickup_address.clone(),
            pickup_at: r.pickup_at.clone(),
            seats: r.seats,
            started: r.status == "in_progress",
            lat: pt.map(|p| p.lat),
            lng: pt.map(|p| p.lng),
            location,
            leg_distance_m: None,
            leg_duration_s: None,
            distance_m: None,
            duration_s: None,
            eta: None,
            path_end: None,
        });
    }
    let dest_name = rows[0].destination.clone();
    let dest_pt = geocode(st, &dest_name).await;

    // Matrix points: [driver?] + located pickups + [destination?].
    let (mut located, unlocated): (Vec<Stop>, Vec<Stop>) =
        stops.into_iter().partition(|s| s.lat.is_some());
    let point = |s: &Stop| LatLng {
        lat: s.lat.unwrap_or_default(),
        lng: s.lng.unwrap_or_default(),
    };
    let mut points = Vec::new();
    let start = origin.as_ref().map(|o| {
        points.push(LatLng {
            lat: o.lat,
            lng: o.lng,
        });
        0
    });
    let offset = points.len();
    points.extend(located.iter().map(point));
    let stop_idx: Vec<usize> = (offset..offset + located.len()).collect();
    let end = dest_pt.map(|(p, _)| {
        points.push(p);
        points.len() - 1
    });

    if stop_idx.len() >= 2 {
        let m = match enabled {
            true => osrm_table(st, &points)
                .await
                .map_err(|e| tracing::warn!("routing matrix unavailable: {e}"))
                .ok(),
            false => None,
        }
        .unwrap_or_else(|| straight_matrix(&points));
        let prev_idx: Option<Vec<usize>> = previous.map(|ids| {
            ids.iter()
                .filter_map(|id| located.iter().position(|s| s.ride_id == *id))
                .map(|k| k + offset)
                .collect()
        });
        let order = choose_order(&m, start, &stop_idx, end, prev_idx.as_deref());
        let mut slots: Vec<Option<Stop>> = located.into_iter().map(Some).collect();
        located = order
            .iter()
            .filter_map(|&p| slots[p - offset].take())
            .collect();
    }

    // Road route through the chosen order.
    let mut route_points = Vec::new();
    if let Some(o) = &origin {
        route_points.push(LatLng {
            lat: o.lat,
            lng: o.lng,
        });
    }
    route_points.extend(located.iter().map(point));
    if let Some((p, _)) = dest_pt {
        route_points.push(p);
    }
    let mut approximate = false;
    let road = if route_points.len() >= 2 {
        let road = match enabled {
            true => osrm_route(st, &route_points)
                .await
                .map_err(|e| tracing::warn!("routing unavailable: {e}"))
                .ok(),
            false => None,
        };
        road.unwrap_or_else(|| {
            approximate = true;
            straight_route(&route_points)
        })
    } else {
        straight_route(&route_points)
    };

    // Walk the legs: cumulative distance/time and ETAs (only when the driver's position is known).
    let started_at = Local::now();
    let eta = |secs: f64| {
        origin.as_ref().map(|_| {
            (started_at + chrono::Duration::seconds(secs.round() as i64))
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string()
        })
    };
    let (mut cum_d, mut cum_t) = (0.0_f64, 0.0_f64);
    let mut pi = usize::from(origin.is_some()); // index into route_points
    for s in located.iter_mut() {
        if pi > 0 {
            let (d, t) = road.legs[pi - 1];
            cum_d += d;
            cum_t += t;
            s.leg_distance_m = Some(d.round());
            s.leg_duration_s = Some(t.round());
        }
        s.distance_m = Some(cum_d.round());
        s.duration_s = Some(cum_t.round());
        s.eta = eta(cum_t);
        s.path_end = road.splits.get(pi).copied();
        cum_t += PICKUP_DWELL_S;
        pi += 1;
    }
    let mut destination = Destination {
        name: dest_name,
        lat: dest_pt.map(|(p, _)| p.lat),
        lng: dest_pt.map(|(p, _)| p.lng),
        approx: dest_pt.is_some_and(|(_, a)| a),
        distance_m: None,
        duration_s: None,
        eta: None,
        path_end: None,
    };
    if dest_pt.is_some() && pi > 0 {
        let (d, t) = road.legs[pi - 1];
        cum_d += d;
        cum_t += t;
        destination.distance_m = Some(cum_d.round());
        destination.duration_s = Some(cum_t.round());
        destination.eta = eta(cum_t);
        destination.path_end = road.splits.get(pi).copied();
    } else if !located.is_empty() {
        // No destination point: the route ends at the last pickup (drop its dwell time).
        cum_t -= PICKUP_DWELL_S;
    }

    let order = located.iter().map(|s| s.ride_id).collect();
    let mut all = located;
    all.extend(unlocated);
    let route = TripRoute {
        trip_id: trip,
        live,
        origin,
        stops: all,
        onboard,
        destination,
        distance_m: cum_d.round(),
        duration_s: cum_t.max(0.0).round(),
        approximate,
        computed_at: now(),
        geometry: if route_points.len() >= 2 {
            road.geometry
        } else {
            Vec::new()
        },
    };
    (route, order)
}

/// What one ride's viewer may see of the trip route: distance and ETA to their pickup
/// (or, once aboard, to the destination) and how many stops come first. Never other
/// passengers' positions: the route line is included only for the driver and staff
/// (`full_view`), or when it leads straight to this passenger with no one else's pickup on it.
/// `None` until the driver's position is known.
pub fn ride_eta(
    route: &TripRoute,
    ride_id: i64,
    picked_up: bool,
    full_view: bool,
) -> Option<Value> {
    route.origin.as_ref()?;
    let path = |end: Option<usize>, stops_before: usize| -> Option<Vec<[f64; 2]>> {
        let end = end?;
        (full_view || stops_before == 0)
            .then(|| route.geometry.get(..=end).map(<[_]>::to_vec))
            .flatten()
    };
    let base = |stops_before: usize| {
        json!({
            "stops_before": stops_before,
            "approximate": route.approximate,
            "computed_at": route.computed_at,
        })
    };
    if !picked_up {
        let k = route.stops.iter().position(|s| s.ride_id == ride_id)?;
        let s = &route.stops[k];
        let mut out = base(k);
        out["phase"] = json!("pickup");
        if s.lat.is_none() {
            out["located"] = json!(false);
            return Some(out);
        }
        out["located"] = json!(true);
        out["distance_m"] = json!(s.distance_m);
        out["duration_s"] = json!(s.duration_s);
        out["eta"] = json!(s.eta);
        out["location"] = json!(s.location);
        out["target"] = json!([s.lat, s.lng]);
        if let Some(p) = path(s.path_end, k) {
            out["path"] = json!(p);
        }
        Some(out)
    } else {
        let d = &route.destination;
        let before = route.stops.len();
        let mut out = base(before);
        out["phase"] = json!("dropoff");
        out["destination"] = json!(d.name);
        if d.lat.is_none() {
            out["located"] = json!(false);
            return Some(out);
        }
        out["located"] = json!(true);
        out["distance_m"] = json!(d.distance_m);
        out["duration_s"] = json!(d.duration_s);
        out["eta"] = json!(d.eta);
        out["target"] = json!([d.lat, d.lng]);
        if let Some(p) = path(d.path_end, before) {
            out["path"] = json!(p);
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn place_label_builds_a_malaysian_address() {
        let p = json!({
            "name": "Wisma Damai", "housenumber": "12", "street": "Jalan SS 2/24",
            "district": "SS 2", "postcode": "47300", "city": "Petaling Jaya", "state": "Selangor"
        });
        assert_eq!(
            super::place_label(&p).as_deref(),
            Some("Wisma Damai, 12 Jalan SS 2/24, SS 2, 47300 Petaling Jaya, Selangor")
        );
        // A town on its own is too vague to be a pickup point.
        assert_eq!(
            super::place_label(&json!({ "name": "Petaling Jaya" })),
            None
        );
        // No repeats when a place is named after its street.
        let p =
            json!({ "name": "Jalan Gasing", "street": "Jalan Gasing", "city": "Petaling Jaya" });
        assert_eq!(
            super::place_label(&p).as_deref(),
            Some("Jalan Gasing, Petaling Jaya")
        );
    }

    #[test]
    fn parse_photon_keeps_country_dedupes_and_limits() {
        let f = |name: &str, cc: &str, lng: f64| {
            json!({
                "geometry": { "coordinates": [lng, 3.1] },
                "properties": { "name": name, "city": "Petaling Jaya", "countrycode": cc }
            })
        };
        let mut feats = vec![
            f("A", "MY", 101.6),
            f("A", "MY", 101.7),
            f("B", "SG", 103.8),
        ];
        for i in 0..10 {
            feats.push(f(&format!("P{i}"), "MY", 101.6));
        }
        let v = json!({ "features": feats });
        let got = super::parse_photon(&v, "my");
        assert_eq!(got.len(), super::PLACES_LIMIT);
        assert_eq!(got[0].label, "A, Petaling Jaya");
        assert!(got.iter().all(|p| !p.label.starts_with("B,")));
        assert_eq!(
            super::parse_photon(&v, "")
                .iter()
                .filter(|p| p.label.starts_with("B,"))
                .count(),
            1
        );
    }

    use super::*;

    fn line_matrix(xs: &[f64]) -> Vec<Vec<f64>> {
        xs.iter()
            .map(|a| xs.iter().map(|b| (a - b).abs()).collect())
            .collect()
    }

    #[test]
    fn haversine_is_sane() {
        let pj = LatLng {
            lat: 3.1073,
            lng: 101.6067,
        };
        let kl = LatLng {
            lat: 3.1390,
            lng: 101.6869,
        };
        let d = haversine_m(pj, kl);
        assert!((9_000.0..10_000.0).contains(&d), "{d}");
        assert_eq!(haversine_m(pj, pj), 0.0);
    }

    #[test]
    fn exact_order_visits_stops_along_the_way() {
        // Driver at 0, stops at 7, 2, 5, destination at 10: best is 2 → 5 → 7.
        let m = line_matrix(&[0.0, 7.0, 2.0, 5.0, 10.0]);
        assert_eq!(best_order(&m, Some(0), &[1, 2, 3], Some(4)), vec![2, 3, 1]);
        // Destination behind the driver: collect the far stop first, then come back.
        let m = line_matrix(&[5.0, 9.0, 7.0, 0.0]);
        assert_eq!(best_order(&m, Some(0), &[1, 2], Some(3)), vec![1, 2]);
    }

    #[test]
    fn order_without_driver_position_or_destination() {
        let m = line_matrix(&[4.0, 1.0, 9.0, 6.0]);
        let order = best_order(&m, None, &[0, 1, 2, 3], None);
        let cost = path_cost(&m, None, &order, None);
        assert_eq!(cost, 8.0, "{order:?}"); // a straight sweep 1 → 4 → 6 → 9 (or back)
    }

    #[test]
    fn heuristic_handles_large_trips() {
        let xs: Vec<f64> = vec![
            0.0, 13.0, 3.0, 11.0, 5.0, 9.0, 1.0, 7.0, 12.0, 2.0, 8.0, 20.0,
        ];
        let m = line_matrix(&xs);
        let stops: Vec<usize> = (1..11).collect();
        let order = best_order(&m, Some(0), &stops, Some(11));
        let mut sorted = order.clone();
        sorted.sort();
        assert_eq!(sorted, stops, "every stop exactly once");
        assert_eq!(path_cost(&m, Some(0), &order, Some(11)), 20.0);
    }

    #[test]
    fn keeps_previous_order_unless_clearly_better() {
        // Costs in seconds: the previous order is 30 s worse than the best → kept.
        let m = line_matrix(&[0.0, 100.0, 115.0, 200.0]);
        assert_eq!(
            choose_order(&m, Some(0), &[1, 2], Some(3), Some(&[2, 1])),
            vec![2, 1]
        );
        // 400 s worse → replaced.
        let m = line_matrix(&[0.0, 100.0, 300.0, 400.0]);
        assert_eq!(
            choose_order(&m, Some(0), &[1, 2], Some(3), Some(&[2, 1])),
            vec![1, 2]
        );
        // A different set of stops ignores the previous order.
        assert_eq!(
            choose_order(&m, Some(0), &[1, 2], Some(3), Some(&[2])),
            vec![1, 2]
        );
    }

    #[test]
    fn address_variants_drop_leading_parts() {
        assert_eq!(
            address_variants("No 12,  Jalan SS 2/24, 47300 Petaling Jaya, Selangor"),
            vec![
                "No 12, Jalan SS 2/24, 47300 Petaling Jaya, Selangor",
                "Jalan SS 2/24, 47300 Petaling Jaya, Selangor",
                "47300 Petaling Jaya, Selangor",
            ]
        );
        assert_eq!(address_variants("PJ State CMC"), vec!["PJ State CMC"]);
        assert!(address_variants(" , ").is_empty());
        assert_eq!(cache_key("  PJ   State CMC "), "pj state cmc");
    }

    #[test]
    fn split_points_follow_the_line() {
        let line = [[0.0, 0.0], [0.0, 1.0], [0.0, 2.0], [0.0, 3.0], [0.0, 4.0]];
        let w = |lng| LatLng { lat: 0.0, lng };
        assert_eq!(
            split_points(&line, &[w(0.0), w(2.1), w(2.9), w(4.0)]),
            vec![0, 2, 3, 4]
        );
        assert_eq!(split_points(&[], &[w(0.0), w(1.0)]), vec![0, 0]);
    }

    fn sample_route() -> TripRoute {
        let stop = |id, k: usize, lat: Option<f64>| Stop {
            ride_id: id,
            passenger: format!("P{id}"),
            address: format!("A{id}"),
            pickup_at: "2026-11-12T09:00".into(),
            seats: 1,
            started: true,
            lat,
            lng: lat.map(|_| 101.6),
            location: lat.map(|_| "address"),
            leg_distance_m: Some(1000.0),
            leg_duration_s: Some(120.0),
            distance_m: Some(1000.0 * (k + 1) as f64),
            duration_s: Some(180.0 * (k + 1) as f64),
            eta: Some("2026-11-12T09:05:00".into()),
            path_end: lat.map(|_| k + 1),
        };
        TripRoute {
            trip_id: 1,
            live: true,
            origin: Some(Origin {
                lat: 3.0,
                lng: 101.6,
                updated_at: "x".into(),
            }),
            stops: vec![
                stop(10, 0, Some(3.01)),
                stop(11, 1, Some(3.02)),
                stop(12, 2, None),
            ],
            onboard: 1,
            destination: Destination {
                name: "Church".into(),
                lat: Some(3.05),
                lng: Some(101.6),
                approx: false,
                distance_m: Some(5000.0),
                duration_s: Some(600.0),
                eta: Some("2026-11-12T09:12:00".into()),
                path_end: Some(3),
            },
            distance_m: 5000.0,
            duration_s: 600.0,
            approximate: false,
            computed_at: "now".into(),
            geometry: vec![[3.0, 101.6], [3.01, 101.6], [3.02, 101.6], [3.05, 101.6]],
        }
    }

    #[test]
    fn rider_eta_hides_other_pickups() {
        let r = sample_route();
        let first = ride_eta(&r, 10, false, false).unwrap();
        assert_eq!(first["stops_before"], 0);
        assert_eq!(first["path"].as_array().unwrap().len(), 2);
        let second = ride_eta(&r, 11, false, false).unwrap();
        assert_eq!(second["stops_before"], 1);
        assert_eq!(second["distance_m"], 2000.0);
        assert!(
            second.get("path").is_none(),
            "would reveal the first pickup"
        );
        assert!(!second.to_string().contains("3.01"));
        // The driver sees the line to any stop.
        assert_eq!(
            ride_eta(&r, 11, false, true).unwrap()["path"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        // A stop the geocoder couldn't place.
        let lost = ride_eta(&r, 12, false, false).unwrap();
        assert_eq!(lost["located"], false);
        assert!(lost.get("eta").is_none());
        // Aboard: ETA to the destination, behind the remaining pickups.
        let aboard = ride_eta(&r, 99, true, false).unwrap();
        assert_eq!(aboard["phase"], "dropoff");
        assert_eq!(aboard["stops_before"], 3);
        assert!(aboard.get("path").is_none());
        // Unknown driver position ⇒ nothing.
        let mut no_origin = sample_route();
        no_origin.origin = None;
        assert!(ride_eta(&no_origin, 10, false, false).is_none());
    }
}
