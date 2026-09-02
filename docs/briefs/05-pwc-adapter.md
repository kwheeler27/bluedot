# Brief 05 — Prince William County adapter (DC-0.2)

*Built 2026-09-02 under the approved plan and the DC-0 cadence; zero new dependencies (county ArcGIS REST returns JSON with `outSR=4326`), so the brief rides in the PR.*

## What
`bluedot ingest pwc` snapshots the county's own Data Center **Buildings** (layer 9, 247 points) and **Campuses** (layer 10, 75 polygons) from the Build-Out Analysis MapServer behind the county's public interactive map — the richest per-facility record set in the source research.

## Why
SOURCES-government.md ranked it #2 for value-for-effort, and it delivers what ECHO can't: building-level lifecycle (`Completed / Under Construction / Pending / Planned`), **year built and occupancy date** (the "when did it go online" of the original ask), permit and rezoning case numbers, and **four distinct floor-area figures per building** — assessed, approved, permitted, and taxed GFA, from different county systems. Those are different measures, so they land as four attributes, each stating its county source — the dossier's competing-claims view on real data.

## Solution notes
- Entities: `pwc/bld/<GlobalID>` (level `facility`) and `pwc/campus/<GlobalID>` (new level `campus`), GUID braces stripped; names as recorded (falling back to address), geocoded — campus label points are the mean of the outer polygon ring (an approximation for display, documented).
- Claims (per record, snapshot vintage `pwc-<UTC date>`): `dc:stage` (closed vocab per layer, fail-loudly), `dc:recorded_name`, `dc:address`, `dc:parcel_gpin`, `dc:year_built`, `dc:occupied_date`, `dc:permit_status` (the named permit case is the asserting `source_record`), `dc:gfa_sqft` (+`_approved`/`_permitted`/`_taxed`), campus `dc:zoning_case`, `dc:gfa_planned_sqft`, `dc:gfa_remaining_sqft`, `dc:acreage`. The county's `LastEditDate` is each claim's `published_at`; `0` is treated as the county's null for numerics.
- Layer 11 (zoning applications, 61 rows) is deliberately deferred to DC-1 — it is docket/pipeline material.

## Acceptance
1. Live snapshot: 247 buildings + 75 campuses → 322 entities (all geocoded) with ~2,3XX claims; zero unsourced values; unknown status vocab / ArcGIS error-in-200 / empty layer all refuse loudly.
2. Offline tests on an 8-row real fixture (all statuses both layers, Iron Mountain VA-1 with its 2017-09-24 occupancy and assessed 165,230 sqft, a zero-GFA under-construction building).
3. Demo query (j): county buildings by stage with GFA sums; ECHO queries re-scoped per dataset so mixed vintages can't cross-contaminate.
