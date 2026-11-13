//! The topology's closed word sets, each one of Xmip's own fixed answers: the
//! kind of a drawn thing, where a fact came from, what an exchange means and
//! how two of a cluster's nodes are related. Each value is one word a
//! publication writes and one name a person reads, written here once
//! (ADR-0052, amendment 2026-09-25); a word a reader does not know reads as
//! its set's fallback, so an unfamiliar publication still draws.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

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

impl Origin {
    /// What one fact is where two drawings each said it: configured and
    /// observed together are both, whatever order they are joined in, and a
    /// fact said the same way twice is that (ADR-0052, amendment
    /// 2026-11-13).
    #[must_use]
    pub const fn joined(self, other: Self) -> Self {
        match (self, other) {
            (Self::Configured, Self::Configured) => Self::Configured,
            (Self::Observed, Self::Observed) => Self::Observed,
            _ => Self::Both,
        }
    }
}

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

/// How the two ends of a link are related in their cluster: by the roles
/// each declares and the order of `[storage] nodes`, never by a name (the
/// owner, 2026-11-12: *Related by its role in the cluster, not by name*;
/// ADR-0052 and ADR-0056, amendments 2026-11-13). [`Relation::Traffic`] is
/// an observed line no relation explains: inside a node, and between a node
/// and a Party.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Relation {
    /// A node that does not declare storage, into the Storage node it
    /// writes into: the first `[storage] nodes` lists.
    WritesInto,
    /// That Storage node, to each node whose roles serve process or send.
    HandsWorkTo,
    /// The first-listed Storage node, to each later-listed one.
    CopiesTo,
    /// A node declaring the operational role, to every other member.
    Operates,
    /// A node declaring the monitoring role, to every other member.
    Monitors,
    /// A node declaring both, to every other member.
    OperatesAndMonitors,
    /// Traffic no relation explains.
    Traffic,
}

worded!(Relation, Traffic, {
    WritesInto => "writes-into" as "writes into",
    HandsWorkTo => "hands-work-to" as "hands work to",
    CopiesTo => "copies-to" as "copies to",
    Operates => "operates" as "operates",
    Monitors => "monitors" as "monitors",
    OperatesAndMonitors => "operates-and-monitors" as "operates and monitors",
    Traffic => "traffic" as "traffic",
});

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
        for relation in Relation::ALL {
            assert_eq!(Relation::named(relation.word()), Some(*relation));
        }
        assert_eq!(NodeKind::VirtualMachine.word(), "virtual-machine");
        assert_eq!(NodeKind::VirtualMachine.name(), "virtual machine");
        assert_eq!(Origin::Both.name(), "configured and observed");
        assert_eq!(Pattern::PublishConsume.name(), "Publish → consume");
        assert_eq!(Relation::CopiesTo.name(), "copies to");
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
        assert_eq!(Relation::default(), Relation::Traffic);
    }

    #[test]
    fn configured_and_observed_join_to_both_in_either_order() {
        use Origin::{Both, Configured, Observed};
        for (one, other, joined) in [
            (Configured, Observed, Both),
            (Observed, Configured, Both),
            (Configured, Configured, Configured),
            (Observed, Observed, Observed),
            (Both, Configured, Both),
            (Observed, Both, Both),
        ] {
            assert_eq!(one.joined(other), joined, "{one:?} and {other:?}");
        }
    }
}
