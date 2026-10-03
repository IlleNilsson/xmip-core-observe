//! A member of the cluster a node does not hear now, as the node publishes
//! it (ADR-0065, amendment 2026-10-02): the Events that member raises do
//! not reach the Event subscriptions held on this node until it is heard
//! again. Nothing missing is silent: a subscriber is told with every
//! delivery, and an operator reads this line beside the Event
//! subscriptions — *not hearing `<node>` since `<time>`: `<why>`*.

use serde::{Deserialize, Serialize};

/// One member not heard, by one node.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Unheard {
    /// The node that does not hear it: `xmip:///<cluster>/node/<name>`.
    pub by: String,
    /// The member not heard: `xmip:///<cluster>/node/<name>`.
    pub node: String,
    /// Since when, in unix nanoseconds.
    pub since_unix_nanos: i64,
    /// Why, in words.
    pub why: String,
}

impl Unheard {
    /// The line an operator reads: *not hearing `<node>` since `<time>`:
    /// `<why>*, the time as RFC 3339 to the second. Every surface says it
    /// in these words.
    #[must_use]
    pub fn said(&self) -> String {
        let stamped = codec::civil::rfc3339_nanos(i128::from(self.since_unix_nanos));
        let second = stamped.split('.').next().unwrap_or(&stamped);
        format!("not hearing {} since {second}Z: {}", self.node, self.why)
    }
}

/// An unheard member as a publication writes it.
#[derive(Serialize, Deserialize)]
pub(crate) struct UnheardDocument {
    #[serde(default)]
    by: String,
    #[serde(default)]
    node: String,
    #[serde(default)]
    since_unix_nanos: i64,
    #[serde(default)]
    why: String,
}

impl UnheardDocument {
    pub(crate) fn of(unheard: &Unheard) -> Self {
        Self {
            by: unheard.by.clone(),
            node: unheard.node.clone(),
            since_unix_nanos: unheard.since_unix_nanos,
            why: unheard.why.clone(),
        }
    }

    pub(crate) fn into_unheard(self) -> Unheard {
        Unheard {
            by: self.by,
            node: self.node,
            since_unix_nanos: self.since_unix_nanos,
            why: self.why,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use configure::fixture::test_cluster;

    #[test]
    fn an_unheard_member_is_said_in_one_line() {
        let cluster = test_cluster();
        let unheard = Unheard {
            by: cluster.node_scope(0),
            node: cluster.node_scope(1),
            since_unix_nanos: 1_790_000_000_123_456_789,
            why: "connection refused".to_string(),
        };
        assert_eq!(
            unheard.said(),
            format!(
                "not hearing {} since 2026-09-21T14:13:20Z: connection refused",
                cluster.node_scope(1)
            )
        );
    }
}
