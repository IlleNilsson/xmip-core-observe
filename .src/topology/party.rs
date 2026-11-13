//! The Parties outside a cluster's nodes and the links to them (ADR-0019;
//! ADR-0052, amendment 2026-09-29). The owner, 2026-09-29: *Something is
//! sending streams to a Xmip Node. A Xmip Node sends streams to somethings.*
//! A link carries what its stage counted over the Party's transports
//! (`Counted::at`) and the worst leaf of the Party's endpoints beneath the
//! stage, and says which endpoint that was.
//!
//! Which Parties there are is the publisher's to say. Where its
//! configuration names them ([`Party`], [`draw`]) — the Essentials'
//! `[[parties]]`, each on its share of the transports (ADR-0028, amendment
//! 2026-10-13) — each is drawn once for every side it is on, linked to each
//! stage it faces. Where its publication cannot say which Parties a
//! Location's `accept.party` takes (ADR-0018, amendment 2026-09-30; open
//! problem 33) — a running node, a node a roll reads — one Party is drawn
//! per node facing outside, on each side ([`facing`]): [`TX_PARTY`] 1, 2, …,
//! each sending into one receiving or executing node, and [`RX_PARTY`] 1,
//! 2, …, each delivered to by one sending or executing node, numbered per
//! side in the order the nodes stand on the map (`layout::place_order`),
//! never by a name (the owner, 2026-11-12; the labels his, 2026-11-13;
//! ADR-0052, amendment 2026-11-13). Drawn here once for both.

mod over;

use node::Stage;

use super::draw::{CLUSTER, drawn, mood, node_id, worst};
use super::{Relation, link_id};
use crate::counted::Counted;
use crate::snapshot::{HealthRecord, Snapshot};
use crate::topology::{NodeKind, Origin, Pattern, Topology, TopologyLink, TopologyNode};

/// What a Party sending into one node is labelled, numbered after it.
pub const TX_PARTY: &str = "TX-Party";

/// What a Party one node delivers to is labelled, numbered after it.
pub const RX_PARTY: &str = "RX-Party";

/// A Party to draw, and the endpoints — by the label each is drawn with —
/// it is reached over on each side. `None` on a side is every endpoint a
/// stage holds; an empty list is not on that side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Party<'a> {
    pub name: &'a str,
    /// What it sends into the receive stages over.
    pub sends: Option<&'a [String]>,
    /// What the send stages deliver to it over.
    pub receives: Option<&'a [String]>,
}

/// Which side of the cluster a Party is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    /// It sends into the cluster's receive stages.
    Sending,
    /// The cluster's send stages deliver to it.
    Receiving,
}

impl Side {
    /// The stage of the message path that faces a Party on this side.
    const fn stage(self) -> Stage {
        match self {
            Side::Sending => Stage::Receive,
            Side::Receiving => Stage::Send,
        }
    }

    /// The segment of the Party's id that tells its two boxes apart.
    const fn word(self) -> &'static str {
        match self {
            Side::Sending => "sending",
            Side::Receiving => "receiving",
        }
    }

    /// What a Party facing one node on this side is called.
    const fn numbered(self) -> &'static str {
        match self {
            Side::Sending => TX_PARTY,
            Side::Receiving => RX_PARTY,
        }
    }
}

/// What is drawn: the snapshot, the cluster's root scope, the Party — its
/// id and what it is called — and the side.
struct Drawing<'a> {
    snapshot: &'a Snapshot,
    root: &'a str,
    id: String,
    /// The scope its box is drawn at: a named Party's `<root>/party/<name>`,
    /// one facing a node its id beneath the root.
    scope: String,
    party: String,
    side: Side,
    /// The endpoints it is reached over on this side; `None`, every one.
    over: Option<&'a [String]>,
    /// Each Location by its name, and the transport its address chose:
    /// empty where the endpoints are transports themselves.
    transports: &'a [(String, String)],
}

/// The Party's endpoints beneath one stage, the worst leaf beneath them, the
/// endpoint it is under, and how many there are.
struct Facing {
    endpoints: Vec<String>,
    worst: Option<(String, HealthRecord)>,
}

/// Add each of `parties` on each side any named node's stage faces it,
/// with a link to or from each such stage; nothing on a side no stage
/// faces. Call it once the stages and their endpoints are drawn
/// ([`super::draw::members`]).
pub fn draw(
    snapshot: &Snapshot,
    root: &str,
    names: &[&str],
    parties: &[Party<'_>],
    topology: &mut Topology,
) {
    for party in parties {
        for (side, over) in [
            (Side::Sending, party.sends),
            (Side::Receiving, party.receives),
        ] {
            if over.is_some_and(<[String]>::is_empty) {
                continue;
            }
            let drawing = Drawing {
                snapshot,
                root,
                id: format!("party/{}/{}", side.word(), party.name),
                scope: format!("{root}/party/{}", party.name),
                party: party.name.to_string(),
                side,
                over,
                transports: &[],
            };
            drawing.side(topology, names);
        }
    }
}

/// Add one Party per node of `names` facing outside, on each side its stage
/// faces it, over every endpoint, each Location reached over the transport
/// `transports` names it with: [`TX_PARTY`] and [`RX_PARTY`], numbered per
/// side in the order `names` gives, the order the nodes stand on the map.
/// Its id names the node it faces, so a node's own drawing and the
/// cluster's meet whatever number each gave it.
pub fn facing(
    snapshot: &Snapshot,
    root: &str,
    names: &[&str],
    transports: &[(String, String)],
    topology: &mut Topology,
) {
    for side in [Side::Sending, Side::Receiving] {
        let mut number = 1;
        for name in names {
            let id = format!("party/{}/{}", side.word(), node_id(name));
            let drawing = Drawing {
                snapshot,
                root,
                scope: format!("{root}/{id}"),
                id,
                party: format!("{} {number}", side.numbered()),
                side,
                over: None,
                transports,
            };
            if drawing.side(topology, &[name]) {
                number += 1;
            }
        }
    }
}

impl Drawing<'_> {
    /// The Party on this side, linked to each of the nodes `names` whose
    /// stage faces it: whether any did.
    fn side(&self, topology: &mut Topology, names: &[&str]) -> bool {
        let mut facing = Vec::new();
        let mut links = Vec::new();
        for name in names {
            if let Some((stage, link)) = self.link(topology, name) {
                facing.push((*name, stage));
                links.push(link);
            }
        }
        if links.is_empty() {
            return false;
        }
        topology.nodes.push(self.party(&facing));
        topology.links.extend(links);
        true
    }

    /// Whether the Party is reached over the endpoint labeled `label`.
    fn reaches(&self, label: &str) -> bool {
        self.over
            .is_none_or(|over| over.iter().any(|one| one == label))
    }

    /// The Party's endpoints the stage `stage_id` holds on the drawn
    /// topology, and the worst leaf beneath them in the snapshot's own
    /// order, worst first (ADR-0041: the worst record, not a rollup).
    /// `None` where the stage holds endpoints and none is the Party's.
    fn facing(&self, topology: &Topology, stage_id: &str) -> Option<Facing> {
        let held: Vec<&TopologyNode> = topology
            .nodes
            .iter()
            .filter(|node| node.parent == stage_id && node.kind == NodeKind::Endpoint)
            .collect();
        let mine: Vec<&TopologyNode> = held
            .iter()
            .copied()
            .filter(|endpoint| self.reaches(&endpoint.label))
            .collect();
        if mine.is_empty() && !held.is_empty() {
            return None;
        }
        let worst = mine
            .iter()
            .filter_map(|endpoint| {
                worst(self.snapshot, &endpoint.scope).map(|record| (endpoint.label.clone(), record))
            })
            .min_by(|(_, a), (_, b)| a.standing().cmp(&b.standing()));
        Some(Facing {
            endpoints: mine.iter().map(|endpoint| endpoint.label.clone()).collect(),
            worst,
        })
    }

    /// What the stage at `scope` counted over the Party's endpoints: the
    /// whole stage's count where it is reached over every one.
    fn volume(&self, scope: &str, stage: Stage, facing: &Facing) -> u64 {
        let measured = |at: &str| {
            self.snapshot
                .measure(at, Counted::at(stage))
                .map_or(0, |count| count.value)
        };
        match self.over {
            None => measured(scope),
            Some(_) => facing
                .endpoints
                .iter()
                .map(|endpoint| measured(&format!("{scope}/{endpoint}")))
                .sum(),
        }
    }

    /// The link between the Party and the node `name`'s stage that faces
    /// it, where that stage is drawn and holds the Party's endpoints or
    /// none yet.
    fn link(&self, topology: &Topology, name: &str) -> Option<(Facing, TopologyLink)> {
        let stage = self.side.stage();
        let stage_id = format!("{}/{}", node_id(name), stage.name());
        if !topology.nodes.iter().any(|node| node.id == stage_id) {
            return None;
        }
        let facing = self.facing(topology, &stage_id)?;
        let scope = format!("{}/node/{name}/{}", self.root, stage.name());
        let volume = self.volume(&scope, stage, &facing);
        let (health, said) = mood(facing.worst.as_ref().map(|(_, record)| record));
        let party = &self.party;
        let (from, to, ends) = match self.side {
            Side::Sending => (
                self.id.clone(),
                stage_id,
                format!("{party} sends into {name}"),
            ),
            Side::Receiving => (
                stage_id,
                self.id.clone(),
                format!("{name} delivers to {party}"),
            ),
        };
        let link = TopologyLink {
            id: link_id(&from, &to, Relation::Traffic),
            from,
            to,
            pattern: Pattern::SendReceive,
            // The stage is configured, and the Location accepts or presents
            // the Party; an endpoint reported on is observed.
            origin: if facing.endpoints.is_empty() {
                Origin::Configured
            } else {
                Origin::Both
            },
            relation: Relation::Traffic,
            protocol: self.over_what(&facing),
            state: health,
            volume,
            rate: 0.0,
            latency_ms: 0.0,
            progress: 0.0,
            attempts: 0,
            evidence: facing.worst.as_ref().map_or_else(
                || format!("{ends}; no transport reported yet"),
                |(endpoint, _)| {
                    let over = self.over_what(&facing);
                    let at = self.at_what(&facing);
                    format!("{ends} over {over}{at}; worst over {endpoint}: {said}")
                },
            ),
        };
        Some((facing, link))
    }

    /// The Party's box on this side: Fine or Holding over the worst leaf of
    /// every stage it faces (ADR-0041), and what that leaf said.
    fn party(&self, facing: &[(&str, Facing)]) -> TopologyNode {
        let worse = facing
            .iter()
            .filter_map(|(name, facing)| {
                facing
                    .worst
                    .as_ref()
                    .map(|(endpoint, record)| (*name, endpoint, record))
            })
            .min_by(|(_, _, a), (_, _, b)| a.standing().cmp(&b.standing()));
        let scope = self.scope.clone();
        let (state, said) = drawn(worse.map(|(_, _, record)| record), &scope);
        let names: Vec<&str> = facing.iter().map(|(name, _)| *name).collect();
        let verb = match self.side {
            Side::Sending => "sends into",
            Side::Receiving => "is delivered to by",
        };
        let evidence = worse.map_or_else(
            || format!("{verb} {}; no transport reported yet", names.join(", ")),
            |(name, endpoint, _)| {
                format!(
                    "{verb} {}; worst at {name} over {endpoint}: {said}",
                    names.join(", ")
                )
            },
        );
        TopologyNode {
            id: self.id.clone(),
            parent: CLUSTER.to_string(),
            label: self.party.clone(),
            kind: NodeKind::Party,
            scope,
            state,
            origin: if worse.is_some() {
                Origin::Both
            } else {
                Origin::Configured
            },
            load: 0.0,
            activity: 0.0,
            evidence,
            roles: String::new(),
            place: None,
        }
    }
}

#[cfg(test)]
mod tests;
