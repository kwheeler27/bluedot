# Display geometry fixtures

Boundary geometry for the compiled map page (`dc/map.html`). These are
**display fixtures, not claims**: no figure on any page is sourced from
them — they only draw the ground and the land shapes; every number comes
from the claims store.

| File | Source | Captured | Processing |
| --- | --- | --- | --- |
| `us-counties-topo.json` | us-atlas@3 `counties-10m.json` (Census cartographic boundaries, repackaged; coordinates are unprojected lon/lat) | 2026-09-03 | verbatim |
| `pwc-region-planar.json` | Census TIGERweb `State_County/MapServer/13` (PWC + 7 neighbors) | 2026-09-03 | rings stride-simplified; pre-flattened to a local plane `[lon·cos(38.7°), −lat]` so planar rendering is immune to spherical winding-order semantics |
| `pwc-campus-planar.json` | Prince William County `Build_Out_Analysis/MapServer/10` campus polygons (name, status, PlannedGFA, GlobalID) | 2026-09-03 | same flattening; GlobalID keys match the `pwc/campus/` entity registry |

Known debt (mirrors Basin's): migrate `us-counties-topo.json` to Census
TIGER direct. Refresh `pwc-campus-planar.json` when the monthly snapshot
shows campus changes; the ingest engine capturing polygon geometry as
first-class data is the designated follow-up in the map decision record.
