//! The Parties outside a cluster's nodes and the links to them (ADR-0019;
//! ADR-0052, amendment 2026-09-29). The owner, 2026-09-29: *Something is
//! sending streams to a Xmip Node. A Xmip Node sends streams to somethings.*
//! Each Party is drawn for every side it is on: the one that sends into a
//! receive stage, linked to each receive stage it sends into, and the one a
//! send stage delivers to, linked from each send stage that delivers to it —
//! never one box per far end. A link carries what its stage counted over the
//! Party's transports (`Counted::at`) and the worst leaf of the Party's
//! endpoints beneath the stage, and says which endpoint that was.
//!
//! Which Parties there are, and over which transports each is reached on
//! each side, is the publisher's to say ([`Party`]): the Playground's are its
//! configuration's `[[parties]]`, each on its share of the transports
//! (ADR-0028, amendment 2026-10-13); a running node's is `any-party`, over
//! every endpoint, while its publication does not say which Parties a
//! Location's `accept.party` takes (ADR-0018, amendment 2026-09-30; open
//! problem 33). Drawn here once for both.

use node::Stage;

use super::draw::{CLUSTER, drawn, mood, node_id, worst};
use crate::counted::Counted;
use crate::snapshot::{HealthRecord, Snapshot};
use crate::topology::{NodeKind, Origin, Pattern, Topology, TopologyLink, TopologyNode};

/// A Party to draw, and the endpoints — by the transport each is labelled
/// with — it is reached over on each side. `None` on a side is every
/// endpoint a stage holds; an empty list is not on that side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Party<'a> {
    pub name: &'a str,
    /// What it sends into the receive stages over.
    pub sends: Option<&'a [String]>,
    /// What the send stages deliver to it over.
    pub receives: Option<&'a [String]>,
}

impl<'a> Party<'a> {
    /// `name`, on both sides over every endpoint.
    #[must_use]
    pub const fn everywhere(name: &'a str) -> Self {
        Self {
            name,
            sends: None,
            receives: None,
        }
    }
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
}

/// What is drawn: the snapshot, the cluster's root scope, the Party and
/// the side.
struct Drawing<'a> {
    snapshot: &'a Snapshot,
    root: &'a str,
    party: &'a str,
    side: Side,
    /// The endpoints it is reached over on this side; `None`, every one.
    over: Option<&'a [String]>,
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
                party: party.name,
                side,
                over,
            };
            drawing.side(topology, names);
        }
    }
}

impl Drawing<'_> {
    fn side(&self, topology: &mut Topology, names: &[&str]) {
        let mut facing = Vec::new();
        let mut links = Vec::new();
        for name in names {
            if let Some((stage, link)) = self.link(topology, name) {
                facing.push((*name, stage));
                links.push(link);
            }
        }
        if links.is_empty() {
            return;
        }
        topology.nodes.push(self.party(&facing));
        topology.links.extend(links);
    }

    /// The id of the Party's box on this side.
    fn party_id(&self) -> String {
        format!("party/{}/{}", self.side.word(), self.party)
    }

    /// Whether the Party is reached over the endpoint labelled `label`.
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
        let party = self.party;
        let (from, to, ends) = match self.side {
            Side::Sending => (
                self.party_id(),
                stage_id,
                format!("{party} sends into {name}"),
            ),
            Side::Receiving => (
                stage_id,
                self.party_id(),
                format!("{name} delivers to {party}"),
            ),
        };
        let link = TopologyLink {
            id: format!("party/{}/{party}/{name}", self.side.word()),
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
        let scope = format!("{}/party/{}", self.root, self.party);
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
            id: self.party_id(),
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
    match facing.endpoints.as_slice() {
        [] => "no transport reported".to_string(),
        [one] => one.clone(),
        many => format!("{} transports", many.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::{Party, draw};
    use crate::health::Health;
    use crate::snapshot::{Count, HealthRecord, Snapshot};
    use crate::topology::{Topology, draw::members};
    use crate::{Counted, NodeKind};

    use configure::fixture::test_cluster;

    fn root() -> String {
        test_cluster().scope()
    }

    /// The test cluster's first node, the one drawn.
    fn node() -> String {
        test_cluster().node(0).name.clone()
    }

    fn health(snapshot: &mut Snapshot, leaf: &str, health: Health) {
        snapshot.record_health(HealthRecord {
            scope: format!("{}/node/{}/{leaf}", root(), node()),
            health,
            severity: if health == Health::Fine { 0 } else { 60 },
            evidence: format!("{leaf} said so"),
            observed_unix_nanos: 7,
        });
    }

    fn count(snapshot: &mut Snapshot, at: &str, counted: Counted, value: u64) {
        snapshot.record_count(Count {
            scope: format!("{}/node/{}/{at}", root(), node()),
            counted,
            value,
            window_start_unix_nanos: 7,
            window_end_unix_nanos: 7,
            observed_unix_nanos: 7,
        });
    }

    /// The test cluster's first node receiving over `a` and `b` and sending over `a`, `b`
    /// failed at receive; two Parties, the first sending over `a` and
    /// receiving over `b`, the second the other way round.
    fn drawn() -> Topology {
        let mut snapshot = Snapshot::new();
        health(&mut snapshot, "receive/a/json", Health::Fine);
        health(&mut snapshot, "receive/b/json", Health::Done);
        health(&mut snapshot, "send/a/json", Health::Fine);
        count(&mut snapshot, "receive/a", Counted::Streams, 3);
        count(&mut snapshot, "receive/b", Counted::Streams, 4);
        count(&mut snapshot, "send/a", Counted::Messages, 5);
        let mut topology = Topology {
            source: String::new(),
            observed_unix_nanos: 9,
            nodes: members(&snapshot, &root(), &[node().as_str()]),
            links: Vec::new(),
        };
        let (a, b) = (["a".to_string()], ["b".to_string()]);
        let parties = [
            Party {
                name: "one",
                sends: Some(&a),
                receives: Some(&b),
            },
            Party {
                name: "two",
                sends: Some(&b),
                receives: Some(&a),
            },
        ];
        draw(
            &snapshot,
            &root(),
            &[node().as_str()],
            &parties,
            &mut topology,
        );
        topology
    }

    #[test]
    fn each_party_is_drawn_over_its_own_transports_on_each_side() {
        let topology = drawn();
        let parties: Vec<&str> = topology
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Party)
            .map(|node| node.id.as_str())
            .collect();
        // The first is delivered to over b, which no send stage holds.
        assert_eq!(
            parties,
            [
                "party/sending/one",
                "party/sending/two",
                "party/receiving/two"
            ]
        );
        let link = |id: &str| {
            topology
                .links
                .iter()
                .find(|link| link.id == id)
                .unwrap_or_else(|| panic!("{id}"))
        };
        let one = link(&format!("party/sending/one/{}", node()));
        assert_eq!(
            (one.protocol.as_str(), one.volume, one.state),
            ("a", 3, Health::Fine)
        );
        let two = link(&format!("party/sending/two/{}", node()));
        assert_eq!(
            (two.protocol.as_str(), two.volume, two.state),
            ("b", 4, Health::Done)
        );
        assert_eq!(link(&format!("party/receiving/two/{}", node())).volume, 5);
    }

    #[test]
    fn a_party_everywhere_faces_every_endpoint_and_the_whole_count() {
        let mut snapshot = Snapshot::new();
        health(&mut snapshot, "receive/a/json", Health::Fine);
        health(&mut snapshot, "receive/b/json", Health::Fine);
        count(&mut snapshot, "receive", Counted::Streams, 9);
        let mut topology = Topology {
            source: String::new(),
            observed_unix_nanos: 9,
            nodes: members(&snapshot, &root(), &[node().as_str()]),
            links: Vec::new(),
        };
        draw(
            &snapshot,
            &root(),
            &[node().as_str()],
            &[Party::everywhere("any")],
            &mut topology,
        );
        let link = &topology.links[0];
        assert_eq!((link.protocol.as_str(), link.volume), ("2 transports", 9));
    }
}
