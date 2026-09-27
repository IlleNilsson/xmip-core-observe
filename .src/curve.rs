//! A curve: a node's throughput over time, as the file a surface reads
//! (ADR-0029), written and read here and nowhere else.
//!
//! [`History`] keeps the series in memory; a publisher writes the node's own
//! series of the kinds an operator watches over time to a file, and a surface
//! reads it back. The file's shape — `node` and `[[points]]` of `counted`,
//! `observed_unix_nanos` and `value` — was written by the Playground and
//! walked again by the estate's `Get-XmipHistory` until 2026-09-24 (open
//! problem 25). It has one home now: the Playground writes through
//! [`Curve::to_toml`], and a surface reads through [`Curve::read`] in the
//! runtime's library (`xmip_curve_read_v1`, `xmip_operate.h` section 8).
//!
//! A counted kind no one is called is skipped, as a publication's reader
//! skips it; a file that is not this shape at all is refused whole.

use serde::{Deserialize, Serialize};

use crate::counted::Counted;
use crate::history::History;
use crate::snapshot::Count;

/// A node's curve, read or about to be written.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Curve {
    /// The scope whose series it is.
    pub node: String,
    /// One point per kind per observation, oldest first within a kind.
    pub points: Vec<Count>,
}

impl Curve {
    /// The kinds a curve carries: what an operator watches over time.
    pub const KINDS: [Counted; 3] = [Counted::Streams, Counted::Messages, Counted::Bytes];

    /// The node's own series of [`Curve::KINDS`], at its exact scope — a
    /// series is not rolled up on the way out.
    #[must_use]
    pub fn of(node: &str, history: &History) -> Self {
        Self {
            node: node.to_string(),
            points: Self::KINDS
                .into_iter()
                .flat_map(|counted| history.count_series(node, counted))
                .collect(),
        }
    }

    /// The curve as the TOML its readers read.
    #[must_use]
    pub fn to_toml(&self) -> String {
        let document = Document {
            node: self.node.clone(),
            points: self
                .points
                .iter()
                .map(|point| PointDocument {
                    counted: point.counted.word().to_string(),
                    observed_unix_nanos: point.observed_unix_nanos,
                    value: point.value,
                })
                .collect(),
        };
        toml::to_string(&document).unwrap_or_default()
    }

    /// A curve read back from its TOML: every point at the curve's node, its
    /// window the instant it was observed.
    ///
    /// # Errors
    ///
    /// When the text is not TOML of this shape, in the parser's words.
    pub fn read(text: &str) -> Result<Self, String> {
        let document: Document = toml::from_str(text).map_err(|error| error.to_string())?;
        let node = document.node;
        let points = document
            .points
            .into_iter()
            .filter_map(|point| {
                Some(Count {
                    scope: node.clone(),
                    counted: Counted::named(&point.counted)?,
                    value: point.value,
                    window_start_unix_nanos: point.observed_unix_nanos,
                    window_end_unix_nanos: point.observed_unix_nanos,
                    observed_unix_nanos: point.observed_unix_nanos,
                })
            })
            .collect();
        Ok(Self { node, points })
    }
}

#[derive(Serialize, Deserialize)]
struct Document {
    #[serde(default)]
    node: String,
    #[serde(default)]
    points: Vec<PointDocument>,
}

#[derive(Serialize, Deserialize)]
struct PointDocument {
    counted: String,
    #[serde(default)]
    observed_unix_nanos: i64,
    #[serde(default)]
    value: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::Snapshot;

    fn count(scope: &str, counted: Counted, value: u64, at: i64) -> Count {
        Count {
            scope: scope.to_string(),
            counted,
            value,
            window_start_unix_nanos: at,
            window_end_unix_nanos: at,
            observed_unix_nanos: at,
        }
    }

    #[test]
    fn a_nodes_own_series_crosses_the_file_whole() {
        let mut history = History::default();
        for (value, at) in [(3, 10), (5, 20)] {
            let mut snapshot = Snapshot::new();
            snapshot.record_count(count("xmip:///n", Counted::Bytes, value, at));
            snapshot.record_count(count("xmip:///n", Counted::Failed, 1, at));
            snapshot.record_count(count("xmip:///n/receive", Counted::Streams, 9, at));
            history.record(&snapshot);
        }

        let curve = Curve::of("xmip:///n", &history);
        assert_eq!(
            curve.points.len(),
            2,
            "the node's own bytes, nothing rolled up"
        );

        let text = curve.to_toml();
        assert!(text.contains("counted = \"bytes\""), "{text}");
        assert_eq!(Curve::read(&text), Ok(curve));
    }

    #[test]
    fn an_unknown_kind_is_skipped_and_a_stranger_is_refused() {
        let read = Curve::read(
            "node = \"xmip:///n\"\n[[points]]\ncounted = \"throughput\"\nvalue = 1\n\
             [[points]]\ncounted = \"messages\"\nvalue = 7\nobserved_unix_nanos = 4\n",
        )
        .expect("reads");

        assert_eq!(read.points, [count("xmip:///n", Counted::Messages, 7, 4)]);
        assert!(Curve::read("points = 3").is_err());
        assert_eq!(Curve::read("").map(|curve| curve.points.len()), Ok(0));
    }
}
