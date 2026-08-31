//! Blue Dot ingestion/conformance engine.
//!
//! This is the *library* half of the `bluedot` package: all real logic lives
//! here so it can be unit-tested, integration-tested from `tests/`, and later
//! bound into Python. `main.rs` is a thin shell over it.
//!
//! Module map (one job each):
//! - [`acs`]    — Census ACS API client + conformance into the fact schema
//! - [`fact`]   — the fact schema v0 (`Fact`, `Annotation`)
//! - [`time`]   — tiny `Date`/`Timestamp` types (no date crate: a few lines of integer math)
//! - [`jsonl`]  — atomic JSON Lines writer
//! - [`config`] — where the Census API key comes from
//! - [`error`]  — the crate's single error type
//!
//! (`//!` is an "inner" doc comment — it documents the thing it is inside of,
//! here the crate. `///` is an "outer" doc comment — it documents the next item.)

pub mod acs;
pub mod config;
pub mod error;
pub mod fact;
pub mod jsonl;
pub mod time;

// Re-export so callers write `bluedot::Error` instead of `bluedot::error::Error`.
pub use error::Error;
