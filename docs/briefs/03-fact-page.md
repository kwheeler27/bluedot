# Brief 03 — the fact page + entity registry v0

*Approved 2026-09-02. Review anchor for the task-3 PR.*

## What
`bluedot-atlas page geoId/06037 pep:POPESTIMATE 2022-07-01` compiles a **static HTML page** from `data/facts.parquet` — mockup 03 of the design direction made real: the vintage ladder with press-a-date interactivity, in the deck's design language. Alongside it, the smallest real **entity registry**: the engine starts keeping the names it already parses and previously threw away.

## Why
Closes the loop from store to screen with the signature feature first (build-and-iterate: a thin slice of the *product*, not more pipeline), and forces the next two schema truths into existence: entities need names, indicators need labels. Traces to the design brief's Stage 0 and mockup 03. **No server** — a compiled page is the product thesis in miniature, and nothing long-lived runs on the 8 GB laptop.

## Use cases
- Open the compiled LA page: the four-vintage ladder with working date buttons, every row carrying published_at · boundary_year · source URL.
- Ask for a key that isn't in the store → loud non-zero error naming *which part* of the key is unknown; no file written.
- The Connecticut pair: Hartford County (facts stop after vintage 2021) vs Capitol Planning Region.

## Solution
- **Registry v0 (Rust, ADR-0014).** Both `conform()`s now return `Conformed { facts, entities }`; `Entity { entity_id, name, level (state|county), boundary_year, vintage, source_dataset }` comes from the `NAME`/`CTYNAME`/`STNAME` columns already being read. Written as `<out>/entities/<vintage>.jsonl` beside the facts. Deliberately **not deduplicated**: ACS says "Los Angeles County, California", PEP says "Los Angeles County"; canonicalization is the real registry's job, later. `--out` now names the data root (default `data`), producing `facts/` and `entities/` under it.
- **Indicator seed (Python).** `indicators.py` — label, unit, universe, timeframe, one-line definition for the three indicators we have; commented as the embryo of the semantic layer, moving to YAML when it grows.
- **Page compiler (Python).** `page` subcommand: DuckDB pulls all vintages for the key plus a display name from `entities.parquet` (which `build-facts` now also produces, refusing duplicate registry keys); a `string.Template` page embeds the rows as JSON; vanilla JS drives the date buttons. Static file, opens from `file://`, byte-stable given the same Parquet.
- **Degrade politely, fail loudly.** Missing registry name or indicator declaration → the page compiles with the raw id and a "pending" chip (the name is decoration). Unknown entity/indicator/valid_time → exit 1 naming the unknown part and listing available valid_from values (the number is the product).
- **Out of scope:** any server, the explore-canvas spike (task 4, own brief), narrative generation, maps, name canonicalization, MOE rendering beyond the annotation.

## Acceptance
1. Re-ingest emits entities: 6 files, 19,223 registry rows (ACS 6,443 + PEP 12,780); `build-facts` writes `entities.parquet` and refuses duplicate `(entity_id, vintage, source_dataset)`.
2. The LA page renders the exact ladder (…138 / …765 / …447 / …524) with working buttons and per-row provenance; recompiling from the same Parquet is byte-identical.
3. Unknown entity, indicator, or valid_time → exit 1, named cause, no file.
4. PR references this brief; adds ADR-0014.

## Dependencies (approved 2026-09-02)
None — both languages use what's already approved.
