//! `bluedot` binary — a thin shell over the `bluedot` library.
//!
//! ```text
//! bluedot ingest acs --indicator B01003_001 --vintage 2021 --vintage 2023 [--out data]
//! bluedot ingest pep --vintage 2022 --vintage 2023 --vintage 2024 --vintage 2025 [--out data]
//! ```
//!
//! Argument parsing is by hand: two subcommands still don't justify a CLI crate.
//! `main` delegates to `run` so errors print with `Display` (and their cause
//! chain) rather than the `Debug` dump of `fn main() -> Result`.

use std::error::Error as _; // brings `.source()` into scope without naming the trait
use std::path::{Path, PathBuf};
use std::time::Instant;

use bluedot::fact::Conformed;
use bluedot::time::Timestamp;
use bluedot::{Error, acs, config, echo, jsonl, pep, pwc};

const USAGE: &str = "usage:
  bluedot ingest acs --indicator <TABLE_VAR> --vintage <YEAR> [--vintage <YEAR>...] [--out <DIR>]
  bluedot ingest pep --vintage <YEAR> [--vintage <YEAR>...] [--out <DIR>]
  bluedot ingest echo [--naics <CODE>] [--out <DIR>]         # snapshot-dated; default NAICS 518210
  bluedot ingest pwc [--out <DIR>]                           # Prince William County VA layers; snapshot-dated

acs/pep write <DIR>/facts/<vintage>.jsonl + <DIR>/entities/<vintage>.jsonl;
echo writes <DIR>/entities/<vintage>.jsonl + <DIR>/claims/<vintage>.jsonl
(default DIR: data). Everything is fetched and conformed before any write.
`acs` needs CENSUS_API_KEY in the environment or ./.env; `pep` reads public
files and needs no key.";

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
    IngestEcho {
        naics: String,
        out_dir: PathBuf,
    },
    IngestPwc {
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
        ("ingest", s @ ("acs" | "pep" | "echo" | "pwc")) => s, // `s @ pattern` binds what the pattern matched
        _ => return Err(usage(&format!("unknown command {first:?} {second:?}"))),
    };

    let (mut indicator, mut vintages, mut out_dir) = (None, Vec::new(), PathBuf::from("data"));
    let mut naics = String::from("518210");
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        let value = it
            .next()
            .ok_or_else(|| usage(&format!("{flag} needs a value")))?;
        match flag.as_str() {
            "--indicator" if source == "acs" => indicator = Some(value.clone()),
            "--naics" if source == "echo" => naics = value.clone(),
            "--vintage" if source == "echo" || source == "pwc" => {
                return Err(usage("snapshot-dated sources take no --vintage"));
            }
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
    if source == "echo" {
        return Ok(Command::IngestEcho { naics, out_dir });
    }
    if source == "pwc" {
        return Ok(Command::IngestPwc { out_dir });
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
/// complete run. Each vintage writes its facts and its registry rows.
fn write_outputs(out_dir: &Path, outputs: &[(String, Conformed)]) -> Result<(), Error> {
    for (vintage, conformed) in outputs {
        let facts_path = out_dir.join("facts").join(format!("{vintage}.jsonl"));
        jsonl::write_atomic(&facts_path, &conformed.facts)?;
        let entities_path = out_dir.join("entities").join(format!("{vintage}.jsonl"));
        jsonl::write_atomic(&entities_path, &conformed.entities)?;
        println!(
            "wrote {} ({} facts) + {} ({} entities)",
            facts_path.display(),
            conformed.facts.len(),
            entities_path.display(),
            conformed.entities.len()
        );
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
                let conformed = client.county_data(&req)?;
                eprintln!(
                    "{}: {} facts, {} entities fetched in {:.1}s",
                    req.vintage(),
                    conformed.facts.len(),
                    conformed.entities.len(),
                    started.elapsed().as_secs_f64()
                );
                outputs.push((req.vintage(), conformed));
            }
            write_outputs(&out_dir, &outputs)
        }
        Command::IngestEcho { naics, out_dir } => {
            let req = echo::Request::new(&naics, Timestamp::now().date())?;
            let started = Instant::now();
            let conformed = echo::Client::new().facilities(&req)?;
            eprintln!(
                "{}: {} entities, {} claims fetched in {:.1}s",
                req.vintage(),
                conformed.entities.len(),
                conformed.claims.len(),
                started.elapsed().as_secs_f64()
            );
            let entities_path = out_dir
                .join("entities")
                .join(format!("{}.jsonl", req.vintage()));
            jsonl::write_atomic(&entities_path, &conformed.entities)?;
            let claims_path = out_dir
                .join("claims")
                .join(format!("{}.jsonl", req.vintage()));
            jsonl::write_atomic(&claims_path, &conformed.claims)?;
            println!(
                "wrote {} ({} entities) + {} ({} claims)",
                entities_path.display(),
                conformed.entities.len(),
                claims_path.display(),
                conformed.claims.len()
            );
            Ok(())
        }
        Command::IngestPwc { out_dir } => {
            let req = pwc::Request::new(Timestamp::now().date());
            let started = Instant::now();
            let conformed = pwc::Client::new().facilities(&req)?;
            eprintln!(
                "{}: {} entities, {} claims fetched in {:.1}s",
                req.vintage(),
                conformed.entities.len(),
                conformed.claims.len(),
                started.elapsed().as_secs_f64()
            );
            let entities_path = out_dir
                .join("entities")
                .join(format!("{}.jsonl", req.vintage()));
            jsonl::write_atomic(&entities_path, &conformed.entities)?;
            let claims_path = out_dir
                .join("claims")
                .join(format!("{}.jsonl", req.vintage()));
            jsonl::write_atomic(&claims_path, &conformed.claims)?;
            println!(
                "wrote {} ({} entities) + {} ({} claims)",
                entities_path.display(),
                conformed.entities.len(),
                claims_path.display(),
                conformed.claims.len()
            );
            Ok(())
        }
        Command::IngestPep { vintages, out_dir } => {
            let client = pep::Client::new();
            let mut outputs = Vec::with_capacity(vintages.len());
            for &year in &vintages {
                let req = pep::Request::new(year);
                let started = Instant::now();
                let conformed = client.county_data(&req)?;
                eprintln!(
                    "{}: {} facts, {} entities fetched in {:.1}s",
                    req.vintage(),
                    conformed.facts.len(),
                    conformed.entities.len(),
                    started.elapsed().as_secs_f64()
                );
                outputs.push((req.vintage(), conformed));
            }
            write_outputs(&out_dir, &outputs)
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
