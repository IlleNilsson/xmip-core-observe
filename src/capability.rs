//! Where a node publishes what it declares it can do: one health record at
//! `<node>/capability`, its evidence the declaration (ADR-0056 clause 1).
//!
//! The declaration and its evidence are `node::Capability`'s; where it sits
//! in the snapshot is the snapshot's, and is said here once. A publisher
//! writes the record at [`scope`], and every reader — the Playground drawing
//! a node's stages, a surface listing what each node declared — reads it by
//! [`declared`], which the runtime's cdylib forwards as
//! `xmip_capability_published_v1` (`xmip_operate.h` section 7).

use node::Capability;

use crate::scope::Scope;

/// The leaf a node's capability record sits at, beneath the node.
const LEAF: &str = "capability";

/// The scope a node at `node_scope` publishes its capability at.
#[must_use]
pub fn scope(node_scope: &str) -> String {
    format!("{}/{LEAF}", node_scope.trim_end_matches('/'))
}

/// What the record at `record_scope` declares, when it is a capability
/// record: the name of the node it sits beneath and the capability its
/// evidence says. `None` for any other record.
///
/// # Errors
///
/// The inner result is the REFUSED sentence when the evidence names a word
/// that is no stage (ADR-0055); the record is still a capability record.
#[must_use]
pub fn declared<'a>(
    record_scope: &'a str,
    evidence: &str,
) -> Option<(&'a str, Result<Capability, String>)> {
    let parts: Vec<&str> = Scope::new(record_scope).segments().collect();
    match parts.as_slice() {
        [.., node, leaf] if *leaf == LEAF => Some((*node, Capability::from_evidence(evidence))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use node::Stage;

    #[test]
    fn a_node_publishes_beneath_itself_and_reads_back_by_the_same_place() {
        let at = scope("xmip:///C1/node/alpha");
        assert_eq!(at, "xmip:///C1/node/alpha/capability");

        let evidence = Capability::of(&[Stage::Receive]).evidence();
        let (node, said) = declared(&at, &evidence).expect("a capability record");
        assert_eq!(node, "alpha");
        assert_eq!(said, Ok(Capability::of(&[Stage::Receive])));
    }

    #[test]
    fn any_other_record_is_no_declaration() {
        assert!(declared("xmip:///C1/node/alpha/receive/tcp", "declares send").is_none());
        assert!(declared("xmip:///capability", "declares send").is_none());
        assert!(declared("", "").is_none());
    }

    #[test]
    fn a_refused_declaration_is_still_the_nodes() {
        let (node, said) =
            declared("xmip:///n/edge-01/capability", "declares relay; offline;").expect("one");
        assert_eq!(node, "edge-01");
        assert!(said.expect_err("refused").starts_with("REFUSED"));
    }
}
