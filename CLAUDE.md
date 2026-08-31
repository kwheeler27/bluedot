# Blue Dot

Named for Carl Sagan's "pale blue dot" — the world as it actually is, seen
clearly and honestly. Plain facts about the planet, with provenance.

AI-native atlas + almanac: question-driven exploration and report generation over
public statistical data, with bitemporality, versioned geographic boundaries, and
a governed semantic layer under the LLM.

Full design context: @docs/BLUEDOT_BRIEF.md

## Non-negotiable design rules
- Fact store is keyed on (entity_id, indicator_id, valid_time, vintage). Never
  collapse the two time axes.
- Geographic boundaries are versioned. Any spatial join must be boundary-year-aware.
- The LLM never queries raw tables — only the semantic layer. Queries the semantic
  layer can't validate must fail loudly (error), never return a plausible number.
- Every user-facing figure carries provenance: source, vintage, geographic
  definition.
- We are NOT building a knowledge graph. Entity registry + indicator ontology
  live in plain tables/YAML.

## Stack
- Ingestion/conformance: Rust (I'm learning Rust — explain idioms when
  non-obvious; don't silently use advanced patterns without a comment)
- Storage/query: Parquet + DuckDB (spatial extension); H3 for spatial indexing
- Analysis/glue: Python (my primary language)
- Rendering: deck.gl / MapLibre
- Agent surface: MCP server (later stage)

## Working conventions
- I'm a senior staff data/systems engineer, intermediate in Rust — assume deep
  data modeling knowledge, don't assume Rust fluency.
- Prefer small vertical slices over broad scaffolding. Current focus: Stage 0
  (see brief) — ACS county demographics end to end.
- Ask before adding dependencies or new services.
