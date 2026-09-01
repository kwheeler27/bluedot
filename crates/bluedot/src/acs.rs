//! Census ACS 5-year API: fetch one variable for every county-equivalent in a
//! vintage and conform the response into `Fact`s.
//!
//! The API returns JSON as an array of arrays — a header row of column names,
//! then one row per geography — with every cell a string (or `null`):
//!
//! ```text
//! [["NAME","B01003_001E","B01003_001M","state","county"],
//!  ["Fairfield County, Connecticut","956446","-555555555","09","001"], ...]
//! ```
//!
//! Fetching (network) and conforming (pure) are separate functions so the
//! conformance logic is tested offline against recorded responses.

use crate::Error;
use crate::fact::{Annotation, Fact};
use crate::http;
use crate::time::{Date, Timestamp};

pub const DATASET: &str = "acs/acs5";
const BASE_URL: &str = "https://api.census.gov/data";

/// Publication dates of ACS 5-year releases, by endpoint year. Hand-maintained
/// from Census Bureau press releases (verified 2026-08-30); the API does not
/// report them. This is the `published_at` — knowledge time — of every fact.
pub const RELEASES: &[(u16, Date)] = &[
    (2021, Date::new(2022, 12, 8)),
    (2022, Date::new(2023, 12, 7)),
    (2023, Date::new(2024, 12, 12)),
    (2024, Date::new(2026, 1, 29)),
];

/// Which field an annotation code is defined for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Estimate,
    Moe,
    Either,
}

/// The Census Bureau's annotation codes ("Notes on ACS Estimate and Annotation
/// Values", verified 2026-08-30). Anything else negative and nine digits long is
/// an error, not a value — we do not guess.
const SENTINELS: &[(i64, Annotation, Field)] = &[
    (
        -666_666_666,
        Annotation::InsufficientSample,
        Field::Estimate,
    ),
    (-999_999_999, Annotation::NotDisplayable, Field::Either),
    (-888_888_888, Annotation::NotApplicable, Field::Either),
    (-222_222_222, Annotation::MoeInsufficientSample, Field::Moe),
    (-333_333_333, Annotation::MoeOpenEndedMedian, Field::Moe),
    (-555_555_555, Annotation::Controlled, Field::Moe),
];

/// Everything about one request that ends up in the facts it produces.
#[derive(Debug, Clone)]
pub struct Request {
    pub vintage_year: u16,
    /// Table + variable without the E/M suffix, e.g. `B01003_001`.
    pub variable: String,
}

impl Request {
    pub fn new(vintage_year: u16, variable: &str) -> Result<Self, Error> {
        let ok = !variable.is_empty()
            && variable
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
            && !variable.ends_with('E')
            && !variable.ends_with('M');
        if !ok {
            return Err(Error::Usage(format!(
                "indicator {variable:?} should look like B01003_001 (no E/M suffix)"
            )));
        }
        Ok(Request {
            vintage_year,
            variable: variable.to_owned(),
        })
    }

    pub fn published_at(&self) -> Result<Date, Error> {
        RELEASES
            .iter()
            .find(|(year, _)| *year == self.vintage_year)
            .map(|(_, date)| *date)
            .ok_or(Error::UnsupportedVintage {
                dataset: DATASET,
                year: self.vintage_year,
            })
    }

    /// The request URL **without** the key — this is what goes into provenance.
    pub fn url(&self) -> String {
        format!(
            "{BASE_URL}/{}/acs/acs5?get=NAME,{v}E,{v}M&for=county:*",
            self.vintage_year,
            v = self.variable
        )
    }

    pub fn vintage(&self) -> String {
        format!("acs5-{}", self.vintage_year)
    }

    pub fn indicator_id(&self) -> String {
        format!("acs:{}", self.variable)
    }

    /// ACS 5-year covers the five calendar years ending in the vintage year:
    /// `[Jan 1 of year-4, Jan 1 of year+1)`.
    pub fn valid_interval(&self) -> (Date, Date) {
        let y = i32::from(self.vintage_year);
        (Date::new(y - 4, 1, 1), Date::new(y + 1, 1, 1))
    }
}

/// HTTP client configured to *not* be helpful: no redirects, no status→error
/// conversion. We look at what the API actually sent and decide ourselves.
pub struct Client {
    agent: ureq::Agent,
    key: String,
}

impl Client {
    pub fn new(key: String) -> Self {
        // The shared agent follows no redirects: a keyless/bad-key request here is
        // answered with a 302 to an HTML page that returns 200, and following it
        // would turn an error into a plausible-looking success (ADR-0005).
        Client {
            agent: http::agent(),
            key,
        }
    }

    /// Fetch and conform: every county-equivalent's value for `req`.
    pub fn county_facts(&self, req: &Request) -> Result<Vec<Fact>, Error> {
        let retrieved_at = Timestamp::now();
        let body = self.fetch(&req.url())?;
        conform(&body, req, retrieved_at)
    }

    /// GET `url` (key appended here, never stored) and return the body text.
    fn fetch(&self, url: &str) -> Result<String, Error> {
        let keyed = format!("{url}&key={}", self.key);
        let mut resp = self
            .agent
            .get(&keyed)
            .call()
            .map_err(|source| Error::Http {
                url: url.to_owned(),
                source,
            })?;

        if resp.headers().contains_key("X-DataWebAPI-KeyError") {
            return Err(Error::ApiKeyRejected {
                url: url.to_owned(),
            });
        }
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
        // Some APIs echo the request (query string included) in error bodies.
        // Scrub the key before the body can reach an error message or a log.
        let body = body.replace(&self.key, "[REDACTED]");
        if status != 200 {
            return Err(Error::UnexpectedStatus {
                url: url.to_owned(),
                status,
                body_head: head(&body),
            });
        }
        Ok(body)
    }
}

/// Turn an API response body into facts. Pure: no network, no clock.
pub fn conform(body: &str, req: &Request, retrieved_at: Timestamp) -> Result<Vec<Fact>, Error> {
    let url = req.url();
    // Every cell is a JSON string or null — hence `Option<String>`.
    let rows: Vec<Vec<Option<String>>> =
        serde_json::from_str(body).map_err(|source| Error::NotJson {
            url: url.clone(),
            source,
            body_head: head(body),
        })?;
    let shape_err = |detail: String| Error::BadResponseShape {
        url: url.clone(),
        detail,
    };

    let (header, data) = rows
        .split_first()
        .ok_or_else(|| shape_err("empty response".into()))?;
    let column = |name: &str| -> Result<usize, Error> {
        header
            .iter()
            .position(|h| h.as_deref() == Some(name))
            .ok_or_else(|| shape_err(format!("no column {name:?} in header {header:?}")))
    };
    let (estimate_col, moe_col) = (
        column(&format!("{}E", req.variable))?,
        column(&format!("{}M", req.variable))?,
    );
    let (state_col, county_col) = (column("state")?, column("county")?);

    let published_at = req.published_at()?;
    let (valid_from, valid_to) = req.valid_interval();

    let mut facts = Vec::with_capacity(data.len());
    for (i, row) in data.iter().enumerate() {
        if row.len() != header.len() {
            return Err(shape_err(format!(
                "row {} has {} cells, header has {}",
                i + 1,
                row.len(),
                header.len()
            )));
        }
        let cell = |col: usize| row[col].as_deref();
        let (Some(state), Some(county)) = (cell(state_col), cell(county_col)) else {
            return Err(shape_err(format!(
                "row {} has a null state/county code",
                i + 1
            )));
        };
        let geoid = format!("{state}{county}");

        let (value, value_annotation) =
            parse_measure(cell(estimate_col), Field::Estimate, "estimate", &geoid, req)?;
        let (moe, moe_annotation) = parse_measure(cell(moe_col), Field::Moe, "moe", &geoid, req)?;

        facts.push(Fact {
            entity_id: format!("geoId/{geoid}"),
            indicator_id: req.indicator_id(),
            valid_from,
            valid_to,
            vintage: req.vintage(),
            published_at,
            value,
            moe,
            value_annotation,
            moe_annotation,
            // ACS geography vintage = the endpoint year (ACS Geography Handbook ch. 2).
            boundary_year: req.vintage_year,
            source_dataset: DATASET.to_owned(),
            source_url: url.clone(),
            retrieved_at,
        });
    }
    Ok(facts)
}

/// One cell → either a number or an annotation, never both, never neither.
fn parse_measure(
    text: Option<&str>,
    field: Field,
    field_name: &'static str,
    geoid: &str,
    req: &Request,
) -> Result<(Option<f64>, Option<Annotation>), Error> {
    let Some(text) = text else {
        return Ok((None, Some(Annotation::Missing)));
    };
    // Annotation codes are integers; check those before accepting any number.
    if let Ok(code) = text.parse::<i64>()
        && let Some((_, annotation, defined_for)) = SENTINELS.iter().find(|(c, _, _)| *c == code)
    {
        return if *defined_for == Field::Either || *defined_for == field {
            Ok((None, Some(*annotation)))
        } else {
            Err(Error::SentinelOnWrongField {
                code,
                field: field_name,
                geoid: geoid.to_owned(),
                variable: req.variable.clone(),
            })
        };
    }
    match text.parse::<f64>() {
        // Rust's float parser accepts "NaN", "inf" and "Infinity", and serde_json
        // would then write them as JSON null — a null value with no annotation,
        // the silent failure ADR-0005 forbids. They are not numbers to us.
        Ok(v) if !v.is_finite() => Err(Error::BadNumber {
            text: text.to_owned(),
            field: field_name,
            geoid: geoid.to_owned(),
            variable: req.variable.clone(),
        }),
        // Nine-digit negative numbers are the sentinel pattern; an unlisted one is
        // more likely a code we haven't catalogued than a real value.
        Ok(v) if v <= -100_000_000.0 => Err(Error::UnknownSentinel {
            code: v as i64,
            field: field_name,
            geoid: geoid.to_owned(),
            variable: req.variable.clone(),
        }),
        Ok(v) => Ok((Some(v), None)),
        Err(_) => Err(Error::BadNumber {
            text: text.to_owned(),
            field: field_name,
            geoid: geoid.to_owned(),
            variable: req.variable.clone(),
        }),
    }
}

/// First bytes of a body, for error messages (never the whole thing).
fn head(body: &str) -> String {
    body.chars().take(120).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // `include_str!` bakes the fixture into the test binary at compile time; the
    // path is relative to this source file.
    const CT_2021: &str = include_str!("../tests/fixtures/acs5-2021-ct.json");
    const CT_2023: &str = include_str!("../tests/fixtures/acs5-2023-ct.json");
    const T0: Timestamp = Timestamp {
        unix_seconds: 1_756_612_800,
    }; // 2025-08-31T04:00:00Z

    fn req(year: u16) -> Request {
        Request::new(year, "B01003_001").unwrap()
    }

    #[test]
    fn vintage_2021_has_eight_connecticut_counties() {
        let facts = conform(CT_2021, &req(2021), T0).unwrap();
        let ids: Vec<&str> = facts.iter().map(|f| f.entity_id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "geoId/09001",
                "geoId/09003",
                "geoId/09005",
                "geoId/09007",
                "geoId/09009",
                "geoId/09011",
                "geoId/09013",
                "geoId/09015"
            ]
        );
        let fairfield = &facts[0];
        assert_eq!(fairfield.value, Some(956_446.0));
        assert_eq!(
            (fairfield.moe, fairfield.moe_annotation),
            (None, Some(Annotation::Controlled))
        );
        assert_eq!(fairfield.value_annotation, None);
        assert_eq!(fairfield.vintage, "acs5-2021");
        assert_eq!(fairfield.published_at, Date::new(2022, 12, 8));
        assert_eq!(
            (fairfield.valid_from, fairfield.valid_to),
            (Date::new(2017, 1, 1), Date::new(2022, 1, 1))
        );
        assert_eq!(fairfield.boundary_year, 2021);
        assert_eq!(fairfield.indicator_id, "acs:B01003_001");
        assert!(
            fairfield
                .source_url
                .ends_with("/2021/acs/acs5?get=NAME,B01003_001E,B01003_001M&for=county:*")
        );
        assert!(!fairfield.source_url.contains("key="));
    }

    #[test]
    fn vintage_2023_has_nine_planning_regions_and_no_legacy_counties() {
        let facts = conform(CT_2023, &req(2023), T0).unwrap();
        assert_eq!(facts.len(), 9);
        assert!(
            facts
                .iter()
                .all(|f| f.entity_id.as_str() >= "geoId/09110" && f.boundary_year == 2023)
        );
        let legacy = conform(CT_2021, &req(2021), T0).unwrap();
        assert!(
            legacy
                .iter()
                .all(|old| !facts.iter().any(|new| new.entity_id == old.entity_id))
        );
    }

    #[test]
    fn serializes_to_the_documented_column_vocabulary() {
        let facts = conform(CT_2023, &req(2023), T0).unwrap();
        let json = serde_json::to_string(&facts[0]).unwrap();
        assert!(json.starts_with(r#"{"entity_id":"geoId/09110","indicator_id":"acs:B01003_001","valid_from":"2019-01-01","valid_to":"2024-01-01","vintage":"acs5-2023","published_at":"2024-12-12","value":969029.0,"moe":null,"value_annotation":null,"moe_annotation":"controlled","boundary_year":2023,"#), "{json}");
    }

    fn body(estimate: &str, moe: &str) -> String {
        format!(
            r#"[["NAME","B01003_001E","B01003_001M","state","county"],["X",{estimate},{moe},"06","037"]]"#
        )
    }

    #[test]
    fn decodes_every_known_sentinel_on_its_field() {
        let cases = [
            (
                r#""-666666666""#,
                r#""-222222222""#,
                Some(Annotation::InsufficientSample),
                Some(Annotation::MoeInsufficientSample),
            ),
            (
                r#""-999999999""#,
                r#""-999999999""#,
                Some(Annotation::NotDisplayable),
                Some(Annotation::NotDisplayable),
            ),
            (
                r#""-888888888""#,
                r#""-888888888""#,
                Some(Annotation::NotApplicable),
                Some(Annotation::NotApplicable),
            ),
            (
                r#""42.5""#,
                r#""-333333333""#,
                None,
                Some(Annotation::MoeOpenEndedMedian),
            ),
            (
                "null",
                "null",
                Some(Annotation::Missing),
                Some(Annotation::Missing),
            ),
        ];
        for (e, m, want_e, want_m) in cases {
            let f = &conform(&body(e, m), &req(2023), T0).unwrap()[0];
            assert_eq!(
                (f.value_annotation, f.moe_annotation),
                (want_e, want_m),
                "estimate={e} moe={m}"
            );
            assert_eq!(f.value.is_none(), want_e.is_some());
            assert_eq!(f.moe.is_none(), want_m.is_some());
        }
    }

    #[test]
    fn refuses_what_it_does_not_understand() {
        let unknown = conform(&body(r#""-444444444""#, r#""1""#), &req(2023), T0).unwrap_err();
        assert!(
            matches!(
                unknown,
                Error::UnknownSentinel {
                    code: -444_444_444,
                    field: "estimate",
                    ..
                }
            ),
            "{unknown}"
        );

        let wrong_field = conform(&body(r#""-555555555""#, r#""1""#), &req(2023), T0).unwrap_err();
        assert!(
            matches!(
                wrong_field,
                Error::SentinelOnWrongField {
                    code: -555_555_555,
                    ..
                }
            ),
            "{wrong_field}"
        );

        let text = conform(&body(r#""n/a""#, r#""1""#), &req(2023), T0).unwrap_err();
        assert!(matches!(text, Error::BadNumber { .. }), "{text}");
        for not_a_number in ["NaN", "nan", "inf", "-inf", "Infinity"] {
            let err = conform(
                &body(&format!("{not_a_number:?}"), r#""1""#),
                &req(2023),
                T0,
            )
            .unwrap_err();
            assert!(
                matches!(err, Error::BadNumber { .. }),
                "{not_a_number}: {err}"
            );
        }

        let no_column = conform(
            r#"[["NAME","state","county"],["X","06","037"]]"#,
            &req(2023),
            T0,
        )
        .unwrap_err();
        assert!(
            matches!(no_column, Error::BadResponseShape { .. }),
            "{no_column}"
        );

        let html = conform("<html>Missing Key</html>", &req(2023), T0).unwrap_err();
        assert!(matches!(html, Error::NotJson { .. }), "{html}");

        assert!(matches!(
            conform(CT_2023, &req(2019), T0).unwrap_err(),
            Error::UnsupportedVintage { year: 2019, .. }
        ));
        assert!(matches!(
            Request::new(2023, "B01003_001E").unwrap_err(),
            Error::Usage(_)
        ));
    }
}
