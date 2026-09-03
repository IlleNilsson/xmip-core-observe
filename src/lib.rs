#![forbid(unsafe_code)]

//! Observation: what is happening now, and what is unhealthy.
//!
//! observability-model.md section 6. Near-real-time and never synchronous —
//! Receive, Process and Send must never wait for it. What they write lands in
//! a [`Snapshot`], and the operator boundary in `xmip_operate.h` reads that
//! snapshot and nothing else. ADR-0027 clause 6.
//!
//! `Grey` and `Black` left on 2026-09-04. No document defined them; section 6
//! has three states and ADR-0027 adds a fourth that only a surface may say.

pub mod snapshot;

pub use snapshot::{Count, Counted, Health, HealthRecord, Snapshot};
