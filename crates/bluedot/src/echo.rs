//! EPA ECHO / ICIS-Air facilities — the Data Center Atlas's first source (DC-0.1).
//!
//! Facilities holding air permits under NAICS 518210 (data processing/hosting):
//! backup diesel generators need air permits, which makes permit records a
//! near-census of serious data centers (the method behind Business Insider's
//! 2025 map). The ECHO REST flow is two steps: `get_facilities` runs the query
//! and returns a row count plus an ephemeral query id; `get_qid` returns every
//! row as JSON in one page (verified 2026-09-02: 468 rows, all geocoded).
//!
//! Each fetch is a **snapshot**: the vintage is the retrieval date **in UTC**
//! (an evening US run lands on the next UTC day — the vintage is a label and
//! consistency beats local intuition), and each claim's valid time is that one
//! day — ECHO refreshes weekly and says nothing about when a status changed,
//! so we assert only "this is what the record said on this date". A same-day
//! rerun atomically replaces that day's snapshot; snapshots accumulate into
//! the pipeline history (plan §DC-1).

use serde_json::Value;

use crate::Error;
use crate::claim::{Claim, Confidence, ensure_unique_claim_keys};
use crate::fact::{Entity, Level};
use crate::http;
use crate::time::{Date, Timestamp};

pub const DATASET: &str = "epa/echo/air";
const BASE: &str = "https://echodata.epa.gov/echo";

/// The observed `AIRStatus` vocabulary (2026-09-02) mapped to `dc:stage`
/// tokens. The token is the source's own vocabulary normalized, not an
/// interpretation; display-level mapping to the atlas lifecycle happens in the
/// semantic layer later. An AIRStatus outside this list fails the batch.
const STAGE_TOKENS: &[(&str, &str)] = &[
    ("Operating", "operating"),
    ("Under Construction", "under_construction"),
    ("Planned Facility", "planned_facility"),
    ("Permanently Closed", "permanently_closed"),
    ("Temporarily Closed", "temporarily_closed"),
    ("No Operating Status In ICIS", "no_operating_status_in_icis"),
];

#[derive(Debug, Clone)]
pub struct Request {
    /// NAICS code filter (`p_ncs`), default 518210.
    pub naics: String,
    /// Snapshot date — becomes the vintage and the claims' valid day.
    pub snapshot: Date,
}

impl Request {
    pub fn new(naics: &str, snapshot: Date) -> Result<Self, Error> {
        if naics.is_empty() || !naics.chars().all(|c| c.is_ascii_digit()) {
            return Err(Error::Usage(format!("--naics {naics:?} should be digits")));
        }
        Ok(Request {
            naics: naics.to_owned(),
            snapshot,
        })
    }

    pub fn vintage(&self) -> String {
        format!("echo-{}", self.snapshot)
    }

    /// Provenance URL: the query that produced the rows (step 1; the qid in
    /// step 2 is ephemeral and meaningless later).
    pub fn url(&self) -> String {
        format!(
            "{BASE}/air_rest_services.get_facilities?output=JSON&p_ncs={}",
            self.naics
        )
    }
}

pub struct Client {
    agent: ureq::Agent,
}

impl Client {
    pub fn new() -> Self {
        Client {
            agent: http::agent(),
        }
    }

    /// Two-step fetch, then conform. No key required.
    pub fn facilities(&self, req: &Request) -> Result<Conformed, Error> {
        let retrieved_at = Timestamp::now();
        let query_url = req.url();
        let step1 = self.get_json(&query_url)?;
        let qid = step1["Results"]["QueryID"]
            .as_str()
            .map(str::to_owned)
            .or_else(|| step1["Results"]["QueryID"].as_u64().map(|n| n.to_string()))
            .ok_or_else(|| Error::BadResponseShape {
                url: query_url.clone(),
                detail: format!(
                    "no QueryID; Results head: {:.200}",
                    step1["Results"].to_string()
                ),
            })?;
        let rows_url = format!("{BASE}/air_rest_services.get_qid?output=JSON&qid={qid}");
        let step2 = self.get_json(&rows_url)?;
        conform(&step2.to_string(), req, retrieved_at)
    }

    fn get_json(&self, url: &str) -> Result<Value, Error> {
        let mut resp = self.agent.get(url).call().map_err(|source| Error::Http {
            url: url.to_owned(),
            source,
        })?;
        let status = resp.status().as_u16();
        let body = resp
            .body_mut()
            .with_config()
            .limit(http::BODY_LIMIT_BYTES)
            .read_to_string()
            .map_err(|source| Error::Http {
                url: url.to_owned(),
                source,
            })?;
        if status != 200 {
            return Err(Error::UnexpectedStatus {
                url: url.to_owned(),
                status,
                body_head: body.chars().take(120).collect(),
            });
        }
        let value: Value = serde_json::from_str(&body).map_err(|source| Error::NotJson {
            url: url.to_owned(),
            source,
            body_head: body.chars().take(120).collect(),
        })?;
        // ECHO reports its own errors inside a 200 (row-limit, bad params) —
        // surface them instead of failing later on a missing field.
        if let Some(msg) = value["Results"]["Error"]["ErrorMessage"].as_str() {
            return Err(Error::BadResponseShape {
                url: url.to_owned(),
                detail: format!("ECHO error: {msg}"),
            });
        }
        Ok(value)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

/// What an ECHO conform pass produces.
#[derive(Debug)]
pub struct Conformed {
    pub entities: Vec<Entity>,
    pub claims: Vec<Claim>,
}

/// Turn a get_qid response body into entities + claims. Pure.
///
/// One facility can carry several permit records (13 of 468 FRS ids repeat,
/// verified 2026-09-02): the FRS `RegistryID` becomes ONE entity, and every
/// record contributes its own stage claim keyed by its `SourceID` — competing
/// records coexist rather than being silently collapsed.
pub fn conform(body: &str, req: &Request, retrieved_at: Timestamp) -> Result<Conformed, Error> {
    let url = req.url();
    let shape_err = |detail: String| Error::BadResponseShape {
        url: url.clone(),
        detail,
    };
    let value: Value = serde_json::from_str(body).map_err(|source| Error::NotJson {
        url: url.clone(),
        source,
        body_head: body.chars().take(120).collect(),
    })?;
    let rows = value["Results"]["Facilities"]
        .as_array()
        .ok_or_else(|| shape_err("no Results.Facilities array".into()))?;
    // Zero rows from a source whose premise is a near-census of an active
    // industry is a broken filter or an outage dressed as success, not data.
    if rows.is_empty() {
        return Err(shape_err("zero facilities returned".into()));
    }

    let (valid_from, valid_to) = (req.snapshot, req.snapshot.next_day());
    let vintage = req.vintage();
    let mut entities: Vec<Entity> = Vec::new();
    let mut claims = Vec::with_capacity(rows.len());

    for (i, row) in rows.iter().enumerate() {
        let text = |key: &str| -> Result<String, Error> {
            match row[key].as_str().map(str::trim) {
                Some(v) if !v.is_empty() => Ok(v.to_owned()),
                _ => Err(shape_err(format!("row {}: missing {key}", i + 1))),
            }
        };
        let registry_id = text("RegistryID")?;
        let source_id = text("SourceID")?;
        let name = text("AIRName")?;
        let air_status = text("AIRStatus")?;
        let entity_id = format!("frs/{registry_id}");

        // Coordinates arrive as strings; parse or fail — a facility without a
        // parseable location is a record we don't understand (all 468 verified
        // rows are geocoded).
        let coord = |key: &str| -> Result<f64, Error> {
            text(key)?.parse::<f64>().map_err(|_| Error::BadNumber {
                text: row[key].to_string(),
                field: key.to_owned(),
                geoid: registry_id.clone(),
                variable: "echo get_qid".to_owned(),
            })
        };
        let (lat, lon) = (coord("FacLat")?, coord("FacLong")?);

        let stage = STAGE_TOKENS
            .iter()
            .find(|(raw, _)| *raw == air_status)
            .map(|(_, token)| *token)
            .ok_or_else(|| shape_err(format!("row {}: AIRStatus {air_status:?} not in the known vocabulary — verify against ECHO and extend STAGE_TOKENS deliberately", i + 1)))?;

        // One entity per FRS id (first record's name wins; both records' names
        // remain reachable through their claims' source_record).
        if !entities.iter().any(|e| e.entity_id == entity_id) {
            entities.push(Entity {
                entity_id: entity_id.clone(),
                name: name.clone(),
                level: Level::Facility,
                boundary_year: self_year(req.snapshot),
                vintage: vintage.clone(),
                source_dataset: DATASET.to_owned(),
                lat: Some(lat),
                lon: Some(lon),
            });
        }

        // One template, three claims per record via struct-update syntax
        // (`..template.clone()` starts from a clone and overrides the named
        // fields). dc:state keeps by-state queries possible before the spatial
        // join exists; dc:recorded_name preserves EVERY record's stated name —
        // the entity keeps the first record's name (the registry holds one per
        // source+vintage, ADR-0014), so disagreements between a facility's
        // records stay visible as claims instead of being lost. The first
        // record's coordinates win; coordinate disagreement between records of
        // one FRS id is accepted undetected in v0.
        let template = Claim {
            entity_id: entity_id.clone(),
            attribute_id: String::new(),
            valid_from,
            valid_to,
            vintage: vintage.clone(),
            source_record: source_id.clone(),
            value_text: None,
            value_num: None,
            unit: None,
            stated_by: "EPA ICIS-Air (state-reported)".to_owned(),
            confidence: Confidence::ConfirmedByRecord,
            published_at: req.snapshot,
            source_dataset: DATASET.to_owned(),
            source_url: url.clone(),
            retrieved_at,
        };
        claims.push(Claim {
            attribute_id: "dc:stage".to_owned(),
            value_text: Some(stage.to_owned()),
            ..template.clone()
        });
        claims.push(Claim {
            attribute_id: "dc:state".to_owned(),
            value_text: Some(text("AIRState")?),
            ..template.clone()
        });
        claims.push(Claim {
            attribute_id: "dc:recorded_name".to_owned(),
            value_text: Some(name.clone()),
            ..template
        });
    }
    ensure_unique_claim_keys(&claims)?;
    Ok(Conformed { entities, claims })
}

fn self_year(d: Date) -> u16 {
    d.year as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUBSET: &str = include_str!("../tests/fixtures/echo-518210-subset.json");
    const T0: Timestamp = Timestamp {
        unix_seconds: 1_756_800_000,
    };

    fn req() -> Request {
        Request::new("518210", Date::new(2026, 9, 2)).unwrap()
    }

    #[test]
    fn conforms_entities_and_stage_claims() {
        let c = conform(SUBSET, &req(), T0).unwrap();
        let stage_claims: Vec<_> = c
            .claims
            .iter()
            .filter(|cl| cl.attribute_id == "dc:stage")
            .collect();
        let state_claims: Vec<_> = c
            .claims
            .iter()
            .filter(|cl| cl.attribute_id == "dc:state")
            .collect();
        let name_claims: Vec<_> = c
            .claims
            .iter()
            .filter(|cl| cl.attribute_id == "dc:recorded_name")
            .collect();
        assert_eq!(
            c.claims.len(),
            stage_claims.len() + state_claims.len() + name_claims.len()
        );
        assert_eq!(stage_claims.len(), name_claims.len());
        // the twice-recorded FRS id keeps BOTH stated names, as claims
        let names: Vec<_> = name_claims
            .iter()
            .filter(|cl| cl.entity_id == "frs/110031223771")
            .filter_map(|cl| cl.value_text.as_deref())
            .collect();
        assert!(
            names.contains(&"AMAZON DATA SERVICES, INC. IAD-6 IAD-13 IAD-54"),
            "{names:?}"
        );
        assert!(names.contains(&"VADATA INC MEG FOUR"), "{names:?}");
        assert_eq!(
            stage_claims.len(),
            state_claims.len(),
            "one stage + one state claim per record"
        );
        assert!(
            stage_claims.len() > c.entities.len(),
            "duplicate-FRS records must yield extra claims"
        );
        let mut counts = std::collections::HashMap::new();
        for cl in &stage_claims {
            *counts.entry(cl.entity_id.as_str()).or_insert(0) += 1;
        }
        assert_eq!(
            counts.values().filter(|n| **n > 1).count(),
            1,
            "exactly one multi-record facility"
        );
        assert!(
            state_claims
                .iter()
                .any(|cl| cl.value_text.as_deref() == Some("VA"))
        );
        let mut ids: Vec<_> = c.entities.iter().map(|e| e.entity_id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), c.entities.len(), "entities unique per FRS id");
        for e in &c.entities {
            assert_eq!(e.level, Level::Facility);
            assert!(e.entity_id.starts_with("frs/"));
            assert!(e.lat.is_some() && e.lon.is_some());
        }
        for cl in &c.claims {
            assert_eq!(
                (cl.valid_from, cl.valid_to),
                (Date::new(2026, 9, 2), Date::new(2026, 9, 3))
            );
            assert_eq!(cl.vintage, "echo-2026-09-02");
            assert_eq!(cl.confidence, Confidence::ConfirmedByRecord);
            assert!(!cl.source_record.is_empty());
        }
        for cl in &stage_claims {
            assert!(
                STAGE_TOKENS
                    .iter()
                    .any(|(_, t)| Some(*t) == cl.value_text.as_deref())
            );
        }
        let json = serde_json::to_string(&c.claims[0]).unwrap();
        assert!(
            json.contains(r#""confidence":"confirmed_by_record""#),
            "{json}"
        );
    }

    #[test]
    fn refuses_what_it_does_not_understand() {
        let mangle = |from: &str, to: &str| SUBSET.replacen(from, to, 1);

        let unknown = conform(&mangle("Operating", "Vibing"), &req(), T0).unwrap_err();
        assert!(
            matches!(&unknown, Error::BadResponseShape { detail, .. } if detail.contains("Vibing")),
            "{unknown}"
        );

        let no_latlon =
            conform(&mangle(r#""FacLat""#, r#""FacLatMissing""#), &req(), T0).unwrap_err();
        assert!(
            matches!(no_latlon, Error::BadResponseShape { .. }),
            "{no_latlon}"
        );

        let echo_err = conform(
            r#"{"Results":{"Error":{"ErrorMessage":"boom"}}}"#,
            &req(),
            T0,
        )
        .unwrap_err();
        assert!(
            matches!(echo_err, Error::BadResponseShape { .. }),
            "{echo_err}"
        );

        let empty = conform(r#"{"Results":{"Facilities":[]}}"#, &req(), T0).unwrap_err();
        assert!(
            matches!(&empty, Error::BadResponseShape { detail, .. } if detail.contains("zero")),
            "{empty}"
        );

        // a literally repeated record must refuse the batch with the claim-key error
        let mut v: serde_json::Value = serde_json::from_str(SUBSET).unwrap();
        let rows = v["Results"]["Facilities"].as_array_mut().unwrap();
        let first = rows[0].clone();
        rows.push(first);
        let dup = conform(&v.to_string(), &req(), T0).unwrap_err();
        assert!(matches!(dup, Error::DuplicateClaimKey { .. }), "{dup}");
        assert!(matches!(
            Request::new("51x", Date::new(2026, 9, 2)).unwrap_err(),
            Error::Usage(_)
        ));
    }
}
