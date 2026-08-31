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
cargo run -p bluedot                # Rust engine
cargo test                          # Rust tests
(cd atlas && uv run bluedot-atlas)  # Python package (uv installs Python 3.13 and syncs the venv on first run)
```
