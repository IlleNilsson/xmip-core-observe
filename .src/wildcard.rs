//! The one wildcard over scopes (ADR-0052, amendment 2026-09-19; ADR-0059
//! clauses 7 and 8): a thing that selects among scopes that already exist
//! takes a wildcard, and the wildcard is `*` and `?` — never a regular
//! expression, so `Rust.Style` is the name the operator typed and not a
//! pattern that also catches `RustXStyle`.
//!
//! Written once, here. Until 2026-09-29 it was `ScopePattern.Like` in
//! `Xmip.Surface`, and the audit read needed it in Rust; the runtime's
//! library forwards it as `xmip_scope_matches_v1` (`xmip_operate.h`
//! section 7) and `ScopePattern.Matches` calls that. Where it cannot be
//! PowerShell's `-like` exactly, it says so:
//!
//! - `*` and `?` are the only metacharacters; `[`, `]` and a backtick are
//!   literal. No scope the estate publishes carries one.
//! - `*` crosses a `/`, as it does in `-like`, so
//!   `xmip:///<cluster>/*/receive` reaches a stage however deep the node
//!   sits.
//! - Both sides are read as scopes first ([`Scope`]): the scheme and the
//!   authority go and empty segments drop, so `<cluster>/node/<prefix>*`
//!   and `xmip:///<cluster>/node/<prefix>*` are one pattern.
//! - Case-insensitive, which is what `-like` is by default. The scheme is
//!   read the one way [`Scope`] reads it, lower-case, so `XMIP:///<cluster>`
//!   is a path and not a scope.

use crate::scope::Scope;

/// Whether `pattern` asks for a group rather than naming one scope. A
/// pattern with no wildcard is an exact scope.
#[must_use]
pub fn has_wildcard(pattern: &str) -> bool {
    pattern.contains(['*', '?'])
}

/// Whether `candidate` is what `pattern` names. A pattern with no wildcard
/// matches that one scope exactly — never a substring of it and never what
/// is beneath it; being beneath is [`Scope::contains`], a different question.
#[must_use]
pub fn matches(candidate: &str, pattern: &str) -> bool {
    let text: Vec<char> = normal(candidate).chars().collect();
    let pattern: Vec<char> = normal(pattern).chars().collect();

    like(&text, &pattern)
}

/// A scope as the comparison reads it: its segments joined by `/`, without
/// the scheme, the authority or an empty segment.
fn normal(scope: &str) -> String {
    Scope::new(scope).segments().collect::<Vec<_>>().join("/")
}

/// `-like` itself: `*` for any run of characters including none, `?` for
/// exactly one, everything else literal, the whole text or nothing.
/// Iterative with one backtrack point, so a pattern of several stars over
/// eleven thousand scopes cannot fall off a stack.
fn like(text: &[char], pattern: &[char]) -> bool {
    let (mut at, mut step) = (0, 0);
    let mut star: Option<usize> = None;
    let mut resume = 0;

    while at < text.len() {
        match pattern.get(step) {
            Some('?') => {
                at += 1;
                step += 1;
            }
            Some('*') => {
                star = Some(step);
                step += 1;
                resume = at;
            }
            Some(&wanted) if same(wanted, text[at]) => {
                at += 1;
                step += 1;
            }
            _ => match star {
                Some(from) => {
                    step = from + 1;
                    resume += 1;
                    at = resume;
                }
                None => return false,
            },
        }
    }

    pattern[step.min(pattern.len())..]
        .iter()
        .all(|rest| *rest == '*')
}

/// One character against another, without regard to case.
fn same(wanted: char, seen: char) -> bool {
    wanted == seen || wanted.to_uppercase().eq(seen.to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use configure::fixture::test_cluster;

    /// The node's name without its last character: what a `?` stands in
    /// for.
    fn stem(name: &str) -> String {
        let mut stem = name.to_string();
        stem.pop();
        stem
    }

    #[test]
    fn a_pattern_with_no_wildcard_is_the_scope_itself() {
        let cluster = test_cluster();
        let (root, node) = (cluster.scope(), cluster.node_scope(0));
        assert!(!has_wildcard(&node));
        assert!(matches(&node, &node));
        assert!(!matches(&node, "node"));
        assert!(!matches(&node, &format!("{root}/node")));
        assert!(!matches(&format!("{node}bet"), &node));
    }

    #[test]
    fn a_dot_is_a_dot_and_not_any_character() {
        let root = test_cluster().scope();
        assert!(matches(&format!("{root}/test/Rust.Style"), "*/Rust.*"));
        assert!(!matches(&format!("{root}/test/RustXStyle"), "*/Rust.*"));
    }

    #[test]
    fn a_star_is_any_run_and_a_question_mark_exactly_one() {
        let cluster = test_cluster();
        let (root, node) = (cluster.scope(), cluster.node_scope(0));
        let name = &cluster.node(0).name;
        let first: String = name.chars().take(1).collect();
        let prefix = format!("{root}/node/{first}*");
        let one = format!("{root}/node/{}?", stem(name));
        assert!(has_wildcard(&prefix));
        assert!(matches(&node, &prefix));
        assert!(matches(&node, &one));
        assert!(!matches(&format!("{node}s"), &one));
        assert!(matches(&node, "*"));
        assert!(matches("", "*"), "the root is matched by a star alone");
        assert!(!matches("", &cluster.name));
        assert!(matches(&root, &format!("{root}*")));
    }

    #[test]
    fn a_star_crosses_a_slash_and_backtracks() {
        let cluster = test_cluster();
        let (root, node) = (cluster.scope(), cluster.node_scope(0));
        let pattern = format!("{root}/*/receive");
        assert!(matches(&format!("{node}/receive"), &pattern));
        assert!(!matches(&format!("{node}/receive/tcp"), &pattern));
        assert!(matches(&format!("{root}/aXbXc"), "*X*X*"));
        assert!(!matches(&format!("{root}/aXb"), "*X*X*"));
    }

    #[test]
    fn case_is_ignored_and_the_scheme_is_lower_case() {
        let cluster = test_cluster();
        let (name, node) = (&cluster.name, &cluster.node(0).name);
        let first: String = node.chars().take(1).collect();
        assert!(matches(
            &format!(
                "xmip:///{}/node/å{}",
                name.to_uppercase(),
                node.to_uppercase()
            ),
            &format!("{}/NODE/Å{}*", name.to_lowercase(), first.to_lowercase())
        ));
        let node = cluster.node_scope(0);
        assert!(!matches(&node, &node.replacen("xmip", "XMIP", 1)));
    }

    #[test]
    fn a_set_and_a_backtick_are_literal() {
        let cluster = test_cluster();
        let (name, root) = (&cluster.name, cluster.scope());
        assert!(matches(&format!("{root}/[a]"), &format!("{name}/[a]")));
        assert!(!matches(&format!("{root}/a"), &format!("{name}/[a]")));
    }
}
