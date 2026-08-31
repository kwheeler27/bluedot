//! Fact schema v0 — see `docs/briefs/01-acs-facts-v0.md` and ADR-0001/0012/0013.
//!
//! One `Fact` is one observed value of one indicator for one entity over one
//! valid-time interval, as published in one vintage, with its provenance.

use serde::Serialize;

use crate::time::{Date, Timestamp};

/// Field order here is the column order in the JSON Lines output.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Fact {
    // ---- the key (ADR-0001): entity × indicator × valid_time × vintage ----
    /// Data Commons-style identifier, e.g. `geoId/06037` (ADR-0012).
    pub entity_id: String,
    /// Source-prefixed indicator, e.g. `acs:B01003_001` (estimate and MOE are one fact).
    pub indicator_id: String,
    /// Valid time as a half-open interval `[valid_from, valid_to)` (ADR-0013).
    pub valid_from: Date,
    pub valid_to: Date,
    /// The release this value comes from, e.g. `acs5-2023`.
    pub vintage: String,

    // ---- knowledge time ----
    /// When the vintage was published — what an "as of knowledge date" query compares against.
    pub published_at: Date,

    // ---- the observation ----
    /// The estimate. `None` exactly when `value_annotation` is `Some`.
    pub value: Option<f64>,
    /// The published 90% margin of error. `None` exactly when `moe_annotation` is `Some`.
    pub moe: Option<f64>,
    /// Why `value` is absent, decoded from the source's annotation code.
    pub value_annotation: Option<Annotation>,
    /// Why `moe` is absent. The source annotates estimate and MOE independently,
    /// which is why these are two columns and not one.
    pub moe_annotation: Option<Annotation>,

    // ---- geography version (ADR-0003) ----
    /// The year of the boundary definitions this release used.
    pub boundary_year: u16,

    // ---- provenance (ADR-0010) ----
    pub source_dataset: String,
    /// The request that produced this row, with the API key removed.
    pub source_url: String,
    pub retrieved_at: Timestamp,
}

/// Why a value is absent. Names follow the Census Bureau's "Notes on ACS Estimate
/// and Annotation Values"; the numeric codes live in `acs::SENTINELS`.
///
/// `rename_all = "snake_case"` makes serde write `insufficient_sample`, not
/// `InsufficientSample` — the on-disk vocabulary is lowercase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Annotation {
    /// Estimate could not be computed: insufficient sample observations (`-`).
    InsufficientSample,
    /// Cannot be displayed: insufficient sample cases in this geography (`N`).
    NotDisplayable,
    /// Not applicable or not available (`(X)`).
    NotApplicable,
    /// MOE could not be computed: insufficient sample observations (`**`).
    MoeInsufficientSample,
    /// MOE could not be computed: median falls in an open-ended interval (`***`).
    MoeOpenEndedMedian,
    /// MOE not appropriate: the estimate is controlled to an independent population/housing estimate (`*****`).
    Controlled,
    /// The API returned JSON `null` — no official annotation code accompanies it.
    Missing,
}
