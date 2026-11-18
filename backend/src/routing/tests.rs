use serde_json::json;

use super::geocode::PLACES_LIMIT;
use super::*;

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
    let p = json!({ "name": "Jalan Gasing", "street": "Jalan Gasing", "city": "Petaling Jaya" });
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
    assert_eq!(got.len(), PLACES_LIMIT);
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
            shared: true,
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

#[test]
fn rider_eta_hides_unshared_return_destination() {
    let mut r = sample_route();
    r.stops.clear();
    r.destination.name = "12 Jalan Secret Home".into();
    r.destination.shared = false;

    let hidden = ride_eta(&r, 99, true, false).unwrap();
    assert_eq!(hidden["phase"], "dropoff");
    assert_eq!(hidden["located"], false);
    assert!(hidden.get("destination").is_none());
    assert!(hidden.get("target").is_none());
    assert!(hidden.get("eta").is_none());
    assert!(hidden.get("path").is_none());

    let driver = ride_eta(&r, 99, true, true).unwrap();
    assert_eq!(driver["located"], true);
    assert_eq!(driver["destination"], "12 Jalan Secret Home");
    assert!(driver.get("target").is_some());
    assert!(driver.get("path").is_some());
}
