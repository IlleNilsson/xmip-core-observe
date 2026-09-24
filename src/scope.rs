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

/// The scheme every scope carries, lower-case: `XMIP:///C1` is a path, not a
/// scope, as `ScopeTree` and `ScopePattern` read it (ADR-0052, amendment
/// 2026-09-14).
const SCHEME: &str = "xmip://";

/// A scope, read as the path it names in the one tree.
///
/// Borrowed from the text it was read from. Two scopes are equal when they
/// name the same path: `xmip://edge-01/n/receive` and `xmip:///n/receive/`
/// are one place.
///
/// **The rule has two writers, on purpose.** `ScopeTree` in `Xmip.Surface`
/// writes it again in C#, because a surface loads no Rust library (the owner,
/// 2026-09-24; ADR-0052, amendment of that date). Both are held to one set
/// of cases, `src/scope-vector.toml` beside this file, which a test on each
/// side reads: change a case there first, then both writers.
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

    /// Every case in `scope-vector.toml`, the file `ScopeTreeTest` in
    /// `Xmip.Surface.Test` reads too.
    #[test]
    fn every_shared_case_holds() {
        let vector: toml::Table =
            toml::from_str(include_str!("scope-vector.toml")).expect("the vector parses");
        let cases = vector["case"].as_array().expect("a [[case]] array");
        assert!(!cases.is_empty());

        for case in cases {
            let text = |key: &str| case[key].as_str().expect(key);
            let (why, scope) = (text("why"), Scope::new(text("scope")));

            assert_eq!(
                scope.contains(Scope::new(text("candidate"))),
                case["contains"].as_bool().expect("contains"),
                "{why}"
            );
            if let Some(path) = case.get("path") {
                assert_eq!(scope.path(), path.as_str().expect("path"), "{why}");
            }
            if let Some(is_root) = case.get("is_root") {
                assert_eq!(
                    scope.is_root(),
                    is_root.as_bool().expect("is_root"),
                    "{why}"
                );
            }
        }
    }

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
    fn a_slash_at_either_end_is_ignored() {
        assert!(beneath("xmip:///n/receive/a", "xmip:///n/receive/"));
        assert!(beneath("xmip:///n/receive", "xmip:///n/receive/"));
        assert!(beneath("xmip:///n/receive/", "xmip:///n/receive"));
    }

    #[test]
    fn an_empty_scope_is_above_everything() {
        assert!(beneath("xmip:///n/receive/a", ""));
        assert!(beneath("xmip:///n", "/"));
        assert!(Scope::new("").is_root());
    }

    #[test]
    fn the_authority_is_not_part_of_the_tree() {
        // ADR-0027 clause 3: an omitted host means estate-wide, and the tree
        // is the path. The text comparison this replaced said no to both.
        assert!(beneath("xmip://edge-01/n/receive/a", "xmip:///n"));
        assert!(beneath("xmip:///n/receive/a", "xmip://ops@edge-01:7400/n"));
        assert_eq!(Scope::new("xmip://edge-01/n"), Scope::new("xmip:///n/"));
        assert!(Scope::new("xmip://edge-01").is_root());
    }

    #[test]
    fn text_without_the_scheme_is_a_path() {
        assert!(beneath("xmip:///n/receive/a", "n/receive"));
        assert!(beneath("n/receive/a", "xmip:///n"));
        // The scheme is lower-case; anything else is the start of a path.
        assert_eq!(Scope::new("XMIP:///C1").path(), "XMIP:///C1");
    }
}
