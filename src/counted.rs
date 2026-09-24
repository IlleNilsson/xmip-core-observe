//! What a count counts, what each kind is called, and which kind a stage of
//! the message path counts: ADR-0027 clause 5, once.
//!
//! The words are what a publication writes and a surface reads; they were
//! written by the Playground and parsed again by `Xmip.Surface` until
//! 2026-09-24 (open problem 25). The runtime's cdylib forwards them to a
//! surface over `xmip_operate.h` section 7 (`xmip_counted_word_v1`,
//! `xmip_stage_counted_v1`), and a publication's reader is
//! [`crate::publication`], so no surface keeps a table of its own.

use node::Stage;

/// What a count counts. Never a bare number — ADR-0027 clause 5.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Counted {
    Streams,
    Messages,
    Journeys,
    Bytes,
    /// Delivery or processing attempts awaiting another try.
    Retrying,
    /// Delivery or processing outcomes that ended unsuccessfully.
    Failed,
}

impl Counted {
    /// Every kind, in the order a publication lists them.
    pub const ALL: [Counted; 6] = [
        Counted::Streams,
        Counted::Messages,
        Counted::Journeys,
        Counted::Bytes,
        Counted::Retrying,
        Counted::Failed,
    ];

    /// The kind as the word the estate uses, lower case: what a publication
    /// writes and what a command names a figure by.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Counted::Streams => "streams",
            Counted::Messages => "messages",
            Counted::Journeys => "journeys",
            Counted::Bytes => "bytes",
            Counted::Retrying => "retrying",
            Counted::Failed => "failed",
        }
    }

    /// The kind a word names, exactly and in lower case, or `None` when no
    /// kind is called that.
    #[must_use]
    pub fn named(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|counted| counted.word() == word)
    }

    /// What a stage of the message path counts, the three words kept apart:
    /// Streams at Receive, Journeys in Process, Messages at Send.
    #[must_use]
    pub const fn at(stage: Stage) -> Self {
        match stage {
            Stage::Receive => Counted::Streams,
            Stage::Process => Counted::Journeys,
            Stage::Send => Counted::Messages,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_one_word_and_reads_back_from_it() {
        assert_eq!(
            Counted::ALL.map(Counted::word),
            [
                "streams", "messages", "journeys", "bytes", "retrying", "failed"
            ]
        );
        for counted in Counted::ALL {
            assert_eq!(Counted::named(counted.word()), Some(counted));
        }
        assert_eq!(Counted::named("Streams"), None);
        assert_eq!(Counted::named("throughput"), None);
    }

    #[test]
    fn each_stage_counts_its_own_kind() {
        assert_eq!(
            Stage::ALL.map(Counted::at),
            [Counted::Streams, Counted::Journeys, Counted::Messages]
        );
    }
}
