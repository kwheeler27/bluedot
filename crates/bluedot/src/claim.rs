//! The claim — the second record kind in the store (Data Center Atlas plan §Schema).
//!
//! A `Fact` (schema v0) is a numeric observation of a declared statistical
//! indicator. A `Claim` is broader: *someone asserts something about an entity*
//! — a lifecycle stage, a capacity figure, a role. Same bitemporal spine
//! (valid time × vintage) and provenance discipline as facts, extended with
//! who said it and how sure we are. Claims are never merged, averaged, or
//! reconciled at ingest; competing claims coexist and carry their sources.

use serde::Serialize;

use crate::Error;
use crate::time::{Date, Timestamp};

/// Field order is the column order of the JSON Lines / Parquet output.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Claim {
    // ---- key: entity × attribute × valid_time × vintage × source record ----
    pub entity_id: String,
    /// Namespaced attribute, e.g. `dc:stage`. Vocabulary lives with the source
    /// adapter for now; the semantic layer will own it later (ADR-0004).
    pub attribute_id: String,
    /// Valid time as a half-open interval (ADR-0013). For an observed-as-of
    /// snapshot, the one-day interval of the snapshot date.
    pub valid_from: Date,
    pub valid_to: Date,
    /// The snapshot/release this claim was learned from, e.g. `echo-2026-09-02`.
    pub vintage: String,
    /// The specific record inside the source that asserts this (permit id,
    /// docket number, filing accession…). Part of the key: one facility can
    /// carry several records in one vintage, and each keeps its own claim.
    pub source_record: String,

    // ---- the assertion ----
    /// Exactly one of `value_text` / `value_num` is set.
    pub value_text: Option<String>,
    pub value_num: Option<f64>,
    pub unit: Option<String>,
    /// Who asserts it (agency, company, document author) — as stated, not resolved.
    pub stated_by: String,
    pub confidence: Confidence,

    // ---- knowledge time + provenance (ADR-0010) ----
    pub published_at: Date,
    pub source_dataset: String,
    pub source_url: String,
    pub retrieved_at: Timestamp,
}

/// How the claim is backed. The tier is part of the claim and renders with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    /// Backed by an official record (permit, docket, filing).
    ConfirmedByRecord,
    /// Reported (press, company statement) but not record-backed.
    Reported,
    /// Circulating without an identifiable record or on-record statement.
    Rumored,
}

/// Refuse a batch whose claim key is not unique — same discipline as
/// `fact::ensure_unique_keys`, same reasoning: `ingest` succeeding must itself
/// be the guarantee.
pub fn ensure_unique_claim_keys(claims: &[Claim]) -> Result<(), Error> {
    let mut seen = std::collections::HashSet::with_capacity(claims.len());
    for c in claims {
        let key = (
            c.entity_id.as_str(),
            c.attribute_id.as_str(),
            c.valid_from,
            c.valid_to,
            c.vintage.as_str(),
            c.source_record.as_str(),
        );
        if !seen.insert(key) {
            return Err(Error::DuplicateClaimKey {
                entity_id: c.entity_id.clone(),
                attribute_id: c.attribute_id.clone(),
                valid_from: c.valid_from,
                vintage: c.vintage.clone(),
                source_record: c.source_record.clone(),
            });
        }
    }
    Ok(())
}
