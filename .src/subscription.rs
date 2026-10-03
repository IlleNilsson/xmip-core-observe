//! A Subscription as a node publishes it: what it subscribes to, where it
//! leads, the file it is configured in, whether an operator has paused it,
//! and what it has picked up and holds (ADR-0013, amendment 2026-09-30).
//!
//! A Subscription is configuration: drawn in an Xmip Application, bound by
//! a node's TOML, added and removed there and nowhere else (ADR-0064). The
//! runtime keeps its standing — paused or active, and the Messages it holds
//! while paused — in the node's runtime store; what a surface lists is this
//! record, published with the node's health and counts, so a surface reading
//! a snapshot lists the Subscriptions of every node in the cluster without
//! touching one. It is not an [`crate::EventSubscription`], which tells a
//! Party what Xmip did.

use serde::{Deserialize, Serialize};

use crate::pause_state::PauseState;

/// One Subscription on one node, as published.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Subscription {
    /// The node that routes by it: `xmip:///<cluster>/node/<name>`.
    pub node: String,
    /// Its configured name, unique on its node; with [`Self::node`] it names
    /// one Subscription.
    pub name: String,
    /// The Xmip Application it is drawn in.
    pub application: String,
    /// What it subscribes to: its filter, as configured.
    pub filter: String,
    /// Where it leads, in words: the Send Port, Send Port Group or Xmip
    /// Process a Message it picks up goes to.
    pub destination: String,
    /// The file it is configured in.
    pub file: String,
    /// Its entry in that file, as the file says it.
    pub configuration: String,
    /// Paused: what it matches is held, not picked up, until it is resumed.
    pub state: PauseState,
    /// Who paused it; empty while it is active.
    pub by: String,
    /// Messages it picked up since the node started.
    pub picked_up: u64,
    /// Messages it holds, matched while it was paused and not yet picked up.
    pub held: u64,
    /// When its state began, in unix nanoseconds: when it was paused or
    /// resumed, or when the node took it up.
    pub since_unix_nanos: i64,
}

/// A Subscription as a publication writes it.
#[derive(Serialize, Deserialize)]
pub(crate) struct SubscriptionDocument {
    #[serde(default)]
    node: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    application: String,
    #[serde(default)]
    filter: String,
    #[serde(default)]
    destination: String,
    #[serde(default)]
    file: String,
    #[serde(default)]
    configuration: String,
    #[serde(default)]
    state: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    by: String,
    #[serde(default)]
    picked_up: u64,
    #[serde(default)]
    held: u64,
    #[serde(default)]
    since_unix_nanos: i64,
}

impl SubscriptionDocument {
    pub(crate) fn of(subscription: &Subscription) -> Self {
        Self {
            node: subscription.node.clone(),
            name: subscription.name.clone(),
            application: subscription.application.clone(),
            filter: subscription.filter.clone(),
            destination: subscription.destination.clone(),
            file: subscription.file.clone(),
            configuration: subscription.configuration.clone(),
            state: subscription.state.word().to_string(),
            by: subscription.by.clone(),
            picked_up: subscription.picked_up,
            held: subscription.held,
            since_unix_nanos: subscription.since_unix_nanos,
        }
    }

    pub(crate) fn into_subscription(self) -> Subscription {
        Subscription {
            node: self.node,
            name: self.name,
            application: self.application,
            filter: self.filter,
            destination: self.destination,
            file: self.file,
            configuration: self.configuration,
            state: PauseState::read(&self.state),
            by: self.by,
            picked_up: self.picked_up,
            held: self.held,
            since_unix_nanos: self.since_unix_nanos,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use configure::fixture::test_cluster;

    #[test]
    fn a_subscription_reads_back_as_written() {
        let written = Subscription {
            node: test_cluster().node_scope(0),
            name: "structured".to_string(),
            state: PauseState::Paused,
            by: "ilian".to_string(),
            held: 3,
            ..Subscription::default()
        };
        let read = SubscriptionDocument::of(&written).into_subscription();
        assert_eq!(read, written);
    }
}
