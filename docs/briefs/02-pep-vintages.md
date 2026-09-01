# Brief 02 — PEP county totals across four vintages (true back-revisions)

*Approved 2026-09-01 (drafted 2026-08-31). Review anchor for the task-2 PR.*

## What
A `bluedot ingest pep --vintage 2022 --vintage 2023 --vintage 2024 --vintage 2025` command that downloads each Vintage's county-totals CSV from the Population Estimates Program, conforms every year's July-1 estimate (and the April-1-2020 base) for all counties and states into fact schema v0, and writes JSONL — plus demo queries showing the same valid time answered differently by each knowledge date, and an "as of" query that picks the belief current on any given day.

## Why
Task 1 gave us the vintage axis as *overlapping windows* — ACS never revises a release. PEP is where real revisions live: every vintage restates every year back to 2020. That makes it the Stage 0 signature feature ("as of knowledge date") on real data, and the second half of the design brief's benchmark: ask a generic LLM tool for LA County's 2022 population and you get one of four numbers, uncited and un-dated. Traces to ADR-0001 and the design brief's Stage 0.

Verified live 2026-08-31 / 09-01:
- The API route is dead for this: `pep/population` exists only for vintages 2019 and 2021 (the latter without county geography); 2022+ are 404. The Vintage CSVs are the source.
- Four vintages exist: `co-est2022/2023/2024/2025-alldata.csv` — 3,195 rows each (50 states + DC + 3,144 county-equivalents; Puerto Rico is a separate file), one `POPESTIMATE<year>` column per estimated year (so 51–99 columns), Connecticut already planning regions in all four.
- Release dates, confirmed against Census press releases: 2023-03-30 · 2024-03-14 · 2025-03-13 · 2026-03-26.
- **Encoding varies by vintage**: v2022 is UTF-8, v2023–v2025 are Latin-1 (`0xF1` in "Doña Ana County"). The file-layout PDF is silent. A blind Latin-1 decode would corrupt the UTF-8 vintage's "ñ" into "Ã±" with no error — so the decoder validates UTF-8 first and falls back to Latin-1.
- LA County, July 1, 2022, by vintage: 9,721,138 → 9,719,765 → 9,748,447 → 9,748,524.

## Use cases
- "LA County population on July 1, 2022?" → four answers, each with its publication date.
- "What did we believe about that on 2024-01-01?" → 9,721,138 — Vintage 2023 wasn't published until March 2024.
- "How much did Vintage 2025 revise the 2024 estimates, and where most?" → the revision itself is queryable.

## Solution

### Fact mapping (schema v0 unchanged — it absorbs a second source as-is)
| column | PEP value | note |
|---|---|---|
| `entity_id` | `geoId/06037` (SUMLEV 050), `geoId/06` (SUMLEV 040) | states come free from the same file — the first non-county entities |
| `indicator_id` | `pep:POPESTIMATE` · `pep:ESTIMATESBASE` | July-1 resident population · April-1-2020 estimates base (also revised across vintages) |
| `valid_from`, `valid_to` | `[2022-07-01, 2022-07-02)` · `[2020-04-01, 2020-04-02)` | point-in-time per ADR-0013 |
| `vintage`, `published_at` | `pep-2024`, `2025-03-13` | hand-kept release table, press-release-verified |
| `value`, `moe` | non-negative integer, `null` | PEP publishes no MOE → `moe_annotation = not_applicable` (existing `(X)` vocabulary; no new code) |
| `boundary_year` | vintage year | |
| provenance | `pep/co-est2024-alldata`, the CSV URL, `retrieved_at` | no key involved |

### Approach
New `pep.rs` mirroring `acs.rs`: fetch (shared no-redirect agent, extracted to `http.rs`) and a pure `conform()` tested offline on byte-preserved 12-row subsets of the real v2022 (UTF-8) and v2025 (Latin-1) files. Estimate columns are discovered from the header (`POPESTIMATE…`) and must be exactly `2020..=vintage`, so Vintage 2026 will need only a release-table entry. CSV parsing via the `csv` crate (approved 2026-09-01); rows are read as `StringRecord`s with header-name lookup rather than a serde row-struct, because the columns vary by vintage — a deviation from the draft brief's "derive(Deserialize) row struct" line. Python `build-facts` needs no loader changes; four demo queries added (revision, as-of, largest revisions, and an ACS-vs-PEP non-comparability caution).

### Alternatives rejected
PEP via the API (dead for 2022+). Data Commons as the PEP source (carries only the latest vintage — the thing we need is precisely what it discards). Hard-coded year columns (breaks every release). A single assumed encoding (silently corrupts one vintage or the other).

### Out of scope
Puerto Rico file; components of change (births/deaths/migration — later indicators); the 2010–2019 vintage series; the semantic layer; retries.

## Acceptance
1. Four files: v2022 = 3,195×4 = 12,780 · v2023 = 15,975 · v2024 = 19,170 · v2025 = 22,365 — 70,290 PEP facts, 76,733 with ACS; fact key unique across both sources (existing duplicate check).
2. Same invariants as task 1; unknown SUMLEV, missing column, wrong year set, ragged row, negative or non-integer count, undecodable body → error, no output.
3. Offline tests cover both encodings via the real fixtures, header-driven year discovery, and every refusal path.
4. Demo query (d) returns the four LA values above; (e) for 2024-01-01 returns 9,721,138.
5. The PR references this brief; no new ADR (the schema absorbed a second source unchanged — worth saying out loud).

## Dependencies (approved 2026-09-01)
Rust: `csv`. Python: none.
