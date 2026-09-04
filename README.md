# Blue Dot

*Named for Carl Sagan's "pale blue dot" — the world as it actually is, seen clearly and honestly.*

Blue Dot is an AI-native atlas and almanac: you ask a question about a place, and it compiles the answer — picks the public statistical data, joins the right vintages and boundaries, renders the map, and writes the narrative. Every figure it produces carries its provenance: the source, the vintage it was published in, and the geographic definition it was measured against. Underneath is a bitemporal fact store keyed on `(entity_id, indicator_id, valid_time, vintage)` and a governed semantic layer the LLM must plan against, so a question the system can't answer correctly fails loudly instead of returning a plausible number.

**Live: [bluedot-xi.vercel.app](https://bluedot-xi.vercel.app)** — the compiled site: curated fact ladders and the Data Center Atlas.

## Status

Stage 0: one vertical slice — US county demographics from the Census ACS, end to end — plus the first domain atlas (data centers, [docs/atlas/data-centers/PLAN.md](docs/atlas/data-centers/PLAN.md)). The design is in [docs/BLUEDOT_BRIEF.md](docs/BLUEDOT_BRIEF.md), the architecture decisions in [docs/DECISIONS.md](docs/DECISIONS.md), and the competitive scan in [docs/COMPETITIVE_LANDSCAPE.md](docs/COMPETITIVE_LANDSCAPE.md).

The site is static, compiled from the fact store by `bluedot-atlas site` and deployed with the Vercel CLI from `site/`: once per machine `npx vercel@59.7.0 link --yes --project bluedot` (the link lands in `site/.vercel/`, which is gitignored along with the rest of `site/`), then `npx vercel@59.7.0 deploy --prod --yes` per release. The generator is the source of truth; automating build+deploy in CI is a queued follow-up (brief 08).

## Layout

| Path | What |
| --- | --- |
| `crates/bluedot/` | Rust ingestion/conformance engine — parses sources into the fact schema. A Cargo workspace rooted at `Cargo.toml`. |
| `atlas/` | Python analysis/glue package `bluedot-atlas` (import `bluedot_atlas`), managed with [uv](https://docs.astral.sh/uv/). |
| `docs/` | Brief, decision records, competitive landscape. |

Rendering (deck.gl / MapLibre) and the MCP server come in later stages; nothing for them exists yet.

## Run

```sh
# one-time: a free Census API key (required since May 2026) — https://api.census.gov/data/key_signup.html
echo 'CENSUS_API_KEY=your-key' > .env          # .env is gitignored

cargo run -p bluedot -- ingest acs --indicator B01003_001 --vintage 2021 --vintage 2023
cargo run -p bluedot -- ingest pep --vintage 2022 --vintage 2023 --vintage 2024 --vintage 2025
#   → data/facts/<vintage>.jsonl (fact schema v0) + data/entities/<vintage>.jsonl (registry v0)
#     pep files are public — no key needed for that one

cargo run -p bluedot -- ingest echo
#   → data/entities/echo-<today>.jsonl + data/claims/echo-<today>.jsonl — EPA air-permitted
#     data centers (Data Center Atlas DC-0); snapshot-dated, no key needed

uv run --project atlas bluedot-atlas build-facts
#   → data/facts.parquet + data/entities.parquet, then the demo queries

uv run --project atlas bluedot-atlas page geoId/06037 pep:POPESTIMATE 2022-07-01
#   → data/pages/geoId-06037.pep-POPESTIMATE.2022-07-01.html — a compiled fact page
#     (the vintage ladder; open it in a browser straight from the file)

cargo test && cargo clippy --all-targets -- -D warnings   # offline; fixtures under crates/bluedot/tests/fixtures
```

## Snapshots

`snapshots/` is the committed archive of dated source pulls (entities + claims JSON Lines) for the [Data Center Atlas](docs/atlas/data-centers/PLAN.md). A GitHub Actions workflow ([snapshot.yml](.github/workflows/snapshot.yml)) re-pulls EPA ECHO and the Prince William County layers on the 1st of each month and opens a PR with the new vintages — merging it is the archival act. To fold archived snapshots into your local store: `cp -n snapshots/entities/* data/entities/ && cp -n snapshots/claims/* data/claims/ && cp -n snapshots/geometry/* data/geometry/`, then re-run `build-facts` (`-n` skips files you already have; if a same-vintage file slips in twice, the duplicate-key check refuses loudly — that's the guard working).

Run everything from the repo root: the engine writes to `./data/` and the Python steps read from it. `uv` downloads and manages Python 3.13 itself; if you use pyenv, note that `atlas/.python-version` makes a bare `python3` inside `atlas/` complain — use `uv run`.
