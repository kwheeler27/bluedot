//! Prince William County VA — Data Center Buildings & Campuses (DC-0.2).
//!
//! The county's own planning GIS publishes the richest per-facility record set
//! found anywhere in the source research: building-level status, year built,
//! occupancy date, permit cases, and FOUR different floor-area figures per
//! building (assessed / approved / permitted / taxed — different measures from
//! different county systems, kept as separate attributes, never merged), plus
//! campus polygons with rezoning cases and planned floor area, plus
//! zoning-application sites (layer 11) — the county's pre-permit pipeline,
//! the earliest public signal a data center exists. ArcGIS REST,
//! `outSR=4326`, all three layers under their 2,000-row page limit (verified
//! 2026-09-02: 247 buildings, 75 campuses, 61 sites).
//!
//! Snapshot semantics as in [`crate::echo`] (UTC vintage, one-day valid
//! intervals), with one refinement: each record's `published_at` is the
//! county's own `LastEditDate` where present — the county says when it last
//! touched the record, so we use it. Layer 11 carries no edit dates at all
//! (verified: null on every row), so site claims fall back to the snapshot
//! date.

use serde_json::Value;

use crate::Error;
use crate::claim::{Claim, Confidence, ensure_unique_claim_keys};
use crate::fact::{Entity, Geometry, Level};
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
/// Zoning application status vocabulary (layer 11, verified 2026-09-02).
/// "By-Right" means the existing zoning already allows data-center use — no
/// discretionary approval happens, but a site-plan record still exists.
const SITE_STATUSES: &[(&str, &str)] = &[
    ("Approved", "approved"),
    ("Pending", "pending"),
    ("By-Right", "by_right"),
];
/// Zoning application type vocabulary (layer 11).
const APP_TYPES: &[(&str, &str)] = &[
    ("Rezoning", "rezoning"),
    ("Special Use", "special_use"),
    ("By-Right", "by_right"),
];
/// Application workclass vocabulary (layer 11). "Proffer Amendment" = a
/// change to conditions attached to an earlier rezoning of the same land.
const WORKCLASSES: &[(&str, &str)] = &[
    ("Proffer Amendment", "proffer_amendment"),
    ("Non-Residential", "non_residential"),
    ("Mixed Use", "mixed_use"),
    ("Special Use", "special_use"),
    ("By-Right", "by_right"),
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

    pub fn sites_url(&self) -> String {
        format!("{BASE}/11/query?where=1%3D1&outFields=*&returnGeometry=true&outSR=4326&f=json")
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
        let sites = self.get_text(&req.sites_url())?;
        conform(&buildings, &campuses, &sites, req, retrieved_at)
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
    pub geometries: Vec<Geometry>,
}

/// Pure conformance over the three layers' query responses.
pub fn conform(
    buildings_body: &str,
    campuses_body: &str,
    sites_body: &str,
    req: &Request,
    retrieved_at: Timestamp,
) -> Result<Conformed, Error> {
    let mut entities = Vec::new();
    let mut claims = Vec::new();
    let mut geometries = Vec::new();
    conform_layer(
        buildings_body,
        LayerKind::Building,
        req,
        retrieved_at,
        &mut entities,
        &mut claims,
        &mut geometries,
    )?;
    conform_layer(
        campuses_body,
        LayerKind::Campus,
        req,
        retrieved_at,
        &mut entities,
        &mut claims,
        &mut geometries,
    )?;
    conform_layer(
        sites_body,
        LayerKind::Site,
        req,
        retrieved_at,
        &mut entities,
        &mut claims,
        &mut geometries,
    )?;
    ensure_unique_claim_keys(&claims)?;
    Ok(Conformed {
        entities,
        claims,
        geometries,
    })
}

#[derive(Clone, Copy, PartialEq)]
enum LayerKind {
    Building,
    Campus,
    Site,
}

fn conform_layer(
    body: &str,
    kind: LayerKind,
    req: &Request,
    retrieved_at: Timestamp,
    entities: &mut Vec<Entity>,
    claims: &mut Vec<Claim>,
    geometries: &mut Vec<Geometry>,
) -> Result<(), Error> {
    let url = match kind {
        LayerKind::Building => req.buildings_url(),
        LayerKind::Campus => req.campuses_url(),
        LayerKind::Site => req.sites_url(),
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
            LayerKind::Site => ("pwc/site", s("CaseName"), SITE_STATUSES, need("Status")?),
        };
        let name = name
            .map(str::to_owned)
            .ok_or_else(|| shape_err(format!("row {}: no name or address", i + 1)))?;
        let entity_id = format!("{prefix}/{gid}");

        // Coordinates: point layers carry geometry.x/y; polygon layers carry
        // rings — we take the mean of the outer ring's vertices as a label
        // point (an approximation for display, not survey geometry).
        let (lon, lat) = match kind {
            LayerKind::Building | LayerKind::Site => (
                row["geometry"]["x"]
                    .as_f64()
                    .ok_or_else(|| shape_err(format!("row {}: no geometry.x", i + 1)))?,
                row["geometry"]["y"]
                    .as_f64()
                    .ok_or_else(|| shape_err(format!("row {}: no geometry.y", i + 1)))?,
            ),
            LayerKind::Campus => {
                // Capture every ring, fully validated — the polygons are data
                // now (brief 09), not just raw material for a label point.
                let raw = row["geometry"]["rings"]
                    .as_array()
                    .filter(|r| !r.is_empty())
                    .ok_or_else(|| shape_err(format!("row {}: no polygon rings", i + 1)))?;
                let mut rings: Vec<Vec<[f64; 2]>> = Vec::with_capacity(raw.len());
                for ring in raw {
                    let ring = ring
                        .as_array()
                        .filter(|r| r.len() >= 3)
                        .ok_or_else(|| shape_err(format!("row {}: degenerate ring", i + 1)))?;
                    let mut out = Vec::with_capacity(ring.len());
                    for v in ring {
                        // `let ... else` (Rust 1.65+): bind or bail in one step,
                        // without an Option dance.
                        let (Some(x), Some(y)) = (v[0].as_f64(), v[1].as_f64()) else {
                            return Err(shape_err(format!("row {}: unparseable ring vertex", i + 1)));
                        };
                        if !x.is_finite() || !y.is_finite() {
                            return Err(shape_err(format!("row {}: non-finite ring vertex", i + 1)));
                        }
                        out.push([x, y]);
                    }
                    rings.push(out);
                }
                // Label point: mean of the outer ring, with a closed ring's
                // repeated last vertex dropped so it isn't double-weighted.
                let outer = &rings[0];
                let closed = outer.len() > 1 && outer.first() == outer.last();
                let outer = &outer[..outer.len() - usize::from(closed)];
                let (mut sx, mut sy) = (0.0, 0.0);
                for v in outer {
                    sx += v[0];
                    sy += v[1];
                }
                let c = (sx / outer.len() as f64, sy / outer.len() as f64);
                geometries.push(Geometry {
                    entity_id: entity_id.clone(),
                    vintage: vintage.clone(),
                    source_dataset: DATASET.to_owned(),
                    rings,
                    retrieved_at,
                });
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
            level: match kind {
                LayerKind::Building => Level::Facility,
                // A zoning site is campus-granularity land: where it sits in
                // the lifecycle is a claim (dc:zoning_status), not a level —
                // same reasoning that lets layer 10 hold "Planned" campuses.
                LayerKind::Campus | LayerKind::Site => Level::Campus,
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
        push_text("dc:recorded_name", name, None, None);

        // A helper for layer 11's three closed vocabularies beyond status.
        let in_vocab = |vocab: &'static [(&str, &str)], raw: &str, what: &'static str| {
            vocab
                .iter()
                .find(|(r, _)| *r == raw)
                .map(|(_, token)| *token)
                .ok_or_else(|| shape_err(format!("row {}: {what} {raw:?} unknown", i + 1)))
        };

        match kind {
            LayerKind::Building => {
                push_text("dc:stage", stage.to_owned(), None, None);
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
                push_text("dc:stage", stage.to_owned(), None, None);
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
            LayerKind::Site => {
                // The zoning case is the asserting record for the claims
                // that describe the application itself — the `PermitCase`
                // convention from layer 9. A case can span several parcels
                // (six cases cover 2-3 sites each, verified 2026-09-02),
                // which is why the entity key is GlobalID, not the case.
                let case = need("ZoningCaseNumber")?;
                push_text(
                    "dc:zoning_status",
                    stage.to_owned(),
                    Some(case.clone()),
                    None,
                );
                let app_type = in_vocab(APP_TYPES, &need("AppType")?, "application type")?;
                push_text(
                    "dc:application_type",
                    app_type.to_owned(),
                    Some(case.clone()),
                    None,
                );
                let workclass = in_vocab(WORKCLASSES, &need("Workclass")?, "workclass")?;
                push_text(
                    "dc:application_workclass",
                    workclass.to_owned(),
                    Some(case.clone()),
                    None,
                );
                if let Some(ms) = attrs["AppAccDate"].as_i64() {
                    let d = Date::from_unix_days(ms.div_euclid(86_400_000));
                    push_text(
                        "dc:application_accepted",
                        d.to_string(),
                        Some(case.clone()),
                        None,
                    );
                }
                push_text("dc:zoning_case", case, None, None);
                // District codes (M-2, PBD, ...) are an open set — recorded
                // verbatim, unlike the closed status vocabularies above.
                if let Some(z) = s("Zoned") {
                    push_text("dc:zoning_district", z.to_owned(), None, None);
                }
                if let Some(addr) = s("Address") {
                    push_text("dc:address", addr.to_owned(), None, None);
                }
                let mut push_num = |attribute: &str, v: f64, unit: &str| {
                    claims.push(Claim {
                        attribute_id: attribute.to_owned(),
                        value_num: Some(v),
                        unit: Some(unit.to_owned()),
                        ..template.clone()
                    });
                };
                // Site GFA is entitled/proposed floor area — the same
                // semantic as layer 10's PlannedGFA, so the same attribute.
                if let Some(g) = n("GFA") {
                    push_num("dc:gfa_planned_sqft", g, "sqft");
                }
                if let Some(a) = n("Acreage") {
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
    // statuses) + 6 zoning sites (all three statuses, every application type
    // and workclass, and two Amazon by-right sites sharing one case number),
    // geometry included.
    const SUBSET: &str = include_str!("../tests/fixtures/pwc-subset.json");
    const T0: Timestamp = Timestamp {
        unix_seconds: 1_756_800_000,
    };

    fn bodies() -> (String, String, String) {
        let v: serde_json::Value = serde_json::from_str(SUBSET).unwrap();
        (
            v["buildings"].to_string(),
            v["campuses"].to_string(),
            v["sites"].to_string(),
        )
    }

    fn req() -> Request {
        Request::new(Date::new(2026, 9, 2))
    }

    #[test]
    fn conforms_buildings_and_campuses() {
        let (b, c, s) = bodies();
        let out = conform(&b, &c, &s, &req(), T0).unwrap();
        assert_eq!(out.entities.len(), 14);
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
            9
        );
        assert_eq!(
            out.entities
                .iter()
                .filter(|e| e.entity_id.starts_with("pwc/site/"))
                .count(),
            6
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

        // a "<Null>" permit case falls back to the record gid, never the sentinel
        let va89 = out
            .entities
            .iter()
            .find(|e| e.name.contains("VA-8/9"))
            .unwrap();
        let pfrag = va89.entity_id.strip_prefix("pwc/bld/").unwrap();
        let ps = by(pfrag, "dc:permit_status").unwrap();
        assert_ne!(ps.source_record, "<Null>");
        assert_eq!(ps.source_record, pfrag);

        // a completed campus's RemainingGFA of 0 is a real claim, not an absence
        let wellington = out
            .entities
            .iter()
            .find(|e| e.name.contains("Wellington South"))
            .unwrap();
        let wfrag = wellington.entity_id.strip_prefix("pwc/campus/").unwrap();
        assert_eq!(
            by(wfrag, "dc:gfa_remaining_sqft").unwrap().value_num,
            Some(0.0)
        );

        // campus polygons are captured as data (brief 09): one geometry
        // per campus, none for point layers, rings validated
        assert_eq!(out.geometries.len(), 3);
        assert!(
            out.geometries
                .iter()
                .all(|g| g.entity_id.starts_with("pwc/campus/")
                    && !g.rings.is_empty()
                    && g.rings[0].len() >= 3
                    && g.vintage == out.entities[0].vintage)
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

        // Zoning sites: a site is a Campus-level entity; its application
        // status is dc:zoning_status (never dc:stage), asserted by the case.
        let gainesville = out
            .entities
            .iter()
            .find(|e| e.name.starts_with("Gainesville East"))
            .unwrap();
        assert_eq!(gainesville.level, Level::Campus);
        let gfrag = gainesville.entity_id.strip_prefix("pwc/site/").unwrap();
        assert!(by(gfrag, "dc:stage").is_none());
        let status = by(gfrag, "dc:zoning_status").unwrap();
        assert_eq!(status.value_text.as_deref(), Some("pending"));
        assert_eq!(status.source_record, "SUP2023-00006");
        assert_eq!(
            by(gfrag, "dc:application_type")
                .unwrap()
                .value_text
                .as_deref(),
            Some("special_use")
        );
        assert_eq!(
            by(gfrag, "dc:application_accepted")
                .unwrap()
                .value_text
                .as_deref(),
            Some("2022-11-14")
        );
        // Layer 11 has no LastEditDate — published_at falls back to snapshot.
        assert_eq!(status.published_at, Date::new(2026, 9, 2));

        // Bethlehem: entitled GFA + open-set district code recorded verbatim.
        let beth = out
            .entities
            .iter()
            .find(|e| e.name.starts_with("Bethlehem"))
            .unwrap();
        let bfrag = beth.entity_id.strip_prefix("pwc/site/").unwrap();
        let gfa = by(bfrag, "dc:gfa_planned_sqft").unwrap();
        assert_eq!(gfa.value_num, Some(719_742.0));
        assert_eq!(by(bfrag, "dc:acreage").unwrap().value_num, Some(45.46));
        assert_eq!(
            by(bfrag, "dc:zoning_district")
                .unwrap()
                .value_text
                .as_deref(),
            Some("M-2")
        );
        assert_eq!(
            by(bfrag, "dc:zoning_status").unwrap().value_text.as_deref(),
            Some("approved")
        );

        // Two Amazon by-right sites share one case number: the case is a
        // claim on each, and the entities stay distinct (GlobalID keys).
        let amazon: Vec<_> = out
            .claims
            .iter()
            .filter(|cl| {
                cl.attribute_id == "dc:zoning_case"
                    && cl.value_text.as_deref() == Some("REZ1969-0021")
            })
            .collect();
        assert_eq!(amazon.len(), 2);
        assert_ne!(amazon[0].entity_id, amazon[1].entity_id);
        let iad7 = out
            .entities
            .iter()
            .find(|e| e.name == "Amazon AWS IAD 7")
            .unwrap();
        let afrag = iad7.entity_id.strip_prefix("pwc/site/").unwrap();
        assert_eq!(
            by(afrag, "dc:zoning_status").unwrap().value_text.as_deref(),
            Some("by_right")
        );
        assert_eq!(
            by(afrag, "dc:application_workclass")
                .unwrap()
                .value_text
                .as_deref(),
            Some("by_right")
        );
    }

    #[test]
    fn refuses_what_it_does_not_understand() {
        let (b, c, s) = bodies();
        let err = |bb: &str, cc: &str, ss: &str| conform(bb, cc, ss, &req(), T0).unwrap_err();

        let unknown = err(&b.replacen("Under Construction", "Vibes", 1), &c, &s);
        assert!(
            matches!(&unknown, Error::BadResponseShape { detail, .. } if detail.contains("Vibes")),
            "{unknown}"
        );

        let empty = err(r#"{"features":[]}"#, &c, &s);
        assert!(
            matches!(&empty, Error::BadResponseShape { detail, .. } if detail.contains("zero")),
            "{empty}"
        );

        let arcgis = err(r#"{"error":{"code":400,"message":"boom"}}"#, &c, &s);
        assert!(
            matches!(&arcgis, Error::BadResponseShape { detail, .. } if detail.contains("boom")),
            "{arcgis}"
        );

        let no_gid = err(&b.replacen("GlobalID", "GoneID", 1), &c, &s);
        assert!(
            matches!(&no_gid, Error::BadResponseShape { detail, .. } if detail.contains("GlobalID")),
            "{no_gid}"
        );

        // Site-layer refusals: unknown status ("Pending" appears exactly once
        // in the fixture — Gainesville East), unknown workclass ("Mixed Use"
        // likewise — Aura), and a missing zoning case number.
        let bad_status = err(&b, &c, &s.replacen("Pending", "Vibes", 1));
        assert!(
            matches!(&bad_status, Error::BadResponseShape { detail, .. } if detail.contains("Vibes")),
            "{bad_status}"
        );

        let bad_workclass = err(&b, &c, &s.replacen("Mixed Use", "Vibes", 1));
        assert!(
            matches!(&bad_workclass, Error::BadResponseShape { detail, .. } if detail.contains("workclass")),
            "{bad_workclass}"
        );

        let no_case = err(&b, &c, &s.replacen("ZoningCaseNumber", "ZoningGone", 1));
        assert!(
            matches!(&no_case, Error::BadResponseShape { detail, .. } if detail.contains("ZoningCaseNumber")),
            "{no_case}"
        );
    }
}
