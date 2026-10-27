//! Advice on a Receive Location that goes one Stream at a time: how many
//! wait behind it, and that a Correlation would spread them (ADR-0024
//! and ADR-0066, amendments 2026-10-22).
//!
//! The owner, 2026-10-22: *If I want to open up for parallelism I can use
//! context values … I can widen and strangle parallelism at any
//! configurable point.* A Location that names no set carries one Stream at
//! a time; when Streams wait behind it, the operation views say so and say
//! what would spread them, never what to set.

use super::Advice;

/// How many Streams may wait behind a Location's own sequence before it is
/// advised to be spread: more than one waiting means work is held back.
pub const WAITING: u64 = 1;

/// The advice for the Receive Location at `scope`, named `location`, whose
/// own sequence holds `held` places — the one at work and those waiting
/// behind it; none where no more than [`WAITING`] wait.
#[must_use]
pub fn sequence_backlog(scope: &str, location: &str, held: u64) -> Option<Advice> {
    let waiting = held.saturating_sub(1);
    if waiting <= WAITING {
        return None;
    }
    Some(Advice {
        scope: scope.to_string(),
        said: format!(
            "{location} carries 1 Stream at a time; {waiting} wait. A Correlation keyed \
             by a context value its Messages carry would spread them."
        ),
        evidence: vec![
            ("handled at once".to_string(), "1".to_string()),
            ("waiting".to_string(), waiting.to_string()),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_location_with_streams_waiting_is_advised_to_be_spread() {
        let scope = format!(
            "{}/receive/In",
            configure::fixture::test_cluster().node_scope(0)
        );
        assert_eq!(sequence_backlog(&scope, "In", 2), None);
        let advice = sequence_backlog(&scope, "In", 341).expect("advised");
        assert!(advice.said.contains("340 wait"), "{}", advice.said);
        assert_eq!(advice.evidence[1], ("waiting".to_string(), "340".to_string()));
    }
}
