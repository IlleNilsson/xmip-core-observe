//! The rows of a cluster's map, top to bottom: which bands there are, how
//! many rows each takes, and where each row stands in the unit square.

use super::{BELOW_PROCESSING, Band, Column, TopologyNode, fraction};

/// The rows of a cluster's map, top to bottom, and where each stands.
pub(super) struct Rows {
    /// Where each band's first row stands before the map is scaled.
    first: Vec<(Band, f64)>,
    /// How far down the last row stands before the map is scaled.
    last: f64,
}

impl Rows {
    pub(super) fn of(stood: &[(&TopologyNode, Band, Column, f64)]) -> Self {
        let count = |band: Band, column: Option<Column>| {
            stood
                .iter()
                .filter(|one| one.1 == band && column.is_none_or(|column| one.2 == column))
                .count()
        };
        let path = [Column::Receiving, Column::Storage, Column::Sending]
            .map(|column| count(Band::Path, Some(column)))
            .into_iter()
            .max()
            .unwrap_or(0);
        let mut first = Vec::new();
        let (mut down, mut last) = (0.0, 0.0);
        for (band, rows) in [
            (Band::Operation, count(Band::Operation, None).min(1)),
            (Band::Processing, count(Band::Processing, None).min(1)),
            (Band::Path, path),
            (Band::Standby, count(Band::Standby, None)),
            (Band::Executing, count(Band::Executing, None).min(1)),
        ] {
            if rows == 0 {
                continue;
            }
            first.push((band, down));
            last = down + fraction(rows - 1);
            down += match band {
                Band::Processing => BELOW_PROCESSING,
                _ => fraction(rows),
            };
        }
        Self { first, last }
    }

    /// Where row `index` of `band` stands, scaled to the unit square.
    pub(super) fn y(&self, band: Band, index: usize) -> f64 {
        let at = self
            .first
            .iter()
            .find(|(one, _)| *one == band)
            .map_or(0.0, |(_, first)| first + fraction(index));
        if self.last <= f64::EPSILON {
            0.5
        } else {
            at / self.last
        }
    }
}
