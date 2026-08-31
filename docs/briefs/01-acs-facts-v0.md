# Brief 01 — Fact schema v0 + ACS county ingest, two vintages

*Approved 2026-08-30. The first Stage 0 vertical slice. Review anchor for PR #1.*

## What
A `bluedot ingest acs` command that fetches one ACS 5-year indicator (`B01003_001`, total population) for every US county-equivalent in two vintages (2021 and 2023), conforms each value into the fact schema, and writes it out as JSON Lines — plus the Python step that loads it into Parquet and three DuckDB queries that show the two things this project is about.

## Why
It exercises all four key columns on real data in week one, and ACS counties contain a boundary-change artifact without waiting for NHGIS: starting with the 2022 releases, Connecticut's 8 counties (FIPS 09001–09015) were replaced by 9 planning regions (FIPS 09110–09190) as county-equivalents. Pulling vintages 2021 and 2023 puts the brief's Stage 0 benchmark — "a figure generic tools get subtly wrong" — in the first dataset. Traces to the design brief's Stage 0 ("ACS county demographics end to end") and ADR-0001, ADR-0003, ADR-0010.

Verified live on 2026-08-30 before building: vintage 2021 returns the 8 counties, vintage 2023 the 9 planning regions, no overlap; 3,221 vs 3,222 county-equivalents nationally; Los Angeles County 10,019,635 → 9,848,406. The margin of error for total population is the sentinel `-555555555` ("controlled") on ~96% of counties, so the sentinel-decoding path runs on real data.

## Use cases
- "Population of Los Angeles County — and how did that number differ between the 2021 and 2023 releases?" (the vintage axis, side by side)
- "Population time series for Hartford County, CT" → a system that says *this entity stops existing after the 2021 release and its successor geography is different*, instead of a silently truncated or mismatched series (the boundary axis)
- Every row answers "where did this number come from?" without a lookup elsewhere (provenance)

## Proposed solution

### Fact schema v0
One JSON object per row. The key is `(entity_id, indicator_id, valid_time, vintage)`, with `valid_time` stored as a half-open interval.

| column | example | note |
|---|---|---|
| `entity_id` | `geoId/09001` | Data Commons' identifier scheme — trivially joinable to a source we plan to consume (ADR-0012) |
| `indicator_id` | `acs:B01003_001` | table + variable, no `E`/`M` suffix — estimate and MOE are one fact |
| `valid_from`, `valid_to` | `2019-01-01`, `2024-01-01` | ACS 5-year is a 60-month period → half-open interval `[from, to)` (ADR-0013) |
| `vintage` | `acs5-2023` | the release |
| `published_at` | `2024-12-12` | knowledge time as a date, from a small hand-kept release table (the API doesn't say); what the "as of" toggle compares against |
| `value`, `moe` | `9848406`, `null` | nullable; MOE is the published 90% MOE |
| `value_annotation`, `moe_annotation` | `null`, `controlled` | ACS sentinel codes decoded into names — two columns because the source annotates the estimate and the MOE independently (the approved draft had one `annotation` column; changed during build, see PR). An unrecognized sentinel aborts the run (ADR-0005 in miniature) |
| `boundary_year` | `2023` | the county-equivalent definitions that release used; `geoId/09001` rows say 2021, `geoId/09110` rows say 2023 |
| `source_dataset`, `source_url`, `retrieved_at` | `acs/acs5`, request URL **without the key**, timestamp | provenance (ADR-0010) |

### Approach
Rust reads `CENSUS_API_KEY` from the environment (or `.env` in the working directory), makes one request per vintage, parses the array-of-arrays JSON, conforms rows, and writes `data/facts/acs5-{vintage}.jsonl` atomically (temp file + rename — never a partial file). The client does not follow redirects and checks the `X-DataWebAPI-KeyError` header: a keyless request 302s to an HTML page that returns HTTP 200, which is exactly the "plausible-looking success" this project exists to refuse.

Python: `uv run --project atlas bluedot-atlas build-facts` → DuckDB `read_json` → `data/facts.parquet`, then three queries: (a) LA County across both vintages; (b) Connecticut — 8 entities in 2021, 9 different entities in 2023, zero overlap; (c) annotated-row counts per vintage.

### Alternatives rejected
- **PEP (Population Estimates) as the first source** — it has true back-revisions, the textbook vintage case, but ACS is the design brief's Stage 0 spine. PEP is a natural task 3.
- **Data Commons MCP as the first source** — puts a layer between us and the vintage/geography details we're trying to expose.
- **Parquet written from Rust** — the right end state per ADR-0006, but the `arrow`/`parquet` crates are a heavy compile on 8 GB and a large API surface while the record shape is still moving. JSONL now; Parquet-from-Rust once the schema stops changing. This is a temporary deviation from ADR-0006's stated boundary.
- **CSV output** — loses null/type fidelity.

### Out of scope
Tracts, other indicators, H3, the entity registry/YAML, the semantic layer, retries/backoff, any UI, a CLI framework, a Python test suite (no Python logic worth testing yet).

## Acceptance
1. `cargo run -p bluedot -- ingest acs --indicator B01003_001 --vintage 2021 --vintage 2023` produces two JSONL files: 3,221 and 3,222 rows.
2. Every column populated on every row; `value` is null exactly when `value_annotation` is set, `moe` exactly when `moe_annotation` is set; an unknown sentinel exits non-zero naming the row.
3. Missing key, key error, redirect, or non-JSON body → one explicit error naming the cause; no output file left behind.
4. `cargo test` runs offline against recorded API responses (Connecticut, both vintages): sentinel decoding, geography parsing, serialization.
5. The three DuckDB queries return the expected shapes; the Connecticut one is the demo.
6. The PR references this brief, adds ADR-0012 and ADR-0013, and corrects the design brief's "free, no-auth" to "free, key required (as of Aug 2026)".

## Dependencies (approved 2026-08-30)
Rust: `ureq`, `serde` (derive), `serde_json`. Python: `duckdb`.
