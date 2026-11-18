//! Pickup stop ordering: exact solver for small trips and nearest-neighbour + 2-opt heuristic.

/// The new best order must save this much before a driver's stops are reshuffled.
const KEEP_ORDER_MARGIN_S: f64 = 60.0;
/// Most pickups solved exactly (8! orders); larger trips use a heuristic.
const EXACT_MAX: usize = 8;

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
