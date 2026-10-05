//! A mood as a publication writes it: [`Health::word`], and a word no mood
//! is called read as `Stressed`, so it shows and is looked at.

use serde::{Deserialize, Deserializer, Serializer};

use crate::health::Health;

/// What a mood no one is called reads as.
pub(crate) const fn unknown() -> Health {
    Health::Stressed
}

#[allow(clippy::trivially_copy_pass_by_ref)] // serde's `with` passes a reference
pub(crate) fn serialize<S: Serializer>(health: &Health, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(health.word())
}

pub(crate) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Health, D::Error> {
    let word = String::deserialize(deserializer)?;
    Ok(Health::named(&word).unwrap_or_else(unknown))
}
