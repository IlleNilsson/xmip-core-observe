//! The figures observation exports: what an operator's own monitoring — an
//! OpenTelemetry collector, a Prometheus server — reads of a node, named
//! once.
//!
//! A figure is a name, a unit, a line saying what it is, how its value
//! reads over time, and the points a [`Snapshot`] holds of it, one per
//! scope. The exporters under this repository write the same figures, each
//! in its own format: `otlp` as OpenTelemetry metrics, `prometheus` as the
//! text exposition format. Neither keeps a list of its own, so a figure
//! added here reaches both, and one read differently by the two is not
//! possible (observability-model.md section 6).
//!
//! A mood is exported as its rank, `Fine` 0 to `Holding` 6, with its word
//! beside it on every point; a count as the value its window holds. Nothing
//! here decides what a monitoring system alerts on: the mood already says
//! what an operator should do.

use crate::counted::Counted;
use crate::health::Health;
use crate::snapshot::{Count, HealthRecord, Snapshot};

/// How a figure's value reads over time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A level at the moment it was observed: a mood, a severity, what is
    /// awaiting another try.
    Gauge,
    /// A count over its window, starting again from nothing each window:
    /// what arrived, ran, was sent or failed in it. Never negative.
    Window,
}

/// Where a figure's points come from in a snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Source {
    Mood,
    Severity,
    Count(Counted),
}

/// One figure an exporter writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Figure {
    /// The name, segment by segment: OTLP joins the segments with dots,
    /// `xmip.health.severity`, Prometheus with underscores.
    pub segments: &'static [&'static str],
    /// The unit as UCUM writes it and OTLP carries it: `By` for bytes, an
    /// annotation in braces for a count of things, which carries no unit.
    pub unit: &'static str,
    /// What the figure is, in one line.
    pub description: &'static str,
    pub kind: Kind,
    source: Source,
}

/// One point of a figure: the scope it is of, its value, and when.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point<'a> {
    /// The Xmip URI of the scope, as the snapshot holds it.
    pub scope: &'a str,
    /// Where the scope stands in [`Reading::scopes`].
    pub at: usize,
    /// The scope's mood, for a figure read from its health; `None` for a
    /// count.
    pub mood: Option<Health>,
    pub value: u64,
    /// Where the window opened, for a [`Kind::Window`] figure; when the
    /// value was observed otherwise.
    pub start_unix_nanos: i64,
    /// Where the window closed, or when the value was observed.
    pub time_unix_nanos: i64,
}

impl Point<'_> {
    /// The name the scope travels under beside a point's value: an
    /// attribute in OTLP, a label in Prometheus.
    pub const SCOPE: &'static str = "scope";

    /// The name the mood's word travels under, where a point has one.
    pub const MOOD: &'static str = "mood";
}

/// A figure of what a count counts.
const fn counted(
    segments: &'static [&'static str],
    unit: &'static str,
    description: &'static str,
    kind: Kind,
    counted: Counted,
) -> Figure {
    Figure {
        segments,
        unit,
        description,
        kind,
        source: Source::Count(counted),
    }
}

/// Every figure, in the order an export writes them.
pub const FIGURES: [Figure; 8] = [
    Figure {
        segments: &["xmip", "health"],
        unit: "{mood}",
        description: "The mood of a scope by its rank: fine 0, paused 1, working 2, \
                      stressed 3, exhausted 4, done 5, holding 6 (ADR-0041).",
        kind: Kind::Gauge,
        source: Source::Mood,
    },
    Figure {
        segments: &["xmip", "health", "severity"],
        unit: "{severity}",
        description: "How far a scope is into its mood, 0 the mildest and 100 as bad \
                      as that mood gets.",
        kind: Kind::Gauge,
        source: Source::Severity,
    },
    counted(
        &["xmip", Counted::Streams.word()],
        "{stream}",
        "Streams received in the window.",
        Kind::Window,
        Counted::Streams,
    ),
    counted(
        &["xmip", Counted::Messages.word()],
        "{message}",
        "Messages sent in the window.",
        Kind::Window,
        Counted::Messages,
    ),
    counted(
        &["xmip", Counted::Journeys.word()],
        "{journey}",
        "Journeys run in the window.",
        Kind::Window,
        Counted::Journeys,
    ),
    counted(
        &["xmip", Counted::Bytes.word()],
        "By",
        "Bytes carried in the window.",
        Kind::Window,
        Counted::Bytes,
    ),
    counted(
        &["xmip", Counted::Retrying.word()],
        "{attempt}",
        "Delivery or processing attempts awaiting another try.",
        Kind::Gauge,
        Counted::Retrying,
    ),
    counted(
        &["xmip", Counted::Failed.word()],
        "{outcome}",
        "Delivery or processing outcomes that ended unsuccessfully in the window.",
        Kind::Window,
        Counted::Failed,
    ),
];

impl Figure {
    /// The name, its segments joined by `separator`.
    #[must_use]
    pub fn name(&self, separator: char) -> String {
        let mut name = String::with_capacity(32);
        for (index, segment) in self.segments.iter().enumerate() {
            if index > 0 {
                name.push(separator);
            }
            name.push_str(segment);
        }
        name
    }
}

/// A snapshot read for export, in one pass over each of its two maps: the
/// scopes it holds, each once and in order, and each figure's points
/// knowing which of those scopes they are of.
///
/// An export writes figure after figure, and every figure's points carry
/// their scope: asked of the snapshot once per figure, the counts were
/// walked six times over, and a scope's text written once per figure it
/// has — eight times for a scope with a mood and every count. Read here
/// once, an exporter writes each scope's text once ([`Reading::scopes`])
/// and each point takes its scope's by [`Point::at`].
pub struct Reading<'s> {
    scopes: Vec<&'s str>,
    health: Vec<(&'s HealthRecord, usize)>,
    counts: [Vec<(&'s Count, usize)>; Counted::ALL.len()],
}

impl<'s> Reading<'s> {
    /// Read `snapshot`. Its health and its counts are each in scope order,
    /// so the scopes are the two merged.
    #[must_use]
    pub fn of(snapshot: &'s Snapshot) -> Self {
        let mut health = snapshot.health_records().peekable();
        let mut counts = snapshot.all_counts().peekable();
        let mut reading = Self {
            scopes: Vec::new(),
            health: Vec::new(),
            counts: Default::default(),
        };
        loop {
            let next = match (health.peek(), counts.peek()) {
                (Some(record), Some(count)) => record.scope.as_str().min(count.scope.as_str()),
                (Some(record), None) => record.scope.as_str(),
                (None, Some(count)) => count.scope.as_str(),
                (None, None) => break,
            };
            let at = reading.scopes.len();
            reading.scopes.push(next);
            while let Some(record) = health.next_if(|record| record.scope == next) {
                reading.health.push((record, at));
            }
            while let Some(count) = counts.next_if(|count| count.scope == next) {
                reading.counts[count.counted as usize].push((count, at));
            }
        }
        reading
    }

    /// Every scope the snapshot holds a point of, each once, in order.
    #[must_use]
    pub fn scopes(&self) -> &[&'s str] {
        &self.scopes
    }

    /// The points of `figure`, one per scope, in scope order.
    #[must_use]
    pub fn points(&self, figure: &Figure) -> Points<'_, 's> {
        type Health<'r, 's> = &'r [(&'s HealthRecord, usize)];
        type Counts<'r, 's> = &'r [(&'s Count, usize)];
        let (health, counts): (Health<'_, 's>, Counts<'_, 's>) = match figure.source {
            Source::Mood | Source::Severity => (&self.health, &[]),
            Source::Count(counted) => (&[], &self.counts[counted as usize]),
        };
        Points {
            source: figure.source,
            health: health.iter(),
            counts: counts.iter(),
        }
    }
}

/// The points of one figure in a [`Reading`].
pub struct Points<'r, 's> {
    source: Source,
    health: std::slice::Iter<'r, (&'s HealthRecord, usize)>,
    counts: std::slice::Iter<'r, (&'s Count, usize)>,
}

impl<'s> Iterator for Points<'_, 's> {
    type Item = Point<'s>;

    fn next(&mut self) -> Option<Point<'s>> {
        if let Some(&(count, at)) = self.counts.next() {
            return Some(Point {
                scope: &count.scope,
                at,
                mood: None,
                value: count.value,
                start_unix_nanos: count.window_start_unix_nanos,
                time_unix_nanos: count.window_end_unix_nanos,
            });
        }
        let &(record, at) = self.health.next()?;
        Some(Point {
            scope: &record.scope,
            at,
            mood: Some(record.health),
            value: if self.source == Source::Severity {
                u64::from(record.severity)
            } else {
                u64::from(record.health.rank())
            },
            start_unix_nanos: record.observed_unix_nanos,
            time_unix_nanos: record.observed_unix_nanos,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = self.health.len() + self.counts.len();
        (left, Some(left))
    }
}

impl ExactSizeIterator for Points<'_, '_> {}

#[cfg(test)]
mod tests {
    use super::*;
    use configure::fixture::test_cluster;

    /// `leaf` beneath the test cluster's first node.
    fn at(leaf: &str) -> String {
        format!("{}/{leaf}", test_cluster().node_scope(0))
    }

    fn snapshot() -> Snapshot {
        let mut snapshot = Snapshot::new();
        snapshot.record_health(HealthRecord {
            scope: at("receive/a"),
            health: Health::Stressed,
            severity: 40,
            evidence: String::new(),
            observed_unix_nanos: 9,
        });
        snapshot.record_count(Count {
            scope: at("receive/a"),
            counted: Counted::Bytes,
            value: 512,
            window_start_unix_nanos: 1,
            window_end_unix_nanos: 5,
            observed_unix_nanos: 5,
        });
        snapshot
    }

    #[test]
    fn every_counted_kind_has_one_figure_named_by_its_word() {
        for kind in Counted::ALL {
            let named: Vec<&Figure> = FIGURES
                .iter()
                .filter(|figure| figure.source == Source::Count(kind))
                .collect();
            assert_eq!(named.len(), 1, "{kind:?}");
            assert_eq!(named[0].name('.'), format!("xmip.{}", kind.word()));
        }
    }

    #[test]
    fn a_mood_is_its_rank_with_its_word_and_a_severity_its_number() {
        let snapshot = snapshot();
        let mood: Vec<Point> = Reading::of(&snapshot).points(&FIGURES[0]).collect();
        assert_eq!(mood.len(), 1);
        assert_eq!((mood[0].value, mood[0].mood), (3, Some(Health::Stressed)));
        let severity: Vec<Point> = Reading::of(&snapshot).points(&FIGURES[1]).collect();
        assert_eq!(severity[0].value, 40);
        assert_eq!(FIGURES[1].name('_'), "xmip_health_severity");
    }

    #[test]
    fn a_count_is_its_window_and_reaches_only_its_own_figure() {
        let snapshot = snapshot();
        let bytes = FIGURES
            .iter()
            .find(|figure| figure.unit == "By")
            .expect("bytes");
        let points: Vec<Point> = Reading::of(&snapshot).points(bytes).collect();
        assert_eq!(points.len(), 1);
        assert_eq!(
            (
                points[0].value,
                points[0].start_unix_nanos,
                points[0].time_unix_nanos
            ),
            (512, 1, 5)
        );
        assert_eq!(points[0].mood, None);
        let others: usize = FIGURES
            .iter()
            .filter(|figure| figure.unit != "By")
            .filter(|figure| matches!(figure.source, Source::Count(_)))
            .map(|figure| Reading::of(&snapshot).points(figure).len())
            .sum();
        assert_eq!(others, 0);
    }

    #[test]
    fn every_scope_is_read_once_and_every_point_knows_its_own() {
        let mut snapshot = snapshot();
        // A scope counted and never given a mood, and one before it in order.
        for scope in [at("send/b"), at("process/p")] {
            snapshot.record_count(Count {
                scope,
                counted: Counted::Failed,
                value: 1,
                window_start_unix_nanos: 1,
                window_end_unix_nanos: 5,
                observed_unix_nanos: 5,
            });
        }
        let reading = Reading::of(&snapshot);
        assert_eq!(
            reading.scopes(),
            [at("process/p"), at("receive/a"), at("send/b")]
        );
        for figure in &FIGURES {
            for point in reading.points(figure) {
                assert_eq!(reading.scopes()[point.at], point.scope);
            }
        }
        // What a count counts is its place in the list of every kind.
        for (place, counted) in Counted::ALL.into_iter().enumerate() {
            assert_eq!(counted as usize, place);
        }
    }
}
