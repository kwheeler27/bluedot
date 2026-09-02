# Data Center Atlas — trackers, corporate & open-data sources (with licensing)

*Research brief, verified 2026-09-02. Sibling: [SOURCES-government.md](SOURCES-government.md).*

## Commercial trackers (excluded from ingestion)

| Source | Coverage | Facility-level? | Access | Reuse |
|---|---|---|---|---|
| DataCenterMap.com | 12,259 listings, 179 countries | Yes (operator-submitted) | Free browse | ToS likely restrictive |
| Baxtel | 8,000+ sites incl. per-site power | Yes | Paid | Commercial |
| DC Byte | 8,300+ facilities (satellite + filings + visits) | Yes | Paid | Commercial |
| SemiAnalysis datacenter model | 5,000+ sites, MW (property records/FOIA/satellite) | Yes | Paid, institutional | Commercial |
| Synergy Research | Market shares, ~320 cos. | No | Subscription | n/a |
| CBRE / JLL / Cushman & Wakefield | Metro aggregate MW/vacancy | No | Free PDFs | Citable, no rows |

**Business Insider (2025):** confirmed — 1,240 US facilities (built/approved through 2024) via state-by-state backup-generator air permits + shell-company unmasking (some records via litigation); interactive map + searchable table. **Bulk download unconfirmed — direct outreach recommended.** Washington Post equivalent: not verified this pass (found instead: PNNL IM3 Data Center Atlas, dcmap.us, usdatamap.com).

## Open data (license-clean or license-managed)

- **OpenStreetMap** `telecom=data_center`: ≈1,553 US nodes (Apr 2026, secondhand — medium confidence). **ODbL**: attribution + share-alike on derivative *databases* — an OSM-derived slice republished in our Parquet must remain ODbL and attributed ("© OpenStreetMap contributors"); rendered maps need attribution only. Precedent: PNNL licenses its whole IM3 derived atlas ODbL. Keep OSM rows in a separately-licensed table.
- **Open Infrastructure Map**: 3,500+ global DCs, GeoPackage/Shapefile exports, same ODbL terms — pre-cleaned OSM.
- **Wikidata** (CC0): SPARQL 2026-09-02 → ~153 data-center items worldwide. Cross-reference/dedup key only.
- **PeeringDB** `fac` table (REST /api/fac): operator-submitted colo facilities + addresses — **highest-value open source, but bulk data-reuse license UNRESOLVED; verify at docs.peeringdb.com before ingesting.** Free self-service registration otherwise.
- Wikipedia lists: CC BY-SA (share-alike caution as OSM).

## Corporate/official disclosure

- **Meta** (datacenters.atmeta.com): most transparent — named campuses, city-level, some MW/PUE/water.
- **Google** (datacenters.google/locations), **Microsoft** (datacenters.microsoft.com + Azure geographies): region/city, not addresses.
- **AWS**: Region/AZ counts only; withholds addresses. **Oracle/Apple**: no authoritative pages found; third-party counts conflict — don't trust.
- **Colo operators with open location lists**: Equinix (220+ IBX), Digital Realty (300+), Iron Mountain (30+). CyrusOne/QTS/Vantage/Aligned/Switch/STACK/CloudHQ/NTT: industry norm is a locations page but unverified per company this pass.

## Capacity disclosure — confirmed
No facility-level FLOPS/GPU/storage disclosures exist. Public proxies in order of availability: critical IT-load MW → building sqft → interconnection/substation MW → permitted genset MW. Third parties back into GPU-equivalents from MW + chip power + assumed PUE (SemiAnalysis-style; construction-physics.com worked example).

## Global preview
- **EU**: Delegated Reg. 2024/1364 — facilities ≥500 kW report annually to an EU database (2026: notify authority Apr 27, report May 15, CY2025 data). Public row-level access unresolved.
- **Germany**: BAFA public energy-efficiency register (name/owner/operator/size/load, due Mar 31 annually) — most promising per-facility EU registry (secondhand via law-firm summaries).
- **UK**: data centers now Critical National Infrastructure (2024) — trend toward less disclosure; no public register.
- **APAC**: no facility registries found; market aggregates only (C&W: +7,103 MW H1 2026 pipeline → 26,455 MW).

## Top 6 non-government for v0
1. PeeringDB (license check first) · 2. OSM via Overpass (ODbL-isolated) · 3. Open Infrastructure Map exports · 4. Operator/hyperscaler official lists · 5. Business Insider dataset if bulk access confirmed (else replicate via permits) · 6. Wikidata (CC0 dedup keys).
