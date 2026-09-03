//! What the operator boundary reads: health and counts, published here and
//! read from `xmip_operate.h`.
//!
//! ADR-0027 clause 6: the runtime publishes, the boundary reads, and nothing
//! across it asks the hot path. This is the published thing. Receive, Process
//! and Send write into it asynchronously; a surface reads whatever is here.
//!
//! Scope is an Xmip URI path over the execution tree, clause 4. A record at
//! `xmip:///edge-01/transport/ftp` sits beneath `xmip:///edge-01/transport`
//! and beneath `xmip:///edge-01`, so asking for a node gets everything the
//! node holds. That prefix rule is the whole aggregation model: health up the
//! tree is the worst beneath, and a count up the tree is the sum beneath.

use std::collections::BTreeMap;

/// observability-model.md section 6. Worst wins upward, and that is this
/// enum's ordering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Health {
    Green,
    Yellow,
    Red,
}

/// What a count counts. Never a bare number — ADR-0027 clause 5.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Counted {
    Streams,
    Messages,
    Journeys,
    Bytes,
}

/// One scope's health, the one line that explains it, and when it was seen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HealthRecord {
    pub scope: String,
    pub health: Health,
    pub evidence: String,
    pub observed_unix_nanos: i64,
}

/// One count over a window, and when it was taken.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Count {
    pub scope: String,
    pub counted: Counted,
    pub value: u64,
    pub window_start_unix_nanos: i64,
    pub window_end_unix_nanos: i64,
    pub observed_unix_nanos: i64,
}

/// The published state of a node. One per node, written by the node.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    health: BTreeMap<String, HealthRecord>,
    counts: BTreeMap<(String, Counted), Count>,
}

impl Snapshot {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a scope's health. Replaces what was there for that scope.
    pub fn record_health(&mut self, record: HealthRecord) {
        self.health.insert(record.scope.clone(), record);
    }

    /// Record a count. Replaces what was there for that scope and kind.
    pub fn record_count(&mut self, count: Count) {
        self.counts
            .insert((count.scope.clone(), count.counted), count);
    }

    /// Health at and beneath a scope, worst first. Empty when nothing is
    /// recorded there.
    #[must_use]
    pub fn health(&self, scope: &str) -> Vec<HealthRecord> {
        let mut found: Vec<HealthRecord> = self
            .health
            .values()
            .filter(|record| beneath(&record.scope, scope))
            .cloned()
            .collect();

        found.sort_by(|a, b| b.health.cmp(&a.health).then(a.scope.cmp(&b.scope)));
        found
    }

    /// The worst health at or beneath a scope, or `None` when nothing is
    /// recorded there. Section 6: an installation showing green means every
    /// endpoint beneath it is green.
    #[must_use]
    pub fn worst(&self, scope: &str) -> Option<Health> {
        self.health(scope).first().map(|record| record.health)
    }

    /// One kind of count, summed over everything at and beneath a scope.
    ///
    /// The window is the union of the parts and the observation is the
    /// oldest, so a reader sees the staleness of the stalest part rather than
    /// the freshness of the freshest. `None` when nothing is recorded.
    #[must_use]
    pub fn measure(&self, scope: &str, counted: Counted) -> Option<Count> {
        let parts: Vec<&Count> = self
            .counts
            .values()
            .filter(|count| count.counted == counted && beneath(&count.scope, scope))
            .collect();

        let first = parts.first()?;

        Some(Count {
            scope: scope.to_string(),
            counted,
            value: parts.iter().map(|count| count.value).sum(),
            window_start_unix_nanos: parts
                .iter()
                .map(|count| count.window_start_unix_nanos)
                .min()
                .unwrap_or(first.window_start_unix_nanos),
            window_end_unix_nanos: parts
                .iter()
                .map(|count| count.window_end_unix_nanos)
                .max()
                .unwrap_or(first.window_end_unix_nanos),
            observed_unix_nanos: parts
                .iter()
                .map(|count| count.observed_unix_nanos)
                .min()
                .unwrap_or(first.observed_unix_nanos),
        })
    }
}

/// Whether `candidate` is `scope` or sits beneath it in the tree.
///
/// `xmip:///a/b` is beneath `xmip:///a`; `xmip:///ab` is not, because a
/// prefix of characters is not a prefix of path segments.
fn beneath(candidate: &str, scope: &str) -> bool {
    let scope = scope.trim_end_matches('/');

    candidate == scope
        || candidate
            .strip_prefix(scope)
            .is_some_and(|rest| rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn health(scope: &str, health: Health, evidence: &str) -> HealthRecord {
        HealthRecord {
            scope: scope.to_string(),
            health,
            evidence: evidence.to_string(),
            observed_unix_nanos: 1_000,
        }
    }

    fn count(scope: &str, counted: Counted, value: u64, at: i64) -> Count {
        Count {
            scope: scope.to_string(),
            counted,
            value,
            window_start_unix_nanos: at - 60,
            window_end_unix_nanos: at,
            observed_unix_nanos: at,
        }
    }

    #[test]
    fn a_node_is_as_healthy_as_its_worst_part() {
        let mut snapshot = Snapshot::new();

        snapshot.record_health(health("xmip:///edge-01/transport/ftp", Health::Green, ""));
        snapshot.record_health(health(
            "xmip:///edge-01/transport/sftp",
            Health::Red,
            "connection refused by partner-x",
        ));

        assert_eq!(snapshot.worst("xmip:///edge-01"), Some(Health::Red));
        assert_eq!(
            snapshot.health("xmip:///edge-01")[0].evidence,
            "connection refused by partner-x"
        );
    }

    #[test]
    fn a_scope_with_nothing_beneath_it_has_no_health_rather_than_green() {
        let snapshot = Snapshot::new();

        assert_eq!(snapshot.worst("xmip:///edge-01"), None);
    }

    #[test]
    fn a_count_for_a_node_is_the_sum_of_its_parts() {
        // ADR-0027 clause 5: Cluster and Node figures are sums over the tree,
        // not a separate concept.
        let mut snapshot = Snapshot::new();

        snapshot.record_count(count(
            "xmip:///edge-01/transport/ftp",
            Counted::Streams,
            40,
            2_000,
        ));
        snapshot.record_count(count(
            "xmip:///edge-01/transport/sftp",
            Counted::Streams,
            2,
            3_000,
        ));
        snapshot.record_count(count(
            "xmip:///edge-01/transport/ftp",
            Counted::Bytes,
            9_000,
            2_000,
        ));

        let streams = snapshot
            .measure("xmip:///edge-01", Counted::Streams)
            .expect("recorded");

        assert_eq!(streams.value, 42);
        assert_eq!(streams.counted, Counted::Streams);
        assert_eq!(
            streams.observed_unix_nanos, 2_000,
            "the stalest part decides staleness"
        );
    }

    #[test]
    fn kinds_are_never_summed_together() {
        // Streams at a Receive Location and bytes through it are different
        // quantities, and Xmip does not pretend otherwise.
        let mut snapshot = Snapshot::new();

        snapshot.record_count(count("xmip:///edge-01/x", Counted::Streams, 1, 1));
        snapshot.record_count(count("xmip:///edge-01/x", Counted::Bytes, 500, 1));

        assert_eq!(
            snapshot
                .measure("xmip:///edge-01", Counted::Streams)
                .map(|c| c.value),
            Some(1)
        );
        assert_eq!(
            snapshot
                .measure("xmip:///edge-01", Counted::Bytes)
                .map(|c| c.value),
            Some(500)
        );
        assert_eq!(snapshot.measure("xmip:///edge-01", Counted::Journeys), None);
    }

    #[test]
    fn beneath_is_by_path_segment_not_by_character() {
        assert!(beneath("xmip:///a/b", "xmip:///a"));
        assert!(beneath("xmip:///a", "xmip:///a"));
        assert!(!beneath("xmip:///ab", "xmip:///a"));
        assert!(beneath("xmip:///a/b", "xmip:///a/"));
    }

    #[test]
    fn a_second_record_for_a_scope_replaces_the_first() {
        let mut snapshot = Snapshot::new();

        snapshot.record_health(health("xmip:///n", Health::Red, "down"));
        snapshot.record_health(health("xmip:///n", Health::Green, ""));

        assert_eq!(snapshot.worst("xmip:///n"), Some(Health::Green));
        assert_eq!(snapshot.health("xmip:///n").len(), 1);
    }
}
