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
    /// What each node was started with, as `node::Capability::entry` writes
    /// it: `R1=receive+send`, or the bare name of a node that declared no
    /// stage (ADR-0056).
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// The nodes among them that may assume the internet (ADR-0045).
    #[serde(default)]
    pub online: Vec<String>,
    /// The stress level's name.
    #[serde(default)]
    pub stress: String,
}

/// The four lists of a run, as the header numbers them for a reader that
/// asks for one at a time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunList {
    Tests,
    Nodes,
    Capabilities,
    Online,
}

impl RunList {
    /// Every list, in the header's order.
    pub const ALL: [RunList; 4] = [
        RunList::Tests,
        RunList::Nodes,
        RunList::Capabilities,
        RunList::Online,
    ];
}

impl Run {
    /// One of the run's lists.
    #[must_use]
    pub fn list(&self, which: RunList) -> &[String] {
        match which {
            RunList::Tests => &self.tests,
            RunList::Nodes => &self.nodes,
            RunList::Capabilities => &self.capabilities,
            RunList::Online => &self.online,
        }
    }
}
