//! The Journeys that failed at one Send Port, as the node sending it
//! publishes them (`runtime-model.md` section 13; `xmip_operate.h` section
//! 16).
//!
//! A Journey whose every Send Location failed its tries is written Failed
//! and waits in its Send Port's queue for an operator's Retry or Dismiss.
//! The node publishes, for each Port it sends, how many wait and the oldest
//! of them with why, so a surface reading a snapshot lists them at the
//! Port's scope without touching a node, and leaves an act on one as an
//! order ([`crate::Noun::Journey`]). The rest are read from Xmip Storage a
//! page at a time, through the runtime's library.

use serde::{Deserialize, Serialize};

/// The Journeys that failed at one node's Send Port, as published.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailedJourneys {
    /// The node that sends the Port: `xmip:///<cluster>/node/<name>`.
    #[serde(default)]
    pub node: String,
    /// The Send Port, by its configured name.
    #[serde(default)]
    pub send_port: String,
    /// How many failed Journeys wait in its queue.
    #[serde(default)]
    pub count: u64,
    /// The oldest of them, oldest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub journeys: Vec<FailedJourney>,
}

/// One Journey that failed, waiting for an operator.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailedJourney {
    /// The Journey, by its identifier: what Retry and Dismiss name.
    #[serde(default)]
    pub journey: String,
    /// Its place in its Send Port's queue: oldest lowest.
    #[serde(default)]
    pub sequence: u64,
    /// Why it failed, in words.
    #[serde(default)]
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use configure::fixture::test_cluster;

    #[test]
    fn the_failed_journeys_of_a_send_port_read_back_as_written() {
        let written = FailedJourneys {
            node: test_cluster().node_scope(0),
            send_port: "Out".to_string(),
            count: 2,
            journeys: vec![FailedJourney {
                journey: "0199a000-0000-7000-8000-000000000001".to_string(),
                sequence: 4,
                reason: "Out: the far end refused it".to_string(),
            }],
        };
        let text = toml::to_string(&written).expect("written");
        let read: FailedJourneys = toml::from_str(&text).expect("read");
        assert_eq!(read, written);
    }
}
