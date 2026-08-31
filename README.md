# Blue Dot

*Named for Carl Sagan's "pale blue dot" — the world as it actually is, seen clearly and honestly.*

Blue Dot is an AI-native atlas and almanac: you ask a question about a place, and it compiles the answer — picks the public statistical data, joins the right vintages and boundaries, renders the map, and writes the narrative. Every figure it produces carries its provenance: the source, the vintage it was published in, and the geographic definition it was measured against. Underneath is a bitemporal fact store keyed on `(entity_id, indicator_id, valid_time, vintage)` and a governed semantic layer the LLM must plan against, so a question the system can't answer correctly fails loudly instead of returning a plausible number.

## Status

Stage 0: one vertical slice — US county demographics from the Census ACS, end to end. The design is in [docs/BLUEDOT_BRIEF.md](docs/BLUEDOT_BRIEF.md), the architecture decisions in [docs/DECISIONS.md](docs/DECISIONS.md), and the competitive scan in [docs/COMPETITIVE_LANDSCAPE.md](docs/COMPETITIVE_LANDSCAPE.md).

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
#   → data/facts/acs5-2021.jsonl, data/facts/acs5-2023.jsonl (fact schema v0, one JSON object per line)

uv run --project atlas bluedot-atlas build-facts
#   → data/facts.parquet, then prints the demo queries (vintage axis, Connecticut boundary change, annotations)

cargo test && cargo clippy --all-targets -- -D warnings   # Rust tests + lint (offline; fixtures under crates/bluedot/tests/fixtures)
```

Run everything from the repo root: the engine writes to `./data/` and the Python step reads from it. `uv` downloads and manages Python 3.13 itself; if you use pyenv, note that `atlas/.python-version` makes a bare `python3` inside `atlas/` complain — use `uv run`.
