//! How near a line runs to a thing it does not join: what the operation
//! band's places are chosen by, and what the layout's tests hold every map
//! to. Exact below [`CLEARANCE`]; a pair farther apart than that on either
//! axis is not measured, since nothing is chosen by how clear beyond clear
//! enough it is.

use std::collections::HashMap;

use super::Place;

/// How far, in the unit square, a line keeps from every thing it does not
/// join: about half a drawn node's height on a canvas the unit square is
/// scaled to.
pub const CLEARANCE: f64 = 0.05;

/// A point of the map.
type Point = (f64, f64);

/// The distance from `point` to the segment `from` → `to`, or `None` where
/// the point lies more than [`CLEARANCE`] outside the segment's box.
fn distance((x, y): Point, from: Point, to: Point) -> Option<f64> {
    let outside = x < from.0.min(to.0) - CLEARANCE
        || x > from.0.max(to.0) + CLEARANCE
        || y < from.1.min(to.1) - CLEARANCE
        || y > from.1.max(to.1) + CLEARANCE;
    if outside {
        return None;
    }
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let length = dx.mul_add(dx, dy * dy);
    let along = if length <= f64::EPSILON {
        0.0
    } else {
        ((x - from.0).mul_add(dx, (y - from.1) * dy) / length).clamp(0.0, 1.0)
    };
    let (nearest_x, nearest_y) = (along.mul_add(dx, from.0), along.mul_add(dy, from.1));
    Some((x - nearest_x).hypot(y - nearest_y))
}

/// Where each of `places` stands, by its id.
pub(super) fn by_id(places: &[Place]) -> HashMap<&str, Point> {
    places
        .iter()
        .map(|place| (place.id.as_str(), (place.x, place.y)))
        .collect()
}

/// The nearest any of `lines` runs to a place among `places` it does not
/// join; one where no line passes any thing. A map is clear where this is
/// at least [`CLEARANCE`].
#[must_use]
pub fn clearance(places: &[Place], lines: &[(&str, &str)]) -> f64 {
    let at = by_id(places);
    let mut nearest: f64 = 1.0;
    for &(from, to) in lines {
        let (Some(one), Some(other)) = (at.get(from), at.get(to)) else {
            continue;
        };
        for (id, point) in &at {
            if *id != from
                && *id != to
                && let Some(apart) = distance(*point, *one, *other)
            {
                nearest = nearest.min(apart);
            }
        }
    }
    nearest
}

/// How clear the map is around the thing `id` were it to stand at
/// `standing`: the nearest its own lines run to another thing, and the
/// nearest another line runs to it. `at` holds where the others stand. A
/// measure that falls to `bar` stops there: a candidate no clearer than one
/// already found is not measured further.
pub(super) fn clearance_at(
    at: &HashMap<&str, Point>,
    lines: &[(&str, &str)],
    (id, standing): (&str, Point),
    bar: f64,
) -> f64 {
    let mut nearest: f64 = 1.0;
    for &(from, to) in lines {
        let ends = (
            if from == id {
                Some(&standing)
            } else {
                at.get(from)
            },
            if to == id {
                Some(&standing)
            } else {
                at.get(to)
            },
        );
        let (Some(one), Some(other)) = ends else {
            continue;
        };
        if from == id || to == id {
            for (other_id, point) in at {
                if *other_id != from
                    && *other_id != to
                    && *other_id != id
                    && let Some(apart) = distance(*point, *one, *other)
                {
                    nearest = nearest.min(apart);
                }
            }
        } else if let Some(apart) = distance(standing, *one, *other) {
            nearest = nearest.min(apart);
        }
        if nearest <= bar {
            return nearest;
        }
    }
    nearest
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(said: Option<f64>, expected: f64) -> bool {
        said.is_some_and(|said| (said - expected).abs() < 1e-12)
    }

    #[test]
    fn a_point_is_measured_to_the_segment_not_the_line_through_it() {
        assert!(near(distance((0.5, 0.01), (0.0, 0.0), (1.0, 0.0)), 0.01));
        assert!(near(distance((0.03, 0.04), (0.0, 0.0), (0.0, 0.0)), 0.05));
        assert_eq!(
            distance((2.0, 0.0), (0.0, 0.0), (1.0, 0.0)),
            None,
            "far beyond its end is not measured"
        );
    }
}
