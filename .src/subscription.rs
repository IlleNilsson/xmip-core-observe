//! An Event subscription as a node publishes it: who subscribed, where it
//! is held, what it asks for, whether its delivery is held, and what its
//! queue has done (ADR-0065, amendment 2026-09-29).
//!
//! The subscription itself lives in a node's hub (`xmip-core-event`), and
//! none is persisted; what a surface lists is this record, published with
//! the node's health and counts, so a surface reading a snapshot lists the
//! subscriptions of every node in the cluster without touching one. The
//! words a state is written in are here, once.

use serde::{Deserialize, Serialize};

/// Whether a subscription's Events are handed over.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SubscriptionState {
    /// Delivered as they arrive.
    #[default]
    Active,
    /// Held by an operator: the queue keeps filling up to its capacity and
    /// nothing is handed over until it is resumed.
    Paused,
}

impl SubscriptionState {
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
}

/// One subscription, as published.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Subscription {
    /// The node whose hub holds it: `xmip:///<cluster>/node/<name>`.
    pub node: String,
    /// Its number in that hub; with [`Self::node`] it names one subscription.
    pub id: u64,
    /// The Party subscribed, by the name it was declared with — what an
    /// operator reads; empty where it was declared with none.
    pub subscriber: String,
    /// The Party subscribed, by its identifier.
    pub party: String,
    /// What it subscribes to, in words: the Event types and outcomes its
    /// filter asks for.
    pub action: String,
    /// The scope its filter reaches: the Events of what happened there or
    /// beneath.
    pub scope: String,
    pub state: SubscriptionState,
    /// Events waiting in its queue.
    pub queued: u64,
    /// How many its queue holds.
    pub capacity: u64,
    /// Events handed over since it was made.
    pub delivered: u64,
    /// Matching Events a full queue refused since it was made.
    pub missed: u64,
    /// When it was made, in unix nanoseconds.
    pub since_unix_nanos: i64,
}

/// A subscription as a publication writes it.
#[derive(Serialize, Deserialize)]
pub(crate) struct SubscriptionDocument {
    #[serde(default)]
    node: String,
    #[serde(default)]
    id: u64,
    #[serde(default)]
    subscriber: String,
    #[serde(default)]
    party: String,
    #[serde(default)]
    action: String,
    #[serde(default)]
    scope: String,
    /// A state no one is called reads as paused: held is the safer guess
    /// for something a reader does not know.
    #[serde(default)]
    state: String,
    #[serde(default)]
    queued: u64,
    #[serde(default)]
    capacity: u64,
    #[serde(default)]
    delivered: u64,
    #[serde(default)]
    missed: u64,
    #[serde(default)]
    since_unix_nanos: i64,
}

impl SubscriptionDocument {
    pub(crate) fn of(subscription: &Subscription) -> Self {
        Self {
            node: subscription.node.clone(),
            id: subscription.id,
            subscriber: subscription.subscriber.clone(),
            party: subscription.party.clone(),
            action: subscription.action.clone(),
            scope: subscription.scope.clone(),
            state: subscription.state.word().to_string(),
            queued: subscription.queued,
            capacity: subscription.capacity,
            delivered: subscription.delivered,
            missed: subscription.missed,
            since_unix_nanos: subscription.since_unix_nanos,
        }
    }

    pub(crate) fn into_subscription(self) -> Subscription {
        Subscription {
            node: self.node,
            id: self.id,
            subscriber: self.subscriber,
            party: self.party,
            action: self.action,
            scope: self.scope,
            state: SubscriptionState::named(&self.state).unwrap_or(SubscriptionState::Paused),
            queued: self.queued,
            capacity: self.capacity,
            delivered: self.delivered,
            missed: self.missed,
            since_unix_nanos: self.since_unix_nanos,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_state_is_its_word_and_a_stranger_reads_as_held() {
        for state in SubscriptionState::ALL {
            assert_eq!(SubscriptionState::named(state.word()), Some(state));
        }
        assert_eq!(SubscriptionState::named("Paused"), None);

        let document = SubscriptionDocument {
            state: "sulking".to_string(),
            ..SubscriptionDocument::of(&Subscription::default())
        };
        assert_eq!(
            document.into_subscription().state,
            SubscriptionState::Paused
        );
    }
}
