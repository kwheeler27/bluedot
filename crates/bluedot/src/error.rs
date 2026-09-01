//! The crate's single error type.
//!
//! Rust has no exceptions: fallible functions return `Result<T, E>`, and `?`
//! returns early with the `E`. We use one `enum` for the whole crate so every
//! failure carries the context a person needs to act on it (which URL, which
//! row, which code) — the "fail loudly, never plausibly" rule of ADR-0005.
//!
//! There are crates (`thiserror`, `anyhow`) that generate most of this; the first
//! time through it is worth seeing what they generate.

use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub enum Error {
    /// `CENSUS_API_KEY` is neither in the environment nor in `./.env`.
    MissingApiKey,
    /// The request itself failed (DNS, TLS, timeout, ...). `source` is ureq's error.
    Http { url: String, source: ureq::Error },
    /// The API answered with its key-rejection redirect (`X-DataWebAPI-KeyError`).
    ApiKeyRejected { url: String },
    /// Any status other than 200. `body_head` is the first bytes of the body, for diagnosis.
    UnexpectedStatus {
        url: String,
        status: u16,
        body_head: String,
    },
    /// A 200 whose body isn't the JSON we expect.
    NotJson {
        url: String,
        source: serde_json::Error,
        body_head: String,
    },
    /// Valid JSON, wrong shape (missing header row, missing column, ragged row).
    BadResponseShape { url: String, detail: String },
    /// A negative "annotation" number we don't have in our table. We refuse to guess.
    UnknownSentinel {
        code: i64,
        field: &'static str,
        geoid: String,
        variable: String,
    },
    /// A known annotation code appeared on a field it isn't defined for (e.g. a MOE-only code on an estimate).
    SentinelOnWrongField {
        code: i64,
        field: &'static str,
        geoid: String,
        variable: String,
    },
    /// Neither a number nor a known annotation.
    BadNumber {
        text: String,
        field: &'static str,
        geoid: String,
        variable: String,
    },
    /// Valid CSV framing could not be maintained (ragged rows, ...). `source` is the csv crate's error.
    Csv { url: String, source: csv::Error },
    /// We don't have release metadata (publication date) for this vintage of this dataset.
    UnsupportedVintage { dataset: &'static str, year: u16 },
    /// File-system failure, with the path involved.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// Bad command-line usage.
    Usage(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::MissingApiKey => write!(
                f,
                "no Census API key: set CENSUS_API_KEY in the environment or in ./.env \
                 (free key: https://api.census.gov/data/key_signup.html)"
            ),
            Error::Http { url, .. } => write!(f, "request to {url} failed"),
            Error::ApiKeyRejected { url } => write!(
                f,
                "the Census API rejected the key for {url} (X-DataWebAPI-KeyError); \
                 is the key activated? (check the activation link in the signup email)"
            ),
            Error::UnexpectedStatus {
                url,
                status,
                body_head,
            } => {
                write!(
                    f,
                    "{url} returned HTTP {status}; body starts: {body_head:?}"
                )
            }
            Error::NotJson { url, body_head, .. } => {
                write!(
                    f,
                    "{url} returned HTTP 200 but not JSON; body starts: {body_head:?}"
                )
            }
            Error::BadResponseShape { url, detail } => {
                write!(f, "{url}: unexpected response shape: {detail}")
            }
            Error::UnknownSentinel {
                code,
                field,
                geoid,
                variable,
            } => write!(
                f,
                "unknown annotation code {code} in {field} of {variable} for GEOID {geoid}; \
                 add it to acs::SENTINELS only after checking the Census annotation table"
            ),
            Error::SentinelOnWrongField {
                code,
                field,
                geoid,
                variable,
            } => write!(
                f,
                "annotation code {code} appeared in {field} of {variable} for GEOID {geoid}, \
                 but the Census table does not define it for that field"
            ),
            Error::BadNumber {
                text,
                field,
                geoid,
                variable,
            } => {
                write!(
                    f,
                    "{field} of {variable} for GEOID {geoid} is not a number: {text:?}"
                )
            }
            Error::Csv { url, .. } => write!(f, "malformed CSV from {url}"),
            Error::UnsupportedVintage { dataset, year } => write!(
                f,
                "no release metadata for {dataset} vintage {year}; add its publication date \
                 to that source's RELEASES table"
            ),
            Error::Io { path, .. } => write!(f, "I/O error at {}", path.display()),
            Error::Usage(msg) => write!(f, "{msg}"),
        }
    }
}

// `std::error::Error` is the trait that lets errors nest: `source()` returns the
// lower-level error that caused this one, so `main` can print the whole chain.
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Http { source, .. } => Some(source),
            Error::NotJson { source, .. } => Some(source),
            Error::Csv { source, .. } => Some(source),
            Error::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}
