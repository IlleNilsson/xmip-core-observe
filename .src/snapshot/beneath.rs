//! A snapshot's health records in the order of the paths they name, segment
//! by segment, so what is at and beneath a scope stands together and is
//! found by a search and not by a pass over every record: what a drawing
//! that asks of many scopes at once reads (`topology::draw::members`,
//! `topology::relate::relations`, `rollup::nodes`). It finds what
//! [`Snapshot::health`] finds, by the one rule of the tree,
//! [`Scope::contains`], and the worst by the one order, `Standing`.
//!
//! Measured 2026-11-13: drawing and relating a cluster of 25 members asked
//! about 1,000 scopes, each a pass over all 151 records that copied and
//! sorted what it found, 6.5 ms an assembly of the runtime's gatherer;
//! the paths are ordered once a drawing. Since 2026-11-14 the paths are
//! ordered segment by segment, so a scope's records are one run of them
//! and what is beneath one node is that run, borrowed, never copied out
//! (ADR-0052, amendment 2026-11-14).

use std::borrow::Cow;
use std::cmp::Ordering;

use super::{HealthRecord, Snapshot};
use node::Scope;

/// One record by the path its scope names.
type Entry<'a> = (&'a str, &'a HealthRecord);

/// What is at and beneath each scope of one snapshot.
pub struct Beneath<'a> {
    snapshot: &'a Snapshot,
    /// Every record by the path its scope names, in the paths' order
    /// segment by segment ([`by_segments`]): the whole snapshot's, or one
    /// scope's run of them borrowed from it.
    sorted: Cow<'a, [Entry<'a>]>,
}

/// Two paths in the order of their segments, each compared as text: `a/b`
/// before `a/b/c` before `a/b-2`, so what is at and beneath `a/b` stands
/// together — the slash before any other character, where the order of the
/// text would put `a/b-2` between `a/b` and `a/b/c`.
fn by_segments(one: &str, other: &str) -> Ordering {
    let (one, other) = (one.as_bytes(), other.as_bytes());
    match first_difference(one, other) {
        None => one.len().cmp(&other.len()),
        Some(at) => match (one[at], other[at]) {
            (b'/', _) => Ordering::Less,
            (_, b'/') => Ordering::Greater,
            (a, b) => a.cmp(&b),
        },
    }
}

/// Where `one` and `other` first differ within the shorter, compared eight
/// bytes at a time: the paths of one cluster share their first tens.
fn first_difference(one: &[u8], other: &[u8]) -> Option<usize> {
    let common = one.len().min(other.len());
    let (one, other) = (&one[..common], &other[..common]);
    let (words, others) = (one.as_chunks::<8>().0, other.as_chunks::<8>().0);
    let same = 8 * words.iter().zip(others).take_while(|(a, b)| a == b).count();
    one[same..]
        .iter()
        .zip(&other[same..])
        .position(|(a, b)| a != b)
        .map(|at| same + at)
}

impl<'a> Beneath<'a> {
    /// The records of `snapshot`, ordered by path segment by segment.
    #[must_use]
    pub fn of(snapshot: &'a Snapshot) -> Self {
        let mut sorted: Vec<Entry<'a>> = snapshot
            .health_records()
            .map(|record| (Scope::new(&record.scope).path(), record))
            .collect();
        sorted.sort_unstable_by(|one, other| by_segments(one.0, other.0));
        Self {
            snapshot,
            sorted: Cow::Owned(sorted),
        }
    }

    /// The snapshot its records are.
    #[must_use]
    pub const fn snapshot(&self) -> &'a Snapshot {
        self.snapshot
    }

    /// Every record at and beneath `scope`, as [`Snapshot::health`] finds
    /// them, in the order of their paths.
    pub fn records<'s>(
        &'s self,
        scope: &str,
    ) -> impl Iterator<Item = &'a HealthRecord> + use<'s, 'a> {
        self.found(scope).iter().map(|(_, record)| *record)
    }

    /// What is at and beneath `scope` alone, still in the paths' order: one
    /// search, and then every scope at and beneath it asked of a few records
    /// rather than all — what a drawing asks of one node's scopes in. Its
    /// records are borrowed, never copied. A scope outside it finds nothing
    /// here.
    #[must_use]
    pub fn within(&self, scope: &str) -> Beneath<'_> {
        Beneath {
            snapshot: self.snapshot,
            sorted: Cow::Borrowed(self.found(scope)),
        }
    }

    /// The paths at and beneath `scope` with their records: one run of the
    /// paths in their order, from the first not before its own to the last
    /// it contains ([`Scope::contains`]).
    fn found(&self, scope: &str) -> &[Entry<'a>] {
        let within = Scope::new(scope);
        let path = within.path();
        let first = self
            .sorted
            .partition_point(|(one, _)| by_segments(one, path) == Ordering::Less);
        let run = &self.sorted[first..];
        let contained = run
            .iter()
            .take_while(|(one, _)| within.contains(Scope::new(one)))
            .count();
        &run[..contained]
    }

    /// The worst record at or beneath `scope`: the first
    /// [`Snapshot::health`] returns.
    #[must_use]
    pub fn worst(&self, scope: &str) -> Option<&'a HealthRecord> {
        worst(self.records(scope))
    }

    /// Whether anything is recorded at or beneath `scope`.
    #[must_use]
    pub fn holds(&self, scope: &str) -> bool {
        !self.found(scope).is_empty()
    }

    /// Every record at and beneath `<scope>/<leaf>`, as [`Beneath::records`]
    /// finds them, asked without writing the scope out: what a drawing asks
    /// of each stage a node may have reported on, and a relation of each
    /// member's reach.
    pub fn records_beneath<'s>(
        &'s self,
        scope: &'s str,
        leaf: &'s str,
    ) -> impl Iterator<Item = &'a HealthRecord> + use<'s, 'a> {
        let plain = !(scope.ends_with('/') || leaf.is_empty() || leaf.contains('/'));
        let (found, base) = if plain {
            (self.found(scope), Some(Scope::new(scope).path()))
        } else {
            (self.found(&format!("{scope}/{leaf}")), None)
        };
        // Where the leaf is plain, the records beneath `scope` whose path
        // goes on with it; otherwise the scope written out found them.
        let below = move |one: &str| {
            base.is_none_or(|base| {
                after(base, one)
                    .and_then(|rest| rest.strip_prefix(leaf))
                    .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
            })
        };
        found
            .iter()
            .filter(move |(one, _)| below(one))
            .map(|(_, record)| *record)
    }

    /// Each record at and beneath `scope` with the segment of its path just
    /// beneath `scope`, none for a record at `scope` itself: what a drawing
    /// reads of one node's records in one pass. Where `scope` ends in no
    /// slash, a record's segment is a leaf it is found beneath by
    /// [`Beneath::records_beneath`].
    pub fn leaves<'s>(
        &'s self,
        scope: &'s str,
    ) -> impl Iterator<Item = (Option<&'a str>, &'a HealthRecord)> + use<'s, 'a> {
        let base = Scope::new(scope).path();
        self.found(scope).iter().map(move |(one, record)| {
            let leaf = after(base, one).and_then(|rest| rest.split('/').next());
            (leaf, *record)
        })
    }

    /// The worst record at or beneath `<scope>/<leaf>`, as
    /// [`Beneath::worst`] says of it, asked without writing the scope out.
    #[must_use]
    pub fn worst_beneath(&self, scope: &str, leaf: &str) -> Option<&'a HealthRecord> {
        worst(self.records_beneath(scope, leaf))
    }

    /// Whether anything is recorded at or beneath `<scope>/<leaf>`, as
    /// [`Beneath::holds`] says of it, asked without writing the scope out.
    #[must_use]
    pub fn holds_beneath(&self, scope: &str, leaf: &str) -> bool {
        self.records_beneath(scope, leaf).next().is_some()
    }
}

/// The worst of `records` by the one order, `Standing`.
fn worst<'a>(records: impl Iterator<Item = &'a HealthRecord>) -> Option<&'a HealthRecord> {
    records.min_by(|one, other| one.standing().cmp(&other.standing()))
}

/// What follows the path `base` and its slash in `path`: the whole of
/// `path` beneath the root, and none at `base` itself or beside it.
fn after<'p>(base: &str, path: &'p str) -> Option<&'p str> {
    if base.is_empty() {
        Some(path)
    } else {
        path.strip_prefix(base)?.strip_prefix('/')
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
            for leaf in [
                "receive", "send", "Orders", "node", "storage", "nowhere", "",
            ] {
                let written = format!("{scope}/{leaf}");
                let health = snapshot.health(&written);
                let mut found: Vec<&str> = beneath
                    .records_beneath(scope, leaf)
                    .map(|record| record.scope.as_str())
                    .collect();
                let mut expected: Vec<&str> = health.iter().map(|one| one.scope.as_str()).collect();
                found.sort_unstable();
                expected.sort_unstable();
                assert_eq!(found, expected, "beneath {written:?}");
                assert_eq!(
                    beneath.worst_beneath(scope, leaf),
                    health.first(),
                    "the worst beneath {written:?}"
                );
                assert_eq!(
                    beneath.holds_beneath(scope, leaf),
                    !health.is_empty(),
                    "beneath {written:?}"
                );
                if !scope.ends_with('/') && !leaf.is_empty() {
                    let leaves = beneath.leaves(scope).any(|(one, _)| one == Some(leaf));
                    assert_eq!(leaves, !health.is_empty(), "a leaf {written:?}");
                }
            }
        }
    }
}
