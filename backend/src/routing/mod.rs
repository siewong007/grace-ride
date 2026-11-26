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

mod geocode;
mod order;
#[cfg(test)]
mod tests;
mod trip;

pub(crate) use geocode::cache_key;
pub use geocode::{
    address_variants, cached_points, geocode, parse_photon, place_label, search_places,
    warm_geocodes, Place,
};
pub use order::{best_order, choose_order, path_cost};
pub use trip::{driver_route, ride_eta, Destination, Origin, Stop, TripRoute};

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::Mutex;

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
    pub(super) routes: Mutex<HashMap<(i64, i64), Cached>>,
    /// One computation at a time per trip, so the driver and every passenger refreshing
    /// together cost one set of routing calls.
    pub(super) inflight: Mutex<HashMap<(i64, i64), Arc<Mutex<()>>>>,
    geocoder_gate: Mutex<Option<Instant>>,
    /// Recent autocomplete answers, so retyping or several members typing the same
    /// thing don't each cost a Photon request.
    pub(super) places: Mutex<HashMap<String, (Instant, Vec<Place>)>>,
    pub(super) http: reqwest::Client,
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
                .redirect(reqwest::redirect::Policy::none())
                .pool_idle_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(8))
                .build()
                .unwrap_or_default(),
        }
    }
}

pub(super) struct Cached {
    pub(super) key: String,
    pub(super) at: Instant,
    pub(super) route: TripRoute,
    /// Ride ids of located stops in visiting order, so the order stays stable.
    pub(super) order: Vec<i64>,
}

impl RoutingState {
    pub async fn invalidate(&self, driver: i64, trip: i64) {
        self.routes.lock().await.remove(&(driver, trip));
    }

    /// Nominatim allows at most one request per second.
    pub(super) async fn wait_geocoder_slot(&self) {
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
pub(super) const LIVE_TTL: Duration = Duration::from_secs(30);
pub(super) const PLAN_TTL: Duration = Duration::from_secs(300);
/// Boarding time added at each pickup before the car moves on.
pub(super) const PICKUP_DWELL_S: f64 = 60.0;

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

pub(super) fn estimate(a: LatLng, b: LatLng) -> (f64, f64) {
    let d = haversine_m(a, b) * ROAD_FACTOR;
    (d, d / CITY_SPEED_MS)
}

pub(super) fn straight_matrix(points: &[LatLng]) -> Vec<Vec<f64>> {
    points
        .iter()
        .map(|&a| points.iter().map(|&b| estimate(a, b).1).collect())
        .collect()
}

pub(super) fn round5(x: f64) -> f64 {
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
