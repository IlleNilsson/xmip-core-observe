//! Scope: an Xmip URI over the execution tree, and the one question every
//! reader asks of it. ADR-0027 clauses 3 and 4.
//!
//! A scope is `xmip://[userinfo@][host][:port]/path`, and the tree is the
//! path: an omitted host means estate-wide, so the scheme and the authority
//! go before anything is compared. A record at `xmip:///edge-01/transport/ftp`
//! sits beneath `xmip:///edge-01/transport` and beneath `xmip:///edge-01`, so
//! asking for a node gets everything the node holds. That prefix rule is the
//! whole aggregation model, and it is one rule: `snapshot.rs` and
//! `activity.rs` each carried a predicate of their own until 2026-09-14, and
//! until 2026-09-24 this crate compared the whole text while `Xmip.Surface`'s
//! `ScopeTree` compared the path (open problem 25, row k).
//!
//! It is written once, here. The runtime's cdylib forwards it to the surfaces
//! over `xmip_operate.h` section 7 — `xmip_scope_contains_v1` and
//! `xmip_scope_parts_v1` — and `ScopeTree` in `Xmip.Surface` calls those
//! rather than keeping a writing of its own (ADR-0052, amendment 2026-09-24:
//! one implementation, the surfaces call the runtime's exports).

/// The scheme every scope carries, lower-case: `XMIP:///C1` is a path, not a
/// scope, as `ScopeTree` and `ScopePattern` read it (ADR-0052, amendment
/// 2026-09-14).
const SCHEME: &str = "xmip://";

/// A scope, read as the path it names in the one tree.
///
/// Borrowed from the text it was read from. Two scopes are equal when they
/// name the same path: `xmip://edge-01/n/receive` and `xmip:///n/receive/`
/// are one place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scope<'a> {
    /// The path after the scheme and the authority, with no slash at either
    /// end. Empty is the root.
    path: &'a str,
}

impl<'a> Scope<'a> {
    /// Read a scope. Text without the scheme is taken as the path itself, and
    /// empty text, or `/`, is the root.
    ///
    /// **An empty scope is above everything.** The operator boundary crosses a
    /// `{ NULL, 0 }` scope as `""` (`scope_text` in the runtime's
    /// `operate.rs`), so a surface that names no scope is asking for the whole
    /// node. Every caller reads or acts on *at and beneath*, and the whole node
    /// is what is at and beneath nothing in particular.
    #[must_use]
    pub fn new(text: &'a str) -> Self {
        let path = match text.strip_prefix(SCHEME) {
            Some(rest) => rest.split_once('/').map_or("", |(_, path)| path),
            None => text,
        };

        Self {
            path: path.trim_matches('/'),
        }
    }

    /// The path, without the scheme, the authority or a slash at either end.
    #[must_use]
    pub fn path(self) -> &'a str {
        self.path
    }

    /// Whether this is the root, which contains every scope.
    #[must_use]
    pub fn is_root(self) -> bool {
        self.path.is_empty()
    }

    /// The segments of the path, top first: `xmip:///edge-01/receive/orders`
    /// is `edge-01`, `receive`, `orders`. An empty segment — `a//b` — is no
    /// segment, and the root has none. Borrowed from the text the scope was
    /// read from.
    pub fn segments(self) -> impl Iterator<Item = &'a str> {
        self.path.split('/').filter(|segment| !segment.is_empty())
    }

    /// Whether `other` is this scope or sits beneath it in the tree.
    ///
    /// A prefix of segments, not of characters: `xmip:///ab` is not beneath
    /// `xmip:///a`. Compared without splitting either path, because a surface
    /// compares thousands of records at a time.
    #[must_use]
    pub fn contains(self, other: Scope<'_>) -> bool {
        self.is_root()
            || other
                .path
                .strip_prefix(self.path)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn beneath(candidate: &str, scope: &str) -> bool {
        Scope::new(scope).contains(Scope::new(candidate))
    }

    fn parts(scope: &str) -> Vec<&str> {
        Scope::new(scope).segments().collect()
    }

    #[test]
    fn a_scope_is_beneath_itself_and_its_ancestors() {
        assert!(beneath("xmip:///n/receive/a", "xmip:///n/receive/a"));
        assert!(beneath("xmip:///n/receive/a", "xmip:///n/receive"));
        assert!(beneath("xmip:///n/receive/a", "xmip:///n"));
        assert!(beneath("xmip:///n/receive/a", "xmip:///"));
        assert!(!beneath("xmip:///n", "xmip:///n/receive"), "never above");
    }

    #[test]
    fn segments_are_the_unit_not_characters() {
        assert!(!beneath("xmip:///ab", "xmip:///a"));
        assert!(!beneath("xmip:///nx", "xmip:///n"));
        assert!(!beneath("xmip:///n/receive-b", "xmip:///n/receive"));
        assert!(!beneath("xmip:///n", "xmip:///n/receive"));
    }

    #[test]
    fn a_slash_at_either_end_is_ignored() {
        assert!(beneath("xmip:///n/receive/a", "xmip:///n/receive/"));
        assert!(beneath("xmip:///n/receive", "xmip:///n/receive/"));
        assert!(beneath("xmip:///n/receive/", "xmip:///n/receive"));
        assert!(beneath("xmip:///n/receive/a", "/n/receive/"));
        assert_eq!(Scope::new("/n/receive/").path(), "n/receive");
    }

    #[test]
    fn the_root_has_four_spellings_and_contains_everything() {
        for root in ["", "/", "xmip://", "xmip:///"] {
            assert!(Scope::new(root).is_root(), "{root:?} is the root");
            assert_eq!(Scope::new(root).path(), "");
            assert!(beneath("xmip:///n/receive/a", root));
        }
        assert!(beneath("", ""), "the root contains the root");
        assert!(
            !beneath("", "xmip:///n"),
            "a node does not contain the root"
        );
    }

    #[test]
    fn the_authority_is_not_part_of_the_tree() {
        // ADR-0027 clause 3: an omitted host means estate-wide, and the tree
        // is the path. The text comparison this replaced said no to both.
        assert!(beneath("xmip://edge-01/n/receive/a", "xmip:///n"));
        assert!(beneath("xmip:///n/receive/a", "xmip://ops@edge-01:7400/n"));
        assert_eq!(Scope::new("xmip://ops@edge-01:7400/n").path(), "n");
        assert_eq!(Scope::new("xmip://edge-01/n"), Scope::new("xmip:///n/"));
        assert!(Scope::new("xmip://edge-01").is_root());
    }

    #[test]
    fn text_without_the_scheme_is_a_path() {
        assert!(beneath("xmip:///n/receive/a", "n/receive"));
        assert!(beneath("n/receive/a", "xmip:///n"));
        assert_eq!(Scope::new("n/receive").path(), "n/receive");
    }

    #[test]
    fn the_scheme_is_lower_case_and_segments_compare_case_sensitively() {
        // An upper-case scheme is the start of a path, on both sides alike.
        assert_eq!(Scope::new("XMIP:///C1").path(), "XMIP:///C1");
        assert!(!beneath("XMIP:///C1", "xmip:///C1"));
        assert!(beneath("XMIP:///C1/node", "XMIP:///C1"));
        assert!(!beneath("xmip:///N/receive", "xmip:///n"));
    }

    #[test]
    fn a_query_and_a_fragment_are_part_of_the_path_today() {
        // ADR-0027 clause 3 puts a Party filter in the query, and no scope the
        // estate publishes carries one yet; this is the behavior until then.
        assert!(beneath(
            "xmip:///n/receive/a?party=partner-x",
            "xmip:///n/receive"
        ));
        assert_eq!(
            Scope::new("xmip:///n/receive?party=partner-x").path(),
            "n/receive?party=partner-x"
        );
        assert!(!beneath("xmip:///n/receive/a", "xmip:///n/receive?party=x"));
        assert!(!beneath("xmip:///n", "xmip:///n#top"));
    }

    #[test]
    fn the_segments_are_the_path_split_top_first() {
        assert_eq!(
            parts("xmip:///edge-01/receive/orders"),
            ["edge-01", "receive", "orders"]
        );
        assert_eq!(
            parts("xmip://lab:9000/edge-01/receive/"),
            ["edge-01", "receive"]
        );
        assert_eq!(parts("edge-01/receive"), ["edge-01", "receive"]);
        assert_eq!(parts("xmip:///a//b"), ["a", "b"]);
        assert!(parts("xmip:///").is_empty());
        assert!(parts("").is_empty());
        assert!(parts("xmip://edge-01").is_empty());
    }
}
