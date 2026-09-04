//! Fact schema v0 — see `docs/briefs/01-acs-facts-v0.md` and ADR-0001/0012/0013.
//!
//! One `Fact` is one observed value of one indicator for one entity over one
//! valid-time interval, as published in one vintage, with its provenance.

use std::collections::HashSet;

use serde::Serialize;

use crate::Error;
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

/// Refuse a batch whose fact key (ADR-0001) is not unique — a source that
/// repeats a row is a source we don't understand. Both conform() paths call
/// this before returning, so `ingest` succeeding IS the uniqueness guarantee
/// (the Python build step re-checks across files).
///
/// `HashSet::insert` returns false when the value was already present — the
/// whole check is one pass, borrowing `&str`s rather than cloning keys.
pub fn ensure_unique_keys(facts: &[Fact]) -> Result<(), Error> {
    let mut seen = HashSet::with_capacity(facts.len());
    for f in facts {
        let key = (
            f.entity_id.as_str(),
            f.indicator_id.as_str(),
            f.valid_from,
            f.valid_to,
            f.vintage.as_str(),
        );
        if !seen.insert(key) {
            return Err(Error::DuplicateFactKey {
                entity_id: f.entity_id.clone(),
                indicator_id: f.indicator_id.clone(),
                valid_from: f.valid_from,
                vintage: f.vintage.clone(),
            });
        }
    }
    Ok(())
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

/// One row of the entity registry v0 (ADR-0014): what one source called a
/// place, in one vintage. Deliberately per-source and per-vintage — ACS says
/// "Los Angeles County, California" where PEP says "Los Angeles County";
/// choosing a canonical form is the real registry's job, later. No
/// deduplication here, and `build-facts` refuses only exact key repeats.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Entity {
    pub entity_id: String,
    pub name: String,
    pub level: Level,
    pub boundary_year: u16,
    pub vintage: String,
    pub source_dataset: String,
    /// Point coordinates where the source provides them (facilities);
    /// `None` for administrative areas until geometry arrives (ADR-0003).
    pub lat: Option<f64>,
    pub lon: Option<f64>,
}

/// Geographic level. An enum rather than a string so a typo'd level is a
/// compile error inside the engine; serde writes the lowercase wire form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    State,
    County,
    /// A physical site (e.g. a data center building), not an administrative area.
    Facility,
    /// A multi-building development (e.g. a data center campus).
    Campus,
}

/// What one conform pass produces: the facts, and the entities they mention.
#[derive(Debug)]
pub struct Conformed {
    pub facts: Vec<Fact>,
    pub entities: Vec<Entity>,
}

/// Captured source geometry for one entity in one vintage (brief 09).
///
/// Rings are lon/lat exactly as the source publishes them — outer ring
/// first, holes after, unsimplified. Simplification and projection are
/// display concerns that happen at compile time, never at ingest.
#[derive(Debug, Clone, Serialize)]
pub struct Geometry {
    pub entity_id: String,
    pub vintage: String,
    pub source_dataset: String,
    pub rings: Vec<Vec<[f64; 2]>>,
    pub retrieved_at: Timestamp,
}
