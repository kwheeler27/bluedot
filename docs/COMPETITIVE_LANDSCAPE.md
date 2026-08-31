# Competitive Landscape: AI-Native Atlas + Almanac Platforms
*Research compiled August 2026 for the Blue Dot project. Verify figures before quoting externally — see Caveats.*

## TL;DR
- **No incumbent yet occupies the exact "AI-native atlas + almanac" position.** The market splits into four camps — public-data knowledge graphs (Google Data Commons, Our World in Data, USAFacts), AI-native GIS/mapping (Felt, Atlas.co, CARTO, Foursquare/kepler.gl), conversational data-analysis tools (Hex, Julius, ChatGPT/Claude, Perplexity Finance, Statista), and the semantic-layer/agent-infrastructure layer (dbt, Cube, Omni, MCP servers). The white space is a product that fuses rigorous public statistics + spatial rendering + provenance-first generative reports + bitemporality.
- **The strongest differentiators are genuinely under-served**: true bitemporality (valid time vs. knowledge/vintage time), versioned geographic boundaries, and a governed semantic layer under the LLM. Google Data Commons already ships an MCP server and is the closest analog, but it is not bitemporal, does not version boundaries, and does not produce rich, cartographic, provenance-carrying narrative reports.
- **The build-on-public-infra path is viable now**: Census ACS APIs, the hosted Data Commons MCP server, IPUMS/NHGIS historical boundary files (1790–present), H3 spatial indexing, and off-the-shelf bitemporal databases (XTDB 2.0) exist today.

## Key Findings
1. **Google Data Commons is the closest existing analog** and the competitor to watch: a 240-billion-datapoint knowledge graph with a natural-language interface, a public API, DataGemma (RIG/RAG grounding models), and a hosted MCP server (publicly released Sept 29, 2025; hosted GCP version Feb 9, 2026). Handles provenance well (links to source) but is not bitemporal, does not model changing boundaries, and does not generate rich cartographic narrative reports.
2. **AI-native GIS is a well-funded, fast-moving category** focused on enterprise operational mapping, not public statistical exploration. Felt ($15M, July 2025), Atlas.co ($2M pre-seed, Nov 2024), Wherobots ($21.5M Series A, Nov 2024), and incumbent CARTO (pivoting to "Agentic GIS" + MCP). None is positioned as a consumer/analyst almanac of demographic data.
3. **The semantic-layer thesis is validated by the market.** On Spider 2.0 (real enterprise text-to-SQL), o1-preview solves only ~21% of tasks vs. ~91% on Spider 1.0 (Lei et al., ICLR 2025); the top Spider 2.0-Lite performer reaches ~70%. Schema-linking failures account for over 80% of execution failures (arXiv 2604.25149). A governed semantic layer pushes accuracy toward 100% and fails *loudly* rather than *silently*. Omni raised $120M Series C at a $1.5B valuation (April 2026) on this thesis.
4. **Bitemporality and versioned geographies are real, largely-unfilled gaps.** Warehouse "time travel" (Snowflake/Iceberg/Delta) covers system-time only, not valid-time. True bitemporal databases (XTDB 2.0) exist but aren't wired to public statistical data. IPUMS/NHGIS provides historical boundary files and crosswalks, but no consumer/analyst product exposes "what did we believe as of date X."
5. **Agent-consumability is arriving fast.** The U.S. Census Bureau, GPO, and Google Data Commons all shipped MCP servers in 2025. Threat (agents can already query raw stats) and opportunity (none carries a governed semantic layer or bitemporal context).

## Player Profiles

### Category A — Public-data knowledge graphs & almanacs

**Google Data Commons.** Knowledge graph of 240B+ datapoints across hundreds of thousands of statistical variables (UN, WHO, CDC, Census, etc.). AI: natural-language interface; DataGemma (Gemma 2 27B, RIG + RAG grounding); MCP server at `https://api.datacommons.org/mcp`. Provenance: strong (links to source). Temporal: time series, no bitemporal/vintage modeling. Geospatial: entities but not a rendering engine. Free/public. Traction: powers ONE Campaign's "ONE Data Agent." *Most direct competitor to the "trustworthy public-stats brain" component.*

**Our World in Data (OWID).** Research/data publication with interactive Grapher charts, downloadable data, strong editorial rigor and provenance. No prominent AI/chat interface as of early 2026. Handles revisions editorially (e.g., June 2025 coverage of the World Bank raising the extreme-poverty line to $3/day, adding 125M people to estimates — exactly the vintage/revision narrative Blue Dot would automate). Nonprofit. *Gold standard for narrative + provenance that Blue Dot would generate dynamically.*

**USAFacts.** Nonpartisan civic-data org (founded 2016, Steve Ballmer); standardizes federal/state/local data into interactive reports (the 10-K-style "State of the Union in Numbers"). Uses generative AI for grounded written summaries; backend on Astronomer/Airflow (300+ DAGs). June 2026 launched "The Data We Depend On" campaign framing federal-data integrity as an "AI citation era" issue. Nonprofit. *Validates appetite for AI-generated, provenance-grounded civic-data narratives — but US-only, not spatial/bitemporal.*

**Statista.** Commercial statistics aggregator (22,500+ sources). "Research AI" (2025): RAG over Statista content using Claude and Cohere, cites up to 10 sources. Enterprise/academic subscription. Walled garden, not spatial, no bitemporal modeling. *Cautionary analog: curated + AI + citations, but closed.*

### Category B — AI-native GIS & mapping

**CARTO.** Location-intelligence incumbent (founded 2012; $61M Series C Dec 2021, Insight Partners; ~$92M total). Warehouse-native (BigQuery, Snowflake, Databricks, Redshift). 2025–2026 "Agentic GIS": plain-English AI agents, spatial MCP server exposing Workflows as deterministic, versioned, audited tools; "CARTO for Agents" (2026) into Claude/ChatGPT. Deep H3 support. Enterprise (telecom, insurance, retail, logistics, real estate). *Most direct competitor on governed-semantic-layer + agent/MCP + provenance — but enterprise-priced, warehouse-coupled, not a public-data almanac.*

**Felt.** Cloud-native "AI-native GIS" (founded 2021, Oakland; CEO Sam Hashemi). $15M round led by Energize Capital (July 2025); customers include MSCI, County of Santa Barbara. "Ask a question. Get a map." Imports ArcGIS servers and 30+ formats; free personal tier, paid Professional/Enterprise. Over half of customers in energy/climate. Partnered with Wherobots (Feb 2026). *Closest to "chat → map" UX but oriented at bring-your-own-data enterprise GIS.*

**Atlas.co.** Oslo startup (founded 2021; $2M pre-seed Nov 2024, Pale Blue Dot). "Figma of geospatial data" — browser-based collaborative no-code GIS, 30+ formats, live connections (Postgres, BigQuery, Snowflake), 50+ spatial tools, AI assistant "Navi." ~20,000 users, 140+ countries. Urban planning, real estate, renewables, journalism. *Collaboration-first prosumer; not statistical/bitemporal.*

**Foursquare Studio / kepler.gl.** Open-source WebGL geospatial viz (Uber → Linux Foundation/OpenJS, maintained by Foursquare). kepler.gl 3.x: Apache Arrow/GeoParquet (10× faster loads), DuckDB-in-browser; 3.1 added an LLM assistant (with UChicago CSDS). Excellent time-animation, hexbin/H3, large-scale rendering. *Best-in-class rendering/time-scrubbing engine to build on rather than compete with.*

**Wherobots.** Spatial lakehouse/compute engine from Apache Sedona creators (founded 2022; $21.5M Series A Nov 2024, Felicis; $5.5M seed 2023). Infrastructure for large-scale spatial processing + AI. *Potential backend, not a consumer competitor.*

**Also noted:** NV5 GeoAgent (agentic remote sensing, Feb 2026), Esri ArcGIS AI assistants, Google Earth AI / Geospatial Reasoning (Gemini over Earth Engine + Maps + BigQuery), Bunting Labs (YC, "AI-native GIS"), Amazon Bedrock geospatial patterns.

### Category C — Conversational data-analysis tools

**Hex (Magic AI).** Collaborative data notebook (SQL/Python/R) with reactive DAG compute; customers include Anthropic, Reddit, Notion, NBA. 2025 agentic AI: Notebook Agent, Magic, Semantic Model Agent; can use dbt's semantic layer. Professional data teams. *Analyst power tool; agentic-notebook + semantic-layer approach is architecturally instructive.*

**Julius AI.** Consumer/prosumer "chat with your data": upload CSV/Excel/PDF, get charts + code; Notebooks for recurring reports; free tier (15 msgs/mo), paid ~$20–45/mo. Shows generated code but no governed semantic layer; uploaded files, not curated public data. *Demand for "ask → chart" exists, with no rigor guarantees.*

**ChatGPT (Advanced Data Analysis) & Claude.** Code interpreters producing charts/analysis inline; Claude added inline HTML/SVG interactive charts (2025–2026). Documented weakness: in numerical work they produce confident-looking wrong answers rather than admitting uncertainty; time series with gaps can silently miscompute. *This silent-failure mode is precisely the gap a governed semantic layer + provenance-first design closes.*

**Perplexity Finance.** Vertical (mid-2025) synthesizing real-time stock/earnings/SEC data with cited, auditable sources; partners include SEC/EDGAR, FactSet, S&P Global, Morningstar, LSEG. Free + Pro ($20/mo). Traction: Reuters reported a $200M round at $20B valuation (Sept 2025) with ARR approaching $200M; FT (April 2026) reported ARR above $450M in March 2026 at a $23B valuation. *Best proof that "provenance-first, cited, natural-language answers over a curated data domain" wins — Blue Dot is "Perplexity Finance for demography/geography."*

**Wolfram Alpha.** The original computational answer engine over curated data; strong units/quantitative rigor; now an LLM tool/plugin. Not spatial-first, not bitemporal. *Conceptual ancestor of "the almanac that computes."*

### Category D — Semantic-layer & agent infrastructure

Consensus 2025–2026: a **governed semantic layer is foundational for AI analytics**. dbt Labs' 2026 benchmark and Omni's Spider 2.0 analysis show most text-to-SQL errors are schema/semantic failures; a modeled semantic layer pushes accuracy toward 100% and makes failures explicit. Players: dbt Semantic Layer/MetricFlow, Cube (open-source, $25M raised), Omni ($120M Series C at $1.5B, April 2026), warehouse-native layers (Snowflake Semantic Views + Cortex Analyst, Databricks Metric Views, Looker/LookML + Gemini — Google reports LookML reduces AI query errors by up to two-thirds). Agent access standardizing on MCP: Census Bureau, GPO GovInfo, Google Data Commons all shipped MCP servers in 2025. A US Digital Corps pilot found MCP raised accuracy from near 0% to 95% querying USASpending and CDC PLACES data.

## Public Data Infrastructure to Build On
- **U.S. Census / ACS APIs** — 1,700+ datasets (ACS, CPS, decennial, PEP, County Business Patterns); free, no-auth; multiple MCP wrappers exist.
- **Google Data Commons API + hosted MCP server** — unified access to 240B+ datapoints with source provenance.
- **IPUMS / NHGIS** — GIS boundary files 1790–present (states/counties since 1790, tracts since 1910, blocks since 1970), time-series tables, and **geographic crosswalks** — the crucial ingredient for versioned boundaries. NHGIS archives annual snapshots and documents revisions. NHGIS flags that boundaries from different TIGER/Line release years aren't consistently comparable — exactly the problem Blue Dot solves.
- **Eurostat, World Bank** — international coverage (World Bank's June 2025 poverty-line revision is a canonical vintage-change example).
- **H3 (Uber) / S2 (Google)** — H3 native in Databricks (28 functions) and Snowflake, across warehouses via CARTO, used by Foursquare/kepler.gl and Redshift — the de facto analytics grid; S2 is BigQuery's native index. Google's own BigQuery docs note administrative boundaries "change over time and require effort to correct." Plan for both grids; GeoParquet as interchange format.
- **Bitemporal databases** — XTDB 2.0 (GA; Postgres-wire-compatible; SQL:2011 valid-time + system-time on every table); Dolt (Git-for-data, MySQL-compatible, `AS OF` queries). XTDB's framing: any sufficiently complicated data system contains an ad-hoc, bug-ridden implementation of half of a bitemporal database.

## Gap Analysis — Blue Dot's Differentiators
1. **Bitemporality.** *Largely open.* No consumer/analyst product exposes "what did we believe as of date X." Warehouse time-travel is system-time only; OWID/USAFacts handle revisions editorially. Building blocks exist (XTDB, NHGIS snapshots, Census vintages). **Highest-conviction differentiator.**
2. **Versioned geographic boundaries.** *Open, tractable.* NHGIS provides boundaries + crosswalks; CARTO/Google acknowledge the problem but don't solve it as a product feature.
3. **Governed semantic layer under the LLM.** *Validated but contested.* Market agrees on the architecture and incumbents race here — *for private enterprise data*. A semantic layer over *public statistical variables* (units, denominators, universes, margins of error) is not being built. **Differentiator if we own the public-stats ontology.**
4. **Provenance-carrying AI reports.** *Partly served, room to lead.* Data Commons, Perplexity, Statista, USAFacts cite sources; none produces a multi-figure cartographic report where every number carries source + vintage + geographic-definition and every map documents its boundary year.
5. **Agent/MCP consumability.** *Table stakes soon.* Census, GPO, Data Commons MCP servers exist. Necessary, not sufficient — defensibility is the governed, bitemporal, boundary-aware layer behind the endpoint.

## Recommendations
**Stage 0 (0–3 months): prove the provenance + bitemporal wedge on one domain.** US county/tract demographics (ACS) + one narrative use case. Thin agent over Census ACS API (+ optionally Data Commons MCP) answering a question, rendering a choropleth (build on kepler.gl/deck.gl), writing a short narrative where every figure carries source + ACS vintage. Ship an "as of knowledge date" toggle showing how an estimate changed across vintages. *Benchmark:* reproduce, with correct provenance, a figure ChatGPT/Julius get subtly wrong.

**Stage 1 (0–12 months): governed public-stats semantic layer.** Encode units, denominators, universes, margins of error; store bitemporally (XTDB or bitemporal Postgres/DuckDB schema); wire NHGIS boundary vintages + crosswalks. *Benchmark:* semantic-layer answers fail loudly rather than return confident wrong numbers.

**Stage 2 (12–24 months): report generation + agent surface.** Generative cartographic reports with per-figure provenance; own MCP server exposing the governed, bitemporal layer. Position as "Perplexity Finance for demography/geography."

**What would change the plan:**
- Google Data Commons ships bitemporal vintages + versioned boundaries + rich report generation → pivot to a vertical (journalism, urban planning, public health) or superior UX/curation.
- Felt/CARTO adds curated public stats + provenance-first reports → compete on rigor/bitemporality and consumer accessibility.
- MCP-served raw stats + frontier models get "good enough" on rigor → lean harder into the governed/bitemporal layer and auditable provenance.

## Caveats
- Market-size, valuation, and ARR figures (Perplexity, Omni, GIS/semantic-layer market sizes) come from trade press, company self-reporting, or aggregators — indicative, not audited. Perplexity ARR is reported inconsistently ($200M Sept 2025 per Reuters vs. $450M+ March 2026 per FT).
- The AI-native geospatial category moves fast; product features cited are current as of early–mid 2026. CARTO figures come from CARTO's news feed and Tracxn; confirm before quoting.
- "No one does bitemporality/versioned boundaries as a first-class consumer feature" is based on absence of evidence in this scan; a deeper teardown (academic/government pilots, non-English/EU products) is warranted before betting the roadmap on it.
- Vendor forward-looking statements are predictions, not facts.
- Weighted toward U.S. public data and English-language sources; Eurostat-centric and other national-statistics-office AI efforts were not deeply explored.
