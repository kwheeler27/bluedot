//! `bluedot` binary — a thin shell over the `bluedot` library.
//!
//! ```text
//! bluedot ingest acs --indicator B01003_001 --vintage 2021 --vintage 2023 [--out data/facts]
//! bluedot ingest pep --vintage 2022 --vintage 2023 --vintage 2024 --vintage 2025 [--out data/facts]
//! ```
//!
//! Argument parsing is by hand: two subcommands still don't justify a CLI crate.
//! `main` delegates to `run` so errors print with `Display` (and their cause
//! chain) rather than the `Debug` dump of `fn main() -> Result`.

use std::error::Error as _; // brings `.source()` into scope without naming the trait
use std::path::PathBuf;
use std::time::Instant;

use bluedot::fact::Fact;
use bluedot::{Error, acs, config, jsonl, pep};

const USAGE: &str = "usage:
  bluedot ingest acs --indicator <TABLE_VAR> --vintage <YEAR> [--vintage <YEAR>...] [--out <DIR>]
  bluedot ingest pep --vintage <YEAR> [--vintage <YEAR>...] [--out <DIR>]

Writes <DIR>/<vintage>.jsonl (default DIR: data/facts). All vintages are fetched
and conformed before any file is written. `acs` needs CENSUS_API_KEY in the
environment or ./.env; `pep` reads public files and needs no key.";

enum Command {
    Usage,
    IngestAcs {
        indicator: String,
        vintages: Vec<u16>,
        out_dir: PathBuf,
    },
    IngestPep {
        vintages: Vec<u16>,
        out_dir: PathBuf,
    },
}

fn parse_args(args: &[String]) -> Result<Command, Error> {
    let usage = |msg: &str| Error::Usage(format!("{msg}\n{USAGE}"));
    // `[a, b, rest @ ..]` is a slice pattern: first two elements, then the remainder.
    let [first, second, rest @ ..] = args else {
        return Ok(Command::Usage);
    };
    let source = match (first.as_str(), second.as_str()) {
        ("ingest", s @ ("acs" | "pep")) => s, // `s @ pattern` binds what the pattern matched
        _ => return Err(usage(&format!("unknown command {first:?} {second:?}"))),
    };

    let (mut indicator, mut vintages, mut out_dir) =
        (None, Vec::new(), PathBuf::from("data/facts"));
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        let value = it
            .next()
            .ok_or_else(|| usage(&format!("{flag} needs a value")))?;
        match flag.as_str() {
            "--indicator" if source == "acs" => indicator = Some(value.clone()),
            "--vintage" => {
                let year: u16 = value
                    .parse()
                    .map_err(|_| usage(&format!("--vintage {value:?} is not a year")))?;
                if vintages.contains(&year) {
                    return Err(usage(&format!("--vintage {year} given twice")));
                }
                vintages.push(year);
            }
            "--out" => out_dir = PathBuf::from(value),
            other => {
                return Err(usage(&format!(
                    "unknown flag {other:?} for `ingest {source}`"
                )));
            }
        }
    }
    if vintages.is_empty() {
        return Err(usage("at least one --vintage is required"));
    }
    match source {
        "acs" => Ok(Command::IngestAcs {
            indicator: indicator.ok_or_else(|| usage("--indicator is required"))?,
            vintages,
            out_dir,
        }),
        _ => Ok(Command::IngestPep { vintages, out_dir }),
    }
}

/// Shared tail of both ingest paths: nothing reaches this function unless every
/// vintage fetched and conformed, so a late failure can't masquerade as a
/// complete run.
fn write_outputs(outputs: &[(PathBuf, Vec<Fact>)]) -> Result<(), Error> {
    for (path, facts) in outputs {
        jsonl::write_atomic(path, facts)?;
        println!("wrote {} ({} facts)", path.display(), facts.len());
    }
    Ok(())
}

fn run(key: Option<&str>) -> Result<(), Error> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // The two arms are near-twins; a Source trait unifying them is not worth its
    // ceremony at n=2 — revisit at the third source.
    match parse_args(&args)? {
        Command::Usage => {
            println!("{USAGE}");
            Ok(())
        }
        Command::IngestAcs {
            indicator,
            vintages,
            out_dir,
        } => {
            let client = acs::Client::new(key.ok_or(Error::MissingApiKey)?.to_owned());
            let mut outputs = Vec::with_capacity(vintages.len());
            for &year in &vintages {
                let req = acs::Request::new(year, &indicator)?;
                let started = Instant::now();
                let facts = client.county_facts(&req)?;
                eprintln!(
                    "{}: {} facts fetched in {:.1}s",
                    req.vintage(),
                    facts.len(),
                    started.elapsed().as_secs_f64()
                );
                outputs.push((out_dir.join(format!("{}.jsonl", req.vintage())), facts));
            }
            write_outputs(&outputs)
        }
        Command::IngestPep { vintages, out_dir } => {
            let client = pep::Client::new();
            let mut outputs = Vec::with_capacity(vintages.len());
            for &year in &vintages {
                let req = pep::Request::new(year);
                let started = Instant::now();
                let facts = client.county_facts(&req)?;
                eprintln!(
                    "{}: {} facts fetched in {:.1}s",
                    req.vintage(),
                    facts.len(),
                    started.elapsed().as_secs_f64()
                );
                outputs.push((out_dir.join(format!("{}.jsonl", req.vintage())), facts));
            }
            write_outputs(&outputs)
        }
    }
}

fn main() {
    // The key is loaded here, not inside `run`, so error reporting can scrub it
    // from everything printed — one blanket rule, instead of trusting every
    // layer (third-party error messages included) to keep it out of a message.
    let key = config::census_api_key().ok();
    let scrub = |text: String| match &key {
        Some(k) => text.replace(k, "[REDACTED]"),
        None => text,
    };
    if let Err(err) = run(key.as_deref()) {
        eprintln!("error: {}", scrub(err.to_string()));
        let mut cause = err.source();
        while let Some(c) = cause {
            eprintln!("  caused by: {}", scrub(c.to_string()));
            cause = c.source();
        }
        std::process::exit(1);
    }
}
