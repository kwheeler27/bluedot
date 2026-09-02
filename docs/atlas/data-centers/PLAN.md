# Data Center Atlas — execution plan

*Status: awaiting Kevin's review (2026-09-02). Companion mockups: the "Data Center Atlas" artifact. Research appendices: [SOURCES-government.md](SOURCES-government.md), [SOURCES-market-open.md](SOURCES-market-open.md) (both date-stamped 2026-09-02).*

## Mission trace
Blue Dot's vision is rigorous public data about places, compiled per question, with provenance and two time axes. Data centers are the sharpest possible first *domain* atlas: the fastest-changing built infrastructure in the country, no national registry, records scattered across exactly the local/state/federal paper trail Blue Dot exists to conform — and a subject where every existing tracker is either proprietary or unsourced. Bitemporality is not a nicety here; the pipeline (announced → permitted → building → online) IS knowledge-time change.

## Brief

**What.** A public inventory and atlas of US data centers — existing and pipeline — where every attribute (location, lifecycle stage, dates, power, ownership chain, reported tenants) is a dated, sourced claim; rendered as a national explore map, per-facility dossiers, and a pipeline board whose "what changed" view is a query between two knowledge dates.

**Why.** No authoritative registry exists. The public record (air permits, planning dockets, incentive disclosures, SEC property schedules, interconnection queues) supports a real inventory — Business Insider proved it journalistically with 1,240 facilities from air permits in 2025 — but nobody maintains it as an open, provenance-first, continuously-vintaged dataset. That is precisely Blue Dot's shape.

**Use cases.**
- "Show every data center in Loudoun County, with stage and power, as of today — and as of January."
- "What's actually known about this facility, and who said it?" (the dossier: competing capacity claims deliberately unmerged)
- "What entered the pipeline in Arizona this quarter? What slipped? What was withdrawn?"
- "Which operators' announced megawatts actually materialize?" (claims history over knowledge time)

**Honesty stance (locked by research).** Location/stage/dates/owner-of-record: obtainable at scale. Power: partial, via permitted genset MW, interconnection/docket MW, and marketing claims — different quantities, never merged, each labeled. True parent ownership: often hidden behind LLCs — the dossier shows the paper trail's last link plus a separately-maintained linkage table with confidence tiers. **Per-facility compute (GPUs/FLOPS) and storage (bytes): effectively never public** — shown as "not knowable," never estimated (a clearly-labeled modeled layer is a possible later opt-in, DC-2).

## Schema extension: claims

Fact schema v0 carries numeric observations of declared indicators. Facilities need a second kind of record — the **claim**:

```
(entity_id, attribute_id, value_num | value_text, unit,
 valid_from, valid_to,          -- when the claimed thing is/was true
 stated_by,                     -- the org or document that asserts it
 published_at, source_url, source_dataset, retrieved_at,
 confidence)                    -- confirmed-by-record | reported | rumored
```

Same bitemporal spine as ADR-0001 (valid time × knowledge time), same provenance discipline as ADR-0010, extended with *who said it* and *how sure we are*. Lifecycle stages are claims on `dc:stage`; roles (owner / operator / tenant / utility) are claims whose value is another entity — the exact representation is an implementation-brief decision (candidate ADR-0015), not resolved here. Facilities, campuses, and organizations become registry entities (new levels alongside state/county — ADR-0014's registry grows exactly as designed). This stays tables-not-triples (ADR-0009).

## Source tiers (from the research appendices)

**Tier 1 — DC-0 ingests (clean access, high value):**
| Source | Yields | Note |
|---|---|---|
| EPA ECHO / ICIS-Air bulk CSV | facilities + addresses + genset permits, NAICS-filterable, national | the Business Insider method; state completeness varies |
| Official operator lists: Equinix, Digital Realty, Iron Mountain, Meta, Google, Microsoft | locations, names, some MW/PUE | manual per-company adapters; AWS/Oracle/Apple thin |
| SEC EDGAR 10-K property schedules (EQIX, DLR, IRM…) | addresses, sqft, acquisition dates | public REITs only |
| Prince William County VA interactive map | stage, acreage, sqft, zoning cases | richest single-county source; model for others |
| TX Comptroller qualified-data-center list | participant names | names only, cheap |
| Wikidata (CC0) | cross-reference keys | ~153 items globally — dedup aid, not a census |

**Tier 1 — conditional:** PeeringDB `fac` API (high value; **data-reuse license unresolved — verify before bulk ingest**); OSM `telecom=data_center` + Open Infrastructure Map exports (**ODbL: keep OSM-derived rows in a separately-licensed, attributed table — PNNL IM3 precedent — never blended under the repo license**).

**Tier 2 — DC-1:** ERCOT large-load queue (+PJM); VA SCC / Dominion filings; GA PSC large-load reports; the 11 disclosing states' incentive lists; VA DEQ data-center air-permit page (FOIA fallback); county dockets (Loudoun via PEC map; portal-by-portal elsewhere).

**Watch items:** EIA's mandatory data-center survey (pilots end Sept 2026; first real data likely 12+ months out — would be transformative); Business Insider bulk-data outreach (their permit-derived, ownership-unmasked dataset; downloadability unconfirmed); EU database + Germany's BAFA register (global phase); HIFLD (no public layer — moot).

**Excluded:** DataCenterMap, Baxtel, DC Byte, SemiAnalysis — proprietary; no ToS-violating scraping, no paid data assumed.

## Phases

**DC-0 — Seed inventory.** Rust adapters for tier-1 sources → claims + registry entities; location-and-name entity resolution v0 (proximity + normalized names, conservative — unmatched stays unmatched); national map (ties into the explore-canvas track), facility dossier page (the compiled-page machinery from task 3, extended to claims), browse/filter.
*Acceptance:* ≥1,000 US facilities with sourced location+stage; Loudoun + Prince William recall ≥90% against the county trackers; zero unsourced attributes; licensing table in the repo with per-source terms; ODbL isolation demonstrated.

**DC-1 — Pipeline & cadence.** Queues, dockets, incentive lists; **monthly snapshot cadence** — each run is a vintage, making the pipeline board's "changed since" a two-knowledge-date query; LLC→parent linkage v1 (SEC + hand-curated table with confidence tiers); operator pages.
*Acceptance:* two live snapshots diffed in the UI; reproduce, with provenance, a named figure from public reporting (BI-style) and show where we differ and why; stage history renders on every dossier.

**DC-2 — Depth & abroad.** Systematic genset-MW extraction (state portals/FOIA where ECHO is thin); utility-docket mining; EU 2024/1364 database + German BAFA register adapters; optional clearly-labeled modeled-estimates layer (MW→compute heuristics) **only on explicit opt-in**.

## Engineering shape
Per-source Rust adapters exactly like `acs`/`pep` (fetch + pure conform + fixtures + fail-loudly); the claims store lands as Parquet beside facts; Python owns linkage, page compilation, and the map data hand-off. The third source triggers the deferred `Source` trait decision. New dependencies (likely: a PDF/table extraction path for permits, geo distance calc) get asked per the house rule, in the implementation briefs.

## Risks
LLC opacity (mitigate: linkage table + confidence tiers, never guess); queue inflation — speculative/duplicate interconnection filings (mitigate: stage vocabulary distinguishes queue-entry from permit from steel); state-by-state permit fragmentation (mitigate: ECHO bulk first, portals opportunistically); license contamination (mitigate: per-source license column + ODbL isolation + CI check); scope gravity — this domain can swallow the whole project (mitigate: phases gated on Kevin's per-brief approval, demographics track continues).

## Out of scope for DC-0
Compute/storage estimation, tenant verification beyond filings, global coverage, real-time anything, water/energy-use modeling (LBNL territory — later, as *statistical* indicators where published).

## Open questions (also in the mockups, §05)
1. US-only v0 — confirm.
2. Compute/storage: "not knowable" display vs opt-in modeled layer at DC-2.
3. Tenant claims: include with confidence tiers — confirm.
4. Cadence: monthly snapshots — how they run on the 8 GB laptop (manual command vs scheduled) is a real decision.
5. Worth an email to Business Insider about bulk access before we re-derive permits state-by-state?
