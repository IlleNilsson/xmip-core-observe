//! A Message in a node's Dead Message Queue, as the node publishes it
//! (`runtime-model.md` section 9, *The Dead Message Queue is Ledger state*;
//! ADR-0052, amendment 2026-10-01).
//!
//! An accepted Message that no Subscription matched is kept in the Ledger
//! with its receive context, what its gates concluded, its promoted
//! properties and every Subscription's reason for declining. The node
//! publishes its queue's oldest entries with its health and counts, so a
//! surface reading a snapshot lists every node's Dead Message Queue without
//! touching one, opens an entry to show its properties and its declines, and
//! leaves an Operator's Replay as an order ([`crate::Noun::DeadMessage`]).

use serde::{Deserialize, Serialize};

/// One Message in one node's Dead Message Queue, as published.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeadMessage {
    /// The node whose queue keeps it: `xmip:///<cluster>/node/<name>`.
    #[serde(default)]
    pub node: String,
    /// The Message, by its identifier: what a Replay names.
    #[serde(default)]
    pub message: String,
    /// Its place in the queue: oldest lowest.
    #[serde(default)]
    pub sequence: u64,
    /// The Receive Location it arrived at.
    #[serde(default)]
    pub location: String,
    /// When it was received, in unix nanoseconds.
    #[serde(default)]
    pub received_unix_nanos: i64,
    /// What each gate concluded of it, gate and verdict, in the order they
    /// ran.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub validation: Vec<[String; 2]>,
    /// The properties routing read, name and value, by name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub promoted: Vec<[String; 2]>,
    /// Every Subscription asked, and why it declined, in the order asked.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub declines: Vec<[String; 2]>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use configure::fixture::test_cluster;

    #[test]
    fn a_dead_message_reads_back_as_written() {
        let written = DeadMessage {
            node: test_cluster().node_scope(0),
            message: "0199a000-0000-7000-8000-000000000001".to_string(),
            sequence: 3,
            promoted: vec![["MessageType".to_string(), "Invoice".to_string()]],
            declines: vec![["structured".to_string(), "MessageType is Invoice".to_string()]],
            ..DeadMessage::default()
        };
        let text = toml::to_string(&written).expect("written");
        let read: DeadMessage = toml::from_str(&text).expect("read");
        assert_eq!(read, written);
    }
}
