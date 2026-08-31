# Architecture Decisions

Architecture Decision Records (ADRs) for Blue Dot. Append-only: to change a decision, add a new ADR that supersedes the old one and mark the old one `Superseded by ADR-NNNN` — don't edit history. ADR-0001 through ADR-0010 transcribe the decisions in [BLUEDOT_BRIEF.md](BLUEDOT_BRIEF.md) as of the repo's creation; ADR-0011 was made at bootstrap.

Each entry: **Context** (the forces) · **Decision** · **Consequences** (what gets easier, what gets harder) · **Revisit when** (the concrete trigger that would reopen it).

---

## ADR-0001 — Facts are keyed on `(entity_id, indicator_id, valid_time, vintage)`

**Date:** 2026-08-30 · **Status:** Accepted

**Context.** Public statistics are revised after publication: ACS re-releases, benchmark revisions, methodology changes (the World Bank's 2025 poverty-line change added 125M people to its estimates). A single time column cannot distinguish "unemployment in March 2020" from "what we believed about March 2020 unemployment as of March 2021." No consumer or analyst product exposes that second axis today (see the competitive scan); it is Blue Dot's highest-conviction differentiator.

**Decision.** Every fact is keyed on four things: `entity_id` (the place or unit), `indicator_id` (the measured thing, per the semantic layer), `valid_time` (when the measured thing happened), and `vintage` (the release we learned it from — knowledge time). The two time axes are never collapsed: no "latest value" column that gets overwritten, no upsert on `(entity, indicator, valid_time)`.

**Consequences.** "As of knowledge date" queries and vintage-to-vintage diffs become lookups rather than reconstructions, and the Stage 0 signature feature (the vintage toggle) falls straight out of the schema. In exchange, every ingestion must know its vintage, every query must pick one (or be explicit that it wants the latest), and storage grows with each vintage even when values don't change (dedupe later if it matters).

**Revisit when.** Never, for the key itself. The physical representation is ADR-0002's concern.

---

## ADR-0002 — Storage is Parquet + DuckDB (spatial extension); XTDB 2.0 is the named alternative

**Date:** 2026-08-30 · **Status:** Accepted

**Context.** Stage 0 is a single-machine, batch-loaded, read-heavy store on an 8 GB laptop. It needs columnar analytics, spatial operations, and no servers.

**Decision.** Facts live in Parquet files; DuckDB with the spatial extension is the query engine. Bitemporal semantics (ADR-0001) are enforced by the schema and by the semantic layer (ADR-0004), not by the database.

**Consequences.** Zero infrastructure, fast, and both Rust and Python can read and write it; Parquet/GeoParquet is also the interchange format for the rendering stack (ADR-0007). The cost: "as of" queries are hand-written SQL over the `vintage` column, and nothing in the engine stops a careless query from collapsing the axes — the semantic layer is the guard.

**Revisit when.** Bitemporal queries get complicated enough that hand-written SQL is error-prone — for example, `AS OF` across many indicators with different revision cadences. Then evaluate XTDB 2.0 (SQL:2011 valid time + system time on every table, Postgres wire protocol). Dolt (versioned tables, MySQL-compatible, `AS OF`) is the second candidate.

---

## ADR-0003 — Spatial spine: H3 cells as stable keys, administrative boundaries as a versioned dimension

**Date:** 2026-08-30 · **Status:** Accepted

**Context.** Administrative boundaries change — county reorganizations, redistricting, differences between TIGER/Line release years. A long time series drawn over a changing boundary lies silently. NHGIS publishes boundary files from 1790 to the present plus crosswalks between boundary versions; CARTO and Google acknowledge the problem in their docs but don't solve it as a product feature.

**Decision.** H3 cells are the stable spatial join key. Administrative units (counties, tracts, and so on) are a versioned dimension: each geometry carries its boundary year, and every spatial join must state which boundary year it uses. Crosswalks between versions come from NHGIS. Plan for S2 as a second grid (BigQuery-native) later.

**Consequences.** Boundary changes become explicit, queryable, and reportable ("this map uses 2020 county boundaries"), and H3 is supported everywhere Blue Dot data will go (DuckDB, kepler.gl, the warehouses). The costs: one more dimension to model in Stage 0 even though ACS counties barely change, and H3 only approximates polygons — choose resolution deliberately and keep exact geometries for rendering.

**Revisit when.** A source needs a grid H3 can't serve well, or crosswalk math needs area/population weighting beyond what NHGIS ships.

---

## ADR-0004 — A governed semantic layer of declarative indicator definitions, built before any AI feature

**Date:** 2026-08-30 · **Status:** Accepted

**Context.** Text-to-SQL over raw tables fails on real schemas: on Spider 2.0, frontier models solve roughly 21–70% of tasks, and more than 80% of failures are schema-linking errors. Public statistics add their own traps — universes, denominators, margins of error, unit changes across vintages. The market has converged on semantic layers as the fix, but only for private enterprise data; nobody is building one over public statistical variables.

**Decision.** Every indicator is declared in plain YAML or tables: id, source, units, universe, denominator, margin-of-error handling, allowed operations (sum, average, ratio, compare across vintages), and the geographic levels it exists at. The semantic layer compiles validated plans into fact-store queries. It is built in Stages 0–1, before any LLM-facing feature depends on it.

**Consequences.** A small, auditable ontology; an LLM action space that can be enumerated and tested. In exchange, every new indicator needs a declaration (that is the point), and "chat" demos wait until the layer exists.

**Revisit when.** The declaration format can't express a needed operation. Extend the format; never bypass it.

---

## ADR-0005 — The LLM never queries raw tables; anything the semantic layer can't validate fails loudly

**Date:** 2026-08-30 · **Status:** Accepted

**Context.** The documented failure mode of ChatGPT, Claude, and Julius over data is a confident, plausible, wrong number. A product whose value is trust cannot have that failure mode, even rarely.

**Decision.** The LLM plans only against the semantic layer (ADR-0004): declared entities, indicators, and operations. It has no raw-SQL path. A plan the layer can't validate — unknown indicator, disallowed operation, missing vintage, mismatched boundary year — returns an error, never a best-effort number.

**Consequences.** No silent wrong answers, by construction; and because every figure is a lookup against the fact store, provenance (ADR-0010) is free. The cost is coverage: some reasonable questions get "can't answer that yet." Accepted.

**Revisit when.** Never, on the principle. The error experience can and should improve.

---

## ADR-0006 — Ingestion/conformance in Rust; analysis and glue in Python

**Date:** 2026-08-30 · **Status:** Accepted

**Context.** Parsing messy public sources, normalizing them to the fact schema, and computing vintage diffs is correctness-critical and benefits from strong types. Kevin's primary language is Python; learning Rust with real stakes is an explicit goal of the project.

**Decision.** `crates/bluedot` (Rust) owns ingestion → conformance → fact-schema output. `atlas/` (Python, distribution `bluedot-atlas`) owns analysis, notebooks, orchestration glue, and anything where velocity matters more than rigor. The boundary: the Rust engine emits fact-schema Parquet; Python consumes it through DuckDB. PyO3 bindings are possible later without restructuring, which is why the engine is a library with a thin binary (ADR-0011).

**Consequences.** Type-checked conformance where mistakes are most expensive, and the learning goal is served. The costs are two toolchains and slower iteration on the Rust side while learning. Working rule: non-obvious Rust idioms get an explanatory comment, and dependencies are discussed before they're added.

**Revisit when.** A Rust component blocks progress for weeks. Move that piece to Python temporarily and keep the interface.

---

## ADR-0007 — Rendering builds on deck.gl / kepler.gl / MapLibre

**Date:** 2026-08-30 · **Status:** Accepted

**Context.** kepler.gl 3.x already does WebGL choropleths, H3 layers, time animation, Arrow/GeoParquet input, and DuckDB-in-browser. Time-scrubbing choropleths are the demo centerpiece; building a renderer would be years of work to reach parity.

**Decision.** Don't build a renderer. The rendering layer composes deck.gl + MapLibre or embeds kepler.gl, with data handed over as GeoParquet/Arrow. Nothing for rendering exists in the repo yet; that is deliberate.

**Consequences.** Best-in-class rendering for free, in exchange for living within the stack's constraints on UI.

**Revisit when.** The generated-report experience needs something the stack can't do.

---

## ADR-0008 — The agent surface is an MCP server over the governed layer, built in Stage 2

**Date:** 2026-08-30 · **Status:** Accepted

**Context.** The Census Bureau, GPO, and Google Data Commons all shipped MCP servers in 2025; raw statistics over MCP is table stakes. The defensible part is the governed, bitemporal, boundary-aware layer behind the endpoint.

**Decision.** Blue Dot will expose its semantic layer via MCP in Stage 2, after the layer exists. Nothing MCP-related is built before then. Data Commons' hosted MCP server may be consumed as a *source* in Stage 0.

**Consequences.** Agents get the same guardrails as humans. Nothing to build now.

**Revisit when.** The Stage 1 layer is stable.

---

## ADR-0009 — Blue Dot is not a knowledge graph

**Date:** 2026-08-30 · **Status:** Accepted

**Context.** Google Data Commons is a 240-billion-datapoint knowledge graph — breadth-first, Wikipedia-shaped. Competing there is futile and unnecessary.

**Decision.** Blue Dot is a bitemporal fact store plus a semantic layer — depth-first, 10-K-shaped: an audit-ready chart of accounts for public statistics. The only graph-shaped parts are a versioned entity/place registry and an indicator ontology, kept as plain tables and YAML. No triple store, no graph database. Data Commons is consumed as a source.

**Consequences.** A small surface and plain tooling, at the cost of arbitrary relationship traversal. Accepted.

**Revisit when.** A real use case needs multi-hop entity relationships beyond the place hierarchy.

---

## ADR-0010 — Every user-facing figure carries provenance

**Date:** 2026-08-30 · **Status:** Accepted

**Context.** Perplexity Finance proved that cited answers over a curated domain win; USAFacts and Our World in Data do it editorially. Nobody does it per figure, per map, with vintage and boundary year attached.

**Decision.** Every number, chart, and map shown to a user carries its source (dataset and table/variable), vintage (release), and geographic definition (level and boundary year), plus methodology notes where the semantic layer declares them (margins of error, for instance). Provenance is a property of the fact record and flows through the system; it is never re-attached at render time.

**Consequences.** Auditability, and a fact schema that carries provenance columns from day one — the first Stage 0 task must include them. The costs: denser UI, and ingestion that captures provenance rather than just values.

**Revisit when.** Never, on the principle.

---

## ADR-0011 — Repo layout: Cargo workspace at the root, `crates/bluedot` as lib + thin binary, `atlas/` as a uv project

**Date:** 2026-08-30 · **Status:** Accepted

**Context.** Two languages in one repository, a Rust learner, an 8 GB laptop.

**Decision.**
- The repo root `Cargo.toml` is a virtual workspace manifest (`[workspace]` only, `resolver = "3"`) with `crates/bluedot` as its first member. Future crates (a source adapter, a CLI, a PyO3 binding) become sibling members.
- `crates/bluedot` is one package with a library target (`lib.rs`, all logic) and a binary target (`main.rs`, a thin shell). Only library code is reachable from unit tests, `tests/`, and future Python bindings.
- `atlas/` is a uv project with `src/` layout: distribution `bluedot-atlas`, import `bluedot_atlas`, Python pinned to 3.13 via a uv-managed interpreter, `uv_build` backend.
- `Cargo.lock` and `uv.lock` are committed. Edition 2024. Zero dependencies until a task needs one, and every dependency is asked about first.

**Consequences.** One compile cache and editor auto-discovery for Rust; a testable, bindable engine; a Python side that stays independent. The cost is workspace ceremony for a single crate.

**Alternatives rejected.** A single crate at the repo root (converting to a workspace later means moving files); `rust/` and `python/` as peer directories (Cargo and rust-analyzer then need `--manifest-path` / `linkedProjects` everywhere); Python at the repo root beside `Cargo.toml` (two build systems' artifacts in one directory).

**Revisit when.** A second crate arrives (nothing to change) or a PyO3 binding is added (add a `crates/bluedot-py` member).

---

## ADR-0012 — Entity identifiers use Google Data Commons' `geoId/` scheme

**Date:** 2026-08-30 · **Status:** Accepted

**Context.** Every fact needs an `entity_id`. The first source (Census ACS) identifies places by FIPS/GEOID strings whose level is implied by length — `09` a state, `09001` a county, `09001010100` a tract — which is fragile the moment two levels share a table. Google Data Commons, which the design brief names as a source to consume, already assigns stable identifiers to exactly these places.

**Decision.** US Census geographies are identified as Data Commons does: `geoId/` + GEOID (`geoId/06037` for Los Angeles County, `geoId/09110` for Connecticut's Capitol Planning Region). Other sources will get their own prefixes when they arrive. The entity registry (ADR-0009) will map these to names, levels, parents, and boundary versions; until it exists the identifier itself is the registry.

**Consequences.** Joining Blue Dot facts to Data Commons is a string equality. A geography that changes FIPS code is a new entity — which is the correct behavior: Connecticut's 2022 planning regions are different places from its counties, and the schema shows that rather than hiding it. The cost is coupling our identifiers to a scheme we don't control.

**Alternatives rejected.** A house scheme (`us-county:09001`) — no interoperability gain for the same effort. Bare GEOIDs — level-by-length is fragile.

**Revisit when.** Data Commons changes its scheme, or a non-US source needs identifiers Data Commons doesn't cover.

---

## ADR-0013 — `valid_time` is a half-open date interval `[valid_from, valid_to)`

**Date:** 2026-08-30 · **Status:** Accepted

**Context.** ACS 5-year estimates describe a 60-month period (the 2019–2023 release describes January 2019 through December 2023), not a point in time. Other sources will have annual, quarterly, monthly, or point-in-time observations. A single `valid_time` column would either lie about periods or need a parallel "period length" column.

**Decision.** The valid-time component of the fact key is two dates, `valid_from` (inclusive) and `valid_to` (exclusive). A point-in-time observation on date D is `[D, D+1 day)`. A calendar year Y is `[Y-01-01, (Y+1)-01-01)`.

**Consequences.** Overlapping ACS windows (2017–2021 vs 2019–2023) are represented honestly, and interval overlap is a plain comparison (`a.from < b.to AND b.from < a.to`). Half-open intervals abut without gaps or double-counting and sort correctly. The cost is two columns instead of one and the need for the semantic layer to declare, per indicator, which spans are comparable.

**Alternatives rejected.** A single `valid_time` plus a period-length column — two columns anyway, worse ergonomics. Closed intervals — off-by-one bugs at every boundary. A string label like `2019-2023` — not comparable.

**Revisit when.** Never on the principle; storage types (DATE vs TIMESTAMP) may change with sources at sub-day resolution.
