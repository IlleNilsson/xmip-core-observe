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
//! What a value is called — its word, and the name a person reads it by — a
//! surface asks the runtime for by value (`xmip_topology_kind_words_v1` and
//! its two siblings, section 8), and until 2026-09-25 the web GUI kept a copy
//! of both.
//!
//! A word a reader does not know reads as the kind's fallback — a computer,
//! both origins, a send-receive exchange — so an unfamiliar publication still
//! draws, and the one reader decides that once.

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
    /// Each link's rate, per second, over the time since `previous` was
    /// observed: the rise in its volume between the two, divided by the
    /// seconds between them. The publisher states it, once, where both
    /// readings are; a surface draws what was stated (ADR-0052, amendment
    /// 2026-09-25). A link `previous` did not carry, a volume that fell — its
    /// publisher started over — and two readings with no time between them
    /// leave the rate as it was: no interval is no rate, never a zero one.
    pub fn rate_since(&mut self, previous: &Self) {
        let nanos = self.observed_unix_nanos - previous.observed_unix_nanos;
        if nanos <= 0 {
            return;
        }
        #[allow(clippy::cast_precision_loss)] // a rate is a figure to read, not a count
        let seconds = nanos as f64 / 1e9;
        for link in &mut self.links {
            let before = previous.links.iter().find(|before| before.id == link.id);
            if let Some(rise) = before.and_then(|before| link.volume.checked_sub(before.volume)) {
                #[allow(clippy::cast_precision_loss)]
                let rise = rise as f64;
                link.rate = rise / seconds;
            }
        }
    }
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
        assert_eq!(NodeKind::named("mainframe"), None);
        assert_eq!(NodeKind::default(), NodeKind::Computer);
        assert_eq!(Origin::default(), Origin::Both);
        assert_eq!(Pattern::default(), Pattern::SendReceive);
    }

    fn link(id: &str, volume: u64) -> TopologyLink {
        TopologyLink {
            id: id.to_string(),
            from: "node/alpha/receive".to_string(),
            to: "node/beta/process".to_string(),
            pattern: Pattern::SendReceive,
            origin: Origin::Both,
            protocol: "handoff".to_string(),
            state: Health::Fine,
            volume,
            rate: 0.0,
            latency_ms: 0.0,
            progress: 0.0,
            attempts: 0,
            evidence: String::new(),
        }
    }

    fn at(nanos: i64, links: Vec<TopologyLink>) -> Topology {
        Topology {
            observed_unix_nanos: nanos,
            links,
            ..Topology::default()
        }
    }

    #[test]
    fn a_links_rate_is_its_rise_over_the_seconds_between_two_readings() {
        let before = at(
            1_000_000_000,
            vec![link("a", 10), link("fell", 50), link("idle", 4)],
        );
        let mut now = at(
            3_000_000_000,
            vec![
                link("a", 30),
                link("fell", 5),
                link("idle", 4),
                link("new", 7),
            ],
        );
        now.rate_since(&before);
        for (link, rate) in now.links.iter().zip([10.0, 0.0, 0.0, 0.0]) {
            assert!(
                (link.rate - rate).abs() < f64::EPSILON,
                "{}: {}",
                link.id,
                link.rate
            );
        }

        // No interval is no rate: what was stated stays.
        let mut again = at(
            3_000_000_000,
            vec![TopologyLink {
                rate: 2.5,
                ..link("a", 90)
            }],
        );
        again.rate_since(&now);
        assert!((again.links[0].rate - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn a_node_crosses_toml_by_its_words() {
        let node = TopologyNode {
            id: "node/alpha".to_string(),
            parent: "cluster".to_string(),
            label: "alpha".to_string(),
            kind: NodeKind::Node,
            scope: "xmip:///C1/node/alpha".to_string(),
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
