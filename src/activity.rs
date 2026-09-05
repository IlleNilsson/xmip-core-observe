//! Activity: the recent individual items, not only the counts. ADR-0032.
//!
//! [`Snapshot`](crate::Snapshot) is the present and [`History`](crate::History)
//! is the curve; `Activity` is the individual items behind them — the Streams
//! that arrived, the Messages that were sent, the Journeys that ran. Drilling
//! down reaches these, so an operator sees not a redder number but the item
//! behind it.
//!
//! Bounded: the last `capacity` items, oldest dropped. Recent, not retention —
//! it points at an item (scope, id, size, when, a line of detail), it never
//! holds the content. Content that must be opened and replayed is retention's.

use std::collections::VecDeque;

/// Which of the three units an item is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind {
    /// A Stream that arrived.
    Stream,
    /// A Message that was sent.
    Message,
    /// A Journey that ran.
    Journey,
}

impl ItemKind {
    /// The token as it appears in a scope, a filter and a UI.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            ItemKind::Stream => "stream",
            ItemKind::Message => "message",
            ItemKind::Journey => "journey",
        }
    }
}

/// One observed item: what it is, where, its identity, its size, when it was
/// seen, and a short line of detail (an outcome, a Journey state).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub kind: ItemKind,
    pub scope: String,
    pub id: String,
    pub bytes: u64,
    pub detail: String,
    pub observed_unix_nanos: i64,
}

/// The default number of items kept: enough to see what just flowed without
/// unbounded growth.
pub const DEFAULT_CAPACITY: usize = 512;

/// A bounded ring of the most recent observed items.
pub struct Activity {
    capacity: usize,
    items: VecDeque<Item>,
}

impl Activity {
    /// An activity log keeping `capacity` items, clamped to at least one.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            items: VecDeque::new(),
        }
    }

    /// Record one item, dropping the oldest once over capacity.
    pub fn record(&mut self, item: Item) {
        self.items.push_back(item);
        while self.items.len() > self.capacity {
            self.items.pop_front();
        }
    }

    /// The recent items at or beneath `scope`, newest first, at most `limit`.
    /// `kind` filters to one of the three when given.
    #[must_use]
    pub fn recent(&self, scope: &str, kind: Option<ItemKind>, limit: usize) -> Vec<Item> {
        self.items
            .iter()
            .rev()
            .filter(|item| beneath(&item.scope, scope))
            .filter(|item| kind.is_none_or(|wanted| item.kind == wanted))
            .take(limit)
            .cloned()
            .collect()
    }

    /// How many items are held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether nothing has been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Default for Activity {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }
}

/// A candidate scope is at or beneath a query scope when the query's segments
/// are a prefix of the candidate's. An empty query is above everything.
fn beneath(candidate: &str, scope: &str) -> bool {
    let trimmed = scope.trim_end_matches('/');

    trimmed.is_empty()
        || candidate == trimmed
        || (candidate.starts_with(trimmed)
            && candidate.as_bytes().get(trimmed.len()) == Some(&b'/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(kind: ItemKind, scope: &str, id: &str, now: i64) -> Item {
        Item {
            kind,
            scope: scope.to_string(),
            id: id.to_string(),
            bytes: 10,
            detail: "delivered".to_string(),
            observed_unix_nanos: now,
        }
    }

    #[test]
    fn recent_returns_items_beneath_a_scope_newest_first() {
        let mut activity = Activity::default();
        activity.record(item(ItemKind::Stream, "xmip:///n/receive/a", "s1", 1));
        activity.record(item(ItemKind::Message, "xmip:///n/send/b", "m1", 2));
        activity.record(item(ItemKind::Stream, "xmip:///n/receive/a", "s2", 3));

        let at_a = activity.recent("xmip:///n/receive/a", None, 10);
        assert_eq!(at_a.len(), 2);
        assert_eq!(at_a[0].id, "s2", "newest first");
        assert_eq!(at_a[1].id, "s1");

        assert_eq!(
            activity.recent("xmip:///n", None, 10).len(),
            3,
            "beneath the node"
        );
    }

    #[test]
    fn recent_filters_by_kind() {
        let mut activity = Activity::default();
        activity.record(item(ItemKind::Stream, "xmip:///n/x", "s1", 1));
        activity.record(item(ItemKind::Message, "xmip:///n/x", "m1", 2));

        let messages = activity.recent("xmip:///n", Some(ItemKind::Message), 10);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].kind, ItemKind::Message);
    }

    #[test]
    fn recent_honours_the_limit() {
        let mut activity = Activity::default();
        for tick in 0..5 {
            activity.record(item(ItemKind::Stream, "xmip:///n/x", "s", tick));
        }

        assert_eq!(activity.recent("xmip:///n", None, 2).len(), 2);
    }

    #[test]
    fn the_ring_is_bounded_and_drops_the_oldest() {
        let mut activity = Activity::with_capacity(2);
        for tick in 0..5 {
            activity.record(item(
                ItemKind::Stream,
                "xmip:///n/x",
                &format!("s{tick}"),
                tick,
            ));
        }

        assert_eq!(activity.len(), 2);
        let held = activity.recent("xmip:///n", None, 10);
        assert_eq!(held[0].id, "s4");
        assert_eq!(held[1].id, "s3");
    }

    #[test]
    fn an_unrelated_scope_has_nothing() {
        let mut activity = Activity::default();
        activity.record(item(ItemKind::Stream, "xmip:///n/x", "s1", 1));

        assert!(activity.recent("xmip:///other", None, 10).is_empty());
    }
}
