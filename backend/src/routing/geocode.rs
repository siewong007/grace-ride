//! Geocoding (Nominatim + DB cache) and address autocomplete (Photon).

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use serde::Serialize;
use serde_json::Value;

use super::{round5, LatLng};
use crate::state::AppState;
use crate::util::now;

pub(crate) fn cache_key(addr: &str) -> String {
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
    (crate::util::local_now() - chrono::Duration::from_std(d).unwrap_or_default())
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string()
}

/// Error text with its causes, without the request URL (it carries addresses).
pub(super) fn http_err(e: reqwest::Error) -> String {
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
pub(super) async fn send(req: reqwest::RequestBuilder) -> Result<reqwest::Response, String> {
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
    let bytes = crate::util::read_response_bytes_bounded(res, 512 * 1024)
        .await
        .map_err(|e| format!("geocode read failed: {e}"))?;
    let v: Value = serde_json::from_slice(&bytes).map_err(|e| format!("geocode json: {e}"))?;
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
pub(super) const PLACES_LIMIT: usize = 6;
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
        Ok(res) if res.status().is_success() => {
            match crate::util::read_response_bytes_bounded(res, 512 * 1024).await {
                Ok(bytes) => match serde_json::from_slice::<Value>(&bytes) {
                    Ok(v) => parse_photon(&v, &cfg.country),
                    Err(e) => {
                        tracing::warn!("place search unreadable: {e}");
                        return Vec::new();
                    }
                },
                Err(e) => {
                    tracing::warn!("place search unreadable: {e}");
                    return Vec::new();
                }
            }
        }
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
