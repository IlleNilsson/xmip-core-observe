//! The cluster's rollup of its nodes (ADR-0027 decision 8): what no node
//! can say about itself, said by whoever assembles the cluster's view from
//! what each node published — the Essentials' cluster process and the
//! runtime's gatherer (`runtime::plane::gather`) call this one rule
//! (ADR-0052, amendment 2026-11-13). The merge it reads is observe's,
//! `Snapshot::merge`, every table of it. Until 2026-11-13 the rule was the
//! Essentials' cluster process's own.

use crate::health::Health;
use crate::snapshot::{Beneath, HealthRecord, Snapshot};
use crate::topology::draw::node_rollup;

/// The rollup the surface owes, at `<root>/node`: the worst leaf across
/// every one of the `count` nodes, and which node carries it. It sits on the
/// branch the nodes hang under, so the scope tree shows it as that branch's
/// own record and never as a node beside them. One point of severity below
/// the leaf it names, so the way to the problem ends at the leaf and not
/// here. Nothing published yet beneath the nodes is Working.
#[must_use]
pub fn nodes(snapshot: &Snapshot, root: &str, count: usize, now: i64) -> HealthRecord {
    let nodes = node_rollup(root);
    let beneath = Beneath::of(snapshot);
    let (mut leaves, mut fine) = (0, 0);
    for record in beneath.records(&nodes) {
        leaves += 1;
        fine += usize::from(record.health == Health::Fine);
    }
    let (health, severity, evidence) = match beneath.worst(&nodes) {
        Some(record) if record.health != Health::Fine => (
            record.health,
            record.severity.saturating_sub(1),
            format!(
                "{count} nodes, {fine} of {leaves} leaves fine; worst {}: {}",
                record.scope.trim_start_matches(&format!("{nodes}/")),
                record.evidence
            ),
        ),
        Some(_) => (
            Health::Fine,
            0,
            format!("{count} nodes, all {leaves} leaves fine"),
        ),
        None => (
            Health::Working,
            20,
            format!("{count} nodes, nothing published yet"),
        ),
    };
    HealthRecord {
        scope: nodes,
        health,
        severity,
        evidence,
        observed_unix_nanos: now,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use configure::fixture::test_cluster;

    /// A record at `<root>/node/<the test cluster's node at place>/<leaf>`.
    fn recorded(place: usize, leaf: &str, health: Health, severity: u8) -> HealthRecord {
        HealthRecord {
            scope: format!("{}/{leaf}", test_cluster().node_scope(place)),
            health,
            severity,
            evidence: format!("{leaf} said so"),
            observed_unix_nanos: 7,
        }
    }

    #[test]
    fn the_worst_leaf_is_named_one_point_below_it_on_the_nodes_branch() {
        let cluster = test_cluster();
        let root = cluster.scope();
        let mut snapshot = Snapshot::new();
        snapshot.record_health(recorded(0, "receive/orders", Health::Fine, 0));
        snapshot.record_health(recorded(1, "send/invoices", Health::Stressed, 60));
        snapshot.record_health(recorded(1, "system-process", Health::Fine, 0));
        // A test run at the cluster itself is no node's, and not rolled up.
        snapshot.record_health(HealthRecord {
            scope: format!("{root}/round-trip/tcp"),
            ..recorded(0, "x", Health::Exhausted, 90)
        });

        let rolled = nodes(&snapshot, &root, 2, 9);

        assert_eq!(rolled.scope, format!("{root}/node"));
        assert_eq!((rolled.health, rolled.severity), (Health::Stressed, 59));
        let worst = format!("{}/send/invoices", cluster.node(1).name);
        assert_eq!(
            rolled.evidence,
            format!("2 nodes, 2 of 3 leaves fine; worst {worst}: send/invoices said so")
        );
        assert_eq!(rolled.observed_unix_nanos, 9);
    }

    #[test]
    fn every_leaf_fine_is_fine_and_nothing_published_is_working() {
        let root = test_cluster().scope();
        let mut snapshot = Snapshot::new();
        snapshot.record_health(recorded(0, "receive/orders", Health::Fine, 0));
        let fine = nodes(&snapshot, &root, 1, 9);
        assert_eq!(
            (fine.health, fine.severity, fine.evidence.as_str()),
            (Health::Fine, 0, "1 nodes, all 1 leaves fine")
        );

        let nothing = nodes(&Snapshot::new(), &root, 3, 9);
        assert_eq!(
            (nothing.health, nothing.severity, nothing.evidence.as_str()),
            (Health::Working, 20, "3 nodes, nothing published yet")
        );
    }

    #[test]
    fn a_leaf_at_severity_zero_is_never_rolled_below_it() {
        let root = test_cluster().scope();
        let mut snapshot = Snapshot::new();
        snapshot.record_health(recorded(0, "reached", Health::Exhausted, 0));
        let rolled = nodes(&snapshot, &format!("{root}/"), 1, 9);
        assert_eq!(
            rolled.scope,
            format!("{root}/node"),
            "one branch, one scope"
        );
        assert_eq!((rolled.health, rolled.severity), (Health::Exhausted, 0));
    }
}
