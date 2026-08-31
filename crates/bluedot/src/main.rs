//! `bluedot` binary — a thin shell over the `bluedot` library.
//!
//! ```text
//! bluedot ingest acs --indicator B01003_001 --vintage 2021 --vintage 2023 [--out data/facts]
//! ```
//!
//! Argument parsing is by hand: one subcommand does not justify a CLI crate.
//! `main` delegates to `run` so errors are printed with `Display` (and their
//! cause chain) rather than the `Debug` dump you get from `fn main() -> Result`.

use std::error::Error as _; // brings `.source()` into scope without naming the trait
use std::path::PathBuf;
use std::time::Instant;

use bluedot::acs::{Client, Request};
use bluedot::{Error, config, jsonl};

const USAGE: &str = "usage: bluedot ingest acs --indicator <TABLE_VAR> --vintage <YEAR> [--vintage <YEAR>...] [--out <DIR>]
example: bluedot ingest acs --indicator B01003_001 --vintage 2021 --vintage 2023
  writes <DIR>/acs5-<YEAR>.jsonl (default DIR: data/facts); needs CENSUS_API_KEY in the env or ./.env";

enum Command {
    Usage,
    IngestAcs {
        indicator: String,
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
    if (first.as_str(), second.as_str()) != ("ingest", "acs") {
        return Err(usage(&format!("unknown command {first:?} {second:?}")));
    }

    let (mut indicator, mut vintages, mut out_dir) =
        (None, Vec::new(), PathBuf::from("data/facts"));
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        let value = it
            .next()
            .ok_or_else(|| usage(&format!("{flag} needs a value")))?;
        match flag.as_str() {
            "--indicator" => indicator = Some(value.clone()),
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
            other => return Err(usage(&format!("unknown flag {other:?}"))),
        }
    }
    let indicator = indicator.ok_or_else(|| usage("--indicator is required"))?;
    if vintages.is_empty() {
        return Err(usage("at least one --vintage is required"));
    }
    Ok(Command::IngestAcs {
        indicator,
        vintages,
        out_dir,
    })
}

fn run() -> Result<(), Error> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Command::IngestAcs {
        indicator,
        vintages,
        out_dir,
    } = parse_args(&args)?
    else {
        println!("{USAGE}");
        return Ok(());
    };

    let client = Client::new(config::census_api_key()?);

    // Fetch and conform everything first, write only if all of it succeeded:
    // a failure in the second vintage must not leave the first on disk as if
    // the run were complete.
    let mut outputs = Vec::with_capacity(vintages.len());
    for &year in &vintages {
        let req = Request::new(year, &indicator)?;
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
    for (path, facts) in &outputs {
        jsonl::write_atomic(path, facts)?;
        println!("wrote {} ({} facts)", path.display(), facts.len());
    }
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        let mut cause = err.source();
        while let Some(c) = cause {
            eprintln!("  caused by: {c}");
            cause = c.source();
        }
        std::process::exit(1);
    }
}
