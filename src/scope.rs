//! Scope: an Xmip URI path over the execution tree, and the one question every
//! reader asks of it. ADR-0027 clause 4.
//!
//! A record at `xmip:///edge-01/transport/ftp` sits beneath
//! `xmip:///edge-01/transport` and beneath `xmip:///edge-01`, so asking for a
//! node gets everything the node holds. That prefix rule is the whole
//! aggregation model, and it is one rule: `snapshot.rs` and `activity.rs`
//! each carried a predicate of their own until 2026-09-14, and the two
//! disagreed about an empty query (ADR-0044, inside one crate).

/// Whether `candidate` is `scope` or sits beneath it in the tree.
///
/// A prefix of segments, not of characters: `xmip:///ab` is not beneath
/// `xmip:///a`. A trailing slash on the query is ignored.
///
/// **An empty query is above everything.** The operator boundary crosses a
/// `{ NULL, 0 }` scope as `""` (`scope_text` in the runtime's `operate.rs`)
/// and `Xmip.Surface`'s `ScopeTree.Beneath` reads zero segments as a prefix
/// of every path, so a surface that names no scope is asking for the whole
/// node on both sides of the boundary. `Snapshot` used to answer that with
/// nothing — a health read that found nothing, a pause that paused nothing
/// and reported not found — while `Activity` answered with everything. Every
/// caller reads or acts on *at and beneath*, and the whole node is what is at
/// and beneath nothing in particular.
#[must_use]
pub fn beneath(candidate: &str, scope: &str) -> bool {
    let scope = scope.trim_end_matches('/');

    scope.is_empty()
        || candidate == scope
        || candidate
            .strip_prefix(scope)
            .is_some_and(|rest| rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scope_is_beneath_itself_and_its_ancestors() {
        assert!(beneath("xmip:///n/receive/a", "xmip:///n/receive/a"));
        assert!(beneath("xmip:///n/receive/a", "xmip:///n/receive"));
        assert!(beneath("xmip:///n/receive/a", "xmip:///n"));
        assert!(beneath("xmip:///n/receive/a", "xmip:///"));
    }

    #[test]
    fn segments_are_the_unit_not_characters() {
        assert!(!beneath("xmip:///ab", "xmip:///a"));
        assert!(!beneath("xmip:///n/receive-b", "xmip:///n/receive"));
        assert!(!beneath("xmip:///n", "xmip:///n/receive"));
    }

    #[test]
    fn a_trailing_slash_on_the_query_is_ignored() {
        assert!(beneath("xmip:///n/receive/a", "xmip:///n/receive/"));
        assert!(beneath("xmip:///n/receive", "xmip:///n/receive/"));
    }

    #[test]
    fn an_empty_query_is_above_everything() {
        assert!(beneath("xmip:///n/receive/a", ""));
        assert!(beneath("xmip:///n", "/"));
    }
}
