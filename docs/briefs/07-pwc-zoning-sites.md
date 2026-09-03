# Brief 07 — PWC zoning sites: the pre-permit pipeline (DC-1 first slice)

**What.** Extend the Prince William County adapter to layer 11 ("Data Center
Sites") of the same Build_Out_Analysis service: 61 point features, one per
land site subject to a data-center zoning action — rezonings, special-use
permits, and by-right sites. New entities under `pwc/site/`, claims for
zoning status, application type, zoned district, entitled floor area, and
application acceptance date.

**Why.** The atlas currently sees a facility only once it has a building or
an air permit. Zoning is the *earliest* public signal — years before
construction — and it is exactly the knowledge-time layer Blue Dot exists to
capture. This closes the county lifecycle: zoning application → permit →
construction → occupancy → EPA air permit, each stage from its own record.
Chosen over operator lists (needs an HTML-parsing dependency — separate ask)
and SEC 10-K schedules (unstructured; heavier lift) as the highest
value-per-effort next slice: zero new dependencies, proven adapter, and the
monthly snapshot cron picks it up with no workflow change.

**Use cases.** "How much data-center floor area is entitled but not yet
built in Prince William County?" (61 sites: 33 approved, 9 pending, 19
by-right). "Which applications are still pending, and since when?" "Show the
full paper trail for one campus, from rezoning case to occupancy."

**Source facts (probed live 2026-09-02).**
- Layer 11, point geometry, 61 rows, well under the 2,000-row page limit.
- Vocabularies: Status = Approved/Pending/By-Right; AppType =
  Rezoning/Special Use/By-Right; Workclass = Proffer Amendment/
  Non-Residential/Mixed Use/Special Use/By-Right. All enforced — an unknown
  value fails loudly, which makes the monthly cron a drift detector.
- Zero nulls, zero `"<Null>"` sentinels, zero zeros in substantive fields.
- `LastEditDate`/`CreateDate` are null on **all** rows — unlike layers 9/10
  there is no county edit date, so `published_at` falls back to the snapshot
  date (existing behavior, now exercised).
- Six zoning case numbers cover 2–3 sites each (a case can span parcels), so
  **`GlobalID` is the entity key** and the case number is a claim.
- By-right sites still carry case numbers and acceptance dates (no
  discretionary approval happens, but there is still a filed record —
  site plans and legacy rezoning cases both appear).

**Modeling decisions.**
- Entity level: `Campus` — a zoning site is campus-granularity land; its
  lifecycle position lives in claims, consistent with layer-10 "Planned"
  campuses (no new Level variant).
- `dc:zoning_status` is a **new attribute**, not a `dc:stage` value — a
  zoning approval is not a facility lifecycle stage, and conflating them
  would let an "approved" empty field masquerade as an operating facility.
- The zoning case number is the asserting record (`source_record`) for the
  four application-derived claims (`dc:zoning_status`, `dc:application_type`,
  `dc:application_workclass`, `dc:application_accepted`), mirroring how
  layer 9 uses `PermitCase` for `dc:permit_status`. GIS-row claims keep the
  GlobalID.
- `AppAccDate` is deliberately lenient (claim omitted if null, like layer
  9's `OCCDate`) even though the live layer has no nulls today: an absent
  date is missing data, not plausible-wrong data, and a future record
  without one shouldn't kill the monthly cron. The closed vocabularies stay
  strict — they are the drift detector.
- `Zoned` district codes (`M-2`, `PBD`, …) are an open set — recorded
  verbatim as `dc:zoning_district`, no vocabulary enforcement.
- Site GFA is entitled/proposed floor area → reuses `dc:gfa_planned_sqft`
  (same semantic as layer-10 `PlannedGFA`).

**Out of scope.** Linking sites to layer-10 campuses or layer-9 buildings
(same-county linkage is a DC-1 follow-up with the same discipline as brief
06); any other county's zoning records.

**Deliverables.** `pwc.rs` third `LayerKind` + vocabularies; fixture rows
(real captured data, incl. a duplicated case number and a by-right site);
demo query (l) — the entitlement pipeline; live E2E; adversarial review.
