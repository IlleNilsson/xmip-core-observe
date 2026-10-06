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
//!
//! **Now, and what was.** `count`, `journeys` and `blocked` are the Port's
//! queue as it stands — a record is published for every Port the node
//! sends, `count` zero where none waits, so a surface says *none now* from
//! it and never guesses. `last_failure` is history: the last Journey that
//! failed there since the node started, kept after it was retried or
//! dismissed. Until 2026-10-06 the last failure was only words in the
//! Port's evidence, and a surface read it as one failing now.

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
    /// How many failed Journeys wait in its queue now.
    #[serde(default)]
    pub count: u64,
    /// Whether one of them blocks its sequence now: a Sequential Send Port
    /// whose `on_failure` is `block`, sending nothing more of that order
    /// key until an operator acts.
    #[serde(default)]
    pub blocked: bool,
    /// The oldest of them, oldest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub journeys: Vec<FailedJourney>,
    /// The last Journey that failed at the Port since the node started, and
    /// why: history, which may since have been retried or dismissed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_failure: Option<LastFailure>,
}

/// The last Journey that failed at a Send Port, as history.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastFailure {
    /// The Journey, by its identifier.
    #[serde(default)]
    pub journey: String,
    /// Why it failed, in words.
    #[serde(default)]
    pub reason: String,
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
            blocked: true,
            journeys: vec![FailedJourney {
                journey: "0199a000-0000-7000-8000-000000000001".to_string(),
                sequence: 4,
                reason: "Out: the far end refused it".to_string(),
            }],
            last_failure: Some(LastFailure {
                journey: "0199a000-0000-7000-8000-000000000009".to_string(),
                reason: "Out: the far end refused it".to_string(),
            }),
        };
        let text = toml::to_string(&written).expect("written");
        let read: FailedJourneys = toml::from_str(&text).expect("read");
        assert_eq!(read, written);
    }

    #[test]
    fn none_now_is_said_apart_from_the_last_that_failed() {
        let written = FailedJourneys {
            node: test_cluster().node_scope(0),
            send_port: "Out".to_string(),
            last_failure: Some(LastFailure {
                journey: "0199a000-0000-7000-8000-000000000009".to_string(),
                reason: "dismissed since".to_string(),
            }),
            ..FailedJourneys::default()
        };
        let text = toml::to_string(&written).expect("written");
        assert!(text.contains("count = 0"), "{text}");
        let read: FailedJourneys = toml::from_str(&text).expect("read");
        assert_eq!((read.count, read.blocked), (0, false));
        assert_eq!(read, written);
    }
}
