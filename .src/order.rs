//! An operator's act on what a node publishes (ADR-0065, amendment
//! 2026-09-29; ADR-0013, amendment 2026-09-30), written here once: the
//! acts and their words ([`Act`]), what they act on ([`Noun`]) and which acts
//! each takes, and the one act asked of a node ([`Order`]).
//!
//! Four things an operator acts on are published by a node: its Event
//! subscriptions, its Subscriptions, the Messages its Dead Message Queue
//! keeps and the Messages it sends ([`Noun`]). An Event subscription is
//! paused, resumed or removed; a Subscription is paused or resumed only,
//! since it is added and removed in the TOML configuration of the Xmip
//! Application that draws it; a Message in the Dead Message Queue is
//! replayed (ADR-0052, amendment 2026-10-01); a Message that failed is
//! retried or dismissed (`runtime-model.md` section 13).
//!
//! Every act reaches its node through Xmip Storage: the node a surface
//! reaches admits it and writes it there, and the node it is asked of takes
//! it (ADR-0014, amendment 2026-11-13). A snapshot is read and never acted
//! on.

/// What an operator does to what a node publishes.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Act {
    /// Hold it: an Event subscription keeps queuing and hands nothing over,
    /// a Subscription holds what it matches and picks nothing up.
    Pause,
    /// Let go: what was held is handed over or picked up, oldest first.
    Resume,
    /// Unsubscribe an Event subscription. No Subscription takes it.
    Remove,
    /// Route a Message of the Dead Message Queue again, against the
    /// Subscriptions of now.
    Replay,
    /// Send a Message that failed again, from its Send Port's queue.
    Retry,
    /// End a Message that failed, by a decision: Dismissed, its history
    /// kept.
    Dismiss,
}

impl Act {
    /// Every act, in the order a surface offers them.
    pub const ALL: [Self; 6] = [
        Self::Pause,
        Self::Resume,
        Self::Remove,
        Self::Replay,
        Self::Retry,
        Self::Dismiss,
    ];

    /// The word the estate names the act by.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::Remove => "remove",
            Self::Replay => "replay",
            Self::Retry => "retry",
            Self::Dismiss => "dismiss",
        }
    }

    /// The act a word names, exactly.
    #[must_use]
    pub fn named(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|act| act.word() == word)
    }
}

/// What an operator acts on.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Noun {
    /// An Event subscription, by its number in its node's hub.
    EventSubscription,
    /// A Subscription, by its configured name on its node.
    Subscription,
    /// A Message in its node's Dead Message Queue, by its identifier.
    DeadMessage,
    /// A Message a node sends, by its Journey's identifier.
    Message,
}

impl Noun {
    /// Every noun an act names.
    pub const ALL: [Self; 4] = [
        Self::EventSubscription,
        Self::Subscription,
        Self::DeadMessage,
        Self::Message,
    ];

    /// The word an act names it by.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::EventSubscription => "event-subscription",
            Self::Subscription => "subscription",
            Self::DeadMessage => "dead-message",
            Self::Message => "message",
        }
    }

    /// The noun a word names, exactly.
    #[must_use]
    pub fn named(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|noun| noun.word() == word)
    }

    /// The acts it takes, in the order a surface offers them.
    #[must_use]
    pub const fn acts(self) -> &'static [Act] {
        match self {
            Self::EventSubscription => &[Act::Pause, Act::Resume, Act::Remove],
            Self::Subscription => &[Act::Pause, Act::Resume],
            Self::DeadMessage => &[Act::Replay],
            Self::Message => &[Act::Retry, Act::Dismiss],
        }
    }

    /// The act `word` names, if it is one this noun takes; the refusal in
    /// words otherwise.
    ///
    /// # Errors
    /// REFUSED, naming the acts there are, for a word that is no act this
    /// noun takes. A Subscription's refusal of remove says where it is
    /// removed instead.
    pub fn act(self, word: &str) -> Result<Act, String> {
        let words: Vec<&str> = self.acts().iter().map(|act| act.word()).collect();
        match Act::named(word) {
            Some(act) if self.acts().contains(&act) => Ok(act),
            Some(Act::Remove) if self == Self::Subscription => Err(
                "REFUSED: a Subscription is not removed by an act; it is added and removed \
                 in the TOML configuration of the Xmip Application that draws it"
                    .to_string(),
            ),
            _ => Err(format!(
                "REFUSED: '{word}' is no act on {}; the acts are {}",
                self.called(),
                words.join(", ")
            )),
        }
    }

    /// The action an audit record of `act` on it carries.
    #[must_use]
    pub fn action(self, act: Act) -> String {
        let noun = match self {
            Self::EventSubscription => "event",
            Self::Subscription => "subscription",
            Self::DeadMessage => "dead-message",
            Self::Message => "message",
        };
        format!("{noun}.{}", act.word())
    }

    /// What it is called in a sentence.
    #[must_use]
    pub const fn called(self) -> &'static str {
        match self {
            Self::EventSubscription => "an Event subscription",
            Self::Subscription => "a Subscription",
            Self::DeadMessage => "a Message in the Dead Message Queue",
            Self::Message => "a Message that failed",
        }
    }
}

/// One act, on one thing a node publishes, asked of that node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Order {
    /// The node that holds it: `xmip:///<cluster>/node/<name>`.
    pub node: String,
    pub noun: Noun,
    /// Which one: an Event subscription's number, a Subscription's name, a
    /// Message's identifier in the Dead Message Queue, or a failed Message's
    /// Journey's identifier.
    pub target: String,
    pub act: Act,
    /// Who acted, for the audit record.
    pub who: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_act_is_its_word_and_each_noun_takes_its_own() {
        for act in Act::ALL {
            assert_eq!(Act::named(act.word()), Some(act));
        }
        assert_eq!(Act::named("Pause"), None);
        assert_eq!(Noun::EventSubscription.act("remove"), Ok(Act::Remove));
        assert_eq!(Noun::Subscription.act("pause"), Ok(Act::Pause));
        let removed = Noun::Subscription.act("remove").expect_err("refused");
        assert!(removed.contains("TOML configuration"), "{removed}");
        let sulk = Noun::Subscription.act("sulk").expect_err("refused");
        assert!(sulk.contains("the acts are pause, resume"), "{sulk}");
        assert_eq!(Noun::Subscription.action(Act::Pause), "subscription.pause");
        assert_eq!(Noun::EventSubscription.action(Act::Remove), "event.remove");
        assert_eq!(Noun::DeadMessage.act("replay"), Ok(Act::Replay));
        assert_eq!(Noun::DeadMessage.action(Act::Replay), "dead-message.replay");
        let paused = Noun::DeadMessage.act("pause").expect_err("refused");
        assert!(paused.contains("the acts are replay"), "{paused}");
        assert!(Noun::Subscription.act("replay").is_err());
        assert!(Noun::EventSubscription.act("replay").is_err());
        assert_eq!(Noun::Message.act("retry"), Ok(Act::Retry));
        assert_eq!(Noun::Message.act("dismiss"), Ok(Act::Dismiss));
        assert_eq!(Noun::Message.action(Act::Dismiss), "message.dismiss");
        let replayed = Noun::Message.act("replay").expect_err("refused");
        assert!(
            replayed.contains("the acts are retry, dismiss"),
            "{replayed}"
        );
        assert!(Noun::DeadMessage.act("retry").is_err());
        assert_eq!(Noun::named("message"), Some(Noun::Message));
    }
}
