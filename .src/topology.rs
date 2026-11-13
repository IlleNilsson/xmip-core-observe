//! The communication topology a publication carries beside its records: the
//! things that communicate and what passes between them (ADR-0052, amendment
//! 2026-09-14, ruling 3).
//!
//! The model and its words are written here once. A publisher builds a
//! [`Topology`] — drawn from what its nodes reported by [`draw`] and
//! [`party`], the one drawing the Essentials' roll and a running node both
//! call, and related by role by [`relate`] wherever a cluster is drawn —
//! and [`crate::publication`] writes and reads it; a surface gets it through
//! the runtime's reader (`xmip_operate.h` section 8) as the header's values,
//! never as the words, and where each thing stands from [`layout`], which
//! the reader computes (`xmip_publication_layout_v1`). Until 2026-09-24 the
//! Essentials held this model and `Xmip.Surface` parsed its words again
//! (open problem 25). What a value is called — its word, and the name a
//! person reads it by — is [`words`]', and a surface asks the runtime for it
//! by value (`xmip_topology_kind_words_v1` and its siblings, section 8).
//!
//! What moved is stated on the drawing by its publisher, over a window of
//! its own earlier readings ([`window`]): each link's rate and each drawn
//! thing's activity, per second.

pub mod draw;
pub mod from_to;
pub mod layout;
pub mod party;
pub mod relate;
pub mod window;
pub mod words;

pub use words::{NodeKind, Origin, Pattern, Relation};

/// What a link between two nodes runs over: the one way a node hands work
/// on to another, through a Storage node (`terminology.md`, Xmip Storage).
pub const XMIP_STORAGE: &str = "Xmip Storage";

/// What a link from a node declaring the operational or the monitoring role
/// runs over: each node's operate listener (ADR-0014, ADR-0063).
pub const XMIP_OPERATE: &str = "Xmip operate";

use serde::{Deserialize, Serialize};

use crate::health::Health;
use crate::publication::mood;

/// The id of the link `from` → `to`, related as `relation`: the one rule
/// every link is named by, so a relation configured and the traffic observed
/// over it meet and merge, and two relations joining one pair in one
/// direction — a node writing into a Storage node it also monitors — stay
/// two (ADR-0052, amendment 2026-11-13).
#[must_use]
pub fn link_id(from: &str, to: &str, relation: Relation) -> String {
    format!("{from}→{to} {}", relation.word())
}

/// The nodes and links a publication carries under `[topology]`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Topology {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub observed_unix_nanos: i64,
    #[serde(default)]
    pub nodes: Vec<TopologyNode>,
    #[serde(default)]
    pub links: Vec<TopologyLink>,
}

impl Topology {
    /// What the node `name` drew of itself, `node`, merged into this
    /// drawing — a roll's of the cluster (ADR-0052, amendment 2026-11-11).
    /// The node owns itself and what is beneath it, `node/<name>` and
    /// `node/<name>/…`, and nothing else: for an owned thing both drawings
    /// hold, the node's drawing wins — its activity, its load, what it alone
    /// knows — but for its mood, where this drawing's is worse, which comes
    /// with its evidence, and for its roles and its place in `[storage]
    /// nodes`, where it said none and this drawing did: a verdict only the
    /// reader published, a Receive Location's delivery, shows on the Location
    /// the node drew Fine. What this drawing holds of anything the node does
    /// not own stays; what it does not hold is added.
    ///
    /// A link is the node's where either end is, so its Party links, a
    /// Receive Location to its Subscription and the traffic its Storage node
    /// measured join the cluster's drawing as the node drew them. Where one
    /// of the two links sharing an id is configured and the other observed,
    /// the observed one is kept, configured and observed both
    /// ([`Origin::joined`]), whichever order the drawings merge in (ADR-0052,
    /// amendment 2026-11-13).
    pub fn merge(&mut self, name: &str, node: &Self) {
        let own = draw::node_id(name);
        let owned = |id: &str| {
            id.strip_prefix(&own)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
        };
        for drawn in &node.nodes {
            match self.nodes.iter_mut().find(|one| one.id == drawn.id) {
                Some(one) if owned(&drawn.id) => {
                    let mut merged = drawn.clone();
                    if one.state > drawn.state {
                        merged.state = one.state;
                        merged.evidence = std::mem::take(&mut one.evidence);
                    }
                    if merged.roles.is_empty() {
                        merged.roles = std::mem::take(&mut one.roles);
                    }
                    merged.place = merged.place.or(one.place);
                    *one = merged;
                }
                Some(_) => {}
                None => self.nodes.push(drawn.clone()),
            }
        }
        for drawn in &node.links {
            match self.links.iter_mut().find(|one| one.id == drawn.id) {
                Some(one) => {
                    let origin = one.origin.joined(drawn.origin);
                    let observed = |link: &TopologyLink| link.origin != Origin::Configured;
                    let theirs = if observed(drawn) == observed(one) {
                        owned(&drawn.from) || owned(&drawn.to)
                    } else {
                        observed(drawn)
                    };
                    if theirs {
                        *one = drawn.clone();
                    }
                    one.origin = origin;
                }
                None => self.links.push(drawn.clone()),
            }
        }
    }
}

/// One thing that communicates. `parent` is empty at the top; load runs
/// from zero to one. `activity` is what moved through it, per second, over
/// its publisher's [`window::Window`]: zero when nothing moved, as drawn
/// until a window states it (ADR-0052, amendment 2026-11-10). A node of the
/// cluster says the `roles` it declares, as `node::Capability::words` says
/// them, and — a Storage node listed in `[storage] nodes` — its `place` in
/// that list, from zero: what it is related by ([`relate`]), stands by
/// ([`layout`]) and is for (`node::NodeRole::purpose`), never its name
/// (ADR-0052 and ADR-0056, amendments 2026-11-13).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TopologyNode {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub parent: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub kind: NodeKind,
    #[serde(default)]
    pub scope: String,
    #[serde(default = "mood::unknown", with = "mood")]
    pub state: Health,
    #[serde(default)]
    pub origin: Origin,
    #[serde(default)]
    pub load: f64,
    #[serde(default)]
    pub activity: f64,
    #[serde(default)]
    pub evidence: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub roles: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<u32>,
}

/// One communication relationship, `from` one node id `to` another, and how
/// its two ends are related ([`Relation`]). `rate` is the rise in its volume
/// per second over its publisher's [`window::Window`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TopologyLink {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub to: String,
    #[serde(default)]
    pub pattern: Pattern,
    #[serde(default)]
    pub origin: Origin,
    #[serde(default)]
    pub relation: Relation,
    #[serde(default)]
    pub protocol: String,
    #[serde(default = "mood::unknown", with = "mood")]
    pub state: Health,
    #[serde(default)]
    pub volume: u64,
    #[serde(default)]
    pub rate: f64,
    #[serde(default)]
    pub latency_ms: f64,
    #[serde(default)]
    pub progress: f64,
    #[serde(default)]
    pub attempts: u32,
    #[serde(default)]
    pub evidence: String,
}

#[cfg(test)]
mod tests;
