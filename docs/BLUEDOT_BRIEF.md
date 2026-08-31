# Blue Dot — Design Brief

**Blue Dot**: an AI-native atlas + almanac. Named for Carl Sagan's "pale blue dot" —
the world as it actually is, seen clearly and honestly. Down-to-earth, futuristic,
provenance-first.
Namespace: repo `bluedot`; `bluedot` free on crates.io and npm (verified Aug 2026);
PyPI `bluedot` is taken — use `bluedot-atlas` there. Note: an unrelated company
named Bluedot exists; check domain/trademark before commercial launch.

## Vision
A modern, AI-driven fusion of the reference atlas and the almanac: rigorous public
statistical data (demographic, geographic, economic) that is spatially rendered,
continuously updated, temporally deep, and explorable through natural language.
The atlas page is *generated per question*, not browsed: the system picks the data,
joins the vintages, renders the map, and writes the narrative — with every number
traceable to source + vintage + geographic definition.

Positioning shorthand: **"Perplexity Finance for demography and geography."**
AI-native means the question drives compilation — not chat bolted onto dashboards.

## Trajectory
Personal learning project → real product. Also a vehicle for learning Rust.

## Core differentiators (validated by competitive scan, Aug 2026)
1. **Bitemporality** — two time axes: valid time (when it happened) and knowledge/
   vintage time (when we learned it). Supports "what did we believe about 2020
   unemployment as of March 2021." No consumer/analyst product does this today.
2. **Versioned geographic boundaries** — boundaries are a slowly-changing dimension
   (redistricting, county changes). Versioned geometries + crosswalks so long time
   series don't silently lie at boundary changes. IPUMS/NHGIS provides raw material.
3. **Governed semantic layer under the LLM** — the LLM plans against declared
   indicators/entities/operations, never raw tables. Every figure in a generated
   report is a lookup against the compiled fact store. Fail loudly, never silently.
4. **Provenance-carrying generated reports** — source + vintage + methodology +
   boundary-year on every figure and map.
5. **Agent consumability (MCP)** — expose the governed, bitemporal layer as an MCP
   server so agents consume it too. The moat is the layer behind the endpoint, not
   the endpoint.

## Explicit non-goals
- NOT building a public-data knowledge graph (that's Google Data Commons' moat;
  consume it as a source instead). We build a bitemporal fact store + semantic
  layer, with only a thin graph-shaped layer: a versioned place/entity registry
  and an indicator ontology (YAML/tables, not a triple store).
- NOT enterprise bring-your-own-data GIS (Felt/Atlas.co/CARTO lane).
- Analogy: Data Commons is Wikipedia (breadth-first graph); this is a 10-K
  (depth-first, audit-ready chart of accounts for public statistics).

## Architecture decisions
- **Fact store**: keyed on (entity_id, indicator_id, valid_time, vintage).
  Start: Parquet + DuckDB (spatial extension). Consider XTDB 2.0 (SQL:2011
  bitemporal) when bitemporal queries get serious.
- **Spatial spine**: H3 cells as stable join keys; administrative boundaries as a
  versioned mapping on top (NHGIS boundary files 1790–present + crosswalks).
- **Semantic layer**: declarative indicator definitions — units, universe,
  denominator, margins of error, allowed operations. Built before AI features.
- **Ingestion/conformance engine**: **Rust** (learning goal + real stakes) —
  parse messy sources, normalize to fact schema, compute vintage diffs.
  Python where velocity matters.
- **Rendering**: deck.gl / kepler.gl / MapLibre — build on, don't reinvent.
  Time-scrubbing choropleths are the demo centerpiece.
- **Agent surface**: MCP server exposing the governed layer.

## Staged plan
- **Stage 0 (0–3 mo)**: One vertical slice — US county/tract demographics (Census
  ACS). Thin agent over ACS API (+ optionally Data Commons hosted MCP), renders a
  choropleth, writes a short narrative, every figure carries source + ACS vintage.
  Signature feature: "as of knowledge date" toggle showing how an estimate changed
  across vintages. Benchmark: reproduce with correct provenance a figure that
  generic LLM tools get subtly wrong (revision or boundary-change artifact).
- **Stage 1 (0–12 mo)**: Governed public-stats semantic layer + bitemporal store +
  NHGIS boundary vintages/crosswalks wired in. Benchmark: wrong queries fail
  loudly, never return confident wrong numbers.
- **Stage 2 (12–24 mo)**: Generative cartographic reports (map + time series +
  narrative + per-figure provenance) and own MCP server.

## Key data sources
Census ACS APIs (free; API key required as of Aug 2026 — keyless requests redirect to an HTML page); Google Data Commons API + hosted MCP server
(api.datacommons.org/mcp); IPUMS/NHGIS (boundary files + crosswalks + snapshot
archives); Eurostat, World Bank (later, international).

## Competitive landscape (summary — full report in docs/)
Four camps, none occupying this position:
- Public-data knowledge graphs: Google Data Commons (closest analog; watch for
  them adding vintages/boundaries/reports), OWID (gold-standard curation, pre-AI),
  USAFacts, Statista.
- AI-native GIS (enterprise BYO-data): Felt, Atlas.co, CARTO (agentic + MCP,
  warehouse-native), Wherobots/Sedona (infra), kepler.gl (open-source rendering —
  a component for us, not a competitor).
- Conversational analysis: Hex, Julius, ChatGPT/Claude data analysis (silent-
  wrong-number failure mode is exactly what our semantic layer prevents),
  Perplexity Finance (proof the provenance-first pattern wins), Wolfram Alpha.
- Semantic-layer infra: dbt/MetricFlow, Cube, Omni — validates the architecture,
  but all aimed at private enterprise data, not public statistics.

## Risks / what changes the plan
- Data Commons ships bitemporal vintages + versioned boundaries + rich reports →
  pivot to a vertical (journalism, urban planning, public health) or superior UX.
- Felt/CARTO adds curated public stats + provenance reports → compete on rigor,
  bitemporality, consumer accessibility.
- MCP-served raw stats + frontier models get "good enough" → lean harder into the
  governed/bitemporal layer and auditable provenance as the trust differentiator.
