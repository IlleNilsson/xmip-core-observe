//! The recent items beneath a scope, as the file a surface reads (ADR-0032),
//! written and read here and nowhere else.
//!
//! [`Activity`] keeps the items in memory; a publisher writes the most recent
//! beneath its scope to a file — `node` and `[[items]]` of `kind`, `scope`,
//! `id`, `bytes`, `detail` and `observed_unix_nanos`. The Playground wrote
//! that shape with its own copy of the item kinds' words until 2026-09-24
//! (open problem 25); the words are [`ItemKind::name`]'s and the shape is
//! this file's.
//!
//! An item of a kind no one is called is skipped; a file that is not this
//! shape at all is refused whole.

use serde::{Deserialize, Serialize};

use crate::activity::{Activity, Item, ItemKind};

/// The recent items at or beneath a scope, read or about to be written.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Recent {
    /// The scope the items are at or beneath.
    pub node: String,
    /// The items, newest first.
    pub items: Vec<Item>,
}

impl Recent {
    /// The most items a publisher writes: the last rounds, not the ring.
    pub const LIMIT: usize = 400;

    /// The newest [`Recent::LIMIT`] items at or beneath `node`.
    #[must_use]
    pub fn of(node: &str, activity: &Activity) -> Self {
        Self {
            node: node.to_string(),
            items: activity.recent(node, None, Self::LIMIT),
        }
    }

    /// The items as the TOML their readers read.
    #[must_use]
    pub fn to_toml(&self) -> String {
        let document = Document {
            node: self.node.clone(),
            items: self
                .items
                .iter()
                .map(|item| ItemDocument {
                    kind: item.kind.name().to_string(),
                    scope: item.scope.clone(),
                    id: item.id.clone(),
                    bytes: item.bytes,
                    detail: item.detail.clone(),
                    observed_unix_nanos: item.observed_unix_nanos,
                })
                .collect(),
        };
        toml::to_string(&document).unwrap_or_default()
    }

    /// The items read back from their TOML.
    ///
    /// # Errors
    ///
    /// When the text is not TOML of this shape, in the parser's words.
    pub fn read(text: &str) -> Result<Self, String> {
        let document: Document = toml::from_str(text).map_err(|error| error.to_string())?;
        let items = document
            .items
            .into_iter()
            .filter_map(|item| {
                Some(Item {
                    kind: ItemKind::named(&item.kind)?,
                    scope: item.scope,
                    id: item.id,
                    bytes: item.bytes,
                    detail: item.detail,
                    observed_unix_nanos: item.observed_unix_nanos,
                })
            })
            .collect();
        Ok(Self {
            node: document.node,
            items,
        })
    }
}

#[derive(Serialize, Deserialize)]
struct Document {
    #[serde(default)]
    node: String,
    #[serde(default)]
    items: Vec<ItemDocument>,
}

#[derive(Serialize, Deserialize)]
struct ItemDocument {
    kind: String,
    #[serde(default)]
    scope: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    bytes: u64,
    #[serde(default)]
    detail: String,
    #[serde(default)]
    observed_unix_nanos: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(kind: ItemKind, scope: &str, id: &str) -> Item {
        Item {
            kind,
            scope: scope.to_string(),
            id: id.to_string(),
            bytes: 12,
            detail: "delivered".to_string(),
            observed_unix_nanos: 5,
        }
    }

    #[test]
    fn the_newest_items_beneath_a_node_cross_the_file_whole() {
        let mut activity = Activity::default();
        activity.record(item(ItemKind::Stream, "xmip:///n/receive/tcp", "s1"));
        activity.record(item(ItemKind::Journey, "xmip:///n/process/p", "j1"));
        activity.record(item(ItemKind::Message, "xmip:///m/send/tcp", "m1"));

        let recent = Recent::of("xmip:///n", &activity);
        assert_eq!(recent.items.len(), 2, "only what is beneath the node");
        assert_eq!(recent.items[0].id, "j1", "newest first");

        let text = recent.to_toml();
        assert!(text.contains("kind = \"journey\""), "{text}");
        assert_eq!(Recent::read(&text), Ok(recent));
    }

    #[test]
    fn an_unknown_kind_is_skipped_and_a_stranger_is_refused() {
        let read = Recent::read("[[items]]\nkind = \"parcel\"\n[[items]]\nkind = \"stream\"\n")
            .expect("reads");
        assert_eq!(read.items.len(), 1);
        assert_eq!(read.items[0].kind, ItemKind::Stream);
        assert!(Recent::read("items = 1").is_err());
    }
}
