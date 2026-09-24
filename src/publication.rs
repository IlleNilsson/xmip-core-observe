//! A publication: a [`Snapshot`] as the file a surface reads, written and
//! read here and nowhere else.
//!
//! A publisher and its readers are separate processes, and the bridge
//! between them is a file. Its shape — `source`, `node`, `[[records]]`,
//! `[[counts]]`, and a roll's `[run]` and `[topology]` — was written by the
//! Playground and parsed again by `Xmip.Surface`'s `SnapshotOperator` until
//! 2026-09-24 (open problem 25). It has one home now: the Playground writes
//! through [`Publication::to_toml`], reads its nodes' files back through
//! [`Publication::read`], and a surface reads through the same function in
//! the runtime's library (`xmip_publication_read_v1`, `xmip_operate.h`
//! section 8).
//!
//! **TOML, not JSON.** On disk the estate is TOML — the owner's rule; JSON is
//! reserved for what lives in memory or on the wire.
//!
//! **What a reader does not know.** A mood no one is called reads as
//! `Stressed`, so it shows and is looked at; a counted kind no one is called
//! is skipped, because a count of it is no count of anything the reader
//! knows; a topology word falls back as [`crate::topology`] says. A file that
//! is not this shape at all is refused whole.

use serde::{Deserialize, Serialize};

use crate::counted::Counted;
use crate::health::Health;
use crate::run::Run;
use crate::snapshot::{Count, HealthRecord, Snapshot};
use crate::topology::Topology;

/// One publication, read or about to be written.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Publication {
    /// Who published it, in the publisher's words.
    pub source: String,
    /// The scope it publishes at: a node's, or a roll's cluster.
    pub node: String,
    /// The health records beneath `node`, worst first.
    pub records: Vec<HealthRecord>,
    /// The counts: each at the scope it was recorded at, or at `node` where
    /// the publisher summed them there.
    pub counts: Vec<Count>,
    /// What the run was started with, when the publisher says.
    pub run: Option<Run>,
    /// The communication topology, when the publisher draws one.
    pub topology: Option<Topology>,
}

impl Publication {
    /// A roll's publication: the records beneath `node` and every kind summed
    /// at `node`.
    #[must_use]
    pub fn of(source: &str, node: &str, snapshot: &Snapshot) -> Self {
        Self {
            counts: Counted::ALL
                .into_iter()
                .filter_map(|counted| snapshot.measure(node, counted))
                .collect(),
            ..Self::bare(source, node, snapshot)
        }
    }

    /// A node's own publication: the records beneath `node` and every count
    /// at the scope it was recorded at — two tests on one node count the same
    /// kinds, and a cluster assembling its nodes must tell them apart.
    #[must_use]
    pub fn whole(source: &str, node: &str, snapshot: &Snapshot) -> Self {
        Self {
            counts: snapshot.all_counts().cloned().collect(),
            ..Self::bare(source, node, snapshot)
        }
    }

    fn bare(source: &str, node: &str, snapshot: &Snapshot) -> Self {
        Self {
            source: source.to_string(),
            node: node.to_string(),
            records: snapshot.health(node),
            counts: Vec::new(),
            run: None,
            topology: None,
        }
    }

    /// The same publication saying what its run was started with.
    #[must_use]
    pub fn with_run(mut self, run: Option<Run>) -> Self {
        self.run = run;
        self
    }

    /// The same publication carrying its topology.
    #[must_use]
    pub fn with_topology(mut self, topology: Option<Topology>) -> Self {
        self.topology = topology;
        self
    }

    /// The publication as the TOML its readers read. A count at `node`
    /// itself is written without a scope, as a roll's sums always were.
    #[must_use]
    pub fn to_toml(&self) -> String {
        let document = Document {
            source: self.source.clone(),
            node: self.node.clone(),
            records: self.records.iter().map(RecordDocument::of).collect(),
            counts: self
                .counts
                .iter()
                .map(|count| CountDocument {
                    counted: count.counted.word().to_string(),
                    value: count.value,
                    scope: if count.scope == self.node {
                        String::new()
                    } else {
                        count.scope.clone()
                    },
                })
                .collect(),
            run: self.run.clone(),
            topology: self.topology.clone(),
        };
        toml::to_string(&document).unwrap_or_default()
    }

    /// A publication read back from its TOML. Counts are dated by the newest
    /// record, which is when the publisher last looked.
    ///
    /// # Errors
    ///
    /// When the text is not TOML of this shape, in the parser's words.
    pub fn read(text: &str) -> Result<Self, String> {
        let document: Document = toml::from_str(text).map_err(|error| error.to_string())?;
        let newest = document
            .records
            .iter()
            .map(|record| record.observed_unix_nanos)
            .max()
            .unwrap_or(0);
        let node = document.node;
        let counts = document
            .counts
            .into_iter()
            .filter_map(|count| {
                Some(Count {
                    scope: if count.scope.is_empty() {
                        node.clone()
                    } else {
                        count.scope
                    },
                    counted: Counted::named(&count.counted)?,
                    value: count.value,
                    window_start_unix_nanos: newest,
                    window_end_unix_nanos: newest,
                    observed_unix_nanos: newest,
                })
            })
            .collect();
        let topology = document.topology.map(|mut topology| {
            for drawn in &mut topology.nodes {
                if drawn.label.is_empty() {
                    drawn.label.clone_from(&drawn.id);
                }
            }
            topology
        });

        Ok(Self {
            source: document.source,
            node,
            records: document
                .records
                .into_iter()
                .map(RecordDocument::into_record)
                .collect(),
            counts,
            run: document.run,
            topology,
        })
    }

    /// The records and counts as a snapshot, for a publisher assembling
    /// several.
    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        let mut snapshot = Snapshot::new();
        for record in &self.records {
            snapshot.record_health(record.clone());
        }
        for count in &self.counts {
            snapshot.record_count(count.clone());
        }
        snapshot
    }
}

/// The file, as it lies.
#[derive(Serialize, Deserialize)]
struct Document {
    #[serde(default)]
    source: String,
    #[serde(default)]
    node: String,
    #[serde(default)]
    records: Vec<RecordDocument>,
    #[serde(default)]
    counts: Vec<CountDocument>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    run: Option<Run>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    topology: Option<Topology>,
}

#[derive(Serialize, Deserialize)]
struct RecordDocument {
    #[serde(default)]
    scope: String,
    #[serde(default = "mood::unknown", with = "mood")]
    state: Health,
    #[serde(default)]
    severity: u8,
    #[serde(default)]
    evidence: String,
    #[serde(default)]
    observed_unix_nanos: i64,
}

impl RecordDocument {
    fn of(record: &HealthRecord) -> Self {
        Self {
            scope: record.scope.clone(),
            state: record.health,
            severity: record.severity,
            evidence: record.evidence.clone(),
            observed_unix_nanos: record.observed_unix_nanos,
        }
    }

    fn into_record(self) -> HealthRecord {
        HealthRecord {
            scope: self.scope,
            health: self.state,
            severity: self.severity,
            evidence: self.evidence,
            observed_unix_nanos: self.observed_unix_nanos,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct CountDocument {
    counted: String,
    #[serde(default)]
    value: u64,
    /// Where the count was recorded; empty for a count at the publication's
    /// own scope.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    scope: String,
}

/// A mood as a publication writes it: [`Health::word`], and a word no mood
/// is called read as `Stressed`, so it shows and is looked at.
pub(crate) mod mood {
    use serde::{Deserialize, Deserializer, Serializer};

    use crate::health::Health;

    /// What a mood no one is called reads as.
    pub(crate) const fn unknown() -> Health {
        Health::Stressed
    }

    #[allow(clippy::trivially_copy_pass_by_ref)] // serde's `with` passes a reference
    pub(crate) fn serialize<S: Serializer>(
        health: &Health,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(health.word())
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Health, D::Error> {
        let word = String::deserialize(deserializer)?;
        Ok(Health::named(&word).unwrap_or_else(unknown))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::{NodeKind, TopologyNode};

    fn record(scope: &str, health: Health, observed: i64) -> HealthRecord {
        HealthRecord {
            scope: scope.to_string(),
            health,
            severity: if health == Health::Fine { 0 } else { 60 },
            evidence: "said".to_string(),
            observed_unix_nanos: observed,
        }
    }

    fn count(scope: &str, counted: Counted, value: u64) -> Count {
        Count {
            scope: scope.to_string(),
            counted,
            value,
            window_start_unix_nanos: 0,
            window_end_unix_nanos: 0,
            observed_unix_nanos: 0,
        }
    }

    fn published() -> Snapshot {
        let mut snapshot = Snapshot::new();
        snapshot.record_health(record("xmip:///n/receive/a", Health::Fine, 5));
        snapshot.record_health(record("xmip:///n/send/b", Health::Done, 9));
        snapshot.record_count(count("xmip:///n/receive/a", Counted::Streams, 3));
        snapshot.record_count(count("xmip:///n/send/b", Counted::Messages, 2));
        snapshot.record_count(count("xmip:///n/send/b", Counted::Bytes, 40));
        snapshot
    }

    #[test]
    fn a_roll_publishes_its_sums_at_its_scope_and_they_read_back_there() {
        let text = Publication::of("playground — n", "xmip:///n", &published()).to_toml();
        assert!(text.contains("state = \"done\""), "{text}");
        assert!(text.contains("counted = \"streams\""), "{text}");
        assert!(!text.contains("scope = \"xmip:///n\""), "{text}");

        let read = Publication::read(&text).expect("reads");
        assert_eq!(read.node, "xmip:///n");
        assert_eq!(read.records, published().health("xmip:///n"));
        let bytes = read
            .counts
            .iter()
            .find(|count| count.counted == Counted::Bytes)
            .expect("bytes");
        assert_eq!((bytes.scope.as_str(), bytes.value), ("xmip:///n", 40));
        assert_eq!(bytes.observed_unix_nanos, 9, "dated by the newest record");
    }

    #[test]
    fn a_nodes_own_file_keeps_each_count_where_it_was_recorded() {
        let written = published();
        let read = Publication::read(&Publication::whole("n", "xmip:///n", &written).to_toml())
            .expect("reads");
        let snapshot = read.snapshot();

        assert_eq!(snapshot.health("xmip:///n"), written.health("xmip:///n"));
        assert_eq!(
            snapshot
                .measure("xmip:///n/send", Counted::Messages)
                .map(|count| count.value),
            Some(2)
        );
        assert!(
            snapshot
                .measure("xmip:///n/send", Counted::Streams)
                .is_none()
        );
    }

    #[test]
    fn what_a_reader_does_not_know_shows_or_is_skipped_and_a_stranger_is_refused() {
        let text = "node = \"xmip:///n\"\n\
                    [[records]]\nscope = \"xmip:///n/a\"\nstate = \"sulking\"\n\
                    [[counts]]\ncounted = \"throughput\"\nvalue = 9\n\
                    [[counts]]\ncounted = \"failed\"\nvalue = 1\n";
        let read = Publication::read(text).expect("reads");
        assert_eq!(read.records[0].health, Health::Stressed);
        assert_eq!(read.counts.len(), 1);
        assert_eq!(read.counts[0].counted, Counted::Failed);

        assert!(Publication::read("not = [toml").is_err());
        assert_eq!(
            Publication::read("").map(|read| read.node),
            Ok(String::new())
        );
    }

    #[test]
    fn the_run_and_the_topology_ride_along_and_a_missing_label_is_the_id() {
        let run = Run {
            cluster: "C1".to_string(),
            nodes: vec!["R1".to_string()],
            capabilities: vec!["R1=receive".to_string()],
            ..Run::default()
        };
        let topology = Topology {
            source: "drawn".to_string(),
            observed_unix_nanos: 7,
            nodes: vec![TopologyNode {
                id: "cluster".to_string(),
                parent: String::new(),
                label: String::new(),
                kind: NodeKind::Cluster,
                scope: "xmip:///C1".to_string(),
                state: Health::Fine,
                origin: crate::topology::Origin::Configured,
                load: 0.0,
                activity: 0.5,
                evidence: String::new(),
            }],
            links: Vec::new(),
        };
        let text = Publication::of("roll", "xmip:///C1", &Snapshot::new())
            .with_run(Some(run.clone()))
            .with_topology(Some(topology))
            .to_toml();
        assert!(
            text.contains("[run]") && text.contains("[[topology.nodes]]"),
            "{text}"
        );

        let read = Publication::read(&text).expect("reads");
        assert_eq!(read.run, Some(run));
        let drawn = read.topology.expect("a topology");
        assert_eq!(drawn.nodes[0].label, "cluster");
        assert_eq!(drawn.nodes[0].kind, NodeKind::Cluster);
    }
}
