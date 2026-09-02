"""Cross-source entity resolution v0 (DC-0.3) — conservative, evidence-carrying.

Links facility entities that two sources describe as the same physical site.
v0 scope: Prince William County buildings (``pwc/bld/*``) → ECHO air-permit
facilities (``frs/*``) in Virginia. A link is itself a claim (``dc:same_as``)
asserted by this code with confidence ``inferred`` — interpretations are
attributed (CLAUDE.md), and the evidence (distance, shared name tokens) rides
in ``stated_by``. Ambiguity refuses: a building with several plausible ECHO
candidates gets NO link; unmatched simply stays unmatched.
"""

from __future__ import annotations

import json
import math
import re
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


def build_links(data_dir: Path) -> Path:
    entities_pq = data_dir / "entities.parquet"
    claims_pq = data_dir / "claims.parquet"
    for pq in (entities_pq, claims_pq):
        if not pq.exists():
            raise SystemExit(f"{pq} not found — run `bluedot-atlas build-facts` first")
    con = duckdb.connect()
    pwc = con.execute(
        "SELECT DISTINCT entity_id, name, lat, lon FROM read_parquet(?) "
        "WHERE entity_id LIKE 'pwc/bld/%' AND lat IS NOT NULL",
        [str(entities_pq)],
    ).fetchall()
    frs = con.execute(
        """
        SELECT e.entity_id, min(e.name) AS name, min(e.lat) AS lat, min(e.lon) AS lon
        FROM read_parquet(?) e
        JOIN read_parquet(?) c ON c.entity_id = e.entity_id
        WHERE e.entity_id LIKE 'frs/%' AND c.attribute_id = 'dc:state' AND c.value_text = 'VA'
          AND e.lat IS NOT NULL
        GROUP BY e.entity_id
        """,
        [str(entities_pq), str(claims_pq)],
    ).fetchall()
    if not pwc or not frs:
        raise SystemExit(f"nothing to link: {len(pwc)} county buildings, {len(frs)} VA ECHO facilities")

    now = datetime.now(timezone.utc)
    today = now.date()
    vintage = f"link-{today}"
    rows, ambiguous = [], []
    for pid, pname, plat, plon in pwc:
        candidates = []
        for fid, fname, flat, flon in frs:
            d = haversine_m(plat, plon, flat, flon)
            if d > MAX_METERS:
                continue
            shared = name_tokens(pname) & name_tokens(fname)
            if is_match(d, shared):
                candidates.append((d, fid, fname, shared))
        if len(candidates) > 1:
            ambiguous.append((pid, [c[1] for c in candidates]))
            continue
        if len(candidates) != 1:
            continue
        d, fid, fname, shared = candidates[0]
        evidence = f"{d:.0f} m apart" + (f", shared name token(s) {sorted(shared)}" if shared else "")
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
    tmp.write_text("".join(json.dumps(r) + "\n" for r in rows), encoding="utf-8")
    tmp.rename(out)
    print(
        f"wrote {out} — {len(rows)} links from {len(pwc)} county buildings × {len(frs)} VA ECHO facilities; "
        f"{len(ambiguous)} ambiguous (refused)"
    )
    for pid, fids in ambiguous[:5]:
        print(f"  ambiguous: {pid} ~ {fids}")
    return out
