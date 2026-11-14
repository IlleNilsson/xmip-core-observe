//! A snapshot's health records in the order of the paths they name, so what
//! is at and beneath a scope is found by a search and not by a pass over
//! every record: what a drawing that asks of many scopes at once reads
//! (`topology::draw::members`, `topology::relate::relations`). It finds what
//! [`Snapshot::health`] finds, by the one rule of the tree,
//! [`Scope::contains`], and the worst by the one order, `Standing`.
//!
//! Measured 2026-11-13: drawing and relating a cluster of 25 members asked
//! about 1,000 scopes, each a pass over all 151 records that copied and
//! sorted what it found, 6.5 ms an assembly of the runtime's gatherer;
//! the paths are ordered once a drawing.

use super::{HealthRecord, Snapshot};
use node::Scope;

/// What is at and beneath each scope of one snapshot.
pub struct Beneath<'a> {
    snapshot: &'a Snapshot,
    /// Every record by the path its scope names, in the paths' order.
    sorted: Vec<(&'a str, &'a HealthRecord)>,
}

impl<'a> Beneath<'a> {
    /// The records of `snapshot`, ordered by path.
    #[must_use]
    pub fn of(snapshot: &'a Snapshot) -> Self {
        let mut sorted: Vec<(&str, &HealthRecord)> = snapshot
            .health_records()
            .map(|record| (Scope::new(&record.scope).path(), record))
            .collect();
        sorted.sort_unstable_by(|one, other| one.0.cmp(other.0));
        Self { snapshot, sorted }
    }

    /// The snapshot its records are.
    #[must_use]
    pub const fn snapshot(&self) -> &'a Snapshot {
        self.snapshot
    }

    /// Every record at and beneath `scope`, as [`Snapshot::health`] finds
    /// them, in the order of their paths.
    pub fn records<'s>(&'s self, scope: &'s str) -> impl Iterator<Item = &'a HealthRecord> + 's {
        self.found(scope).map(|(_, record)| *record)
    }

    /// What is at and beneath `scope` alone, still in the paths' order: one
    /// search, and then every scope at and beneath it asked of a few records
    /// rather than all — what a drawing asks of one node's scopes in. A
    /// scope outside it finds nothing here.
    #[must_use]
    pub fn within(&self, scope: &str) -> Self {
        Self {
            snapshot: self.snapshot,
            sorted: self.found(scope).copied().collect(),
        }
    }

    /// The paths at and beneath `scope` with their records: the paths that
    /// begin as its does stand together, and of those the ones it contains
    /// ([`Scope::contains`]).
    fn found<'s>(
        &'s self,
        scope: &'s str,
    ) -> impl Iterator<Item = &'s (&'a str, &'a HealthRecord)> + 's {
        let within = Scope::new(scope);
        let path = within.path();
        let first = self.sorted.partition_point(|(one, _)| *one < path);
        self.sorted[first..]
            .iter()
            .take_while(move |(one, _)| one.starts_with(path))
            .filter(move |(one, _)| within.contains(Scope::new(one)))
    }

    /// The worst record at or beneath `scope`: the first
    /// [`Snapshot::health`] returns.
    #[must_use]
    pub fn worst(&self, scope: &str) -> Option<&'a HealthRecord> {
        self.records(scope)
            .min_by(|one, other| one.standing().cmp(&other.standing()))
    }

    /// Whether anything is recorded at or beneath `scope`.
    #[must_use]
    pub fn holds(&self, scope: &str) -> bool {
        self.records(scope).next().is_some()
    }

    /// Whether anything is recorded at or beneath `<scope>/<leaf>`, as
    /// [`Beneath::holds`] says of it, asked without writing the scope out:
    /// what a drawing asks of each stage a node may have reported on.
    #[must_use]
    pub fn holds_beneath(&self, scope: &str, leaf: &str) -> bool {
        if scope.ends_with('/') || leaf.is_empty() || leaf.contains('/') {
            return self.holds(&format!("{scope}/{leaf}"));
        }
        let base = Scope::new(scope).path();
        self.found(scope).any(|(one, _)| {
            let rest = if base.is_empty() {
                Some(*one)
            } else {
                one.strip_prefix(base)
                    .and_then(|rest| rest.strip_prefix('/'))
            };
            rest.and_then(|rest| rest.strip_prefix(leaf))
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::Health;
    use configure::fixture::test_cluster;

    /// Every scope the test asks of, its records spelled as publishers
    /// spell them and as a reader may: with an authority, a trailing slash,
    /// beside a sibling whose name begins as another's does.
    fn snapshot() -> (Snapshot, Vec<String>) {
        let cluster = test_cluster();
        let root = cluster.scope();
        let (first, second) = (cluster.node_scope(0), cluster.node_scope(1));
        let sibling = format!("{first}-2");
        let path = root.trim_start_matches("xmip:///");
        let mut snapshot = Snapshot::new();
        let healths = [
            Health::Fine,
            Health::Stressed,
            Health::Done,
            Health::Working,
        ];
        let leaves = [
            format!("{first}/receive/Orders"),
            format!("{first}/receive/Orders/authorization"),
            format!("{first}/send/Invoices/"),
            format!("{sibling}/receive/Orders"),
            format!("{second}/storage"),
            format!("xmip://host-1/{path}/node/{}/reached", cluster.node(1).name),
            format!("{root}/node"),
            format!("{root}/round-trip/tcp"),
        ];
        for (place, scope) in leaves.iter().enumerate() {
            snapshot.record_health(HealthRecord {
                scope: scope.clone(),
                health: healths[place % healths.len()],
                severity: u8::try_from(place * 10).expect("small"),
                evidence: String::new(),
                observed_unix_nanos: 7,
            });
        }
        let asked = vec![
            root.clone(),
            format!("{root}/"),
            format!("{root}/node"),
            first.clone(),
            format!("{first}/receive"),
            format!("{first}/receive/Orders"),
            format!("{first}/send"),
            sibling,
            second,
            format!("{root}/nowhere"),
            String::new(),
        ];
        (snapshot, asked)
    }

    #[test]
    fn it_finds_what_the_snapshot_finds_at_and_beneath_every_scope_and_the_same_worst() {
        let (snapshot, asked) = snapshot();
        let beneath = Beneath::of(&snapshot);
        // Within the first node: what is beneath it is found there as well,
        // and nothing beside it.
        let first = test_cluster().node_scope(0);
        let own = beneath.within(&first);
        let beneath_first =
            |scope: &&String| **scope == first || scope.starts_with(&format!("{first}/"));
        for scope in asked.iter().filter(beneath_first) {
            let found: Vec<&str> = own.records(scope).map(|one| one.scope.as_str()).collect();
            let all: Vec<&str> = beneath
                .records(scope)
                .map(|one| one.scope.as_str())
                .collect();
            assert_eq!(found, all, "within {first}, at {scope}");
        }
        let beside = format!("{first}-2");
        assert!(
            !own.holds(&beside),
            "a sibling whose name begins as its does"
        );
        for scope in &asked {
            let mut found: Vec<&str> = beneath
                .records(scope)
                .map(|record| record.scope.as_str())
                .collect();
            let health = snapshot.health(scope);
            let mut expected: Vec<&str> = health.iter().map(|one| one.scope.as_str()).collect();
            found.sort_unstable();
            expected.sort_unstable();
            assert_eq!(found, expected, "at and beneath {scope:?}");
            assert_eq!(
                beneath.worst(scope),
                health.first(),
                "the worst at {scope:?}"
            );
            assert_eq!(beneath.holds(scope), !health.is_empty(), "{scope:?}");
            for leaf in ["receive", "send", "Orders", "node", "storage", "nowhere"] {
                let written = format!("{scope}/{leaf}");
                assert_eq!(
                    beneath.holds_beneath(scope, leaf),
                    !snapshot.health(&written).is_empty(),
                    "beneath {written:?}"
                );
            }
        }
    }
}
