//! The communication topology a publication carries beside its records: the
//! things that communicate and what passes between them (ADR-0052, amendment
//! 2026-09-14, ruling 3).
//!
//! The model and its words are written here once. A publisher builds a
//! [`Topology`] — the Playground draws its cluster's from what its nodes
//! reported — and [`crate::publication`] writes and reads it; a surface gets
//! it through the runtime's reader (`xmip_operate.h` section 8) as the
//! header's values, never as the words. Until 2026-09-24 the Playground held
//! this model and `Xmip.Surface` parsed its words again (open problem 25).
//!
//! A word a reader does not know reads as the kind's fallback — a computer,
//! both origins, a send-receive exchange — so an unfamiliar publication still
//! draws, and the one reader decides that once.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::health::Health;
use crate::publication::mood;

/// A closed set of words, each variant one word, with the variant a word no
/// variant is called reads as.
macro_rules! worded {
    ($kind:ident, $unknown:ident, { $($variant:ident => $word:literal),+ $(,)? }) => {
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
}

worded!(NodeKind, Computer, {
    Computer => "computer",
    Server => "server",
    VirtualMachine => "virtual-machine",
    Gateway => "gateway",
    Appliance => "appliance",
    Service => "service",
    Process => "process",
    Interface => "interface",
    Port => "port",
    Protocol => "protocol",
    Location => "location",
    Cluster => "cluster",
    Node => "node",
    Stage => "stage",
    Endpoint => "endpoint",
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
    Configured => "configured",
    Observed => "observed",
    Both => "both",
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
    RequestResponse => "request-response",
    SendReceive => "send-receive",
    PublishConsume => "publish-consume",
    Streaming => "streaming",
    FireAndForget => "fire-and-forget",
    Session => "session",
    Retry => "retry",
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

/// One thing that communicates. `parent` is empty at the top; load and
/// activity run from zero to one.
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

/// One communication relationship, `from` one node id `to` another.
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
        assert_eq!(NodeKind::named("mainframe"), None);
        assert_eq!(NodeKind::default(), NodeKind::Computer);
        assert_eq!(Origin::default(), Origin::Both);
        assert_eq!(Pattern::default(), Pattern::SendReceive);
    }

    #[test]
    fn a_node_crosses_toml_by_its_words() {
        let node = TopologyNode {
            id: "node/R1".to_string(),
            parent: "cluster".to_string(),
            label: "R1".to_string(),
            kind: NodeKind::Node,
            scope: "xmip:///C1/node/R1".to_string(),
            state: Health::Stressed,
            origin: Origin::Configured,
            load: 0.0,
            activity: 1.0,
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
