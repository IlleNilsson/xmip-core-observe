//! The two layouts: a cluster's, by role, and what a drill shows inside a
//! node or a stage.

mod beside;
mod rows;
mod search;

use node::{NodeRole, Stage};

use super::{Band, Column, Place};
use crate::topology::draw::label;
use crate::topology::{NodeKind, TopologyNode};
use beside::beside;
use rows::Rows;
use search::shifted;

/// Half a column: how far a standby stands toward the sending side, so the
/// executing lane's lines rise beside the standbys and not through them.
const HALF_COLUMN: f64 = 0.15;

/// How far apart two things of one band stand across it.
const SPREAD: f64 = 0.15;

/// How far apart two Parties of one side stand down it, at least.
const PARTY_GAP: f64 = 0.12;

/// The gap below the processing band, in rows: near the Storage node it is
/// handed work by, so its own lines are short and the operation band's
/// lines pass it wide.
const BELOW_PROCESSING: f64 = 0.7;

/// Where each thing of a cluster stands: its nodes by their roles, the
/// Parties beside what they face, the operation band where its lines run
/// clear.
pub(super) fn cluster(things: &[&TopologyNode], lines: &[(&str, &str)]) -> Vec<Place> {
    let (parties, standing): (Vec<&TopologyNode>, Vec<&TopologyNode>) = things
        .iter()
        .partition(|thing| thing.kind == NodeKind::Party);
    // Each thing's roles, read once.
    let declared: Vec<(&TopologyNode, Vec<NodeRole>)> = standing
        .iter()
        .map(|thing| (*thing, roles(thing)))
        .collect();
    let primary = declared
        .iter()
        .filter(|(_, roles)| roles.contains(&NodeRole::Storage))
        .min_by(|(one, _), (other, _)| listed(one).cmp(&listed(other)))
        .map(|(thing, _)| thing.id.as_str());
    let mut stood: Vec<(&TopologyNode, Band, Column, f64)> = declared
        .iter()
        .map(|(thing, roles)| {
            let (band, column, x) = by_roles(roles, Some(thing.id.as_str()) == primary);
            (*thing, band, column, x)
        })
        .collect();
    stood.sort_by(|one, other| {
        (one.1, one.2, listed(one.0)).cmp(&(other.1, other.2, listed(other.0)))
    });
    let rows = Rows::of(&stood);
    let mut fixed: Vec<Place> = Vec::new();
    let mut processing: Vec<Place> = Vec::new();
    for (band, column) in bands_and_columns(&stood) {
        let group: Vec<&(&TopologyNode, Band, Column, f64)> = stood
            .iter()
            .filter(|one| (one.1, one.2) == (band, column))
            .collect();
        for (index, (thing, _, _, x)) in group.iter().enumerate() {
            let (x, y) = match band {
                Band::Path | Band::Standby => (*x, rows.y(band, index)),
                _ => (spread(*x, index, group.len()), rows.y(band, 0)),
            };
            match band {
                Band::Operation => {}
                Band::Processing => processing.push(place(thing, (x, y), band, column)),
                _ => fixed.push(place(thing, (x, y), band, column)),
            }
        }
    }
    let operating: Vec<&TopologyNode> = stood
        .iter()
        .filter(|one| one.1 == Band::Operation)
        .map(|one| one.0)
        .collect();
    let y = rows.y(Band::Operation, 0);
    let mut places = shifted(fixed, &processing, (&operating, y), lines);
    places.extend(beside(&parties, &places, lines));
    places
}

/// Inside a node: its stages in path order on one row; anything else in a
/// grid, in the order drawn.
pub(super) fn inside(things: &[&TopologyNode]) -> Vec<Place> {
    let stage = |thing: &TopologyNode| {
        (thing.kind == NodeKind::Stage)
            .then(|| Stage::named(label(&thing.scope)))
            .flatten()
    };
    if !things.is_empty() && things.iter().all(|thing| stage(thing).is_some()) {
        return things
            .iter()
            .filter_map(|thing| {
                let column = match stage(thing)? {
                    Stage::Receive => Column::Receiving,
                    Stage::Process => Column::Storage,
                    Stage::Send => Column::Sending,
                };
                Some(place(thing, (column.x(), 0.5), Band::Inside, column))
            })
            .collect();
    }
    let across = (1..=things.len())
        .find(|n| n * n >= things.len())
        .unwrap_or(1);
    let down = things.len().div_ceil(across).max(1);
    things
        .iter()
        .enumerate()
        .map(|(index, thing)| {
            let (column, row) = (index % across, index / across);
            let x = (fraction(column) + 0.5) / fraction(across);
            let y = (fraction(row) + 0.5) / fraction(down);
            place(thing, (x, y), Band::Inside, Column::Spanning)
        })
        .collect()
}

/// The roles a drawn node says, each word as `node::NodeRole::named` reads
/// it, once per thing; none where it says a word that
/// is no role, as `node::NodeRole::declared` refuses it.
fn roles(thing: &TopologyNode) -> Vec<NodeRole> {
    let mut said = Vec::new();
    for word in thing.roles.split([',', '+']).map(str::trim) {
        match NodeRole::named(word) {
            Some(role) => said.push(role),
            None if word.is_empty() => {}
            None => return Vec::new(),
        }
    }
    said
}

/// What orders things within one column: the place in `[storage] nodes`,
/// one listed nowhere after, then the scope and the id as bytes.
fn listed(thing: &TopologyNode) -> (bool, Option<u32>, &[u8], &[u8]) {
    (
        thing.place.is_none(),
        thing.place,
        thing.scope.as_bytes(),
        thing.id.as_bytes(),
    )
}

/// The band, the column and where across a node declaring `roles` stands —
/// storage, then the stage roles, then the rest — `first` where it is the
/// first-listed Storage node.
pub(super) fn by_roles(roles: &[NodeRole], first: bool) -> (Band, Column, f64) {
    if roles.contains(&NodeRole::Storage) {
        return if first {
            (Band::Path, Column::Storage, Column::Storage.x())
        } else {
            let x = Column::Storage.x() + HALF_COLUMN;
            (Band::Standby, Column::Storage, x)
        };
    }
    let served = Stage::ALL.map(|stage| roles.iter().any(|role| role.stages().contains(&stage)));
    let column = |stage: Stage| match stage {
        Stage::Receive => Column::Receiving,
        Stage::Process => Column::Storage,
        Stage::Send => Column::Sending,
    };
    let stages = || {
        Stage::ALL
            .into_iter()
            .zip(served)
            .filter(|(_, on)| *on)
            .map(|(stage, _)| stage)
    };
    match (stages().count(), stages().next()) {
        (1, Some(Stage::Process)) => (Band::Processing, Column::Storage, Column::Storage.x()),
        (1, Some(one)) => (Band::Path, column(one), column(one).x()),
        (0, _) | (_, None) => (Band::Operation, Column::Spanning, Column::Spanning.x()),
        (many, _) => {
            let mean = stages().map(|stage| column(stage).x()).sum::<f64>() / fraction(many);
            (Band::Executing, Column::Spanning, mean)
        }
    }
}

/// Each band and column there is, top to bottom and left to right.
fn bands_and_columns(stood: &[(&TopologyNode, Band, Column, f64)]) -> Vec<(Band, Column)> {
    let mut seen: Vec<(Band, Column)> = stood.iter().map(|one| (one.1, one.2)).collect();
    seen.dedup();
    seen
}

/// The `index`th of `count` things of one row, spread about `x`, never so
/// far apart that the row leaves the unit square.
fn spread(x: f64, index: usize, count: usize) -> f64 {
    let across = fraction(count.saturating_sub(1));
    let offset = fraction(index) - across / 2.0;
    SPREAD.min(1.0 / across.max(1.0)).mul_add(offset, x)
}

/// `thing` at `(x, y)`.
fn place(thing: &TopologyNode, (x, y): (f64, f64), band: Band, column: Column) -> Place {
    Place {
        id: thing.id.clone(),
        x,
        y,
        band,
        column,
    }
}

/// A count as a coordinate's part.
#[allow(clippy::cast_precision_loss)] // a count of drawn things, far below 2^52
fn fraction(count: usize) -> f64 {
    count as f64
}
