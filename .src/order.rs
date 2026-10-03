//! An operator's act, and the order that carries it to a node a surface
//! reaches through its publication only (ADR-0065, amendment 2026-09-29;
//! ADR-0013, amendment 2026-09-30).
//!
//! Three things an operator acts on are published by a node: its Event
//! subscriptions, its Subscriptions and the Messages its Dead Message Queue
//! keeps ([`Noun`]). The acts and their words are written here once
//! ([`Act`]), with which acts each takes: an Event subscription is paused,
//! resumed or removed; a Subscription is paused or resumed only, since it is
//! added and removed in the TOML configuration; a Message in the Dead
//! Message Queue is replayed (ADR-0052, amendment 2026-10-01).
//!
//! A surface reading a live node applies an act through the runtime's
//! library, in the node's own process. A surface reading a snapshot touches
//! no node: the publication says where its publisher takes orders
//! ([`crate::Publication::orders`]), the surface leaves one there, and the
//! node takes it at its next look and applies it as any act is applied. The
//! file, its place and its shape are written here once:
//! `<orders>/<node name>/<unix nanos>-<sequence>-<act>.toml`, written whole
//! beside itself and renamed into place, so a node never reads half of one.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::scope::Scope;

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
}

impl Act {
    /// Every act, in the order a surface offers them.
    pub const ALL: [Self; 4] = [Self::Pause, Self::Resume, Self::Remove, Self::Replay];

    /// The word the estate names the act by.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::Remove => "remove",
            Self::Replay => "replay",
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
}

impl Noun {
    /// Every noun an order names.
    pub const ALL: [Self; 3] = [Self::EventSubscription, Self::Subscription, Self::DeadMessage];

    /// The word an order names it by.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::EventSubscription => "event-subscription",
            Self::Subscription => "subscription",
            Self::DeadMessage => "dead-message",
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
        }
    }
}

/// One act, on one thing a node publishes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Order {
    /// The node that holds it: `xmip:///<cluster>/node/<name>`.
    pub node: String,
    pub noun: Noun,
    /// Which one: an Event subscription's number, a Subscription's name.
    pub target: String,
    pub act: Act,
    /// Who acted, for the audit record.
    pub who: String,
}

#[derive(Serialize, Deserialize)]
struct Document {
    node: String,
    noun: String,
    target: String,
    act: String,
    who: String,
}

/// Orders left by this process in one nanosecond still sort apart.
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

impl Order {
    /// Leave this order in `orders` for its node to take; the file written.
    ///
    /// # Errors
    /// REFUSED when the order names no node, or an act its noun does not
    /// take; the reason when the file could not be written.
    pub fn leave(&self, orders: &Path) -> Result<PathBuf, String> {
        self.noun.act(self.act.word())?;
        let place = place(orders, &self.node)?;
        fs::create_dir_all(&place).map_err(|error| failed(&place, &error))?;
        let name = format!(
            "{}-{:06}-{}.toml",
            crate::now_unix_nanos(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed) % 1_000_000,
            self.act.word()
        );
        let document = Document {
            node: self.node.clone(),
            noun: self.noun.word().to_string(),
            target: self.target.clone(),
            act: self.act.word().to_string(),
            who: self.who.clone(),
        };
        let text = toml::to_string(&document).map_err(|error| error.to_string())?;
        let part = place.join(format!("{name}.part"));
        let file = place.join(name);
        fs::write(&part, text).map_err(|error| failed(&part, &error))?;
        fs::rename(&part, &file).map_err(|error| failed(&file, &error))?;
        Ok(file)
    }

    /// Every order left in `orders` for the node at `node`, oldest first,
    /// each file removed as it is taken. A file that is no order is removed
    /// too, and said rather than applied.
    #[must_use]
    pub fn take(orders: &Path, node: &str) -> Vec<Result<Self, String>> {
        let Ok(place) = place(orders, node) else {
            return Vec::new();
        };
        let Ok(entries) = fs::read_dir(&place) else {
            return Vec::new();
        };
        let mut files: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "toml")
            })
            .collect();
        files.sort();

        files
            .into_iter()
            .map(|file| {
                let read = fs::read_to_string(&file);
                let _ = fs::remove_file(&file);
                read.map_err(|error| error.to_string())
                    .and_then(|text| Self::read(&text))
                    .map_err(|problem| format!("{}: {problem}", file.display()))
            })
            .collect()
    }

    fn read(text: &str) -> Result<Self, String> {
        let document: Document = toml::from_str(text).map_err(|error| error.to_string())?;
        let noun = Noun::named(&document.noun)
            .ok_or_else(|| format!("REFUSED: '{}' is nothing an order names", document.noun))?;
        let act = noun.act(&document.act)?;
        Ok(Self {
            node: document.node,
            noun,
            target: document.target,
            act,
            who: document.who,
        })
    }
}

/// Where a node's orders lie: beneath `orders`, by the node's name.
fn place(orders: &Path, node: &str) -> Result<PathBuf, String> {
    Scope::new(node)
        .node()
        .filter(|name| !name.is_empty())
        .map(|name| orders.join(name))
        .ok_or_else(|| format!("REFUSED: '{node}' names no node"))
}

fn failed(path: &Path, error: &std::io::Error) -> String {
    format!("could not write {}: {error}", path.display())
}

#[cfg(test)]
mod tests {
    use super::*;
    use configure::fixture::test_cluster;

    fn orders(name: &str) -> PathBuf {
        let at = std::env::temp_dir().join(format!("xmip-order-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&at);
        at
    }

    /// The node an order is for: the test cluster's first.
    fn node() -> String {
        test_cluster().node_scope(0)
    }

    fn order(noun: Noun, target: &str, act: Act) -> Order {
        Order {
            node: node(),
            noun,
            target: target.to_string(),
            act,
            who: "ilian".to_string(),
        }
    }

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
    }

    #[test]
    fn an_order_left_is_taken_once_by_its_node_and_by_no_other() {
        let at = orders("once");
        let pause = order(Noun::Subscription, "structured", Act::Pause);
        let resume = order(Noun::EventSubscription, "3", Act::Resume);
        pause.leave(&at).expect("left");
        resume.leave(&at).expect("left");

        assert!(Order::take(&at, &test_cluster().node_scope(1)).is_empty());
        let taken: Vec<Order> = Order::take(&at, &node())
            .into_iter()
            .collect::<Result<_, _>>()
            .expect("orders");
        assert_eq!(taken, vec![pause, resume], "oldest first");
        assert!(Order::take(&at, &node()).is_empty());
        let _ = fs::remove_dir_all(&at);
    }

    #[test]
    fn no_remove_is_left_for_a_subscription_and_a_stranger_file_is_said() {
        let at = orders("stranger");
        let removed = order(Noun::Subscription, "structured", Act::Remove).leave(&at);
        assert!(removed.is_err_and(|said| said.contains("TOML configuration")));

        let cluster = test_cluster();
        let place = at.join(&cluster.node(0).name);
        fs::create_dir_all(&place).expect("made");
        fs::write(
            place.join("1-1-remove.toml"),
            format!(
                "node = \"{}\"\nnoun = \"subscription\"\ntarget = \"t\"\nact = \"remove\"\n\
                 who = \"w\"\n",
                node()
            ),
        )
        .expect("written");
        let taken = Order::take(&at, &node());
        assert_eq!(taken.len(), 1);
        assert!(
            taken[0]
                .as_ref()
                .is_err_and(|said| said.contains("REFUSED"))
        );

        let nowhere = Order {
            node: cluster.scope(),
            ..order(Noun::EventSubscription, "1", Act::Remove)
        };
        assert!(
            nowhere
                .leave(&at)
                .is_err_and(|error| error.contains("names no node"))
        );
        let _ = fs::remove_dir_all(&at);
    }
}
