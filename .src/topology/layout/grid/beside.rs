//! The Parties beside what they face, a TX-Party on the left and an
//! RX-Party on the right, kept apart down their side.

use std::collections::HashMap;

use super::{Band, Column, PARTY_GAP, Place, TopologyNode, fraction, place};

/// Each Party beside what it faces: one that sends into the cluster to the
/// left, one delivered to on the right, at the mean height of the things its
/// lines reach, and never nearer another of its side than [`PARTY_GAP`].
/// The lines and places are each read once, whatever the count of Parties.
pub(super) fn beside(
    parties: &[&TopologyNode],
    places: &[Place],
    lines: &[(&str, &str)],
) -> Vec<Place> {
    let height: HashMap<&str, f64> = places
        .iter()
        .map(|place| (place.id.as_str(), place.y))
        .collect();
    // For each end of a line, whether it sends and the heights it faces.
    let mut facing: HashMap<&str, (bool, Vec<f64>)> = HashMap::new();
    for &(from, to) in lines {
        let sending = facing.entry(from).or_default();
        sending.0 = true;
        if let Some(y) = height.get(to) {
            sending.1.push(*y);
        }
        if let Some(y) = height.get(from) {
            facing.entry(to).or_default().1.push(*y);
        }
    }
    let mut sides: Vec<Place> = parties
        .iter()
        .map(|party| {
            let (sends, faced) = facing
                .get(party.id.as_str())
                .map_or((false, &[][..]), |(sends, faced)| {
                    (*sends, faced.as_slice())
                });
            let y = if faced.is_empty() {
                0.5
            } else {
                faced.iter().sum::<f64>() / fraction(faced.len())
            };
            let column = if sends {
                Column::TxParty
            } else {
                Column::RxParty
            };
            place(party, (column.x(), y), Band::Path, column)
        })
        .collect();
    for column in [Column::TxParty, Column::RxParty] {
        apart(&mut sides, column);
    }
    sides
}

/// The Parties of one side kept [`PARTY_GAP`] apart, top to bottom, as
/// near where each would stand as they may be — a run pushed apart stays
/// centered on where its Parties would stand — and inside the unit square.
fn apart(sides: &mut [Place], column: Column) {
    let mut side: Vec<&mut Place> = sides
        .iter_mut()
        .filter(|one| one.column == column)
        .collect();
    side.sort_by(|one, other| {
        one.y
            .total_cmp(&other.y)
            .then_with(|| one.id.cmp(&other.id))
    });
    let wanted: f64 = side.iter().map(|one| one.y).sum::<f64>() / fraction(side.len().max(1));
    for index in 1..side.len() {
        let floor = side[index - 1].y + PARTY_GAP;
        if side[index].y < floor {
            side[index].y = floor;
        }
    }
    let now: f64 = side.iter().map(|one| one.y).sum::<f64>() / fraction(side.len().max(1));
    let first = side.first().map_or(0.0, |first| first.y);
    let last = side.last().map_or(0.0, |last| last.y);
    let up = (now - wanted).min(first).max(last - 1.0);
    for one in &mut side {
        one.y -= up;
    }
}
