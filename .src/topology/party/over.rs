//! What a Party's link runs over, said in words: never a Location's name.

use std::collections::BTreeSet;

use super::{Drawing, Facing};

/// What a link runs over, said by `drawing`: never a Location's name.
/// Where the endpoints are transports — the publisher said which the Party
/// is reached over, a simulated node's — the one, or how many. Where they
/// are a running node's Locations, the transport each one's address chose
/// (`Drawing::transports`): the one, or how many, the Locations themselves
/// named in the link's evidence (ADR-0018, amendment 2026-11-11).
impl Drawing<'_> {
    pub(super) fn over_what(&self, facing: &Facing) -> String {
        let words: BTreeSet<&str> = if self.over.is_some() {
            facing.endpoints.iter().map(String::as_str).collect()
        } else {
            facing
                .endpoints
                .iter()
                .flat_map(|location| {
                    self.transports
                        .iter()
                        .filter(move |(named, _)| named == location)
                        .map(|(_, transport)| transport.as_str())
                })
                .collect()
        };
        match words.len() {
            0 => "no transport reported".to_string(),
            1 => words.into_iter().collect(),
            many => format!("{many} transports"),
        }
    }

    /// The Locations a link runs over, where the endpoints are a running
    /// node's Locations: what its evidence names, never its protocol.
    pub(super) fn at_what(&self, facing: &Facing) -> String {
        if self.over.is_some() || facing.endpoints.is_empty() {
            return String::new();
        }
        format!(", at {}", facing.endpoints.join(", "))
    }
}
