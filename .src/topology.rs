//! The communication topology a publication carries beside its records: the
//! things that communicate and what passes between them (ADR-0052, amendment
//! 2026-09-14, ruling 3).
//!
//! The model and its words are written here once. A publisher builds a
//! [`Topology`] — drawn from what its nodes reported by [`draw`] and
//! [`party`], the one drawing the Essentials' roll and a running node both
//! call — and [`crate::publication`] writes and reads it; a surface gets
//! it through the runtime's reader (`xmip_operate.h` section 8) as the
//! header's values, never as the words. Until 2026-09-24 the Essentials held
//! this model and `Xmip.Surface` parsed its words again (open problem 25).
//! What a value is called — its word, and the name a person reads it by — a
//! surface asks the runtime for by value (`xmip_topology_kind_words_v1` and
//! its two siblings, section 8), and until 2026-09-25 the web GUI kept a copy
//! of both.
//!
//! A word a reader does not know reads as the kind's fallback — a computer,
//! both origins, a send-receive exchange — so an unfamiliar publication still
//! draws, and the one reader decides that once.
//!
//! What moved is stated on the drawing by its publisher, over a window of
//! its own earlier readings ([`window`]): each link's rate and each drawn
//! thing's activity, per second.

pub mod draw;
pub mod from_to;
pub mod party;
pub mod window;

/// What a link between two nodes runs over: the one way a node hands work
/// on to another, through a Storage node (`terminology.md`, Xmip Storage).
pub const XMIP_STORAGE: &str = "Xmip Storage";

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::health::Health;
use crate::publication::mood;

/// A closed set of words, each variant one word and the name a person reads
/// it by, with the variant a word no variant is called reads as.
macro_rules! worded {
    (
        $kind:ident, $unknown:ident,
        { $($variant:ident => $word:literal as $name:literal),+ $(,)? }
    ) => {
        impl $kind {
            /// Every variant, in the order the header numbers them.
            pub const ALL: &'static [$kind] = &[$($kind::$variant),+];

            /// The word a publication writes.
            #[must_use]
            pub const fn word(self) -> &'static str {
                match self {
                    $($kind::$variant => $word),+
                }
            }

            /// What a surface calls it where a person reads it — under a
            /// node on the canvas, in the legend, in the inspector's rows —
            /// written here once, so no surface keeps its own (ADR-0052,
            /// amendment 2026-09-25).
            #[must_use]
            pub const fn name(self) -> &'static str {
                match self {
                    $($kind::$variant => $name),+
                }
            }

            /// The variant a word names, exactly, or `None`.
            #[must_use]
            pub fn named(word: &str) -> Option<Self> {
                match word {
                    $($word => Some($kind::$variant),)+
                    _ => None,
                }
            }
        }

        impl Default for $kind {
            /// What a word no variant is called reads as.
            fn default() -> Self {
                $kind::$unknown
            }
        }

        impl Serialize for $kind {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.word())
            }
        }

        impl<'de> Deserialize<'de> for $kind {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let word = String::deserialize(deserializer)?;
                Ok(Self::named(&word).unwrap_or_default())
            }
        }
    };
}

/// The kind of thing shown in the topology.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Computer,
    Server,
    VirtualMachine,
    Gateway,
    Appliance,
    Service,
    Process,
    Interface,
    Port,
    Protocol,
    Location,
    /// An Xmip cluster: the root its nodes hang under.
    Cluster,
    /// One Xmip node of a cluster.
    Node,
    /// A stage of the message path on a node.
    Stage,
    /// A receiving or sending endpoint of a stage, one per transport.
    Endpoint,
    /// A Party outside the cluster (ADR-0019): one that sends into a
    /// Receive Location, or one a Send Location delivers to. Which of the
    /// two a Party is drawn as is said by its links, never by its name.
    Party,
}

worded!(NodeKind, Computer, {
    Computer => "computer" as "computer",
    Server => "server" as "server",
    VirtualMachine => "virtual-machine" as "virtual machine",
    Gateway => "gateway" as "gateway",
    Appliance => "appliance" as "appliance",
    Service => "service" as "service",
    Process => "process" as "process",
    Interface => "interface" as "interface",
    Port => "port" as "port",
    Protocol => "protocol" as "protocol",
    Location => "location" as "location",
    Cluster => "cluster" as "cluster",
    Node => "node" as "node",
    Stage => "stage" as "stage",
    Endpoint => "endpoint" as "endpoint",
    Party => "party" as "party",
});

/// Where a topology fact came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// Declared by configuration but not observed in the current window.
    Configured,
    /// Observed at runtime but not present in the loaded configuration.
    Observed,
    /// Both configured and observed.
    Both,
}

worded!(Origin, Both, {
    Configured => "configured" as "configured",
    Observed => "observed" as "observed",
    Both => "both" as "configured and observed",
});

/// The application meaning of communication, separate from wire traffic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pattern {
    RequestResponse,
    SendReceive,
    PublishConsume,
    Streaming,
    FireAndForget,
    Session,
    Retry,
}

worded!(Pattern, SendReceive, {
    RequestResponse => "request-response" as "Request / response",
    SendReceive => "send-receive" as "Send → receive",
    PublishConsume => "publish-consume" as "Publish → consume",
    Streaming => "streaming" as "Streaming",
    FireAndForget => "fire-and-forget" as "Fire-and-forget",
    Session => "session" as "Session",
    Retry => "retry" as "Retry",
});

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
    /// with its evidence: a verdict only the reader published, a Receive
    /// Location's delivery, shows on the Location the node drew Fine. What
    /// this drawing holds of anything the node does not own stays; what it
    /// does not hold is added. A link is the node's where either end is, so
    /// its Party links, a Receive Location to its Subscription and the
    /// traffic its Storage node measured from and to the other nodes join
    /// the cluster's drawing as the node drew them.
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
                    *one = merged;
                }
                Some(_) => {}
                None => self.nodes.push(drawn.clone()),
            }
        }
        for drawn in &node.links {
            match self.links.iter_mut().find(|one| one.id == drawn.id) {
                Some(one) if owned(&drawn.from) || owned(&drawn.to) => *one = drawn.clone(),
                Some(_) => {}
                None => self.links.push(drawn.clone()),
            }
        }
    }
}

/// One thing that communicates. `parent` is empty at the top; load runs
/// from zero to one. `activity` is what moved through it, per second, over
/// its publisher's [`window::Window`]: zero when nothing moved, as drawn
/// until a window states it (ADR-0052, amendment 2026-11-10).
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
}

/// One communication relationship, `from` one node id `to` another. `rate`
/// is the rise in its volume per second over its publisher's
/// [`window::Window`].
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
mod tests {
    use super::*;
    use configure::fixture::test_cluster;

    #[test]
    fn every_word_reads_back_and_an_unknown_one_reads_as_the_fallback() {
        for kind in NodeKind::ALL {
            assert_eq!(NodeKind::named(kind.word()), Some(*kind));
        }
        for origin in Origin::ALL {
            assert_eq!(Origin::named(origin.word()), Some(*origin));
        }
        for pattern in Pattern::ALL {
            assert_eq!(Pattern::named(pattern.word()), Some(*pattern));
        }
        assert_eq!(NodeKind::VirtualMachine.word(), "virtual-machine");
        assert_eq!(NodeKind::VirtualMachine.name(), "virtual machine");
        assert_eq!(Origin::Both.name(), "configured and observed");
        assert_eq!(Pattern::PublishConsume.name(), "Publish → consume");
        for pattern in Pattern::ALL {
            assert_ne!(
                pattern.name(),
                pattern.word(),
                "a pattern is named, not spelled"
            );
        }
        assert_eq!(NodeKind::named("party"), Some(NodeKind::Party));
        assert_eq!(NodeKind::named("mainframe"), None);
        assert_eq!(NodeKind::default(), NodeKind::Computer);
        assert_eq!(Origin::default(), Origin::Both);
        assert_eq!(Pattern::default(), Pattern::SendReceive);
    }

    fn drawn(id: &str, label: &str) -> TopologyNode {
        TopologyNode {
            id: id.to_string(),
            parent: String::new(),
            label: label.to_string(),
            kind: NodeKind::Node,
            scope: String::new(),
            state: Health::Fine,
            origin: Origin::Both,
            load: 0.0,
            activity: 0.0,
            evidence: String::new(),
        }
    }

    fn linked(from: &str, to: &str, volume: u64) -> TopologyLink {
        TopologyLink {
            id: format!("{from}→{to}"),
            from: from.to_string(),
            to: to.to_string(),
            pattern: Pattern::PublishConsume,
            origin: Origin::Observed,
            protocol: XMIP_STORAGE.to_string(),
            state: Health::Fine,
            volume,
            rate: 0.0,
            latency_ms: 0.0,
            progress: 0.0,
            attempts: 0,
            evidence: String::new(),
        }
    }

    #[test]
    fn a_nodes_own_drawing_wins_for_what_it_owns_but_a_worse_mood_and_its_links_join() {
        let test = test_cluster();
        let (receiver, storer) = (&test.node(0).name, &test.node(1).name);
        let (receiving, storing) = (format!("node/{receiver}"), format!("node/{storer}"));
        let stage = format!("{receiving}/receive");
        let location = format!("{stage}/Sales orders from Party-A");
        let struck = TopologyNode {
            state: Health::Holding,
            evidence: "a document no Subscription takes".to_string(),
            ..drawn(&location, "the roll's")
        };
        let mut cluster = Topology {
            nodes: vec![
                drawn("cluster", "the roll's"),
                drawn(&receiving, "the roll's"),
                drawn(&storing, "the roll's"),
                struck,
            ],
            links: vec![linked("party/sending/any-party", &stage, 3)],
            ..Topology::default()
        };
        let storage = Topology {
            nodes: vec![
                drawn("cluster", "the node's"),
                drawn(&storing, "its own"),
                drawn(&receiving, "drawn by another"),
            ],
            links: vec![linked(&receiving, &storing, 7)],
            ..Topology::default()
        };
        let own_drawing = Topology {
            nodes: vec![
                drawn(&receiving, "its own"),
                drawn(&stage, "Receive"),
                drawn(&location, "its own"),
            ],
            links: vec![linked("party/sending/any-party", &stage, 9)],
            ..Topology::default()
        };

        cluster.merge(storer, &storage);
        cluster.merge(receiver, &own_drawing);

        let found = |id: &str| {
            cluster
                .nodes
                .iter()
                .find(|one| one.id == id)
                .unwrap_or_else(|| panic!("{id}"))
        };
        assert_eq!(found("cluster").label, "the roll's", "no node owns it");
        assert_eq!(found(&storing).label, "its own");
        assert_eq!(
            found(&receiving).label,
            "its own",
            "only the node itself, never a node another drew"
        );
        assert_eq!(found(&stage).label, "Receive", "beneath it, added");
        let location = found(&location);
        assert_eq!(location.label, "its own");
        assert_eq!(
            (location.state, location.evidence.as_str()),
            (Health::Holding, "a document no Subscription takes"),
            "the reader's worse mood stays, with what it said"
        );
        let volumes: Vec<u64> = cluster.links.iter().map(|link| link.volume).collect();
        assert_eq!(
            volumes,
            [9, 7],
            "the node's own Party link wins; the Storage node's traffic joins"
        );
    }

    #[test]
    fn a_node_crosses_toml_by_its_words() {
        let cluster = test_cluster();
        let name = &cluster.node(0).name;
        let node = TopologyNode {
            id: format!("node/{name}"),
            parent: "cluster".to_string(),
            label: name.clone(),
            kind: NodeKind::Node,
            scope: cluster.node_scope(0),
            state: Health::Stressed,
            origin: Origin::Configured,
            load: 0.0,
            activity: 2.5,
            evidence: String::new(),
        };
        let text = toml::to_string(&node).expect("writes");
        assert!(text.contains("kind = \"node\""), "{text}");
        assert!(text.contains("state = \"stressed\""), "{text}");
        assert!(text.contains("origin = \"configured\""), "{text}");
        assert_eq!(toml::from_str::<TopologyNode>(&text), Ok(node));

        let strange: TopologyNode =
            toml::from_str("kind = \"mainframe\"\nstate = \"sulking\"").expect("reads");
        assert_eq!(strange.kind, NodeKind::Computer);
        assert_eq!(strange.state, Health::Stressed);
        assert_eq!(strange.origin, Origin::Both);
    }
}
