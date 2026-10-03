//! Scope: an Xmip URI over the execution tree, and the one question every
//! reader asks of it. ADR-0027 clauses 3 and 4.
//!
//! A scope is `xmip://[userinfo@][host][:port]/path`, and the tree is the
//! path: an omitted host means estate-wide, so the scheme and the authority
//! go before anything is compared. A record at
//! `xmip:///<cluster>/node/<node>/transport/ftp` sits beneath
//! `xmip:///<cluster>/node/<node>/transport` and beneath
//! `xmip:///<cluster>/node/<node>`, so asking for a node gets everything the
//! node holds. That prefix rule is the whole aggregation model, and it is one
//! rule: `snapshot.rs` and `activity.rs` each carried a predicate of their
//! own until 2026-09-14, and until 2026-09-24 this crate compared the whole
//! text while `Xmip.Surface`'s `ScopeTree` compared the path (open problem 25, row k).
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

/// The scheme every scope carries, lower-case: `XMIP:///<cluster>` is a
/// path, not a scope, as `ScopeTree` and `ScopePattern` read it (ADR-0052,
/// amendment 2026-09-14).
const SCHEME: &str = "xmip://";

/// The segment that marks a node beneath its cluster:
/// `xmip:///<cluster>/node/<name>`, the location ADR-0053 gives a node. A
/// kind the shape carries, not a word a name may not use: a node may be
/// called by this word, and its scope is read by position all the same.
pub const NODE: &str = "node";

/// A scope, read as the path it names in the one tree.
///
/// Borrowed from the text it was read from. Two scopes are equal when they
/// name the same path: `xmip://<host>/<cluster>/receive` and
/// `xmip:///<cluster>/receive/` are one place.
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

    /// The segments of the path, top first: `xmip:///<cluster>/receive/orders`
    /// is `<cluster>`, `receive`, `orders`. An empty segment — `a//b` — is no
    /// segment, and the root has none. Borrowed from the text the scope was
    /// read from.
    pub fn segments(self) -> impl Iterator<Item = &'a str> {
        self.path.split('/').filter(|segment| !segment.is_empty())
    }

    /// The node this scope is on: the segment after the node marker beneath
    /// the cluster, `<node>` in `xmip:///<cluster>/node/<node>/receive/tcp`.
    /// `None` for a scope on no node — the cluster, its `node` rollup, a test
    /// run at the cluster — because the cluster is never a node. Read by
    /// position, so no name is taken for a kind (open problem 25, row q).
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
    /// `xmip:///<cluster>/node/<node>/receive/tcp` and
    /// `xmip:///<cluster>/round-trip/send/tcp/json` alike. A cluster's or a
    /// node's name is never read as a stage, so a node called by a stage's
    /// word is no stage.
    #[must_use]
    pub fn stage(self) -> Option<Stage> {
        let beneath = if self.node().is_some() { 3 } else { 1 };
        self.segments().skip(beneath).find_map(Stage::named)
    }

    /// Whether `other` is this scope or sits beneath it in the tree.
    ///
    /// A prefix of segments, not of characters: `xmip:///<cluster>x` is not
    /// beneath `xmip:///<cluster>`. Compared without splitting either path,
    /// because a surface compares thousands of records at a time.
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
    use configure::fixture::test_cluster;

    fn beneath(candidate: &str, scope: &str) -> bool {
        Scope::new(scope).contains(Scope::new(candidate))
    }

    fn parts(scope: &str) -> Vec<&str> {
        Scope::new(scope).segments().collect()
    }

    /// The test cluster's name: the first segment of every scope here.
    fn name() -> String {
        test_cluster().name
    }

    #[test]
    fn a_scope_is_beneath_itself_and_its_ancestors() {
        let n = name();
        let a = format!("xmip:///{n}/receive/a");
        assert!(beneath(&a, &a));
        assert!(beneath(&a, &format!("xmip:///{n}/receive")));
        assert!(beneath(&a, &format!("xmip:///{n}")));
        assert!(beneath(&a, "xmip:///"));
        assert!(
            !beneath(&format!("xmip:///{n}"), &format!("xmip:///{n}/receive")),
            "never above"
        );
    }

    #[test]
    fn segments_are_the_unit_not_characters() {
        let n = name();
        let (root, receive) = (format!("xmip:///{n}"), format!("xmip:///{n}/receive"));
        assert!(!beneath(&format!("xmip:///{n}x"), &root));
        assert!(!beneath(&format!("{receive}-b"), &receive));
        assert!(!beneath(&root, &receive));
    }

    #[test]
    fn a_slash_at_either_end_is_ignored() {
        let n = name();
        let (receive, slashed) = (
            format!("xmip:///{n}/receive"),
            format!("xmip:///{n}/receive/"),
        );
        assert!(beneath(&format!("{receive}/a"), &slashed));
        assert!(beneath(&receive, &slashed));
        assert!(beneath(&slashed, &receive));
        assert!(beneath(&format!("{receive}/a"), &format!("/{n}/receive/")));
        assert_eq!(
            Scope::new(&format!("/{n}/receive/")).path(),
            format!("{n}/receive")
        );
    }

    #[test]
    fn the_root_has_four_spellings_and_contains_everything() {
        let n = name();
        for root in ["", "/", "xmip://", "xmip:///"] {
            assert!(Scope::new(root).is_root(), "{root:?} is the root");
            assert_eq!(Scope::new(root).path(), "");
            assert!(beneath(&format!("xmip:///{n}/receive/a"), root));
        }
        assert!(beneath("", ""), "the root contains the root");
        assert!(
            !beneath("", &format!("xmip:///{n}")),
            "a cluster does not contain the root"
        );
    }

    #[test]
    fn the_authority_is_not_part_of_the_tree() {
        // ADR-0027 clause 3: an omitted host means estate-wide, and the tree
        // is the path. The text comparison this replaced said no to both.
        let n = name();
        let (root, a) = (format!("xmip:///{n}"), format!("xmip:///{n}/receive/a"));
        let hosted = format!("xmip://ops@edge-01:7400/{n}");
        assert!(beneath(&format!("xmip://edge-01/{n}/receive/a"), &root));
        assert!(beneath(&a, &hosted));
        assert_eq!(Scope::new(&hosted).path(), n);
        assert_eq!(
            Scope::new(&format!("xmip://edge-01/{n}")),
            Scope::new(&format!("{root}/"))
        );
        assert!(Scope::new("xmip://edge-01").is_root());
    }

    #[test]
    fn text_without_the_scheme_is_a_path() {
        let n = name();
        assert!(beneath(
            &format!("xmip:///{n}/receive/a"),
            &format!("{n}/receive")
        ));
        assert!(beneath(&format!("{n}/receive/a"), &format!("xmip:///{n}")));
        assert_eq!(
            Scope::new(&format!("{n}/receive")).path(),
            format!("{n}/receive")
        );
    }

    #[test]
    fn the_scheme_is_lower_case_and_segments_compare_case_sensitively() {
        // An upper-case scheme is the start of a path, on both sides alike.
        let cluster = test_cluster();
        let upper = cluster.scope().replacen("xmip", "XMIP", 1);
        assert_eq!(Scope::new(&upper).path(), upper);
        assert!(!beneath(&upper, &cluster.scope()));
        assert!(beneath(&format!("{upper}/node"), &upper));
        let (upper, lower) = (cluster.name.to_uppercase(), cluster.name.to_lowercase());
        assert!(!beneath(
            &format!("xmip:///{upper}/receive"),
            &format!("xmip:///{lower}")
        ));
    }

    #[test]
    fn a_query_and_a_fragment_are_part_of_the_path_today() {
        // ADR-0027 clause 3 puts a Party filter in the query, and no scope the
        // estate publishes carries one yet; this is the behavior until then.
        let n = name();
        let (root, receive) = (format!("xmip:///{n}"), format!("xmip:///{n}/receive"));
        assert!(beneath(&format!("{receive}/a?party=party-x"), &receive));
        assert_eq!(
            Scope::new(&format!("{receive}?party=party-x")).path(),
            format!("{n}/receive?party=party-x")
        );
        assert!(!beneath(
            &format!("{receive}/a"),
            &format!("{receive}?party=x")
        ));
        assert!(!beneath(&root, &format!("{root}#top")));
    }

    #[test]
    fn the_node_follows_the_marker_beneath_the_cluster_and_the_cluster_is_none() {
        let cluster = test_cluster();
        let (root, at, name) = (
            cluster.scope(),
            cluster.node_scope(0),
            &cluster.node(0).name,
        );
        let node = |scope: &str| Scope::new(scope).node().map(str::to_string);
        assert_eq!(node(&format!("{at}/receive/tcp")).as_ref(), Some(name));
        let authority = format!("xmip://edge-01/{}/node/{name}", cluster.name);
        assert_eq!(node(&authority).as_ref(), Some(name));
        // A node called by the marker's word: the kind is read by position.
        let named_node = format!("{root}/{NODE}/{NODE}/send");
        assert_eq!(node(&named_node).as_deref(), Some(NODE));
        for none in [
            "xmip:///".to_string(),
            root.clone(),
            format!("{root}/node"),
            format!("{root}/round-trip/send/tcp"),
            format!("{root}/shared/node/x"),
            format!("{root}/receive/orders"),
        ] {
            assert_eq!(node(&none), None, "{none}");
        }
    }

    #[test]
    fn the_stage_is_beneath_the_node_or_the_cluster_and_never_a_name() {
        let cluster = test_cluster();
        let (root, at) = (cluster.scope(), cluster.node_scope(0));
        let stage = |scope: &str| Scope::new(scope).stage();
        assert_eq!(stage(&format!("{at}/receive/tcp")), Some(Stage::Receive));
        assert_eq!(
            stage(&format!("{root}/round-trip/send/tcp/json")),
            Some(Stage::Send)
        );
        // A node or a cluster called by a stage's word is no stage.
        let (send, receive) = (Stage::Send.name(), Stage::Receive.name());
        assert_eq!(
            stage(&format!("{root}/{NODE}/{send}/process/x")),
            Some(Stage::Process)
        );
        assert_eq!(stage(&format!("{root}/{NODE}/{send}")), None);
        assert_eq!(stage(&format!("xmip:///{receive}/filing/file")), None);
        assert_eq!(stage(&format!("{at}/Receive")), None, "exact");
        assert_eq!(stage("xmip:///"), None);
    }

    #[test]
    fn the_segments_are_the_path_split_top_first() {
        let n = name();
        assert_eq!(
            parts(&format!("xmip:///{n}/receive/orders")),
            [n.as_str(), "receive", "orders"]
        );
        assert_eq!(
            parts(&format!("xmip://edge-01:9000/{n}/receive/")),
            [n.as_str(), "receive"]
        );
        assert_eq!(parts(&format!("{n}/receive")), [n.as_str(), "receive"]);
        assert_eq!(parts(&format!("xmip:///{n}//b")), [n.as_str(), "b"]);
        assert!(parts("xmip:///").is_empty());
        assert!(parts("").is_empty());
        assert!(parts("xmip://edge-01").is_empty());
    }
}
