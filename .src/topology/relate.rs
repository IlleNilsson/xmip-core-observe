//! How a cluster's nodes are related: by the roles each declares and the
//! order of `[storage] nodes`, labelled by what the bindings placed on each,
//! and never by a name (the owner, 2026-11-12: *I miss how the cluster nodes
//! are related*; *Related by its role in the cluster, not by name*; ADR-0052
//! and ADR-0056, amendments 2026-11-13).
//!
//! | Relation | From → to, by role |
//! | --- | --- |
//! | writes into | every member not declaring storage → the configured primary |
//! | hands work to | the configured primary → every member serving process or send |
//! | copies to | the configured primary → each later-listed Storage node |
//! | operates, monitors | a member declaring either → every other member |
//!
//! The configured primary is the Storage node `[storage] nodes` lists first
//! (`runtime-model.md` section 3): the configured lines never move with a
//! promotion, so after one the observed traffic draws at the new primary and
//! the configured line stays dashed, which is the truth. Standbys are reached
//! through *copies to*; every other node's line says which it asks if the
//! primary is gone. Each relation is drawn configured, its volume zero, until
//! traffic crosses it ([`super::Topology::merge`] joins the two by
//! [`super::link_id`]). A relation's mood is the worse of its two ends'
//! `<node>/reached` and `<node>/storage` records, with that record's words,
//! so a node that cannot reach Storage shows a troubled line, never a calm
//! one.
//!
//! The one pairing of stages the rig hands work along, receive to process to
//! send ([`handed`]), is this module's too (ADR-0052, amendment 2026-09-19).
//! A lone node's own publication draws no relation: relations are the
//! cluster's facts, drawn wherever a cluster is.

use node::{Capability, NodeRole, Stage};

use super::draw::{node_id, placed, worst};
use super::{Origin, Pattern, Relation, TopologyLink, XMIP_OPERATE, XMIP_STORAGE, link_id};
use crate::health::Health;
use crate::snapshot::{HealthRecord, Snapshot};

/// One member of a cluster as it is related and drawn: its name, an opaque
/// id here, the roles it declares and — a Storage node `[storage] nodes`
/// lists — its place in that list, from zero.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    pub node: String,
    pub roles: Vec<NodeRole>,
    pub storage_place: Option<u32>,
}

impl Member {
    /// `node`, declaring `roles`, at `storage_place` in `[storage] nodes`.
    #[must_use]
    pub fn new(node: &str, roles: &[NodeRole], storage_place: Option<u32>) -> Self {
        Self {
            node: node.to_string(),
            roles: NodeRole::said(roles),
            storage_place,
        }
    }

    /// Its roles as a drawn node says them (`node::Capability::words`).
    #[must_use]
    pub fn words(&self) -> String {
        Capability::of(&self.roles).words()
    }

    /// Whether it declares `role`.
    #[must_use]
    pub fn declares(&self, role: NodeRole) -> bool {
        self.roles.contains(&role)
    }

    /// Whether one of its roles serves `stage`.
    #[must_use]
    pub fn serves(&self, stage: Stage) -> bool {
        self.roles.iter().any(|role| role.stages().contains(&stage))
    }
}

/// The members declaring storage, in the order `[storage] nodes` lists
/// them, the configured primary first; one listed nowhere after them all.
#[must_use]
pub fn listed(members: &[Member]) -> Vec<&Member> {
    let mut storing: Vec<&Member> = members
        .iter()
        .filter(|member| member.declares(NodeRole::Storage))
        .collect();
    storing.sort_by(|one, other| {
        (one.storage_place.is_none(), one.storage_place)
            .cmp(&(other.storage_place.is_none(), other.storage_place))
            .then_with(|| one.node.as_bytes().cmp(other.node.as_bytes()))
    });
    storing
}

/// Every relation between `members` of the cluster at `root`, configured,
/// labelled by what `snapshot` holds beneath each node's stages and moody
/// as its ends' reach and Storage records say.
#[must_use]
pub fn relations(root: &str, members: &[Member], snapshot: &Snapshot) -> Vec<TopologyLink> {
    let relating = Relating {
        root,
        snapshot,
        listed: listed(members),
    };
    let mut links = Vec::new();
    if let Some(primary) = relating.listed.first() {
        for member in members.iter().filter(|member| member.node != primary.node) {
            if !member.declares(NodeRole::Storage) {
                links.push(relating.storage(member, primary, Relation::WritesInto));
            }
            if member.serves(Stage::Process) || member.serves(Stage::Send) {
                links.push(relating.storage(member, primary, Relation::HandsWorkTo));
            }
        }
        for standby in relating.listed.iter().skip(1) {
            links.push(relating.storage(standby, primary, Relation::CopiesTo));
        }
    }
    for member in members {
        let relation = match (
            member.declares(NodeRole::Operational),
            member.declares(NodeRole::Monitoring),
        ) {
            (true, true) => Relation::OperatesAndMonitors,
            (true, false) => Relation::Operates,
            (false, true) => Relation::Monitors,
            (false, false) => continue,
        };
        for other in members.iter().filter(|other| other.node != member.node) {
            links.push(relating.operate(member, other, relation));
        }
    }
    links
}

/// Every pair of stages work is handed along between `members`: from each
/// that receives to each that processes, and from each that processes to
/// each that sends, in the order they are given — the rig's simulated
/// hand-off (`test/core/essentials`, `handoff.rs`) and the one pairing.
#[must_use]
pub fn handed(members: &[Member]) -> Vec<(String, Stage, String, Stage)> {
    [
        (Stage::Receive, Stage::Process),
        (Stage::Process, Stage::Send),
    ]
    .into_iter()
    .flat_map(|(from, to)| {
        members
            .iter()
            .filter(move |sender| sender.serves(from))
            .flat_map(move |sender| {
                members
                    .iter()
                    .filter(move |receiver| receiver.serves(to))
                    .map(move |receiver| (sender.node.clone(), from, receiver.node.clone(), to))
            })
    })
    .collect()
}

/// What the relations are drawn from.
struct Relating<'a> {
    root: &'a str,
    snapshot: &'a Snapshot,
    listed: Vec<&'a Member>,
}

impl Relating<'_> {
    /// A relation over Xmip Storage between `member` and `storage`, the
    /// configured primary — or, copied to, the standby `member`.
    fn storage(&self, member: &Member, primary: &Member, relation: Relation) -> TopologyLink {
        let (from, to) = match relation {
            Relation::WritesInto => (member, primary),
            _ => (primary, member),
        };
        let standbys: Vec<&str> = self
            .listed
            .iter()
            .skip(1)
            .map(|standby| standby.node.as_str())
            .filter(|standby| *standby != member.node)
            .collect();
        let asked = match (relation, standbys.as_slice()) {
            (Relation::CopiesTo, _) | (_, []) => String::new(),
            (_, [one]) => format!("; {one}, a standby, is asked if {} is gone", primary.node),
            (_, many) => format!(
                "; {}, standbys, are asked if {} is gone",
                many.join(", "),
                primary.node
            ),
        };
        let placed = match relation {
            Relation::CopiesTo => String::new(),
            _ => self.placed(member),
        };
        let pattern = match relation {
            Relation::CopiesTo => Pattern::Streaming,
            _ => Pattern::PublishConsume,
        };
        let said = format!(
            "{} {} {}{placed}{asked}",
            from.node,
            relation.name(),
            to.node
        );
        self.link((from, to), relation, (pattern, XMIP_STORAGE), said)
    }

    /// A relation over the operate listener from `member` to `other`.
    fn operate(&self, member: &Member, other: &Member, relation: Relation) -> TopologyLink {
        let said = format!("{} {} {}", member.node, relation.name(), other.node);
        let ends = (member, other);
        self.link(
            ends,
            relation,
            (Pattern::RequestResponse, XMIP_OPERATE),
            said,
        )
    }

    /// The link, configured, its mood the worse of its ends' reach and
    /// Storage records with that record's words.
    fn link(
        &self,
        (from, to): (&Member, &Member),
        relation: Relation,
        (pattern, protocol): (Pattern, &str),
        said: String,
    ) -> TopologyLink {
        let (state, evidence) = match self.worse(from, to) {
            Some(record) if record.health != Health::Fine => (
                record.health,
                format!("{said}; {}: {}", record.scope, record.evidence),
            ),
            _ => (Health::Fine, said),
        };
        let (from, to) = (node_id(&from.node), node_id(&to.node));
        TopologyLink {
            id: link_id(&from, &to, relation),
            from,
            to,
            pattern,
            origin: Origin::Configured,
            relation,
            protocol: protocol.to_string(),
            state,
            volume: 0,
            rate: 0.0,
            latency_ms: 0.0,
            progress: 0.0,
            attempts: 0,
            evidence,
        }
    }

    /// The worst of each end's `<node>/reached` and `<node>/storage`.
    fn worse(&self, one: &Member, other: &Member) -> Option<HealthRecord> {
        [one, other]
            .into_iter()
            .flat_map(|member| {
                ["reached", "storage"]
                    .map(|leaf| format!("{}/node/{}/{leaf}", self.root, member.node))
            })
            .filter_map(|scope| worst(self.snapshot, &scope))
            .min_by(|a, b| a.standing().cmp(&b.standing()))
    }

    /// What is placed on `member`, each stage it serves by its title: the
    /// Locations, Subscriptions or Ports beneath it, or none placed.
    fn placed(&self, member: &Member) -> String {
        let scope = format!("{}/node/{}", self.root, member.node);
        Stage::ALL
            .into_iter()
            .filter(|stage| member.serves(*stage))
            .map(|stage| {
                let beneath: Vec<String> =
                    placed(self.snapshot, &scope, stage).into_iter().collect();
                if beneath.is_empty() {
                    format!("; {}: none placed", stage.title())
                } else {
                    format!("; {}: {}", stage.title(), beneath.join(", "))
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
