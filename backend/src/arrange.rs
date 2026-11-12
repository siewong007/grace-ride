//! Auto-arrange for the transport planning board: put passengers who have no driver yet
//! into cars, without touching anyone the board already assigned.
//!
//! Hard rules: a car never takes more seats than it has left, a driver who doesn't carpool
//! takes one booking per trip, and nobody drives themselves. Within those rules each
//! passenger goes where they cost least:
//! - **distance** to the nearest passenger already in that car (straight line, from cached
//!   coordinates; without coordinates, sharing a pickup area counts as near);
//! - a strong **penalty** when the driver doesn't serve the passenger's area, and a small
//!   one per extra area a driver covers, so specialists take their area first and drivers
//!   who cover several areas stay free for the others;
//! - a **bonus** for their previous driver, growing with how often they've ridden together.
//!
//! Hardest-to-place passengers go first (assistance needs, then fewest possible cars,
//! then the most seats), then a few passes move single passengers to a cheaper car.
//! Small enough for a board of a few dozen people to run instantly.

use std::collections::HashMap;

use crate::routing::{haversine_m, LatLng};

/// Distance cost unit: kilometres. Penalties and bonuses are in the same unit.
const AREA_PENALTY_KM: f64 = 8.0;
const PER_EXTRA_AREA_KM: f64 = 1.0;
const SAME_AREA_KM: f64 = 1.0;
const OTHER_AREA_KM: f64 = 6.0;
/// Starting an empty car costs a little, so nearby passengers share rather than spread out.
const EMPTY_CAR_KM: f64 = 2.0;
const LAST_DRIVER_KM: f64 = 2.0;
const PER_RIDE_TOGETHER_KM: f64 = 0.5;
const MAX_TOGETHER: i64 = 5;
const IMPROVE_PASSES: usize = 3;
const MIN_GAIN_KM: f64 = 0.3;

#[derive(Clone, Debug)]
pub struct Rider {
    pub ride_id: i64,
    pub passenger_id: i64,
    pub seats: i64,
    pub area_id: i64,
    pub point: Option<LatLng>,
    pub needs_assistance: bool,
    /// Previous driver and how many rides they've shared.
    pub last: Option<(i64, i64)>,
}

#[derive(Clone, Debug)]
pub struct Car {
    pub driver_id: i64,
    /// Passenger seats free for this event (capacity minus seats held elsewhere then).
    pub seats_left: i64,
    pub carpool: bool,
    /// Bookings the driver already has elsewhere around the same time.
    pub other_rides: i64,
    pub area_ids: Vec<i64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    pub ride_id: i64,
    pub driver_id: i64,
    /// Closest passenger sharing the car, and how far in metres (None: no coordinates).
    pub near: Option<(i64, Option<f64>)>,
    pub in_area: bool,
    pub together: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Arrangement {
    pub placed: Vec<Placement>,
    /// Passengers no car had room for.
    pub unplaced: Vec<i64>,
}

/// `fixed`: ride id → driver id for passengers the board already assigned; they stay put.
pub fn arrange(riders: &[Rider], cars: &[Car], fixed: &HashMap<i64, i64>) -> Arrangement {
    let car_idx: HashMap<i64, usize> = cars
        .iter()
        .enumerate()
        .map(|(i, c)| (c.driver_id, i))
        .collect();
    let mut groups: Vec<Vec<usize>> = vec![Vec::new(); cars.len()];
    let mut open: Vec<usize> = Vec::new();
    for (i, r) in riders.iter().enumerate() {
        match fixed.get(&r.ride_id).and_then(|d| car_idx.get(d)) {
            Some(&c) => groups[c].push(i),
            None => open.push(i),
        }
    }

    let fits = |groups: &[Vec<usize>], c: usize, i: usize| -> bool {
        let car = &cars[c];
        let r = &riders[i];
        if r.passenger_id == car.driver_id {
            return false;
        }
        let g = &groups[c];
        let used: i64 = g.iter().map(|&j| riders[j].seats).sum();
        if used + r.seats > car.seats_left {
            return false;
        }
        car.carpool || car.other_rides + g.len() as i64 == 0
    };
    let cost = |groups: &[Vec<usize>], c: usize, i: usize| -> f64 {
        let car = &cars[c];
        let r = &riders[i];
        let others: Vec<usize> = groups[c].iter().copied().filter(|&j| j != i).collect();
        let mut km = if others.is_empty() {
            EMPTY_CAR_KM
        } else {
            others
                .iter()
                .map(|&j| distance_km(r, &riders[j]))
                .fold(f64::INFINITY, f64::min)
        };
        if car.area_ids.contains(&r.area_id) {
            km += PER_EXTRA_AREA_KM * car.area_ids.len().saturating_sub(1) as f64;
        } else {
            km += AREA_PENALTY_KM;
        }
        if let Some((d, times)) = r.last {
            if d == car.driver_id {
                km -= LAST_DRIVER_KM + PER_RIDE_TOGETHER_KM * times.min(MAX_TOGETHER) as f64;
            }
        }
        km
    };

    // Hardest first.
    let options =
        |groups: &[Vec<usize>], i: usize| (0..cars.len()).filter(|&c| fits(groups, c, i)).count();
    open.sort_by_key(|&i| {
        (
            !riders[i].needs_assistance,
            options(&groups, i),
            -riders[i].seats,
            riders[i].ride_id,
        )
    });
    let mut unplaced = Vec::new();
    let mut placed_by_us: Vec<usize> = Vec::new();
    for &i in &open {
        let best = (0..cars.len())
            .filter(|&c| fits(&groups, c, i))
            .map(|c| (c, cost(&groups, c, i)))
            .min_by(|a, b| a.1.total_cmp(&b.1));
        match best {
            Some((c, _)) => {
                groups[c].push(i);
                placed_by_us.push(i);
            }
            None => unplaced.push(riders[i].ride_id),
        }
    }

    // Improve: move one passenger to another car, or swap two passengers between cars,
    // whenever the whole arrangement gets cheaper. Swaps matter when cars are full: e.g. a
    // driver serving two areas filled up from one, leaving the other area's passengers
    // with a driver who doesn't serve them.
    let car_of = |groups: &[Vec<usize>], i: usize| groups.iter().position(|g| g.contains(&i));
    let total = |groups: &[Vec<usize>]| -> f64 {
        placed_by_us
            .iter()
            .filter_map(|&i| car_of(groups, i).map(|c| cost(groups, c, i)))
            .sum()
    };
    for _ in 0..IMPROVE_PASSES {
        let mut improved = false;
        for &i in &placed_by_us {
            let Some(from) = car_of(&groups, i) else {
                continue;
            };
            let before = total(&groups);
            let mut without = groups.clone();
            without[from].retain(|&j| j != i);
            let best = (0..cars.len())
                .filter(|&c| c != from && fits(&without, c, i))
                .map(|c| {
                    let mut g = without.clone();
                    g[c].push(i);
                    let t = total(&g);
                    (g, t)
                })
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((g, after)) = best {
                if after + MIN_GAIN_KM < before {
                    groups = g;
                    improved = true;
                }
            }
        }
        for (x, &i) in placed_by_us.iter().enumerate() {
            for &j in &placed_by_us[x + 1..] {
                let (Some(a), Some(b)) = (car_of(&groups, i), car_of(&groups, j)) else {
                    continue;
                };
                if a == b {
                    continue;
                }
                let mut g = groups.clone();
                g[a].retain(|&k| k != i);
                g[b].retain(|&k| k != j);
                if !fits(&g, b, i) {
                    continue;
                }
                g[b].push(i);
                if !fits(&g, a, j) {
                    continue;
                }
                g[a].push(j);
                if total(&g) + MIN_GAIN_KM < total(&groups) {
                    groups = g;
                    improved = true;
                }
            }
        }
        // Hand a whole group to another driver (neighbours stay together, the driver
        // changes): fixes a driver serving two areas who filled up from one of them.
        // Only for cars whose passengers were all placed here, never pinned ones.
        let movable = |g: &Vec<usize>| g.iter().all(|i| placed_by_us.contains(i));
        for a in 0..cars.len() {
            for b in a + 1..cars.len() {
                if !movable(&groups[a]) || !movable(&groups[b]) {
                    continue;
                }
                let mut g = groups.clone();
                g.swap(a, b);
                let ok = [a, b].iter().all(|&c| {
                    let members = std::mem::take(&mut g[c]);
                    let fits_all = members.iter().enumerate().all(|(k, &i)| {
                        let mut partial = g.clone();
                        partial[c] = members[..k].to_vec();
                        fits(&partial, c, i)
                    });
                    g[c] = members;
                    fits_all
                });
                if ok && total(&g) + MIN_GAIN_KM < total(&groups) {
                    groups = g;
                    improved = true;
                }
            }
        }
        if !improved {
            break;
        }
    }

    // Explain each placement.
    let mut placed = Vec::new();
    for (c, g) in groups.iter().enumerate() {
        for &i in g.iter().filter(|i| placed_by_us.contains(i)) {
            let r = &riders[i];
            let near = g
                .iter()
                .copied()
                .filter(|&j| j != i)
                .map(|j| (j, metres(r, &riders[j])))
                .min_by(|a, b| {
                    let key = |m: Option<f64>, j: usize| {
                        (m.unwrap_or(f64::INFINITY), riders[j].area_id != r.area_id)
                    };
                    key(a.1, a.0)
                        .partial_cmp(&key(b.1, b.0))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(j, m)| (riders[j].ride_id, m));
            placed.push(Placement {
                ride_id: r.ride_id,
                driver_id: cars[c].driver_id,
                near,
                in_area: cars[c].area_ids.contains(&r.area_id),
                together: r
                    .last
                    .filter(|(d, _)| *d == cars[c].driver_id)
                    .map(|(_, t)| t)
                    .unwrap_or(0),
            });
        }
    }
    placed.sort_by_key(|p| p.ride_id);
    unplaced.sort();
    Arrangement { placed, unplaced }
}

fn metres(a: &Rider, b: &Rider) -> Option<f64> {
    Some(haversine_m(a.point?, b.point?))
}

fn distance_km(a: &Rider, b: &Rider) -> f64 {
    match metres(a, b) {
        Some(m) => m / 1000.0,
        None if a.area_id == b.area_id => SAME_AREA_KM,
        None => OTHER_AREA_KM,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rider(id: i64, area: i64, at: Option<(f64, f64)>) -> Rider {
        Rider {
            ride_id: id,
            passenger_id: 100 + id,
            seats: 1,
            area_id: area,
            point: at.map(|(lat, lng)| LatLng { lat, lng }),
            needs_assistance: false,
            last: None,
        }
    }
    fn car(id: i64, seats: i64, areas: &[i64]) -> Car {
        Car {
            driver_id: id,
            seats_left: seats,
            carpool: true,
            other_rides: 0,
            area_ids: areas.to_vec(),
        }
    }
    fn driver_of(a: &Arrangement, ride: i64) -> Option<i64> {
        a.placed
            .iter()
            .find(|p| p.ride_id == ride)
            .map(|p| p.driver_id)
    }

    #[test]
    fn neighbours_share_a_car_and_areas_are_respected() {
        // Two clusters ~10 km apart; each car serves one cluster's area.
        let riders = vec![
            rider(1, 1, Some((3.100, 101.600))),
            rider(2, 1, Some((3.101, 101.601))),
            rider(3, 2, Some((3.190, 101.640))),
            rider(4, 2, Some((3.191, 101.641))),
        ];
        let cars = vec![car(10, 2, &[1]), car(20, 2, &[2])];
        let a = arrange(&riders, &cars, &HashMap::new());
        assert!(a.unplaced.is_empty());
        assert_eq!(driver_of(&a, 1), Some(10));
        assert_eq!(driver_of(&a, 2), Some(10));
        assert_eq!(driver_of(&a, 3), Some(20));
        assert_eq!(driver_of(&a, 4), Some(20));
        let p2 = a.placed.iter().find(|p| p.ride_id == 2).unwrap();
        assert_eq!(p2.near.map(|n| n.0), Some(1));
        assert!(p2.near.unwrap().1.unwrap() < 200.0);
        assert!(p2.in_area);
    }

    #[test]
    fn seats_one_booking_rule_and_self_driving_are_hard_limits() {
        let riders = vec![rider(1, 1, None), rider(2, 1, None), rider(3, 1, None)];
        let mut solo = car(10, 4, &[1]);
        solo.carpool = false;
        let mut me = car(102, 4, &[1]); // the driver is rider 2's passenger
        me.seats_left = 4;
        let a = arrange(&riders, &[solo, me, car(30, 0, &[1])], &HashMap::new());
        assert_eq!(
            a.placed.iter().filter(|p| p.driver_id == 10).count(),
            1,
            "no carpool: one booking"
        );
        assert_ne!(driver_of(&a, 2), Some(102), "nobody drives themselves");
        assert!(
            a.placed.iter().all(|p| p.driver_id != 30),
            "a full car takes nobody"
        );
        assert_eq!(a.placed.len() + a.unplaced.len(), 3);
    }

    #[test]
    fn fixed_assignments_stay_and_count_against_seats() {
        let riders = vec![rider(1, 1, None), rider(2, 1, None), rider(3, 1, None)];
        let cars = vec![car(10, 2, &[1]), car(20, 2, &[1])];
        let fixed = HashMap::from([(1, 20), (2, 20)]);
        let a = arrange(&riders, &cars, &fixed);
        assert_eq!(
            a.placed,
            vec![Placement {
                ride_id: 3,
                driver_id: 10,
                near: None,
                in_area: true,
                together: 0
            }]
        );
    }

    #[test]
    fn previous_driver_wins_a_close_call_and_full_boards_report_unplaced() {
        let mut r = rider(1, 1, None);
        r.last = Some((20, 3));
        let a = arrange(
            &[r, rider(2, 1, None)],
            &[car(10, 1, &[1]), car(20, 1, &[1])],
            &HashMap::new(),
        );
        assert_eq!(driver_of(&a, 1), Some(20));
        assert_eq!(
            a.placed.iter().find(|p| p.ride_id == 1).unwrap().together,
            3
        );

        let a = arrange(
            &[rider(1, 1, None), rider(2, 1, None)],
            &[car(10, 1, &[1])],
            &HashMap::new(),
        );
        assert_eq!(a.placed.len(), 1);
        assert_eq!(a.unplaced.len(), 1);
    }

    #[test]
    fn swaps_free_a_two_area_driver_for_the_area_only_they_serve() {
        // Car 10 serves areas 1 and 2 (2 seats); car 20 serves area 1 only (2 seats).
        // Area-1 riders sit closest to car 10's first pick, so greedy fills car 10 with
        // area 1 and strands an area-2 rider with car 20. A swap fixes it.
        let riders = vec![
            rider(1, 1, Some((3.100, 101.600))),
            rider(2, 1, Some((3.101, 101.600))),
            rider(3, 2, Some((3.130, 101.600))),
            rider(4, 2, Some((3.131, 101.600))),
        ];
        let cars = vec![car(10, 2, &[1, 2]), car(20, 2, &[1])];
        let a = arrange(&riders, &cars, &HashMap::new());
        assert!(a.placed.iter().all(|p| p.in_area), "{a:?}");
        assert_eq!(driver_of(&a, 3), Some(10));
        assert_eq!(driver_of(&a, 4), Some(10));
    }

    #[test]
    fn specialists_take_their_area_so_flexible_drivers_cover_the_rest() {
        // Like a Sunday board: car 10 serves Subang only (4 seats), car 20 serves Subang and
        // Puchong (6 seats), car 30 serves Puchong only (4 seats). 6 Subang + 6 Puchong riders.
        let mut riders = Vec::new();
        for i in 0..6 {
            riders.push(rider(i, 4, Some((3.050 + i as f64 * 0.001, 101.585))));
            riders.push(rider(10 + i, 6, Some((3.025 + i as f64 * 0.001, 101.620))));
        }
        let cars = vec![car(10, 4, &[4]), car(20, 6, &[4, 6]), car(30, 4, &[6])];
        let a = arrange(&riders, &cars, &HashMap::new());
        assert!(a.unplaced.is_empty());
        assert!(a.placed.iter().all(|p| p.in_area), "{a:?}");
    }

    #[test]
    fn assistance_needs_are_placed_first() {
        let mut needs = rider(2, 1, None);
        needs.needs_assistance = true;
        let a = arrange(
            &[rider(1, 1, None), needs],
            &[car(10, 1, &[1])],
            &HashMap::new(),
        );
        assert_eq!(driver_of(&a, 2), Some(10));
        assert_eq!(a.unplaced, vec![1]);
    }
}
