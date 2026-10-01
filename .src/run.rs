//! What a run was started with, published beside its records under `[run]`.
//!
//! The owner, 2026-09-19: the run says what it was started with. A surface
//! showed a cluster and its leaves and nothing of which tests were named,
//! which nodes, which of them online, or how hard — so a board could not be
//! told from the one before it. A publisher that runs tests says it; a node
//! writes none, and a reader that finds no table shows nothing for it. The
//! Playground held this shape until 2026-09-24 and `Xmip.Surface` read it
//! again (open problem 25); the surfaces now read it through the runtime's
//! publication reader (`xmip_operate.h` section 8).

use serde::{Deserialize, Serialize};

/// The choices behind a run, in the words `Start-XmipTest` takes them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Run {
    /// The cluster's name.
    #[serde(default)]
    pub cluster: String,
    /// The tests that run, by the names a person asks for them.
    #[serde(default)]
    pub tests: Vec<String>,
    /// The nodes spawned, one process each; empty when there are none.
    #[serde(default)]
    pub nodes: Vec<String>,
    /// The roles each node was started with, as `node::Capability::entry`
    /// writes them: `alpha=receiving+sending`, or the bare name of a node
    /// that declared no role (ADR-0056, amendment 2026-10-01).
    #[serde(default)]
    pub roles: Vec<String>,
    /// The nodes among them that may assume the internet (ADR-0045).
    #[serde(default)]
    pub online: Vec<String>,
    /// The stress level's name.
    #[serde(default)]
    pub stress: String,
    /// The run declared itself hidden when it was started: an assistant's
    /// test run, which an operator's view leaves out until asked to show it
    /// (the owner, 2026-09-29; ADR-0028, amendment 2026-09-30). Written only
    /// where true, so a run that declared nothing reads as shown.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,
}

/// Whether something a hidden run made is shown: always where it declared
/// nothing, and where it declared itself hidden only when the reader asked
/// to include what is hidden. The one rule every surface applies to a
/// cluster, a run and an audit record alike (ADR-0028 and ADR-0052,
/// amendments 2026-09-30); the runtime forwards it as `xmip_run_shown_v1`.
/// Nothing is read out of a name: a cluster called anything is shown unless
/// its run declared otherwise.
#[must_use]
pub const fn shown(hidden: bool, including_hidden: bool) -> bool {
    !hidden || including_hidden
}

/// The four lists of a run, as the header numbers them for a reader that
/// asks for one at a time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunList {
    Tests,
    Nodes,
    Roles,
    Online,
}

impl RunList {
    /// Every list, in the header's order.
    pub const ALL: [RunList; 4] = [
        RunList::Tests,
        RunList::Nodes,
        RunList::Roles,
        RunList::Online,
    ];
}

impl Run {
    /// Whether this run is shown to a reader that does, or does not, ask to
    /// include what is hidden ([`shown`]).
    #[must_use]
    pub const fn shown(&self, including_hidden: bool) -> bool {
        shown(self.hidden, including_hidden)
    }

    /// One of the run's lists.
    #[must_use]
    pub fn list(&self, which: RunList) -> &[String] {
        match which {
            RunList::Tests => &self.tests,
            RunList::Nodes => &self.nodes,
            RunList::Roles => &self.roles,
            RunList::Online => &self.online,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Publication, Snapshot};

    #[test]
    fn a_hidden_run_is_shown_only_when_asked_and_an_undeclared_one_always() {
        assert!(shown(false, false) && shown(false, true));
        assert!(!shown(true, false) && shown(true, true));

        // Nothing is read out of a name: CT that declared nothing is shown.
        let named = Run {
            cluster: "CT".to_string(),
            ..Run::default()
        };
        assert!(named.shown(false));
        let hidden = Run {
            hidden: true,
            ..named
        };
        assert!(!hidden.shown(false) && hidden.shown(true));
    }

    #[test]
    fn a_hidden_run_says_so_under_its_run_and_a_shown_one_says_nothing() {
        let publish = |run: &Run| {
            Publication::whole("roll", "xmip:///CT", &Snapshot::new())
                .with_run(Some(run.clone()))
                .to_toml()
        };
        let hidden = Run {
            cluster: "CT".to_string(),
            hidden: true,
            ..Run::default()
        };
        let text = publish(&hidden);
        assert!(text.contains("hidden = true"), "{text}");
        assert_eq!(Publication::read(&text).expect("reads").run, Some(hidden));

        let plain = publish(&Run::default());
        assert!(!plain.contains("hidden"), "{plain}");
    }
}
