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
//! one implementation, the surfaces call the runtime's exports). The node a
//! scope is on and its stage there are asked here too, and crossed as
//! `xmip_scope_node_v1`; until 2026-09-25 the Monitor took the first segment
//! for the node and the prompt looked for the marker itself (row q).

use node::Stage;

/// The scheme every scope carries, lower-case: `XMIP:///C1` is a path, not a
/// scope, as `ScopeTree` and `ScopePattern` read it (ADR-0052, amendment
/// 2026-09-14).
const SCHEME: &str = "xmip://";

/// The segment that marks a node beneath its cluster:
/// `xmip:///<cluster>/node/<name>`, the location ADR-0053 gives a node. A
/// kind the shape carries, not a word a name may not use: a node called
/// `node` is `xmip:///C1/node/node`.
pub const NODE: &str = "node";

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

    /// The node this scope is on: the segment after the node marker beneath
    /// the cluster, `alpha` in `xmip:///C1/node/alpha/receive/tcp`. `None`
    /// for a scope on no node — the cluster, its `node` rollup, a test run at
    /// the cluster — because the cluster is never a node. Read by position,
    /// so no name is taken for a kind (open problem 25, row q).
    #[must_use]
    pub fn node(self) -> Option<&'a str> {
        let mut segments = self.segments();
        match (segments.next(), segments.next(), segments.next()) {
            (Some(_), Some(NODE), Some(name)) => Some(name),
            _ => None,
        }
    }

    /// The stage of the message path this scope is on: the first stage word
    /// beneath its node, or, on no node, beneath its cluster —
    /// `xmip:///C1/node/alpha/receive/tcp` and
    /// `xmip:///C1/round-trip/send/tcp/json` alike. A cluster's or a node's
    /// name is never read as a stage, so a node called `send` is no stage.
    #[must_use]
    pub fn stage(self) -> Option<Stage> {
        let beneath = if self.node().is_some() { 3 } else { 1 };
        self.segments().skip(beneath).find_map(Stage::named)
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
    fn the_node_follows_the_marker_beneath_the_cluster_and_the_cluster_is_none() {
        let node = |scope: &'static str| Scope::new(scope).node();
        assert_eq!(node("xmip:///C1/node/alpha/receive/tcp"), Some("alpha"));
        assert_eq!(node("xmip://lab/C1/node/alpha"), Some("alpha"));
        assert_eq!(node("xmip:///C1/node/node/send"), Some("node"));
        for none in [
            "xmip:///",
            "xmip:///C1",
            "xmip:///C1/node",
            "xmip:///C1/round-trip/send/tcp",
            "xmip:///C1/shared/node/x",
            "xmip:///edge-01/receive/orders",
        ] {
            assert_eq!(node(none), None, "{none}");
        }
    }

    #[test]
    fn the_stage_is_beneath_the_node_or_the_cluster_and_never_a_name() {
        let stage = |scope: &str| Scope::new(scope).stage();
        assert_eq!(
            stage("xmip:///C1/node/alpha/receive/tcp"),
            Some(Stage::Receive)
        );
        assert_eq!(
            stage("xmip:///C1/round-trip/send/tcp/json"),
            Some(Stage::Send)
        );
        assert_eq!(
            stage("xmip:///C1/node/send/process/x"),
            Some(Stage::Process)
        );
        assert_eq!(stage("xmip:///C1/node/send"), None);
        assert_eq!(stage("xmip:///receive/filing/file"), None);
        assert_eq!(stage("xmip:///C1/node/alpha/Receive"), None, "exact");
        assert_eq!(stage("xmip:///"), None);
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
