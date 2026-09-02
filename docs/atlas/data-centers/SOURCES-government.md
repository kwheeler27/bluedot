# Data Center Atlas — government & public-records source inventory

*Research brief, verified 2026-09-02 (web search + primary pages; confidence flags inline). Sibling: [SOURCES-market-open.md](SOURCES-market-open.md).*

## Federal

| Source | Coverage / fields | Format | Cadence | License | Effort | Confidence |
|---|---|---|---|---|---|---|
| EPA ECHO / ICIS-Air bulk download (echo.epa.gov/tools/data-downloads) | National; permits incl. NAICS/SIC, facility address, backup-genset info | CSV (ZIP ~64MB) | Rolling | US govt public domain | S/M | High — the Business Insider methodology (1,240 facilities); delegated-state completeness varies |
| SEC EDGAR 10-K property schedules + full-text search | Equinix, Digital Realty, Iron Mountain, Vantage/DigitalBridge: addresses, sqft, dates | HTML/XBRL | Quarterly/annual | Public domain | S/M | High for filers; ~10–20% of facilities (colo only) |
| ERCOT large-load queue (ercot.com/services/rq/large-load-integration) | TX; MW, POI, status; 233 GW queue, 70%+ data centers (Dec 2025) | PDF + third-party trackers (ercotqueue.com, interconnection.fyi) | Periodic | Public filings | M | Medium — no single clean ERCOT spreadsheet confirmed |
| PJM queue (+ Expedited Track, Aug 2026) | 13 states; ≥50 MW large-load track | Downloadable queue data | Ongoing | Public | M | Medium |
| MISO / SPP / CAISO | Aggregate MW under study only (CAISO ~4.5 GW) | PDF/dashboards | Per cycle | Public | L | Low facility yield |
| EIA data-center survey | NOT YET LIVE — pilots (TX, WA, NoVA/DC; 196 companies) conclude ≤2026-09-30; mandatory survey designed after | — | Future | — | Watch | Medium (EIA press release + Senate letter) |
| EIA-860/923/861 | Grid generators ≥1 MW; gensets mostly below threshold, not DC-flagged | CSV/API | Annual | Public | L, low yield | High |
| HIFLD | No public data-center layer found; possibly HIFLD Secure only (vetted access) | — | — | — | Unusable v0 | Medium |
| Census CBP / BLS QCEW (NAICS 518210) | County establishment counts + employment — density cross-check | CSV/API | Annual (~2 yr lag) | Public | S | High |
| LBNL 2024 US Data Center Energy Usage Report | National/regional modeling; no facility appendix found | PDF | One-off | Public | n/a | Medium |
| DOE federal AI sites | 4 named sites (INL, Oak Ridge, Paducah, Savannah River) of 16 RFI'd | Press | Evolving | Public | S | High |

## State

| Source | Yield | Effort |
|---|---|---|
| TX Comptroller qualified-data-center list | Company names only — no locations/amounts (Good Jobs First, Nov 2025) | S |
| VA data-center tax exemption (VEDP) | CONFLICT: VEDP claims authority to publish ranges; Good Jobs First says VA does not disclose recipients — needs manual check | S–M |
| VA DEQ "Issued Air Permits for Data Centers" page | Exists; 403 on direct fetch; FOIA fallback (VA: 5-business-day response) | M |
| GA DOR exemption aggregates | County $ totals only; recipients suppressed | S, low yield |
| 11 disclosing states (AZ CT IL IN MN NV OH PA TX WA WI, per Good Jobs First) | Only 5 disclose amounts; none unmask LLC parents | M each, uneven |
| PUC/SCC dockets (VA SCC/Dominion ~70 GW queue; GA PSC dockets 55378/56002; AZ ACC) | Aggregate MW + occasional named customers | M |
| State air-permit portals (TCEQ etc.) | Per-state HTML search, no bulk — why BI needed FOIA/litigation | L each |

## Local

| Source | Yield | Effort |
|---|---|---|
| Prince William County VA interactive data-center map (Feb 2026) | Best local source: status, acreage, sqft, zoning case #s | S |
| Loudoun County VA | Policy pages only; third-party PEC ArcGIS map fills gap | M |
| City permit portals (Columbus, Maricopa, Atlanta…) | Generic permits, not DC-flagged; manual filtering | L |
| County assessor parcels | Near-universal, fragmented; owner = LLC shell | L per county |

## Top 8 for v0 (value-for-effort)
1. EPA ECHO ICIS-Air bulk CSV · 2. Prince William County map · 3. SEC EDGAR REIT 10-Ks · 4. TX Comptroller list · 5. ERCOT queue + trackers · 6. VA SCC Dominion dockets · 7. GA PSC reports + GA DOR aggregates · 8. CBP/QCEW NAICS 518210.

## Field ceilings (hypothesis confirmed)
Location/status/dates/owner-of-record: mostly obtainable — owner usually an LLC shell (parent unmasking is separate work). MW: sometimes (queue/docket MW ≠ IT load; genset MW = backup proxy). Compute/storage: absent from every government source found; only the future EIA survey proposes "server metrics".

Key URLs: eia.gov/pressroom/releases/press585.php · ercot.com/services/rq/large-load-integration · echo.epa.gov/tools/data-downloads/icis-air-download-summary · epa.gov/stationary-sources-air-pollution/clean-air-act-resources-data-centers · comptroller.texas.gov/taxes/data-centers · goodjobsfirst.org/cloudy-data-costly-deals · deq.virginia.gov (issued air permits for data centers) · pwcva.gov (interactive data center map) · psc.ga.gov/site/downloads/datacenterfactsheet.pdf · dor.georgia.gov (data-center exemption aggregates) · hifld-geoplatform.opendata.arcgis.com · eta-publications.lbl.gov (2024 report) · energy.gov (federal AI sites)
