//! What a run was started with, published beside its records under `[run]`.
//!
//! The owner, 2026-09-19: the run says what it was started with. A surface
//! showed a cluster and its leaves and nothing of which tests were named,
//! which nodes, what each declared, or how hard — so a board could not be
//! told from the one before it. A publisher that runs tests says it; a node
//! writes none, and a reader that finds no table shows nothing for it. The
//! System Tests held this shape until 2026-09-24 and `Xmip.Surface` read it
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
    /// writes them: `<node>=receiving+sending`, or the bare name of a node
    /// that declared no role (ADR-0056, amendment 2026-10-01).
    #[serde(default)]
    pub roles: Vec<String>,
    /// The Node capabilities each node was started with, as
    /// `node::Capability::declared_entry` writes them:
    /// `<node>=online+erp-direct`, a node declaring none left out; `online`
    /// among them where it may assume the internet (ADR-0045; ADR-0056,
    /// amendment 2026-11-24).
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// The nodes among them that are real: each read where it answered, on
    /// a virtual machine of its own or started as `xmip-service`, never
    /// simulated (ADR-0028, amendments 2026-09-19, 2026-10-21 and
    /// 2026-11-25). Every other node of `nodes` is simulated, a process the
    /// run spawned.
    #[serde(default)]
    pub real: Vec<String>,
    /// The stress level's name.
    #[serde(default)]
    pub stress: String,
    /// What the run runs as, in the words of `Start-XmipTest`'s switches:
    /// `Process`, as local processes, or `VirtualMachine`, on the virtual
    /// machines its environment was deployed on (ADR-0028, amendment
    /// 2026-11-25); empty where a publisher says neither.
    #[serde(default)]
    pub runs_as: String,
}

/// The five lists of a run, as the header numbers them for a reader that
/// asks for one at a time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunList {
    Tests,
    Nodes,
    Roles,
    Capabilities,
    Real,
}

impl RunList {
    /// Every list, in the header's order.
    pub const ALL: [RunList; 5] = [
        RunList::Tests,
        RunList::Nodes,
        RunList::Roles,
        RunList::Capabilities,
        RunList::Real,
    ];
}

impl Run {
    /// One of the run's lists.
    #[must_use]
    pub fn list(&self, which: RunList) -> &[String] {
        match which {
            RunList::Tests => &self.tests,
            RunList::Nodes => &self.nodes,
            RunList::Roles => &self.roles,
            RunList::Capabilities => &self.capabilities,
            RunList::Real => &self.real,
        }
    }
}
