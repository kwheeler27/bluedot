# Brief 06 — cross-source entity resolution v0 (DC-0.3)

*Built 2026-09-02 under the approved plan's DC-0 ("location-and-name entity resolution v0 — conservative, unmatched stays unmatched"). Zero new dependencies (stdlib math + duckdb already present); unit tests use stdlib unittest.*

## What
`bluedot-atlas link` links Prince William County buildings (`pwc/bld/*`) to ECHO air-permit facilities (`frs/*`) that describe the same physical site, writing each link as a **claim**: `dc:same_as`, confidence `inferred`, the method and evidence (distance, shared name tokens) attributed in `stated_by`. Re-running `build-facts` folds links into `claims.parquet`, and the dossier view (demo query k) shows one facility's claims from both sources side by side.

## Why
Two sources now describe many of the same buildings from different records — the county's planning GIS and EPA's permit system. Resolution is what turns parallel record sets into *the* inventory, and it must obey the house rules: our own inference is a claim like any other (attributed, tiered `inferred` — a new Confidence variant), never a silent merge. Entities are never rewritten; the link is data you can query, audit, or ignore.

## Rules (v0, deliberately conservative)
- Candidates: ECHO facilities with a `dc:state = VA` claim, geocoded.
- Match iff **≤100 m apart**, or **≤300 m apart with a distinctive shared name token** (stop-list strips LLC/DATA/CENTER/etc.; bare numbers ignored).
- **Ambiguity refuses**: a building with more than one qualifying candidate gets no link and is reported. Unmatched stays unmatched.
- One direction (county building → FRS id): the richer local record points at the national registry key. `source_record` = the target id, so multiple links from one entity can't collide.

## Acceptance
1. Live run over the real store reports links / ambiguous / totals; every link claim carries evidence in `stated_by`; zero forced matches.
2. 7 stdlib-unittest tests on the pure functions (token stripping, haversine scale, decision thresholds).
3. Query (k) renders a real two-source dossier.

## Known limits (v0, stated)
FRS geocodes can be street-address centroids (a campus's buildings may all sit near one point — the ambiguity refusal handles this by *not guessing*); name tokens can't distinguish same-operator neighbors (distance rule bears that load); links are snapshot-dated and regenerated, not curated.
