# Brief 04 — ECHO air-permit adapter + claims schema v0 (DC-0.1)

*Built 2026-09-02 under the approved Data Center Atlas plan (PR #5) and Kevin's "Let's go"; no new dependencies, so this brief rides in the PR as its review anchor rather than blocking ahead of it.*

## What
`bluedot ingest echo` takes a snapshot of EPA ECHO/ICIS-Air facilities under NAICS 518210 — the air-permit near-census of US data centers — conforming each permit record into facility entities (FRS-keyed, geocoded) and bitemporal claims (`dc:stage`, `dc:state`, `dc:recorded_name` — every record's stated name is preserved even when records of one facility disagree). Lands the **claims schema v0** (ADR-0015) that the whole domain atlas builds on.

## Why
Plan DC-0's first ingest, chosen because it is the highest value-for-effort source (SOURCES-government.md #1) and forces the claims schema into existence on real data. Step-0 probing (2026-09-02, live) settled the design: the REST route (`get_facilities` → `get_qid`) returns all 468 rows as JSON in one page, fully geocoded, with a six-value `AIRStatus` lifecycle vocabulary — so no bulk-ZIP handling and **zero new dependencies**. VA leads with 134 facilities (60 in Loudoun); 13 FRS ids carry multiple permit records (with disagreeing stated names — preserved as claims); the row set includes junk ("1 AAA WINDSOR TEST SITE", permanently closed) — kept, correctly staged, never silently filtered.

## Solution
- **Claims v0** (`claim.rs`, ADR-0015): key = (entity, attribute, valid interval, vintage, **source_record**) — the per-record key member is what lets one facility's multiple permit records coexist; exactly one of value_text/value_num; `stated_by` + three-tier `confidence`; uniqueness refused in-engine like facts.
- **Snapshot semantics**: vintage = `echo-<retrieval date>`; each claim's valid time is that one day — ECHO says nothing about when a status changed, so we assert only "this is what the record said on this date". Repeated snapshots become the DC-1 pipeline history.
- **Vocabulary discipline**: `dc:stage` values are the source's own vocabulary normalized (`operating`, `planned_facility`, `under_construction`, `permanently_closed`, `temporarily_closed`, `no_operating_status_in_icis`); an unknown AIRStatus fails the batch. Mapping to the atlas lifecycle is the semantic layer's future job, not the adapter's.
- **Entities**: `frs/<RegistryID>`, level `facility`, name as stated (LLC shells and all), lat/lon (Entity grows optional coordinates; admin areas stay null until geometry lands). One entity per FRS id; every record keeps its own claims.
- **`dc:state` companion claim** keeps by-state queries possible before the spatial join exists.
- **Python**: `build-facts` builds `claims.parquet` (when claims exist) with its own duplicate-key refusal + two DC demo queries (stage counts; top states with pipeline split).

## Acceptance
1. Live snapshot: 468 permit records → 455 entities, 1,404 claims (three per record); all entities geocoded; zero unsourced values; an empty result set is a loud error, never a success.
2. Offline tests on a 9-row real fixture (includes a twice-recorded FRS id with disagreeing names, and all six statuses): conformance shape, vocabulary refusal, missing-coordinate refusal, ECHO-error-inside-200 refusal.
3. `build-facts` produces claims.parquet; queries (h)/(i) show VA on top with an honest pipeline split.
4. PR references this brief; adds ADR-0015.

## Out of scope (per plan)
Other sources, entity resolution across sources, LLC unmasking, the map, MW extraction, cadence automation.
