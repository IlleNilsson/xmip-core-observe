#![forbid(unsafe_code)]

//! Observation: what is happening now, and what is unhealthy.
//!
//! observability-model.md section 6. Near-real-time and never synchronous —
//! Receive, Process and Send must never wait for it. What they write lands in
//! a [`Snapshot`], and the operator boundary in `xmip_operate.h` reads that
//! snapshot and nothing else. ADR-0027 clause 6.
//!
//! `Grey` and `Black` left on 2026-09-04. No document defined them; section 6
//! names the moods — `Fine`, `Working`, `Stressed`, `Exhausted`, `Done` — with
//! `Holding` the rollup only a parent shows (ADR-0041).

pub mod activity;
pub mod capability;
pub mod counted;
pub mod curve;
pub mod dead_message;
pub mod event_subscription;
pub mod figure;
pub mod health;
pub mod history;
pub mod order;
pub mod pause_state;
pub mod publication;
pub mod recent;
pub mod run;
pub mod scope;
pub mod snapshot;
pub mod subscription;
pub mod topology;
pub mod unheard;
pub mod wildcard;

pub use activity::{Activity, DEFAULT_ITEM_CAPACITY, Item, ItemKind};
pub use counted::Counted;
pub use curve::Curve;
pub use dead_message::DeadMessage;
pub use event_subscription::EventSubscription;
pub use figure::{FIGURES, Figure, Kind, Point, Points, Reading};
pub use health::{Health, Standing};
pub use history::{DEFAULT_SERIES_CAPACITY, History};
pub use order::{Act, Noun, Order};
pub use pause_state::PauseState;
pub use publication::Publication;
pub use recent::Recent;
pub use run::{Run, RunList};
pub use scope::Scope;
pub use snapshot::{Count, HealthRecord, Snapshot};
pub use subscription::Subscription;
pub use topology::{NodeKind, Origin, Pattern, Topology, TopologyLink, TopologyNode};
pub use unheard::Unheard;

use xcore::{Clock, SystemClock};

/// Now, as an observe record is stamped: the estate's one clock,
/// `xcore::SystemClock`, in the unit a record and `xmip_operate.h` carry —
/// unix nanoseconds as an `i64`, saturating past it.
#[must_use]
pub fn now_unix_nanos() -> i64 {
    i64::try_from(SystemClock.unix_timestamp_nanos()).unwrap_or(i64::MAX)
}
