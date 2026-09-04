# Brief 09 — campus geometry becomes data, not a fixture

**What.** The PWC ingest captures layer-10 campus polygon rings as a
first-class, snapshot-vintaged output (`data/geometry/pwc-<date>.jsonl`),
and the map page draws its land bays from the store instead of
`geo/pwc-campus-planar.json`. The fixture is deleted.

**Why.** The map decision record designated this follow-up: the engine
already fetches the polygons and throws them away (keeping only a
centroid), while the map depends on a hand-refreshed fixture — a coupling
the review flagged twice (fixture staleness vs store, campus-dossier
mismatch as a build error users can hit). Geometry-as-data means the land
bays, the dossiers, and the monthly snapshot all move together, and the
"re-run ingest so the store matches the fixture" failure mode ceases to
exist. Traces to §4 of the doctrine: what the map draws should carry the
same vintage discipline as what it states.

**Use cases.** Monthly cron lands a new vintage → the map's land bays
update with it, no manual fetch. A campus deleted by the county
disappears from the latest vintage while its history stays queryable.

**Shape (implementation of the accepted map record, not a new decision).**
- Rust: `Conformed` gains `geometries`; each layer-10 feature emits
  `{entity_id, vintage, source_dataset, retrieved_at, rings}` with rings
  as captured (lon/lat, outer + holes, unsimplified — simplification is a
  display concern). Buildings/sites are points; ECHO has no geometry —
  both emit none. `ingest pwc` writes `data/geometry/<vintage>.jsonl`.
- Python: `build-facts` loads geometry JSONL into `geometry.parquet`
  (rings as a JSON string column — DuckDB nesting adds nothing here).
  `map_page.py` builds the land-bay data from geometry at the latest
  vintage joined to the registry (name) and claims (status, planned GFA),
  flattening to the local plane at build time. The campus fixture and its
  `kx_lat` plumbing go away; `geo/` keeps only true display fixtures
  (national boundaries, neighbor-county region).
- The campus-without-dossier guard becomes structural: land bays are
  derived from the same store as dossiers, so the mismatch cannot occur;
  the guard remains for the join to claims (a campus with geometry but no
  status claim fails loudly).

**Out of scope.** Geometry for other sources; simplification pipelines;
serving raw geometry anywhere but the compiled map; PMTiles.

**Deliverables.** Engine + loader + map changes with tests (Rust fixture
already carries real rings); live re-ingest producing the first geometry
vintage; QA screenshots; adversarial review; the map decision record's
status flipped Proposed → Accepted (Kevin merged PR #13) rides along.
