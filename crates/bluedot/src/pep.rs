//! Census Population Estimates Program (PEP): vintage county-totals files.
//!
//! Unlike ACS (an API), PEP vintages 2022+ ship only as static CSV files — the
//! API carries just vintages 2019 and 2021, the latter without county geography
//! (verified 2026-08-31). Each vintage restates the July 1 estimate for every
//! year back to 2020, so the same `(entity, indicator, valid_time)` carries a
//! different value per vintage: true revisions, the second time axis in action.
//!
//! File shape: a header row (`SUMLEV,...,STATE,COUNTY,...,ESTIMATESBASE2020,
//! POPESTIMATE2020,...`) and one row per state (SUMLEV 040) or county-equivalent
//! (SUMLEV 050). The estimate columns grow by one per vintage, so they are
//! discovered from the header, never hard-coded. Encoding varies by vintage —
//! see [`decode`].
//!
//! Fetching (network) and conforming (pure) are separate, as in [`crate::acs`].

use crate::Error;
use crate::fact::{Annotation, Fact};
use crate::http;
use crate::time::{Date, Timestamp};

/// Publication dates of PEP vintage county totals, from Census Bureau press
/// releases (verified 2026-08-31). Hand-maintained: the files don't say.
pub const RELEASES: &[(u16, Date)] = &[
    (2022, Date::new(2023, 3, 30)),
    (2023, Date::new(2024, 3, 14)),
    (2024, Date::new(2025, 3, 13)),
    (2025, Date::new(2026, 3, 26)),
];

#[derive(Debug, Clone)]
pub struct Request {
    pub vintage_year: u16,
}

impl Request {
    pub fn new(vintage_year: u16) -> Self {
        Request { vintage_year }
    }

    pub fn published_at(&self) -> Result<Date, Error> {
        RELEASES
            .iter()
            .find(|(year, _)| *year == self.vintage_year)
            .map(|(_, date)| *date)
            .ok_or(Error::UnsupportedVintage {
                dataset: "pep county totals",
                year: self.vintage_year,
            })
    }

    pub fn url(&self) -> String {
        format!(
            "https://www2.census.gov/programs-surveys/popest/datasets/2020-{y}/counties/totals/co-est{y}-alldata.csv",
            y = self.vintage_year
        )
    }

    pub fn vintage(&self) -> String {
        format!("pep-{}", self.vintage_year)
    }

    pub fn source_dataset(&self) -> String {
        format!("pep/co-est{}-alldata", self.vintage_year)
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

    /// Fetch and conform one vintage. No API key involved: these are public files.
    pub fn county_facts(&self, req: &Request) -> Result<Vec<Fact>, Error> {
        let retrieved_at = Timestamp::now();
        let url = req.url();
        let mut resp = self.agent.get(&url).call().map_err(|source| Error::Http {
            url: url.clone(),
            source,
        })?;
        let status = resp.status().as_u16();
        let bytes = resp
            .body_mut()
            .with_config()
            .limit(http::BODY_LIMIT_BYTES)
            .read_to_vec()
            .map_err(|source| Error::Http {
                url: url.clone(),
                source,
            })?;
        let text = decode(bytes);
        if status != 200 {
            return Err(Error::UnexpectedStatus {
                url,
                status,
                body_head: text.chars().take(120).collect(),
            });
        }
        conform(&text, req, retrieved_at)
    }
}

// Clippy flags a `new()` with no arguments unless `Default` exists too; the
// idiomatic response is to provide it, delegating to `new`.
impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

/// Decode a vintage file. The files are inconsistent: some vintages are UTF-8
/// (2022), others Latin-1 (2024, 2025) — established from the bytes on
/// 2026-09-01; the file-layout PDF is silent on encoding. UTF-8 is tried first
/// because its multi-byte structure makes validity a strong signal; on failure,
/// Latin-1, where every byte is its own code point (`b as char` — true of
/// Latin-1 by design, not of encodings generally). Decoding Latin-1 blindly
/// would corrupt a UTF-8 "ñ" into "Ã±" with no error anywhere.
///
/// (`String::from_utf8` takes the `Vec` by value; the error hands the bytes
/// back via `into_bytes`, so neither path copies the happy case.)
pub fn decode(bytes: Vec<u8>) -> String {
    match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(not_utf8) => not_utf8.into_bytes().iter().map(|&b| b as char).collect(),
    }
}

/// Turn a decoded vintage file into facts. Pure: no network, no clock.
pub fn conform(text: &str, req: &Request, retrieved_at: Timestamp) -> Result<Vec<Fact>, Error> {
    let url = req.url();
    let csv_err = |source: csv::Error| Error::Csv {
        url: url.clone(),
        source,
    };
    let shape_err = |detail: String| Error::BadResponseShape {
        url: url.clone(),
        detail,
    };

    let mut reader = csv::ReaderBuilder::new().from_reader(text.as_bytes());
    let headers = reader.headers().map_err(csv_err)?.clone();
    let column = |name: &str| -> Result<usize, Error> {
        headers
            .iter()
            .position(|h| h == name)
            .ok_or_else(|| shape_err(format!("no column {name:?} in header")))
    };
    let (sumlev_col, state_col, county_col) =
        (column("SUMLEV")?, column("STATE")?, column("COUNTY")?);
    let base_col = column("ESTIMATESBASE2020")?;

    // Estimate columns discovered from the header, so a new vintage needs only a
    // RELEASES entry — but the discovered set must be exactly 2020..=vintage
    // year, or this is not the file we think it is.
    let mut year_cols: Vec<(u16, usize)> = headers
        .iter()
        .enumerate()
        .filter_map(|(i, h)| Some((h.strip_prefix("POPESTIMATE")?.parse::<u16>().ok()?, i)))
        .collect();
    year_cols.sort_unstable();
    let found: Vec<u16> = year_cols.iter().map(|(year, _)| *year).collect();
    let expected: Vec<u16> = (2020..=req.vintage_year).collect();
    if found != expected {
        return Err(shape_err(format!(
            "POPESTIMATE years {found:?}, expected {expected:?}"
        )));
    }

    let published_at = req.published_at()?;
    let (vintage, source_dataset) = (req.vintage(), req.source_dataset());
    // PEP publishes no margins of error, for any row — so `moe` is always null,
    // with the existing "(X) not applicable" annotation saying why.
    let fact =
        |entity_id: &str, indicator: &str, valid_from: Date, valid_to: Date, value: f64| Fact {
            entity_id: entity_id.to_owned(),
            indicator_id: indicator.to_owned(),
            valid_from,
            valid_to,
            vintage: vintage.clone(),
            published_at,
            value: Some(value),
            moe: None,
            value_annotation: None,
            moe_annotation: Some(Annotation::NotApplicable),
            boundary_year: req.vintage_year,
            source_dataset: source_dataset.clone(),
            source_url: url.clone(),
            retrieved_at,
        };

    let mut facts = Vec::new();
    for (i, record) in reader.records().enumerate() {
        let record = record.map_err(csv_err)?; // ragged rows are a csv error already
        let cell = |col: usize| record.get(col).unwrap_or_default();
        let entity_id = match cell(sumlev_col) {
            "040" => format!("geoId/{}", cell(state_col)),
            "050" => format!("geoId/{}{}", cell(state_col), cell(county_col)),
            other => {
                return Err(shape_err(format!(
                    "row {}: unexpected SUMLEV {other:?}",
                    i + 2
                )));
            }
        };
        // `name` flows into an Error that outlives this loop, so the closure must
        // promise 'static — satisfied because every call site passes a literal.
        let count = |col: usize, name: &'static str| parse_count(cell(col), name, &entity_id, req);

        // The estimates base is a point-in-time value for April 1, 2020 (census day)...
        facts.push(fact(
            &entity_id,
            "pep:ESTIMATESBASE",
            Date::new(2020, 4, 1),
            Date::new(2020, 4, 2),
            count(base_col, "ESTIMATESBASE2020")?,
        ));
        // ...and each POPESTIMATE<year> is the July 1 resident population (ADR-0013:
        // a point in time is the one-day half-open interval).
        for &(year, col) in &year_cols {
            let y = i32::from(year);
            facts.push(fact(
                &entity_id,
                "pep:POPESTIMATE",
                Date::new(y, 7, 1),
                Date::new(y, 7, 2),
                count(col, "POPESTIMATE")?,
            ));
        }
    }
    Ok(facts)
}

/// A population count: a non-negative integer, nothing else. PEP has no
/// sentinel codes, so anything that isn't such an integer is a malformed file,
/// not a value.
fn parse_count(
    text: &str,
    field: &'static str,
    entity_id: &str,
    req: &Request,
) -> Result<f64, Error> {
    match text.trim().parse::<i64>() {
        Ok(v) if v >= 0 => Ok(v as f64),
        _ => Err(Error::BadNumber {
            text: text.to_owned(),
            field,
            geoid: entity_id.trim_start_matches("geoId/").to_owned(),
            variable: format!("co-est{}-alldata", req.vintage_year),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `include_bytes!` (not `include_str!`): the v2025 fixture is Latin-1 and
    // would fail compile-time UTF-8 validation as a str. Each fixture is a
    // 12-row subset of the real file — one state, LA County, all 9 Connecticut
    // planning regions, and Doña Ana County NM — bytes preserved.
    const V2022: &[u8] = include_bytes!("../tests/fixtures/pep-v2022-subset.csv"); // a UTF-8 vintage
    const V2025: &[u8] = include_bytes!("../tests/fixtures/pep-v2025-subset.csv"); // a Latin-1 vintage
    const T0: Timestamp = Timestamp {
        unix_seconds: 1_756_712_000,
    };

    fn facts_for(bytes: &[u8], year: u16) -> Vec<Fact> {
        conform(&decode(bytes.to_vec()), &Request::new(year), T0).unwrap()
    }

    fn find<'a>(facts: &'a [Fact], entity: &str, indicator: &str, valid_from: Date) -> &'a Fact {
        facts
            .iter()
            .find(|f| {
                f.entity_id == entity && f.indicator_id == indicator && f.valid_from == valid_from
            })
            .unwrap_or_else(|| panic!("no {indicator} for {entity} at {valid_from}"))
    }

    #[test]
    fn v2022_shape_and_values() {
        let facts = facts_for(V2022, 2022);
        assert_eq!(facts.len(), 12 * 4); // 12 entities × (base + 2020..=2022)
        let la = find(
            &facts,
            "geoId/06037",
            "pep:POPESTIMATE",
            Date::new(2022, 7, 1),
        );
        assert_eq!(la.value, Some(9_721_138.0));
        assert_eq!(la.valid_to, Date::new(2022, 7, 2));
        assert_eq!(la.vintage, "pep-2022");
        assert_eq!(la.published_at, Date::new(2023, 3, 30));
        assert_eq!(
            (la.moe, la.moe_annotation, la.value_annotation),
            (None, Some(Annotation::NotApplicable), None)
        );
        assert_eq!(la.boundary_year, 2022);
        assert_eq!(la.source_dataset, "pep/co-est2022-alldata");
        // the SUMLEV 040 row becomes a state entity from the same file
        let ca = find(
            &facts,
            "geoId/06",
            "pep:ESTIMATESBASE",
            Date::new(2020, 4, 1),
        );
        assert!(ca.value.unwrap() > 39_000_000.0);
        // Connecticut is planning regions in every vintage of this file series
        assert!(facts.iter().any(|f| f.entity_id == "geoId/09110"));
        assert!(!facts.iter().any(|f| f.entity_id == "geoId/09001"));
    }

    #[test]
    fn revision_is_visible_across_vintages() {
        let (v22, v25) = (facts_for(V2022, 2022), facts_for(V2025, 2025));
        let july_2022 = Date::new(2022, 7, 1);
        let before = find(&v22, "geoId/06037", "pep:POPESTIMATE", july_2022);
        let after = find(&v25, "geoId/06037", "pep:POPESTIMATE", july_2022);
        // same entity, same indicator, same valid time — different vintage, different value
        assert_eq!(before.value, Some(9_721_138.0));
        assert_eq!(after.value, Some(9_748_524.0));
        assert_ne!(before.vintage, after.vintage);
        assert_eq!(v25.len(), 12 * 7); // base + 2020..=2025 from the header, not from code
    }

    #[test]
    fn both_encodings_reach_dona_ana() {
        // No runtime assert that V2025 is invalid UTF-8: rustc's `invalid_from_utf8`
        // lint already proves it statically (and flags the call as always-err).
        for (bytes, year) in [(V2022, 2022u16), (V2025, 2025)] {
            assert!(
                facts_for(bytes, year)
                    .iter()
                    .any(|f| f.entity_id == "geoId/35013"),
                "Doña Ana County missing in v{year}"
            );
        }
        assert_eq!(decode(vec![0x44, 0x6f, 0xf1, 0x61]), "Doña"); // Latin-1 path
        assert_eq!(decode("Doña".as_bytes().to_vec()), "Doña"); // UTF-8 path untouched
    }

    #[test]
    fn refuses_what_it_does_not_understand() {
        let hdr =
            "SUMLEV,STATE,COUNTY,ESTIMATESBASE2020,POPESTIMATE2020,POPESTIMATE2021,POPESTIMATE2022";
        let err = |csv: String, year| conform(&csv, &Request::new(year), T0).unwrap_err();

        let sumlev = err(format!("{hdr}\n160,06,037,10,11,12,13\n"), 2022);
        assert!(matches!(sumlev, Error::BadResponseShape { .. }), "{sumlev}");

        let negative = err(format!("{hdr}\n050,06,037,10,11,-12,13\n"), 2022);
        assert!(matches!(negative, Error::BadNumber { .. }), "{negative}");

        let word = err(format!("{hdr}\n050,06,037,10,11,twelve,13\n"), 2022);
        assert!(matches!(word, Error::BadNumber { .. }), "{word}");

        // header stops at 2022 but the request says vintage 2023: wrong file
        let years = err(format!("{hdr}\n050,06,037,10,11,12,13\n"), 2023);
        assert!(matches!(years, Error::BadResponseShape { .. }), "{years}");

        let ragged = err(format!("{hdr}\n050,06,037,10\n"), 2022);
        assert!(matches!(ragged, Error::Csv { .. }), "{ragged}");

        let missing = err("A,B\n1,2\n".to_owned(), 2022);
        assert!(
            matches!(missing, Error::BadResponseShape { .. }),
            "{missing}"
        );

        assert!(matches!(
            Request::new(2019).published_at().unwrap_err(),
            Error::UnsupportedVintage { year: 2019, .. }
        ));
    }
}
