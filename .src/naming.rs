//! How a publication's cluster is named where a person reads it, published
//! under `[cluster]`: the short name the cluster's file declares, which
//! every operator surface shows for the cluster, and the file it comes from,
//! where the publisher knows it (ADR-0028, amendment 2026-11-28). The name
//! the run gives the cluster is the scope's, never repeated here.
//!
//! A node knows its short name from its slice and not the file it was
//! sliced from, so it publishes the short name alone; the System Tests'
//! roll, which reads the Reference Environment, publishes both.

use serde::{Deserialize, Serialize};

/// The cluster's short name and its file, each empty where the publisher
/// does not know it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Naming {
    /// `[service] short_name` of the cluster's file (`configure::ShortName`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub short_name: String,
    /// The cluster's file, as the publisher names it.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub file: String,
}

impl Naming {
    /// Whether the publisher said anything of how its cluster is named.
    #[must_use]
    pub fn said(&self) -> bool {
        !self.short_name.is_empty() || !self.file.is_empty()
    }
}
