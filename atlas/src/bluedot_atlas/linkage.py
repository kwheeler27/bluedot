"""Cross-source entity resolution v0 (DC-0.3) — conservative, evidence-carrying.

Links facility entities that two sources describe as the same physical site.
v0 scope: Prince William County buildings (``pwc/bld/*``) → ECHO air-permit
facilities (``frs/*``) in Virginia. A link is itself a claim (``dc:same_as``)
asserted by this code with confidence ``inferred`` (ADR-0016) — interpretations
are attributed, and the evidence (distance, shared name tokens, and whether the
facility is shared with sibling buildings) rides in ``stated_by``.

Two directions of multiplicity, treated differently on purpose:
- one building near SEVERAL facilities → ambiguous, REFUSED (we won't guess);
- several buildings near ONE facility → legitimate campus semantics (an FRS
  air-permit facility often stands for a whole campus), linked and *flagged* —
  counted in the run summary and named in each such claim's evidence.

Unmatched simply stays unmatched.
"""

from __future__ import annotations

import json
import math
import os
import re
from collections import Counter
from datetime import datetime, timedelta, timezone
from pathlib import Path

import duckdb

# Tokens too generic to identify an operator — sharing one proves nothing.
STOP_TOKENS = {
    "LLC", "INC", "LP", "CORP", "CO", "COMPANY", "HOLDINGS", "OWNER", "PROPERTY",
    "DATA", "CENTER", "CENTERS", "DATACENTER", "BUILDING", "BLDG", "CAMPUS",
    "PHASE", "SERVICES", "SERVICE", "THE", "OF", "AND", "VA", "VIRGINIA",
    "NORTHERN", "SOUTH", "NORTH", "EAST", "WEST", "PARK", "TECHNOLOGY", "TECH",
}
# Same site if very close, or moderately close with a distinctive shared token.
CLOSE_METERS = 100.0
MAX_METERS = 300.0


def name_tokens(name: str) -> set[str]:
    """Distinctive tokens: ≥3 chars, not a stop word, not a bare number."""
    return {
        t
        for t in re.split(r"[^A-Z0-9]+", name.upper())
        if len(t) >= 3 and t not in STOP_TOKENS and not t.isdigit()
    }


def haversine_m(lat1: float, lon1: float, lat2: float, lon2: float) -> float:
    """Great-circle distance in meters (spherical earth, fine at these scales)."""
    p1, p2 = math.radians(lat1), math.radians(lat2)
    dp, dl = math.radians(lat2 - lat1), math.radians(lon2 - lon1)
    a = math.sin(dp / 2) ** 2 + math.cos(p1) * math.cos(p2) * math.sin(dl / 2) ** 2
    return 6_371_000.0 * 2 * math.asin(math.sqrt(a))


def is_match(distance_m: float, shared_tokens: set[str]) -> bool:
    return distance_m <= CLOSE_METERS or (distance_m <= MAX_METERS and bool(shared_tokens))


def match_all(
    pwc_rows: list[tuple[str, str, float, float]],
    frs_rows: list[tuple[str, str, float, float]],
) -> tuple[list[tuple[str, str, float, set[str]]], list[tuple[str, list[str]]]]:
    """Pure matching over (id, name, lat, lon) tuples.

    Returns ``(links, ambiguous)`` where links are
    ``(pwc_id, frs_id, distance_m, shared_tokens)`` and ambiguous entries are
    ``(pwc_id, [candidate frs_ids])`` — refused, never guessed.
    Many-to-one (several links sharing one frs_id) is allowed; callers surface it.
    """
    links, ambiguous = [], []
    for pid, pname, plat, plon in pwc_rows:
        candidates = []
        for fid, fname, flat, flon in frs_rows:
            d = haversine_m(plat, plon, flat, flon)
            if d > MAX_METERS:
                continue
            shared = name_tokens(pname) & name_tokens(fname)
            if is_match(d, shared):
                candidates.append((d, fid, shared))
        if len(candidates) > 1:
            ambiguous.append((pid, sorted(fid for _, fid, _ in candidates)))
        elif candidates:
            d, fid, shared = candidates[0]
            links.append((pid, fid, d, shared))
    return links, ambiguous


def build_links(data_dir: Path) -> Path:
    entities_pq = data_dir / "entities.parquet"
    claims_pq = data_dir / "claims.parquet"
    for pq in (entities_pq, claims_pq):
        if not pq.exists():
            raise SystemExit(f"{pq} not found — run `bluedot-atlas build-facts` first")
    con = duckdb.connect()
    e, c = str(entities_pq), str(claims_pq)
    # Always the LATEST vintage per source: entity rows accumulate per snapshot,
    # and per-column aggregates across vintages could stitch a name from one
    # snapshot to coordinates from another.
    pwc = con.execute(
        """
        SELECT entity_id, name, lat, lon FROM read_parquet(?)
        WHERE entity_id LIKE 'pwc/bld/%' AND lat IS NOT NULL
          AND vintage = (SELECT max(vintage) FROM read_parquet(?)
                         WHERE source_dataset = 'pwcva/build-out-analysis')
        """,
        [e, e],
    ).fetchall()
    frs = con.execute(
        """
        WITH ev AS (SELECT max(vintage) AS v FROM read_parquet(?) WHERE source_dataset = 'epa/echo/air'),
        va AS (
            SELECT DISTINCT cl.entity_id FROM read_parquet(?) cl, ev
            WHERE cl.attribute_id = 'dc:state' AND cl.value_text = 'VA' AND cl.vintage = ev.v
        )
        SELECT en.entity_id, en.name, en.lat, en.lon
        FROM read_parquet(?) en, ev
        WHERE en.entity_id IN (SELECT entity_id FROM va)
          AND en.vintage = ev.v AND en.lat IS NOT NULL
        """,
        [e, c, e],
    ).fetchall()
    if not pwc or not frs:
        raise SystemExit(f"nothing to link: {len(pwc)} county buildings, {len(frs)} VA ECHO facilities")

    links, ambiguous = match_all(pwc, frs)
    shared_targets = Counter(fid for _, fid, _, _ in links)

    now = datetime.now(timezone.utc)
    today = now.date()
    vintage = f"link-{today}"
    rows = []
    for pid, fid, d, shared in links:
        evidence = f"{d:.0f} m apart" + (f", shared name token(s) {sorted(shared)}" if shared else "")
        if shared_targets[fid] > 1:
            evidence += f"; facility shared with {shared_targets[fid] - 1} sibling building(s)"
        rows.append({
            "entity_id": pid,
            "attribute_id": "dc:same_as",
            "valid_from": str(today),
            "valid_to": str(today + timedelta(days=1)),
            "vintage": vintage,
            "source_record": fid,
            "value_text": fid,
            "value_num": None,
            "unit": None,
            "stated_by": f"Blue Dot linkage v0 ({evidence})",
            "confidence": "inferred",
            "published_at": str(today),
            "source_dataset": "bluedot/linkage-v0",
            "source_url": "https://github.com/kwheeler27/bluedot/blob/main/docs/briefs/06-linkage-v0.md",
            "retrieved_at": now.strftime("%Y-%m-%dT%H:%M:%SZ"),
        })

    out = data_dir / "claims" / f"{vintage}.jsonl"
    out.parent.mkdir(parents=True, exist_ok=True)
    tmp = out.with_suffix(".jsonl.tmp")
    with open(tmp, "w", encoding="utf-8") as fh:
        fh.write("".join(json.dumps(r) + "\n" for r in rows))
        fh.flush()
        os.fsync(fh.fileno())  # durability parity with the Rust writer
    tmp.rename(out)

    many_to_one = {fid: n for fid, n in shared_targets.items() if n > 1}
    print(
        f"wrote {out} — {len(rows)} links from {len(pwc)} county buildings × {len(frs)} VA ECHO facilities; "
        f"{len(ambiguous)} ambiguous (refused); {len(many_to_one)} facilities absorb multiple buildings"
    )
    for fid, n in sorted(many_to_one.items()):
        print(f"  many-to-one: {fid} ← {n} buildings (campus-level permit facility, flagged in evidence)")
    for pid, fids in ambiguous[:5]:
        print(f"  ambiguous: {pid} ~ {fids}")
    return out
