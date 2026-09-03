"""Compile the whole public site — static, reproducible, no server (brief 08).

Three kinds of page, all in the fact-page design language (paper ground,
mono figures, amber = knowledge time):

- ``index.html`` — the front door, with the curated fact-page ladders.
- ``dc/index.html`` — Data Center Atlas overview: baked pipeline tables and
  a client-side-filterable directory of every facility entity.
- ``dc/<slug>.html`` — one dossier per entity: registry identity plus every
  claim, all vintages, with stated_by / source record / confidence.
  ``dc:same_as`` claims cross-link the two dossiers of a linked facility.

Like the fact pages, output is byte-stable given the same Parquet: no
wall-clock timestamps — "as of" dates come from the store itself.
"""

import html
import re
from pathlib import Path
from string import Template

import duckdb

from .page import compile_page, fact_page_filename

# The showcase ladders. compile_page fails loudly on an unknown key, so a
# re-bake that drops one of these breaks the site build instead of a link.
CURATED_FACTS = [
    ("geoId/06037", "pep:POPESTIMATE", "2022-07-01"),  # LA County — 4 vintages, a true back-revision
    ("geoId/06037", "acs:B01003_001", "2019-01-01"),  # LA County — ACS 5-year
    ("geoId/17031", "pep:POPESTIMATE", "2022-07-01"),  # Cook County, IL
    ("geoId/48201", "pep:POPESTIMATE", "2022-07-01"),  # Harris County, TX
    ("geoId/04013", "pep:POPESTIMATE", "2022-07-01"),  # Maricopa County, AZ
    ("geoId/09110", "pep:POPESTIMATE", "2022-07-01"),  # Capitol Planning Region, CT — boundary change story
    ("geoId/09110", "acs:B01003_001", "2019-01-01"),  # Capitol Planning Region — ACS view
    ("geoId/06", "pep:POPESTIMATE", "2022-07-01"),  # California — state level
]

_SLUG_OK = re.compile(r"^[A-Za-z0-9._-]+$")

STYLE = """
  :root{ --paper:#F5F4EE; --paper2:#ECEAE1; --hair:#D8D5C8; --ink:#22303C; --soft:#5A6B79;
         --amber:#C69A45; --amber-deep:#8F6D2A;
         --disp:'Bricolage Grotesque',system-ui,sans-serif; --serif:'Newsreader',Georgia,serif;
         --mono:'Spline Sans Mono',ui-monospace,Menlo,monospace; }
  *{box-sizing:border-box}
  body{margin:0; background:var(--paper); color:var(--ink); font-family:var(--serif); font-size:16px; line-height:1.55}
  .wrap{max-width:860px; margin:0 auto; padding:48px 24px 64px}
  .eyebrow{font-family:var(--mono); font-size:11px; font-weight:600; letter-spacing:.18em; color:var(--amber-deep)}
  .eyebrow a{color:inherit; text-decoration:none}
  h1{font-family:var(--disp); font-weight:800; font-size:clamp(28px,5vw,44px); line-height:1.05; margin:10px 0 6px; text-wrap:balance}
  h2{font-family:var(--disp); font-weight:600; font-size:20px; margin:34px 0 8px}
  .kicker{font-family:var(--mono); font-size:11.5px; color:var(--soft); margin:0 0 4px}
  p.lede{font-size:18px; max-width:64ch}
  hr{border:none; border-top:1px solid var(--hair); margin:26px 0}
  a{color:#0F4C7A}
  .cards{display:grid; grid-template-columns:repeat(auto-fit,minmax(260px,1fr)); gap:14px; margin-top:18px}
  .card{display:block; background:#fff; border:1px solid var(--hair); border-radius:4px; padding:18px; text-decoration:none; color:var(--ink)}
  .card b{font-family:var(--disp); font-size:17px}
  .card p{font-size:14px; color:var(--soft); margin:8px 0 0}
  .tablewrap{overflow-x:auto; margin-top:14px}
  table{border-collapse:collapse; width:100%; font-family:var(--mono); font-size:12.5px; white-space:nowrap}
  th{font-size:10px; letter-spacing:.12em; text-transform:uppercase; color:var(--soft); font-weight:600; text-align:left}
  th,td{padding:7px 14px 7px 0; border-bottom:1px solid var(--hair); font-variant-numeric:tabular-nums; vertical-align:top}
  td.num,th.num{text-align:right; padding-right:0}
  td.wraps{white-space:normal; min-width:16ch}
  .chip{display:inline-block; font-family:var(--mono); font-size:10.5px; border:1px solid var(--hair);
        border-radius:2px; padding:1px 6px; color:var(--soft)}
  .chip.amber{border-color:var(--amber); color:var(--amber-deep)}
  .linkbox{background:#FBF6EA; border:1px solid var(--amber); border-radius:4px; padding:12px 16px;
           font-family:var(--mono); font-size:12.5px; margin:18px 0}
  ul.dir{list-style:none; padding:0; margin:10px 0 0; font-family:var(--mono); font-size:13px}
  ul.dir li{padding:5px 0; border-bottom:1px solid var(--hair)}
  ul.dir .meta{color:var(--soft); font-size:11px; margin-left:8px}
  #filter{font-family:var(--mono); font-size:13px; width:100%; max-width:420px; padding:8px 10px;
          border:1px solid var(--hair); border-radius:3px; background:#fff; color:var(--ink)}
  #filter:focus-visible{outline:2px solid #5FA3D6; outline-offset:1px}
  .foot{font-family:var(--mono); font-size:10.5px; color:var(--soft); margin-top:34px; line-height:1.7}
"""

_HEAD = Template("""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>$title</title>
<meta name="description" content="$description">
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Bricolage+Grotesque:opsz,wght@12..96,600;12..96,800&family=Newsreader:ital,opsz,wght@0,6..72,400;1,6..72,400&family=Spline+Sans+Mono:wght@400;500;600&display=swap">
<style>$style</style>
</head>
<body>
<div class="wrap">
""")

INDEX = Template("""$head
  <p class="eyebrow">BLUE DOT</p>
  <h1>The world as it actually is, with receipts.</h1>
  <p class="lede">Blue Dot is an atlas and almanac of public statistical data —
  bitemporal, boundary-aware, provenance-first. Every figure on every page
  traces to a source, a vintage, and a geographic definition. When the
  government revises a number, both beliefs stay on the record.</p>
  <div class="cards">
    <a class="card" href="dc/index.html"><b>Data Center Atlas</b>
      <p>$dc_count facility entities from government records — zoning
      applications, building permits, EPA air permits — including
      cross-source dossiers where two agencies describe (and sometimes
      contradict each other about) the same building.</p></a>
    <a class="card" href="$flagship_href"><b>Fact ladders</b>
      <p>"What did we know, and when did we know it?" — one fact, every
      vintage. Watch a county's population change as the Census revises
      its own past.</p></a>
  </div>
  <h2>Curated fact pages</h2>
  <div class="tablewrap"><table>
    <thead><tr><th>place</th><th>indicator</th><th>valid</th><th>vintages</th></tr></thead>
    <tbody>
$fact_rows
    </tbody>
  </table></div>
  <p class="foot">Compiled from the fact store — static pages, no server (ADR-0001, ADR-0010).
  Store as of $as_of. Source and methods: <a href="https://github.com/kwheeler27/bluedot">github.com/kwheeler27/bluedot</a>.</p>
</div>
</body>
</html>
""")

DC_INDEX = Template("""$head
  <p class="eyebrow"><a href="../index.html">BLUE DOT</a> · DATA CENTER ATLAS</p>
  <h1>Every data center the record can prove.</h1>
  <p class="lede">No national registry of data centers exists. This atlas
  compiles the facilities that government paperwork can prove: EPA air
  permits for backup generators (nationwide), and — for Prince William
  County, Virginia, the densest data-center county on Earth — the county's
  own building, campus, and zoning records. Capacity in compute or storage
  is never public; floor area and permit stages are.</p>

  <h2>United States — EPA air-permit registry</h2>
  <p class="kicker">facilities holding Clean Air Act permits under NAICS 518210, by lifecycle stage · vintage $echo_vintage</p>
  <div class="tablewrap"><table>
    <thead><tr><th>stage</th><th class="num">facilities</th></tr></thead>
    <tbody>$echo_rows</tbody>
  </table></div>

  <h2>Prince William County, VA — buildings</h2>
  <p class="kicker">county planning GIS, building level · vintage $pwc_vintage</p>
  <div class="tablewrap"><table>
    <thead><tr><th>stage</th><th class="num">buildings</th><th class="num">floor area (M sqft)</th></tr></thead>
    <tbody>$bld_rows</tbody>
  </table></div>

  <h2>Prince William County, VA — zoning pipeline</h2>
  <p class="kicker">entitlement before construction: rezonings, special-use permits, by-right sites · vintage $pwc_vintage</p>
  <div class="tablewrap"><table>
    <thead><tr><th>application status</th><th class="num">sites</th><th class="num">entitled floor area (M sqft)</th></tr></thead>
    <tbody>$site_rows</tbody>
  </table></div>

  <h2>Cross-source dossiers</h2>
  <p class="kicker">one facility, two agencies — county permit record ⇄ EPA registry (inferred links, method in the dossier)</p>
  <ul class="dir">
$pair_rows
  </ul>

  <h2>All entities</h2>
  <p class="kicker">$entity_count entities · type to filter</p>
  <input id="filter" type="search" placeholder="filter by name, id, kind, state…" aria-label="Filter entities">
  <ul class="dir" id="dir">
$dir_rows
  </ul>
  <p class="foot">Stages and statuses are each source's own vocabulary, conformed to
  tokens — never merged across sources. Zoning approval is an application status,
  deliberately distinct from facility stage. Store as of $as_of.</p>
</div>
<script>
  var input = document.getElementById("filter");
  var items = Array.prototype.slice.call(document.getElementById("dir").children);
  input.addEventListener("input", function () {
    var q = input.value.toLowerCase();
    items.forEach(function (li) {
      li.hidden = q !== "" && li.textContent.toLowerCase().indexOf(q) === -1;
    });
  });
</script>
</body>
</html>
""")

DOSSIER = Template("""$head
  <p class="eyebrow"><a href="../index.html">BLUE DOT</a> · <a href="index.html">DATA CENTER ATLAS</a> · DOSSIER</p>
  <h1>$name</h1>
  <p class="kicker">$entity_id · $level · $latlon · registered in $source_dataset · boundary year $boundary_year</p>
$linkbox
  <h2>Claims</h2>
  <p class="kicker">every claim, every vintage — knowledge time is part of the record, not noise</p>
  <div class="tablewrap"><table>
    <thead><tr><th>attribute</th><th class="num">value</th><th>unit</th><th>stated by</th><th>record</th><th>vintage</th><th>published</th><th>confidence</th></tr></thead>
    <tbody>
$claim_rows
    </tbody>
  </table></div>
  <p class="foot">Claims are per-source assertions keyed on (entity, attribute, valid time,
  vintage, source record) — never merged at ingest (ADR-0015). ``inferred`` marks Blue Dot's
  own cross-source links, with the evidence in "stated by" (ADR-0016).
  Compiled from data/claims.parquet.</p>
</div>
</body>
</html>
""")


def entity_slug(entity_id: str) -> str:
    """Filesystem-safe slug. Refuses ids outside the known charset rather than
    silently mangling them into a collision."""
    slug = entity_id.replace("/", "-")
    if not _SLUG_OK.match(slug):
        raise SystemExit(f"entity id {entity_id!r} has characters the site slug scheme does not cover")
    return slug


def _esc(v: object) -> str:
    return html.escape(str(v), quote=True)


def _num(v: float) -> str:
    """Display a claim's numeric value: integers without a fake decimal."""
    return f"{int(v):,}" if float(v).is_integer() else f"{v:,}"


def _head(title: str, description: str) -> str:
    return _HEAD.substitute(title=_esc(title), description=_esc(description), style=STYLE)


def _tr(cells: list[tuple[str, str]]) -> str:
    """One escaped table row; each cell is (css_class, text)."""
    tds = "".join(
        f'<td class="{cls}">{_esc(text)}</td>' if cls else f"<td>{_esc(text)}</td>"
        for cls, text in cells
    )
    return f"      <tr>{tds}</tr>"


def build_site(data_dir: Path, out_dir: Path) -> None:
    claims_pq = data_dir / "claims.parquet"
    entities_pq = data_dir / "entities.parquet"
    facts_pq = data_dir / "facts.parquet"
    for pq in (claims_pq, entities_pq, facts_pq):
        if not pq.exists():
            raise SystemExit(f"{pq} not found — run `bluedot-atlas build-facts` first")
    con = duckdb.connect()
    # CREATE VIEW refuses prepared parameters — inline the (local, trusted)
    # paths, quoting any embedded single quote the SQL way.
    q = lambda p: str(p).replace("'", "''")  # noqa: E731
    con.execute(f"CREATE VIEW claims AS SELECT * FROM read_parquet('{q(claims_pq)}')")
    con.execute(f"CREATE VIEW entities AS SELECT * FROM read_parquet('{q(entities_pq)}')")
    (out_dir / "dc").mkdir(parents=True, exist_ok=True)
    (out_dir / "facts").mkdir(parents=True, exist_ok=True)

    # Every page this run produces, so leftovers from an earlier build (a
    # renamed slug, a re-baked entity set) can be pruned at the end — a stale
    # dossier the store no longer backs must never stay deployed.
    written: set[Path] = set()

    def write(path: Path, text: str) -> None:
        path.write_text(text, encoding="utf-8")
        written.add(path.resolve())

    # ---- registry identity: latest vintage per entity. Safe to ORDER BY the
    # vintage label because one entity only ever has one source's vintages
    # (the cross-SOURCE lexical trap from facts.py does not apply per-entity).
    ents = con.execute(
        """
        SELECT entity_id, name, level, boundary_year, source_dataset, lat, lon
        FROM (SELECT *, row_number() OVER (PARTITION BY entity_id ORDER BY vintage DESC) rn
              FROM entities WHERE entity_id IN (SELECT DISTINCT entity_id FROM claims))
        WHERE rn = 1 ORDER BY entity_id
        """
    ).fetchall()
    by_id = {r[0]: r for r in ents}

    # Coverage invariant: every entity that carries claims gets a dossier. An
    # entity in claims with no registry row would otherwise be dropped
    # silently — the quiet-omission cousin of plausible-wrong.
    unregistered = [
        r[0]
        for r in con.execute(
            """SELECT DISTINCT c.entity_id FROM claims c
               LEFT JOIN entities e ON c.entity_id = e.entity_id
               WHERE e.entity_id IS NULL ORDER BY 1"""
        ).fetchall()
    ]
    if unregistered:
        raise SystemExit(
            f"{len(unregistered)} entities carry claims but have no registry row "
            f"(no dossier possible): {', '.join(unregistered[:5])}"
        )

    slugs: dict[str, str] = {}
    for entity_id in by_id:
        slug = entity_slug(entity_id)
        if slug in slugs.values():
            raise SystemExit(f"slug collision: {entity_id!r}")
        slugs[entity_id] = slug

    # ---- dc:same_as links, both directions (target id lives in value_text;
    # the linked pair may span sources, so resolve names via the registry).
    links = con.execute(
        "SELECT entity_id, value_text, stated_by FROM claims WHERE attribute_id = 'dc:same_as' ORDER BY entity_id, value_text"
    ).fetchall()
    linked: dict[str, list[tuple[str, str]]] = {}
    for src, dst, evidence in links:
        linked.setdefault(src, []).append((dst, evidence))
        linked.setdefault(dst, []).append((src, evidence))

    all_claims = con.execute(
        """
        SELECT entity_id, attribute_id, value_text, value_num, unit, stated_by,
               source_record, vintage, published_at, confidence
        FROM claims ORDER BY entity_id, attribute_id, vintage DESC, source_record
        """
    ).fetchall()
    claims_by_entity: dict[str, list[tuple]] = {}
    for row in all_claims:
        claims_by_entity.setdefault(row[0], []).append(row)

    as_of = con.execute("SELECT max(retrieved_at) FROM claims").fetchone()[0].date().isoformat()

    # ---- dossiers
    for entity_id, slug in slugs.items():
        ent = by_id[entity_id]
        _, name, level, boundary_year, source_dataset, lat, lon = ent
        linkbox = ""
        for other, evidence in linked.get(entity_id, []):
            if other not in by_id:
                raise SystemExit(f"dc:same_as target {other!r} has no registry row")
            linkbox += (
                f'  <div class="linkbox">same facility (inferred): '
                f'<a href="{_esc(slugs[other])}.html">{_esc(by_id[other][1])}</a> '
                f"— {_esc(by_id[other][4])}<br><span class=\"chip\">{_esc(evidence)}</span></div>\n"
            )
        rows = []
        for _, attr, vtext, vnum, unit, stated_by, record, vintage, published, conf in claims_by_entity.get(entity_id, []):
            value = vtext if vtext is not None else (_num(vnum) if vnum is not None else "—")
            rows.append(
                _tr(
                    [
                        ("", attr),
                        ("num", value),
                        ("", unit or ""),
                        ("wraps", stated_by),
                        ("", record),
                        ("", vintage),
                        ("", published.isoformat()),
                        ("", conf),
                    ]
                )
            )
        page = DOSSIER.substitute(
            head=_head(f"{name} — Data Center Atlas", f"Dossier: every recorded claim about {name}."),
            name=_esc(name),
            entity_id=_esc(entity_id),
            level=_esc(level),
            latlon=_esc(f"{lat:.5f}, {lon:.5f}" if lat is not None and lon is not None else "no coordinates"),
            source_dataset=_esc(source_dataset),
            boundary_year=_esc(boundary_year),
            linkbox=linkbox,
            claim_rows="\n".join(rows),
        )
        write(out_dir / "dc" / f"{slug}.html", page)

    # ---- DC overview tables (latest vintage per source, as in facts.py)
    def baked(sql: str, params: list | None = None) -> list[tuple]:
        return con.execute(sql, params or []).fetchall()

    def latest_vintage(source_dataset: str) -> str:
        got = baked("SELECT max(vintage) FROM claims WHERE source_dataset = ?", [source_dataset])[0][0]
        if got is None:
            # max() over nothing is NULL — a source that vanished from the
            # store must stop the build, not render empty "vintage None" tables.
            raise SystemExit(f"no claims at all for source {source_dataset!r} — refusing to bake empty tables")
        return got

    echo_vintage = latest_vintage("epa/echo/air")
    pwc_vintage = latest_vintage("pwcva/build-out-analysis")
    echo_rows = baked(
        """SELECT value_text, count(DISTINCT entity_id) FROM claims
           WHERE attribute_id = 'dc:stage' AND source_dataset = 'epa/echo/air' AND vintage = ?
           GROUP BY 1 ORDER BY 2 DESC, 1""",
        [echo_vintage],
    )
    bld_rows = baked(
        """WITH sg AS (SELECT DISTINCT entity_id, value_text AS stage FROM claims
                       WHERE attribute_id = 'dc:stage' AND vintage = ? AND entity_id LIKE 'pwc/bld/%'),
           g AS (SELECT entity_id, max(value_num) AS gfa FROM claims
                 WHERE attribute_id = 'dc:gfa_sqft' AND vintage = ? GROUP BY 1)
           SELECT sg.stage, count(*), round(sum(g.gfa) / 1e6, 2)
           FROM sg LEFT JOIN g USING (entity_id) GROUP BY 1 ORDER BY 2 DESC, 1""",
        [pwc_vintage, pwc_vintage],
    )
    site_rows = baked(
        """WITH st AS (SELECT entity_id, value_text AS status FROM claims
                       WHERE attribute_id = 'dc:zoning_status' AND vintage = ?),
           g AS (SELECT entity_id, value_num AS gfa FROM claims
                 WHERE attribute_id = 'dc:gfa_planned_sqft' AND vintage = ?
                   AND entity_id LIKE 'pwc/site/%')
           SELECT st.status, count(*), round(sum(g.gfa) / 1e6, 2)
           FROM st LEFT JOIN g USING (entity_id) GROUP BY 1 ORDER BY 2 DESC, 1""",
        [pwc_vintage, pwc_vintage],
    )

    pair_rows = []
    for src, dst, _ in links:
        if src not in by_id or dst not in by_id:
            raise SystemExit(f"dc:same_as pair ({src!r}, {dst!r}) missing a registry row")
        pair_rows.append(
            f'      <li><a href="{_esc(slugs[src])}.html">{_esc(by_id[src][1])}</a> ⇄ '
            f'<a href="{_esc(slugs[dst])}.html">{_esc(by_id[dst][1])}</a></li>'
        )

    # directory: kind chip from the id scheme; state for ECHO entities
    states = dict(
        baked(
            "SELECT entity_id, value_text FROM claims WHERE attribute_id = 'dc:state' AND vintage = ?",
            [echo_vintage],
        )
    )
    kinds = {"pwc/bld": "building", "pwc/campus": "campus", "pwc/site": "zoning site", "frs": "EPA facility"}
    dir_rows = []
    for entity_id, slug in sorted(slugs.items(), key=lambda kv: by_id[kv[0]][1].lower()):
        prefix = entity_id.rsplit("/", 1)[0]
        kind = kinds.get(prefix, prefix)
        meta = f"{kind} · {states[entity_id]}" if entity_id in states else kind
        dir_rows.append(
            f'      <li><a href="{_esc(slug)}.html">{_esc(by_id[entity_id][1])}</a>'
            f'<span class="meta">{_esc(meta)} · {_esc(entity_id)}</span></li>'
        )

    dc_index = DC_INDEX.substitute(
        head=_head("Data Center Atlas — Blue Dot", "Every US data center that government records can prove, with provenance."),
        echo_vintage=_esc(echo_vintage),
        pwc_vintage=_esc(pwc_vintage),
        echo_rows="\n".join(_tr([("", s), ("num", f"{n:,}")]) for s, n in echo_rows),
        bld_rows="\n".join(_tr([("", s), ("num", f"{n:,}"), ("num", str(g))]) for s, n, g in bld_rows),
        site_rows="\n".join(_tr([("", s), ("num", f"{n:,}"), ("num", str(g))]) for s, n, g in site_rows),
        pair_rows="\n".join(pair_rows),
        entity_count=f"{len(slugs):,}",
        dir_rows="\n".join(dir_rows),
        as_of=_esc(as_of),
    )
    write(out_dir / "dc" / "index.html", dc_index)

    # ---- curated fact pages + front door
    fact_rows = []
    for entity_id, indicator_id, valid_from in CURATED_FACTS:
        page_path = compile_page(data_dir, entity_id, indicator_id, valid_from, out_dir=out_dir / "facts")
        written.add(page_path.resolve())
        got = con.execute(
            """
            SELECT count(DISTINCT vintage),
                   (SELECT name FROM entities WHERE entity_id = ?
                    ORDER BY boundary_year DESC, source_dataset DESC LIMIT 1)
            FROM read_parquet(?) WHERE entity_id = ? AND indicator_id = ? AND valid_from = ?
            """,
            [entity_id, str(facts_pq), entity_id, indicator_id, valid_from],
        ).fetchone()
        vintage_count, name = got
        href = f"facts/{fact_page_filename(entity_id, indicator_id, valid_from)}"
        fact_rows.append(
            f'      <tr><td><a href="{_esc(href)}">{_esc(name or entity_id)}</a></td>'
            f"<td>{_esc(indicator_id)}</td><td>{_esc(valid_from)}</td>"
            f'<td class="num">{_esc(vintage_count)}</td></tr>'
        )

    flagship = CURATED_FACTS[0]
    index = INDEX.substitute(
        head=_head("Blue Dot", "An atlas and almanac of public statistical data — bitemporal, boundary-aware, provenance-first."),
        dc_count=f"{len(slugs):,}",
        flagship_href=_esc(f"facts/{fact_page_filename(*flagship)}"),
        fact_rows="\n".join(fact_rows),
        as_of=_esc(as_of),
    )
    write(out_dir / "index.html", index)

    # Prune pages a previous build left behind (renamed slugs, re-baked
    # entity sets): anything .html we did not write this run is stale and
    # must not deploy. Non-HTML files (.vercel/, favicons) are untouched.
    stale = [p for p in out_dir.rglob("*.html") if p.resolve() not in written]
    for p in stale:
        p.unlink()
    pruned = f", pruned {len(stale)} stale" if stale else ""
    print(
        f"compiled {out_dir}/ — {len(written)} pages ({len(slugs)} dossiers, "
        f"{len(CURATED_FACTS)} fact ladders{pruned}), store as of {as_of}"
    )
