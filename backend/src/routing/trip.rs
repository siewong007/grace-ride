//! Road routing via OSRM, driver trip route computation, and per-ride ETA projection.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use serde::Serialize;
use serde_json::{json, Value};

use super::geocode::{geocode, http_err, send};
use super::order::choose_order;
use super::{
    estimate, round5, split_points, straight_matrix, Cached, LatLng, LIVE_TTL, PICKUP_DWELL_S,
    PLAN_TTL,
};
use crate::error::AppResult;
use crate::state::AppState;
use crate::util::now;

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
    pub(super) path_end: Option<usize>,
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
    pub(super) path_end: Option<usize>,
    #[serde(skip)]
    pub shared: bool,
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
    (crate::util::local_now() + chrono::Duration::minutes(m))
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
        "SELECT r.id,
                CASE WHEN r.is_proxy = 1 AND r.rider_name <> '' THEN r.rider_name ELSE p.name END AS p_name,
                r.pickup_address, r.destination, r.status, r.picked_up_at, r.pickup_at, r.seats,
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
                "{}:{}:{}:{}:{}:{}",
                r.id,
                r.status,
                r.picked_up_at.is_some(),
                r.plat.is_some(),
                r.pickup_address,
                r.destination
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
    let shared_dest = rows.iter().all(|r| r.destination == rows[0].destination);
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
    let started_at = crate::util::local_now();
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
        shared: shared_dest,
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
        if !d.shared && !full_view {
            return Some(json!({
                "phase": "dropoff",
                "located": false,
                "stops_before": before,
                "approximate": route.approximate,
                "computed_at": route.computed_at,
            }));
        }
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
