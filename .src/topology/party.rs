//! The Parties outside a cluster's nodes and the links to them (ADR-0019;
//! ADR-0052, amendment 2026-09-29). The owner, 2026-09-29: *Something is
//! sending streams to a Xmip Node. A Xmip Node sends streams to somethings.*
//! One Party is drawn for every side it is on: the one that sends into a
//! receive stage, linked to each receive stage it sends into, and the one a
//! send stage delivers to, linked from each send stage that delivers to it —
//! never one box per far end. A link carries what its stage counted
//! (`Counted::at`) and the worst leaf of the endpoints beneath the stage, and
//! says which endpoint that was.
//!
//! Which Party it is, is the publisher's to say: the Playground's is its own
//! `party-x`, and a running node's is the one its configuration names, or
//! `any-party` while a configuration can name none (ADR-0018, amendment
//! 2026-09-30). Drawn here once for both; until 2026-09-30 the Playground
//! held it.

use node::Stage;

use super::draw::{CLUSTER, drawn, mood, node_id, worst};
use crate::counted::Counted;
use crate::snapshot::{HealthRecord, Snapshot};
use crate::topology::{NodeKind, Origin, Pattern, Topology, TopologyLink, TopologyNode};

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
}

/// What is drawn: the snapshot, the cluster's root scope and the Party.
struct Drawing<'a> {
    snapshot: &'a Snapshot,
    root: &'a str,
    party: &'a str,
}

/// The worst leaf beneath one stage's endpoints, the endpoint it is under,
/// and how many endpoints there are.
struct Facing {
    endpoints: usize,
    worst: Option<(String, HealthRecord)>,
}

/// Add `party` on each side any named node's stage faces, with a link to or
/// from each such stage; nothing on a side no stage faces. Call it once the
/// stages and their endpoints are drawn ([`super::draw::members`]).
pub fn draw(snapshot: &Snapshot, root: &str, names: &[&str], party: &str, topology: &mut Topology) {
    let drawing = Drawing {
        snapshot,
        root,
        party,
    };
    for side in [Side::Sending, Side::Receiving] {
        let mut facing = Vec::new();
        let mut links = Vec::new();
        for name in names {
            if let Some((stage, link)) = drawing.link(topology, name, side) {
                facing.push((*name, stage));
                links.push(link);
            }
        }
        if links.is_empty() {
            continue;
        }
        topology.nodes.push(drawing.party(side, &facing));
        topology.links.extend(links);
    }
}

impl Drawing<'_> {
    /// The id of the Party's box on `side`.
    fn party_id(&self, side: Side) -> String {
        format!("party/{}/{}", side.word(), self.party)
    }

    /// The endpoints the stage `stage_id` holds on the drawn topology, and
    /// the worst leaf beneath them in the snapshot's own order, worst first
    /// (ADR-0041: the worst record, not a rollup).
    fn facing(&self, topology: &Topology, stage_id: &str) -> Facing {
        let endpoints: Vec<&TopologyNode> = topology
            .nodes
            .iter()
            .filter(|node| node.parent == stage_id && node.kind == NodeKind::Endpoint)
            .collect();
        let worst = endpoints
            .iter()
            .filter_map(|endpoint| {
                worst(self.snapshot, &endpoint.scope).map(|record| (endpoint.label.clone(), record))
            })
            .min_by(|(_, a), (_, b)| a.standing().cmp(&b.standing()));
        Facing {
            endpoints: endpoints.len(),
            worst,
        }
    }

    /// The link between the Party on `side` and the node `name`'s stage
    /// that faces it, where that stage is drawn.
    fn link(&self, topology: &Topology, name: &str, side: Side) -> Option<(Facing, TopologyLink)> {
        let stage = side.stage();
        let stage_id = format!("{}/{}", node_id(name), stage.name());
        if !topology.nodes.iter().any(|node| node.id == stage_id) {
            return None;
        }
        let facing = self.facing(topology, &stage_id);
        let scope = format!("{}/node/{name}/{}", self.root, stage.name());
        let volume = self
            .snapshot
            .measure(&scope, Counted::at(stage))
            .map_or(0, |count| count.value);
        let (health, said) = mood(facing.worst.as_ref().map(|(_, record)| record));
        let party = self.party;
        let (from, to, ends) = match side {
            Side::Sending => (
                self.party_id(side),
                stage_id,
                format!("{party} sends into {name}"),
            ),
            Side::Receiving => (
                stage_id,
                self.party_id(side),
                format!("{name} delivers to {party}"),
            ),
        };
        let link = TopologyLink {
            id: format!("party/{}/{party}/{name}", side.word()),
            from,
            to,
            pattern: Pattern::SendReceive,
            // The stage is configured, and the Location accepts or presents
            // the Party; an endpoint reported on is observed.
            origin: if facing.endpoints == 0 {
                Origin::Configured
            } else {
                Origin::Both
            },
            protocol: over(&facing),
            state: health,
            volume,
            rate: 0.0,
            latency_ms: 0.0,
            progress: 0.0,
            attempts: 0,
            evidence: facing.worst.as_ref().map_or_else(
                || format!("{ends}; no transport reported yet"),
                |(endpoint, _)| {
                    let over = over(&facing);
                    format!("{ends} over {over}; worst over {endpoint}: {said}")
                },
            ),
        };
        Some((facing, link))
    }

    /// The Party's box on `side`: Fine or Holding over the worst leaf of
    /// every stage it faces (ADR-0041), and what that leaf said.
    fn party(&self, side: Side, facing: &[(&str, Facing)]) -> TopologyNode {
        let worse = facing
            .iter()
            .filter_map(|(name, facing)| {
                facing
                    .worst
                    .as_ref()
                    .map(|(endpoint, record)| (*name, endpoint, record))
            })
            .min_by(|(_, _, a), (_, _, b)| a.standing().cmp(&b.standing()));
        let scope = format!("{}/party/{}", self.root, self.party);
        let (state, said) = drawn(worse.map(|(_, _, record)| record), &scope);
        let names: Vec<&str> = facing.iter().map(|(name, _)| *name).collect();
        let verb = match side {
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
            id: self.party_id(side),
            parent: CLUSTER.to_string(),
            label: self.party.to_string(),
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
        }
    }
}

/// What a link runs over: the one endpoint, or how many.
fn over(facing: &Facing) -> String {
    match (facing.endpoints, &facing.worst) {
        (1, Some((endpoint, _))) => endpoint.clone(),
        (0, _) => "no transport reported".to_string(),
        (1, None) => "1 transport".to_string(),
        (count, _) => format!("{count} transports"),
    }
}
