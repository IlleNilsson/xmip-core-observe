//! A snapshot's health records and counts, each kept by what it is found by
//! — a record by its scope, a count by its scope and what it counted — read
//! from the record itself: the scope is written once, in the record, never
//! again as a key beside it. Until 2026-11-14 every record and count carried
//! a copy of its scope as its key, a text written twice each time a snapshot
//! was recorded or merged (ADR-0052, amendment 2026-11-14).

use std::borrow::Borrow;
use std::cmp::Ordering;

use super::{Count, HealthRecord};

/// A health record, found and ordered by its scope.
#[derive(Clone, Debug)]
pub(super) struct Kept(pub(super) HealthRecord);

impl Borrow<str> for Kept {
    fn borrow(&self) -> &str {
        &self.0.scope
    }
}

impl PartialEq for Kept {
    fn eq(&self, other: &Self) -> bool {
        self.0.scope == other.0.scope
    }
}

impl Eq for Kept {}

impl PartialOrd for Kept {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Kept {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.scope.cmp(&other.0.scope)
    }
}

/// A count, found and ordered by its scope and then what it counted.
#[derive(Clone, Debug)]
pub(super) struct Counting(pub(super) Count);

impl PartialEq for Counting {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Counting {}

impl PartialOrd for Counting {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Counting {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.0.scope.as_str(), self.0.counted).cmp(&(other.0.scope.as_str(), other.0.counted))
    }
}
