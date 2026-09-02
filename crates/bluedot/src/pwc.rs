//! Prince William County VA — Data Center Buildings & Campuses (DC-0.2).
//!
//! The county's own planning GIS publishes the richest per-facility record set
//! found anywhere in the source research: building-level status, year built,
//! occupancy date, permit cases, and FOUR different floor-area figures per
//! building (assessed / approved / permitted / taxed — different measures from
//! different county systems, kept as separate attributes, never merged), plus
//! campus polygons with rezoning cases and planned floor area. ArcGIS REST,
//! `outSR=4326`, both layers under their 2,000-row page limit (verified
//! 2026-09-02: 247 buildings, 75 campuses).
//!
//! Snapshot semantics as in [`crate::echo`] (UTC vintage, one-day valid
//! intervals), with one refinement: each record's `published_at` is the
//! county's own `LastEditDate` where present — the county says when it last
//! touched the record, so we use it.

use serde_json::Value;

use crate::Error;
use crate::claim::{Claim, Confidence, ensure_unique_claim_keys};
use crate::fact::{Entity, Level};
use crate::http;
use crate::time::{Date, Timestamp};

pub const DATASET: &str = "pwcva/build-out-analysis";
const BASE: &str =
    "https://gisweb.pwcva.gov/arcgis/rest/services/Planning/Build_Out_Analysis/MapServer";
const STATED_BY: &str = "Prince William County Planning GIS";

/// Building lifecycle vocabulary (layer 9, verified 2026-09-02).
const BUILDING_STAGES: &[(&str, &str)] = &[
    ("Completed", "completed"),
    ("Under Construction", "under_construction"),
    ("Pending", "pending"),
    ("Planned", "planned"),
];
/// Permit status vocabulary (layer 9).
const PERMIT_STATUSES: &[(&str, &str)] = &[
    ("Finaled", "finaled"),
    ("Issued", "issued"),
    ("Pending", "pending"),
    ("Planned", "planned"),
];
/// Campus project vocabulary (layer 10).
const CAMPUS_STAGES: &[(&str, &str)] = &[
    ("Completed", "completed"),
    ("Pending", "pending"),
    ("Planned", "planned"),
];

#[derive(Debug, Clone)]
pub struct Request {
    pub snapshot: Date,
}

impl Request {
    pub fn new(snapshot: Date) -> Self {
        Request { snapshot }
    }

    pub fn vintage(&self) -> String {
        format!("pwc-{}", self.snapshot)
    }

    pub fn buildings_url(&self) -> String {
        format!("{BASE}/9/query?where=1%3D1&outFields=*&returnGeometry=true&outSR=4326&f=json")
    }

    pub fn campuses_url(&self) -> String {
        format!("{BASE}/10/query?where=1%3D1&outFields=*&returnGeometry=true&outSR=4326&f=json")
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

    pub fn facilities(&self, req: &Request) -> Result<Conformed, Error> {
        let retrieved_at = Timestamp::now();
        let buildings = self.get_text(&req.buildings_url())?;
        let campuses = self.get_text(&req.campuses_url())?;
        conform(&buildings, &campuses, req, retrieved_at)
    }

    fn get_text(&self, url: &str) -> Result<String, Error> {
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
        Ok(body)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct Conformed {
    pub entities: Vec<Entity>,
    pub claims: Vec<Claim>,
}

/// Pure conformance over the two layers' query responses.
pub fn conform(
    buildings_body: &str,
    campuses_body: &str,
    req: &Request,
    retrieved_at: Timestamp,
) -> Result<Conformed, Error> {
    let mut entities = Vec::new();
    let mut claims = Vec::new();
    conform_layer(
        buildings_body,
        LayerKind::Building,
        req,
        retrieved_at,
        &mut entities,
        &mut claims,
    )?;
    conform_layer(
        campuses_body,
        LayerKind::Campus,
        req,
        retrieved_at,
        &mut entities,
        &mut claims,
    )?;
    ensure_unique_claim_keys(&claims)?;
    Ok(Conformed { entities, claims })
}

#[derive(Clone, Copy, PartialEq)]
enum LayerKind {
    Building,
    Campus,
}

fn conform_layer(
    body: &str,
    kind: LayerKind,
    req: &Request,
    retrieved_at: Timestamp,
    entities: &mut Vec<Entity>,
    claims: &mut Vec<Claim>,
) -> Result<(), Error> {
    let url = match kind {
        LayerKind::Building => req.buildings_url(),
        LayerKind::Campus => req.campuses_url(),
    };
    let shape_err = |detail: String| Error::BadResponseShape {
        url: url.clone(),
        detail,
    };
    let value: Value = serde_json::from_str(body).map_err(|source| Error::NotJson {
        url: url.clone(),
        source,
        body_head: body.chars().take(120).collect(),
    })?;
    if let Some(msg) = value["error"]["message"].as_str() {
        return Err(shape_err(format!("ArcGIS error: {msg}")));
    }
    let rows = value["features"]
        .as_array()
        .ok_or_else(|| shape_err("no features array".into()))?;
    if rows.is_empty() {
        return Err(shape_err("zero features returned".into()));
    }

    let (valid_from, valid_to) = (req.snapshot, req.snapshot.next_day());
    let vintage = req.vintage();

    for (i, row) in rows.iter().enumerate() {
        let attrs = &row["attributes"];
        // ArcGIS emits the literal string "<Null>" in some text fields (the
        // captured fixture has PermitCase: "<Null>") — treat it as absent, or
        // it leaks into claim provenance as a fake record id.
        let s = |key: &str| {
            attrs[key]
                .as_str()
                .map(str::trim)
                .filter(|v| !v.is_empty() && *v != "<Null>")
        };
        let n = |key: &str| attrs[key].as_f64().filter(|v| *v != 0.0); // the county uses 0 for "not set"
        let need = |key: &str| {
            s(key)
                .map(str::to_owned)
                .ok_or_else(|| shape_err(format!("row {}: missing {key}", i + 1)))
        };

        // GlobalID is the stable record key: "{ABC-...}" → "abc-..." for a URL-safe id.
        let gid = need("GlobalID")?
            .trim_matches(['{', '}'])
            .to_ascii_lowercase();
        let (prefix, name, stage_vocab, stage_raw) = match kind {
            LayerKind::Building => (
                "pwc/bld",
                s("BuildingName").or_else(|| s("Address")),
                BUILDING_STAGES,
                need("BuildingStatus")?,
            ),
            LayerKind::Campus => (
                "pwc/campus",
                s("CampusName").or_else(|| s("CaseName")),
                CAMPUS_STAGES,
                need("ProjectStatus")?,
            ),
        };
        let name = name
            .map(str::to_owned)
            .ok_or_else(|| shape_err(format!("row {}: no name or address", i + 1)))?;
        let entity_id = format!("{prefix}/{gid}");

        // Coordinates: point layers carry geometry.x/y; polygon layers carry
        // rings — we take the mean of the outer ring's vertices as a label
        // point (an approximation for display, not survey geometry).
        let (lon, lat) = match kind {
            LayerKind::Building => (
                row["geometry"]["x"]
                    .as_f64()
                    .ok_or_else(|| shape_err(format!("row {}: no geometry.x", i + 1)))?,
                row["geometry"]["y"]
                    .as_f64()
                    .ok_or_else(|| shape_err(format!("row {}: no geometry.y", i + 1)))?,
            ),
            LayerKind::Campus => {
                let ring = row["geometry"]["rings"][0]
                    .as_array()
                    .filter(|r| !r.is_empty())
                    .ok_or_else(|| shape_err(format!("row {}: no polygon ring", i + 1)))?;
                // Closed rings repeat the first vertex at the end — drop the
                // repeat so it isn't double-weighted in the mean.
                let closed = ring.len() > 1 && ring.first() == ring.last();
                let ring = &ring[..ring.len() - usize::from(closed)];
                let mut sx = 0.0;
                let mut sy = 0.0;
                for v in ring {
                    sx += v[0].as_f64().unwrap_or(f64::NAN);
                    sy += v[1].as_f64().unwrap_or(f64::NAN);
                }
                let c = (sx / ring.len() as f64, sy / ring.len() as f64);
                if !c.0.is_finite() || !c.1.is_finite() {
                    return Err(shape_err(format!("row {}: unparseable ring vertex", i + 1)));
                }
                c
            }
        };

        // The county's own edit date is the closest thing to a publication
        // date. It is a UTC instant with real time-of-day, so an edit made
        // after ~7-8pm Eastern lands on the next UTC calendar day — accepted,
        // the same UTC-day convention the snapshot vintage uses.
        let published_at = attrs["LastEditDate"]
            .as_i64()
            .map(|ms| Date::from_unix_days(ms.div_euclid(86_400_000)))
            .unwrap_or(req.snapshot);

        entities.push(Entity {
            entity_id: entity_id.clone(),
            name: name.clone(),
            level: if kind == LayerKind::Building {
                Level::Facility
            } else {
                Level::Campus
            },
            boundary_year: req.snapshot.year as u16,
            vintage: vintage.clone(),
            source_dataset: DATASET.to_owned(),
            lat: Some(lat),
            lon: Some(lon),
        });

        let template = Claim {
            entity_id,
            attribute_id: String::new(),
            valid_from,
            valid_to,
            vintage: vintage.clone(),
            source_record: gid.clone(),
            value_text: None,
            value_num: None,
            unit: None,
            stated_by: STATED_BY.to_owned(),
            confidence: Confidence::ConfirmedByRecord,
            published_at,
            source_dataset: DATASET.to_owned(),
            source_url: url.clone(),
            retrieved_at,
        };
        let mut push_text = |attribute: &str,
                             v: String,
                             source_record: Option<String>,
                             stated_by: Option<String>| {
            claims.push(Claim {
                attribute_id: attribute.to_owned(),
                value_text: Some(v),
                source_record: source_record.unwrap_or_else(|| gid.clone()),
                stated_by: stated_by.unwrap_or_else(|| STATED_BY.to_owned()),
                ..template.clone()
            });
        };

        let stage = stage_vocab
            .iter()
            .find(|(raw, _)| *raw == stage_raw)
            .map(|(_, token)| *token)
            .ok_or_else(|| shape_err(format!("row {}: status {stage_raw:?} not in the known vocabulary — verify against the county layer and extend deliberately", i + 1)))?;
        push_text("dc:stage", stage.to_owned(), None, None);
        push_text("dc:recorded_name", name, None, None);

        match kind {
            LayerKind::Building => {
                if let Some(addr) = s("Address") {
                    push_text("dc:address", addr.to_owned(), None, None);
                }
                if let Some(gpin) = s("GPIN") {
                    push_text("dc:parcel_gpin", gpin.to_owned(), None, None);
                }
                if let Some(occ) = attrs["OCCDate"].as_i64() {
                    let d = Date::from_unix_days(occ.div_euclid(86_400_000));
                    push_text("dc:occupied_date", d.to_string(), None, None);
                }
                if let Some(status) = s("PermitStatus") {
                    let token = PERMIT_STATUSES
                        .iter()
                        .find(|(raw, _)| *raw == status)
                        .map(|(_, t)| *t)
                        .ok_or_else(|| {
                            shape_err(format!("row {}: permit status {status:?} unknown", i + 1))
                        })?;
                    // The permit record itself, when named, is the asserting record.
                    push_text(
                        "dc:permit_status",
                        token.to_owned(),
                        s("PermitCase").map(str::to_owned),
                        None,
                    );
                }
                let mut push_num =
                    |attribute: &str, v: f64, unit: &str, stated_by: Option<String>| {
                        claims.push(Claim {
                            attribute_id: attribute.to_owned(),
                            value_num: Some(v),
                            unit: Some(unit.to_owned()),
                            stated_by: stated_by.unwrap_or_else(|| STATED_BY.to_owned()),
                            ..template.clone()
                        });
                    };
                if let Some(y) = n("YearBuilt") {
                    push_num("dc:year_built", y, "year", None);
                }
                if let Some(g) = n("GFA") {
                    // GFASource says which county system stated this figure.
                    let by = s("GFASource").map(|src| format!("{STATED_BY} ({src})"));
                    push_num("dc:gfa_sqft", g, "sqft", by);
                }
                if let Some(g) = n("ApprovedGFA") {
                    push_num("dc:gfa_approved_sqft", g, "sqft", None);
                }
                if let Some(g) = n("PermittedGFA") {
                    push_num("dc:gfa_permitted_sqft", g, "sqft", None);
                }
                if let Some(g) = n("REATaxedGFA") {
                    push_num("dc:gfa_taxed_sqft", g, "sqft", None);
                }
            }
            LayerKind::Campus => {
                if let Some(case) = s("CaseNumber") {
                    push_text("dc:zoning_case", case.to_owned(), None, None);
                }
                let mut push_num = |attribute: &str, v: f64, unit: &str| {
                    claims.push(Claim {
                        attribute_id: attribute.to_owned(),
                        value_num: Some(v),
                        unit: Some(unit.to_owned()),
                        ..template.clone()
                    });
                };
                if let Some(g) = n("PlannedGFA") {
                    push_num("dc:gfa_planned_sqft", g, "sqft");
                }
                // RemainingGFA is the one numeric where 0 is a real value — a
                // fully built-out campus — so it bypasses the 0-as-null rule.
                if let Some(g) = attrs["RemainingGFA"].as_f64() {
                    push_num("dc:gfa_remaining_sqft", g, "sqft");
                }
                if let Some(a) = n("GISAcreage") {
                    push_num("dc:acreage", a, "acres");
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real rows captured 2026-09-02: 5 buildings (all four statuses, Iron
    // Mountain VA-1, one zero-GFA under-construction) + 3 campuses (all three
    // statuses), geometry included.
    const SUBSET: &str = include_str!("../tests/fixtures/pwc-subset.json");
    const T0: Timestamp = Timestamp {
        unix_seconds: 1_756_800_000,
    };

    fn bodies() -> (String, String) {
        let v: serde_json::Value = serde_json::from_str(SUBSET).unwrap();
        (v["buildings"].to_string(), v["campuses"].to_string())
    }

    fn req() -> Request {
        Request::new(Date::new(2026, 9, 2))
    }

    #[test]
    fn conforms_buildings_and_campuses() {
        let (b, c) = bodies();
        let out = conform(&b, &c, &req(), T0).unwrap();
        assert_eq!(out.entities.len(), 8);
        assert_eq!(
            out.entities
                .iter()
                .filter(|e| e.level == Level::Facility)
                .count(),
            5
        );
        assert_eq!(
            out.entities
                .iter()
                .filter(|e| e.level == Level::Campus)
                .count(),
            3
        );
        assert!(
            out.entities
                .iter()
                .all(|e| e.lat.is_some() && e.lon.is_some())
        );

        let by = |eid_frag: &str, attr: &str| {
            out.claims
                .iter()
                .find(|cl| cl.entity_id.contains(eid_frag) && cl.attribute_id == attr)
                .cloned()
        };
        // Iron Mountain VA-1: the flagship record
        let im = out
            .entities
            .iter()
            .find(|e| e.name == "Iron Mountain Data Center VA-1")
            .unwrap();
        let frag = im.entity_id.strip_prefix("pwc/bld/").unwrap().to_owned();
        assert_eq!(
            by(&frag, "dc:stage").unwrap().value_text.as_deref(),
            Some("completed")
        );
        assert_eq!(
            by(&frag, "dc:occupied_date").unwrap().value_text.as_deref(),
            Some("2017-09-24")
        );
        let gfa = by(&frag, "dc:gfa_sqft").unwrap();
        assert_eq!(gfa.value_num, Some(165_230.0));
        assert!(
            gfa.stated_by.contains("Real Estate Assessments"),
            "{}",
            gfa.stated_by
        );
        assert_eq!(gfa.unit.as_deref(), Some("sqft"));
        // published_at comes from the county's LastEditDate, not the snapshot
        assert_ne!(
            by(&frag, "dc:stage").unwrap().published_at,
            Date::new(2026, 9, 2)
        );

        // the zero-GFA building has no dc:gfa_sqft claim — 0 is the county's null
        let zero = out
            .entities
            .iter()
            .find(|e| e.name.starts_with("Stack Northern Virginia"))
            .unwrap();
        let zfrag = zero.entity_id.strip_prefix("pwc/bld/").unwrap();
        assert!(by(zfrag, "dc:gfa_sqft").is_none());
        assert_eq!(
            by(zfrag, "dc:stage").unwrap().value_text.as_deref(),
            Some("under_construction")
        );

        // campuses: stage + zoning case + planned GFA
        let campus = out
            .entities
            .iter()
            .find(|e| e.level == Level::Campus)
            .unwrap();
        let cfrag = campus.entity_id.strip_prefix("pwc/campus/").unwrap();
        assert!(by(cfrag, "dc:stage").is_some());
        assert!(by(cfrag, "dc:zoning_case").is_some());
        assert!(campus.entity_id.starts_with("pwc/campus/"));
        assert!(
            !campus.entity_id.contains('{'),
            "GUID braces must be stripped"
        );
    }

    #[test]
    fn refuses_what_it_does_not_understand() {
        let (b, c) = bodies();
        let err = |bb: &str, cc: &str| conform(bb, cc, &req(), T0).unwrap_err();

        let unknown = err(&b.replacen("Under Construction", "Vibes", 1), &c);
        assert!(
            matches!(&unknown, Error::BadResponseShape { detail, .. } if detail.contains("Vibes")),
            "{unknown}"
        );

        let empty = err(r#"{"features":[]}"#, &c);
        assert!(
            matches!(&empty, Error::BadResponseShape { detail, .. } if detail.contains("zero")),
            "{empty}"
        );

        let arcgis = err(r#"{"error":{"code":400,"message":"boom"}}"#, &c);
        assert!(
            matches!(&arcgis, Error::BadResponseShape { detail, .. } if detail.contains("boom")),
            "{arcgis}"
        );

        let no_gid = err(&b.replacen("GlobalID", "GoneID", 1), &c);
        assert!(
            matches!(&no_gid, Error::BadResponseShape { detail, .. } if detail.contains("GlobalID")),
            "{no_gid}"
        );
    }
}
