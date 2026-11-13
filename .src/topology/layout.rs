//! Where each drawn thing stands: by the roles a node declares and the order
//! `[storage] nodes` lists, never by traffic and never by a name (the owner,
//! 2026-11-12: *We got to straighten out the map. It has to focus on
//! roles*; ADR-0052 and ADR-0056, amendments 2026-11-13). The map stands
//! still while the pulses move.
//!
//! ```text
//!                  TX-Party   receiving    Storage      sending   RX-Party
//! operation band                  OP (operational, monitoring, none)
//! processing band                       PX  PX
//! path rows        TX-Party 1   RX1     DS1 (first listed)   TX1   RX-Party 1
//!                  TX-Party 2   RX2                          TX2   RX-Party 2
//! standby rows                             DS2 (later listed)
//! executing lane   (its Parties from the left)  EX  (to the right)
//! ```
//!
//! Storage first, then the stage roles, then the operation roles: a node
//! declaring storage stands in the Storage column, the first listed on the
//! path row and the others below it, half a column toward the sending side;
//! one serving receive alone or send alone on the path row; processing alone
//! over Storage; executing, or two stage roles, in a lane of its own below,
//! at the mean of its stages' columns; anything else in the operation band
//! on top, where its every line fans down, standing where its lines run
//! clear of every other thing. A Party stands in its side's column beside
//! what it faces. Within a column the order is the place in `[storage]
//! nodes`, then the scope compared as bytes: an identifier that only makes
//! the result deterministic, so renaming can reorder nodes within a column
//! and never moves one across a column or a band.
//!
//! Inside a node, its stages stand in path order: Receive, Work Process,
//! Send. Anything else is set out in a grid in the order drawn.
//!
//! The reader computes it (`xmip_publication_layout_v1`, `xmip_operate.h`
//! section 8), not the publisher: where things stand depends on the drill and
//! on what the reader shows, and a publication carries facts. Positions are
//! in the unit square, `x` left to right and `y` top to bottom; a surface
//! scales them to its canvas.

mod geometry;
mod grid;

use std::collections::{BTreeMap, HashMap, HashSet};

use super::relate::Member;
use super::{NodeKind, Origin, Topology, TopologyNode};
pub use geometry::{CLEARANCE, clearance};

/// A horizontal band of the map, top to bottom.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Band {
    /// Operational, monitoring, development, none declared: on top.
    Operation,
    /// Processing alone, over the Storage column.
    Processing,
    /// The path: receiving, the first-listed Storage node, sending, and the
    /// Parties beside them.
    Path,
    /// The later-listed Storage nodes, below the first.
    Standby,
    /// Executing, or two stage roles: a path of its own below.
    Executing,
    /// Inside a node or a stage: what a drill shows.
    Inside,
}

/// A column of the map, left to right, as the streams run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Column {
    /// The Parties that send into the cluster.
    TxParty,
    /// Receiving, and the Receive stage inside a node.
    Receiving,
    /// Storage, and the Work Process stage inside a node.
    Storage,
    /// Sending, and the Send stage inside a node.
    Sending,
    /// The Parties the cluster delivers to.
    RxParty,
    /// No one column: the operation band, the executing lane, a grid.
    Spanning,
}

impl Band {
    /// Every band, in the order the header numbers them.
    pub const ALL: [Band; 6] = [
        Band::Operation,
        Band::Processing,
        Band::Path,
        Band::Standby,
        Band::Executing,
        Band::Inside,
    ];
}

impl Column {
    /// Every column, in the order the header numbers them.
    pub const ALL: [Column; 6] = [
        Column::TxParty,
        Column::Receiving,
        Column::Storage,
        Column::Sending,
        Column::RxParty,
        Column::Spanning,
    ];

    /// Where the column stands across the map, from zero to one; the middle
    /// for spanning.
    #[must_use]
    pub const fn x(self) -> f64 {
        match self {
            Self::TxParty => 0.0,
            Self::Receiving => 0.2,
            Self::Storage | Self::Spanning => 0.5,
            Self::Sending => 0.8,
            Self::RxParty => 1.0,
        }
    }
}

/// Where one drawn thing stands.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub band: Band,
    pub column: Column,
}

/// Where each thing `topology` draws beneath `focus` stands — the things
/// whose parent is `focus`, the empty focus being the top — or, where
/// `shown` names any, only those of them.
#[must_use]
pub fn layout(topology: &Topology, focus: &str, shown: &[&str]) -> Vec<Place> {
    let mut seen: HashSet<&str> = HashSet::new();
    let things: Vec<&TopologyNode> = topology
        .nodes
        .iter()
        .filter(|node| node.parent == focus)
        .filter(|node| shown.is_empty() || shown.contains(&node.id.as_str()))
        .filter(|node| seen.insert(node.id.as_str()))
        .collect();
    placed(topology, &things)
}

/// Where everything `topology` draws stands beneath every focus it has, each
/// focus laid out once and its things found in one pass over the drawing:
/// what a reader asks of a publication as it reads it, once
/// (`xmip_publication_layout_v1` keeps it with the read publication).
#[must_use]
pub fn every_focus(topology: &Topology) -> BTreeMap<String, Vec<Place>> {
    let mut seen: HashSet<(&str, &str)> = HashSet::with_capacity(topology.nodes.len());
    let mut beneath: HashMap<&str, Vec<&TopologyNode>> = HashMap::new();
    for node in &topology.nodes {
        if seen.insert((node.parent.as_str(), node.id.as_str())) {
            beneath.entry(node.parent.as_str()).or_default().push(node);
        }
    }
    beneath
        .into_iter()
        .map(|(focus, things)| (focus.to_string(), placed(topology, &things)))
        .collect()
}

/// Where `things`, all beneath one focus, stand: a cluster's by role, or
/// what a drill shows inside a node or a stage.
fn placed(topology: &Topology, things: &[&TopologyNode]) -> Vec<Place> {
    if things
        .iter()
        .any(|thing| matches!(thing.kind, NodeKind::Node | NodeKind::Party))
    {
        grid::cluster(things, &lines(topology, things))
    } else {
        grid::inside(things)
    }
}

/// `members` in the order the map stands them, band by band from the top,
/// column by column from the left, each column in its order — what the
/// Parties facing them are numbered by ([`super::party::facing`]).
#[must_use]
pub fn place_order(members: &[Member]) -> Vec<&Member> {
    let first = super::relate::listed(members)
        .first()
        .map(|member| member.node.as_str());
    let mut ordered: Vec<(&Member, (Band, Column))> = members
        .iter()
        .map(|member| {
            let (band, column, _) =
                grid::by_roles(&member.roles, Some(member.node.as_str()) == first);
            (member, (band, column))
        })
        .collect();
    ordered.sort_by(|(one, at), (other, other_at)| {
        (
            at,
            one.storage_place.is_none(),
            one.storage_place,
            one.node.as_bytes(),
        )
            .cmp(&(
                other_at,
                other.storage_place.is_none(),
                other.storage_place,
                other.node.as_bytes(),
            ))
    });
    ordered.into_iter().map(|(member, _)| member).collect()
}

/// The lines between `things` that place them: every link not observed
/// alone, each end drawn as the thing among `things` that holds it — itself
/// or its nearest ancestor — and none within one thing. Traffic no
/// configuration declares never moves a node.
fn lines<'a>(topology: &'a Topology, things: &[&TopologyNode]) -> Vec<(&'a str, &'a str)> {
    let parents: HashMap<&str, &str> = topology
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node.parent.as_str()))
        .collect();
    let drawn: HashSet<&str> = things.iter().map(|thing| thing.id.as_str()).collect();
    let holder = |id: &'a str| {
        let mut at = id;
        for _ in 0..=parents.len() {
            if drawn.contains(at) {
                return Some(at);
            }
            at = parents
                .get(at)
                .copied()
                .filter(|parent| !parent.is_empty())?;
        }
        None
    };
    // Each line once, in the order the drawing links them.
    let mut seen: HashSet<(&str, &str)> = HashSet::new();
    let mut lines = Vec::new();
    for link in topology
        .links
        .iter()
        .filter(|link| link.origin != Origin::Observed)
    {
        if let (Some(from), Some(to)) = (holder(&link.from), holder(&link.to))
            && from != to
            && seen.insert((from, to))
        {
            lines.push((from, to));
        }
    }
    lines
}

#[cfg(test)]
mod tests;
