//! Whether something an operator may pause is paused: an Event subscription
//! (ADR-0065, amendment 2026-09-29) or a Subscription (ADR-0013, amendment
//! 2026-09-30). The words a state is written in are here, once, for both.

/// Whether what an operator may pause is paused.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PauseState {
    /// Working as configured.
    #[default]
    Active,
    /// Held by an operator until it is resumed.
    Paused,
}

impl PauseState {
    /// Every state, active first.
    pub const ALL: [Self; 2] = [Self::Active, Self::Paused];

    /// The word the estate writes the state in.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
        }
    }

    /// The state a word names, exactly.
    #[must_use]
    pub fn named(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|state| state.word() == word)
    }

    /// The state a publication's word reads as: a word no state is called
    /// reads as paused, since held is the safer guess for something a reader
    /// does not know.
    #[must_use]
    pub fn read(word: &str) -> Self {
        Self::named(word).unwrap_or(Self::Paused)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_state_is_its_word_and_a_stranger_reads_as_held() {
        for state in PauseState::ALL {
            assert_eq!(PauseState::named(state.word()), Some(state));
            assert_eq!(PauseState::read(state.word()), state);
        }
        assert_eq!(PauseState::named("Paused"), None);
        assert_eq!(PauseState::read("sulking"), PauseState::Paused);
    }
}
