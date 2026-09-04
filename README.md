# Blue Dot

*Named for Carl Sagan's "pale blue dot" — the world as it actually is, seen clearly and honestly.*

Blue Dot is an AI-native atlas and almanac: you ask a question about a place, and it compiles the answer — picks the public statistical data, joins the right vintages and boundaries, renders the map, and writes the narrative. Every figure it produces carries its provenance: the source, the vintage it was published in, and the geographic definition it was measured against. Underneath is a bitemporal fact store keyed on `(entity_id, indicator_id, valid_time, vintage)` and a governed semantic layer the LLM must plan against, so a question the system can't answer correctly fails loudly instead of returning a plausible number.

**Live: [bluedot-xi.vercel.app](https://bluedot-xi.vercel.app)** — the compiled site: curated fact ladders and the Data Center Atlas.

## Status

Stage 0: one vertical slice — US county demographics from the Census ACS, end to end — plus the first domain atlas (data centers, [docs/atlas/data-centers/PLAN.md](docs/atlas/data-centers/PLAN.md)). The design is in [docs/BLUEDOT_BRIEF.md](docs/BLUEDOT_BRIEF.md), the architecture decisions in [docs/DECISIONS.md](docs/DECISIONS.md), and the competitive scan in [docs/COMPETITIVE_LANDSCAPE.md](docs/COMPETITIVE_LANDSCAPE.md).

The site is static, compiled from the committed store by `bluedot-atlas site`. **Deploys are automatic**: merging to `main` runs `.github/workflows/deploy.yml`, which tests, rebuilds, and publishes to production (decision [2026-09-04](docs/decisions/2026-09-04-ci-build-and-deploy.md)). The conformed JSONL vintages are in `data/`, so a fresh clone builds the whole site with no credentials; `site/` and the parquet are derived and gitignored. The deploy gates on tests and on **map QA** (`npm run qa:map` — renders the compiled map in a headless browser and fails if marks stop painting), so a rendering regression blocks the deploy instead of publishing; screenshots are kept as a run artifact. Deploy from a laptop only during a Vercel outage.

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

`snapshots/` holds the first months of dated source pulls, kept as history: it is **superseded** by the committed store, whose `data/` vintages are the same append-only archive with nothing to fold in. The monthly [snapshot workflow](.github/workflows/snapshot.yml) now re-pulls EPA ECHO and the Prince William County layers on the 1st of each month straight into `data/` and opens a PR; merging it advances the store *and* redeploys the site, so the atlas refreshes without a local step.

Run everything from the repo root: the engine writes to `./data/` and the Python steps read from it. `uv` downloads and manages Python 3.13 itself; if you use pyenv, note that `atlas/.python-version` makes a bare `python3` inside `atlas/` complain — use `uv run`.
