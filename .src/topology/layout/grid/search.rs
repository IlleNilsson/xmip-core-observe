//! Where across their bands the operation band and the processing band
//! stand: every other band's lines pass through them on their way up, so
//! they are moved, as little as may be, until those lines run clear. The
//! search is bounded by the millisecond rule (CONTRIBUTING: over a
//! millisecond beyond load is a defect): candidates nearest the middle are
//! measured first and the first clear one taken, a candidate no clearer than
//! one found is not measured further, and a band whose lines would pass more
//! things than [`SEARCHED`] measures is not searched at all — a cluster that
//! large cannot run every line clear of every node, and stands its operation
//! band in the middle.

use super::{Band, Column, Place, TopologyNode, place, spread};
use crate::topology::layout::geometry::{CLEARANCE, by_id, clearance, clearance_at};

/// How far across the processing band may be shifted from over the Storage
/// column, nearest first, so the operation band's lines pass it.
const SHIFTS: [f64; 9] = [0.0, -0.05, 0.05, -0.1, 0.1, -0.15, 0.15, -0.2, 0.2];

/// The most a band's search measures: a thing's lines times the things they
/// may pass. The test cluster's operation node measures 8 lines past 15
/// things; one relating to every node of a cluster of 176 would measure
/// 52,000, a hundred times over.
const SEARCHED: usize = 2_000;

/// The processing band shifted across by as little as lets every line run
/// clear — or, where none does, run clearest — and the operation band's
/// things each where their own lines run clear.
pub(super) fn shifted(
    mut fixed: Vec<Place>,
    processing: &[Place],
    (operating, y): (&[&TopologyNode], f64),
    lines: &[(&str, &str)],
) -> Vec<Place> {
    let places = fixed.len() + processing.len() + operating.len();
    let bounded = operating
        .iter()
        .all(|thing| touching(lines, &thing.id) * places <= SEARCHED);
    let shifts = if processing.is_empty() || !bounded {
        &SHIFTS[..1]
    } else {
        &SHIFTS[..]
    };
    let mut best: Option<(f64, Vec<Place>)> = None;
    for shift in shifts {
        let mut stood = if shifts.len() == 1 {
            std::mem::take(&mut fixed)
        } else {
            fixed.clone()
        };
        stood.extend(processing.iter().map(|one| Place {
            x: one.x + shift,
            ..one.clone()
        }));
        let mut taken: Vec<f64> = Vec::new();
        for (index, thing) in operating.iter().enumerate() {
            let default = spread(Column::Spanning.x(), index, operating.len());
            let x = if bounded {
                clearest(thing, (default, y), &stood, (lines, &taken))
            } else {
                default
            };
            taken.push(x);
            stood.push(place(thing, (x, y), Band::Operation, Column::Spanning));
        }
        if shifts.len() == 1 {
            return stood;
        }
        let clear = clearance(&stood, lines).min(CLEARANCE);
        if best.as_ref().is_none_or(|(kept, _)| clear > *kept) {
            best = Some((clear, stood));
        }
        if clear >= CLEARANCE {
            break;
        }
    }
    best.map(|(_, stood)| stood).unwrap_or_default()
}

/// How many of `lines` end at `id`.
fn touching(lines: &[(&str, &str)], id: &str) -> usize {
    lines
        .iter()
        .filter(|(from, to)| *from == id || *to == id)
        .count()
}

/// Where across its band a thing of the operation band stands: where its
/// lines run clear of every other thing and no other line runs through it,
/// nearest the middle, kept apart from the band's others in `taken`; else
/// wherever they run clearest. One with no line stands at `default`. Asked
/// in twentieths of the band from the middle out, then in hundredths beside
/// the best: toward the middle where it is clear, either way where not.
fn clearest(
    thing: &TopologyNode,
    (default, y): (f64, f64),
    places: &[Place],
    (lines, taken): (&[(&str, &str)], &[f64]),
) -> f64 {
    if touching(lines, &thing.id) == 0 {
        return default;
    }
    let at = by_id(places);
    let apart = super::SPREAD.min(1.0 / super::fraction(taken.len() + 1));
    let free = |x: f64| taken.iter().all(|other| (other - x).abs() >= apart);
    let measured =
        |x: f64, bar: f64| clearance_at(&at, lines, (&thing.id, (x, y)), bar).min(CLEARANCE);
    let mut best: Option<(f64, f64)> = None;
    for step in 0..=20_u32 {
        // 0.5, 0.45, 0.55, 0.4, … : nearest the middle first, left first.
        let half = f64::from(step.div_ceil(2)) / 20.0;
        let x = if step % 2 == 1 {
            0.5 - half
        } else {
            0.5 + half
        };
        if !free(x) {
            continue;
        }
        let bar = best.map_or(f64::NEG_INFINITY, |(clear, _)| clear);
        let clear = measured(x, bar);
        if clear > bar {
            best = Some((clear, x));
        }
        if clear >= CLEARANCE {
            break;
        }
    }
    let Some((clear, coarse)) = best else {
        return default;
    };
    if clear >= CLEARANCE {
        // Clear: nearer the middle, short of the twentieth already tried.
        let toward = (0.5 - coarse).signum();
        return (1..=4)
            .rev()
            .map(|step| coarse + toward * f64::from(step) / 100.0)
            .find(|x| {
                toward != 0.0 && free(*x) && measured(*x, CLEARANCE - f64::EPSILON) >= CLEARANCE
            })
            .unwrap_or(coarse);
    }
    // Not clear: the clearest beside it, either way.
    let (mut kept, mut at) = (clear, coarse);
    for step in 1..=4_i32 {
        for x in [-1.0, 1.0].map(|way| (coarse + way * f64::from(step) / 100.0).clamp(0.0, 1.0)) {
            if free(x) {
                let here = measured(x, kept);
                if here > kept {
                    (kept, at) = (here, x);
                }
            }
        }
    }
    at
}
