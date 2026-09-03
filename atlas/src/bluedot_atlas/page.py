"""Compile one fact page — a static, reproducible HTML file over the Parquet store.

Mockup 03 of the design direction, made real: the vintage ladder for one fact
key, in the deck's design language (paper ground, mono figures, amber =
knowledge time). No server — the page is compiled, like everything in Blue Dot,
and opens from file://. Given the same Parquet, the output is byte-stable.
"""

import html
import json
from datetime import date, timedelta
from pathlib import Path
from string import Template

import duckdb

from .indicators import INDICATORS

# string.Template ($vars) instead of an f-string: the CSS/JS below is full of
# braces, and Template leaves them alone. The page's own JS avoids `${...}`
# template literals for the same reason.
PAGE = Template("""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>$title</title>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Bricolage+Grotesque:opsz,wght@12..96,600;12..96,800&family=Newsreader:ital,opsz,wght@0,6..72,400;1,6..72,400&family=Spline+Sans+Mono:wght@400;500;600&display=swap">
<style>
  :root{ --paper:#F5F4EE; --paper2:#ECEAE1; --hair:#D8D5C8; --ink:#22303C; --soft:#5A6B79;
         --amber:#C69A45; --amber-deep:#8F6D2A;
         --disp:'Bricolage Grotesque',system-ui,sans-serif; --serif:'Newsreader',Georgia,serif;
         --mono:'Spline Sans Mono',ui-monospace,Menlo,monospace; }
  *{box-sizing:border-box}
  body{margin:0; background:var(--paper); color:var(--ink); font-family:var(--serif); font-size:16px; line-height:1.55}
  .wrap{max-width:760px; margin:0 auto; padding:48px 24px 64px}
  .eyebrow{font-family:var(--mono); font-size:11px; font-weight:600; letter-spacing:.18em; color:var(--amber-deep)}
  h1{font-family:var(--disp); font-weight:800; font-size:clamp(28px,5vw,44px); line-height:1.05; margin:10px 0 6px; text-wrap:balance}
  .kicker{font-family:var(--mono); font-size:11.5px; color:var(--soft); margin:0 0 4px}
  .defn{font-style:italic; color:var(--soft); max-width:62ch; margin:14px 0 0}
  .pending{font-family:var(--mono); font-size:11px; color:var(--soft); border:1px dashed var(--hair); border-radius:3px; padding:2px 7px}
  hr{border:none; border-top:1px solid var(--hair); margin:26px 0}
  .vbtns{display:flex; flex-wrap:wrap; gap:8px; margin:0 0 18px}
  .vbtn{font-family:var(--mono); font-size:12px; background:#fff; color:var(--soft); border:1px solid var(--hair);
        border-radius:3px; padding:7px 12px; cursor:pointer}
  .vbtn[aria-pressed="true"]{border-color:var(--amber-deep); color:var(--amber-deep); font-weight:600; background:#FBF6EA}
  .vbtn:focus-visible{outline:2px solid #5FA3D6; outline-offset:2px}
  .figure{font-family:var(--mono); font-weight:600; font-size:clamp(34px,7vw,52px); letter-spacing:-.02em;
          font-variant-numeric:tabular-nums; line-height:1.1}
  .figcap{font-family:var(--mono); font-size:11px; letter-spacing:.06em; text-transform:uppercase; color:var(--soft); margin-top:2px}
  .chip{display:inline-block; font-family:var(--mono); font-size:10.5px; border:1px solid var(--amber);
        border-radius:2px; padding:2px 7px; color:var(--amber-deep); margin-top:10px}
  .delta{font-family:var(--mono); font-size:12.5px; color:var(--soft); margin-top:8px}
  .delta b{color:var(--amber-deep)}
  .tablewrap{overflow-x:auto; margin-top:26px}
  table{border-collapse:collapse; width:100%; font-family:var(--mono); font-size:12.5px; white-space:nowrap}
  th{font-size:10px; letter-spacing:.12em; text-transform:uppercase; color:var(--soft); font-weight:600; text-align:left}
  th,td{padding:7px 14px 7px 0; border-bottom:1px solid var(--hair); font-variant-numeric:tabular-nums}
  td.num,th.num{text-align:right; padding-right:0}
  tr.on td{color:var(--amber-deep); font-weight:600}
  td a{color:#0F4C7A}
  .foot{font-family:var(--mono); font-size:10.5px; color:var(--soft); margin-top:34px; line-height:1.7}
</style>
</head>
<body>
<div class="wrap">
  <p class="eyebrow">BLUE DOT · FACT</p>
  <h1>$name</h1>
  <p class="kicker">$entity_id · $indicator_id · valid $valid_label · $timeframe $registry_note</p>
  <p class="defn">$definition</p>
  <hr>
  <p class="kicker">WHAT DID WE KNOW, AND WHEN DID WE KNOW IT?</p>
  <div class="vbtns" id="vbtns" role="group" aria-label="Choose a knowledge date"></div>
  <div class="figure" id="figure"></div>
  <div class="figcap">$figcap</div>
  <div><span class="chip" id="chip"></span></div>
  <div class="delta" id="delta"></div>
  <div class="tablewrap">
    <table aria-label="All vintages of this fact">
      <thead><tr><th>vintage</th><th>published</th><th class="num">value</th><th class="num">restates prior by</th><th>moe</th><th>boundary yr</th><th>source</th></tr></thead>
      <tbody id="ladder"></tbody>
    </table>
  </div>
  <p class="foot">Every row is one vintage of the same fact — same entity, same indicator, same valid time (ADR-0001).
  Compiled from data/facts.parquet by bluedot-atlas; registry v0 supplies the display name (ADR-0014).
  Latest retrieval shown: $retrieved.</p>
</div>
<script>
  var ROWS = $payload;
  var fmt = function (n) { return n === null ? "—" : Math.round(n).toLocaleString("en-US"); };
  var sfmt = function (n) { return (n >= 0 ? "+" : "\\u2212") + Math.round(Math.abs(n)).toLocaleString("en-US"); };
  var el = function (id) { return document.getElementById(id); };
  var btnbox = el("vbtns"), ladder = el("ladder"), btns = [], trs = [];
  ROWS.forEach(function (r, i) {
    var b = document.createElement("button");
    b.className = "vbtn"; b.type = "button"; b.textContent = r.published_at;
    b.addEventListener("click", function () { render(i); });
    btnbox.appendChild(b); btns.push(b);
    var prior = i ? ROWS[i - 1] : null;
    var restate = (prior && r.value !== null && prior.value !== null) ? sfmt(r.value - prior.value) : "—";
    var tr = document.createElement("tr");
    // DOM APIs, not innerHTML: payload strings are data, never markup.
    var cells = [r.vintage, r.published_at, fmt(r.value), restate,
                 r.moe !== null ? fmt(r.moe) : (r.moe_annotation || "—"), String(r.boundary_year)];
    cells.forEach(function (text, k) {
      var td = document.createElement("td");
      if (k === 2 || k === 3) td.className = "num";
      td.textContent = text;
      tr.appendChild(td);
    });
    var srcTd = document.createElement("td");
    var a = document.createElement("a");
    if (r.source_url.indexOf("https://") === 0) { a.href = r.source_url; }
    a.textContent = r.source_dataset + " ↗";
    srcTd.appendChild(a);
    tr.appendChild(srcTd);
    ladder.appendChild(tr); trs.push(tr);
  });
  function render(i) {
    var r = ROWS[i];
    el("figure").textContent = r.value === null ? (r.value_annotation || "—") : fmt(r.value);
    el("chip").textContent = r.source_dataset + " · " + r.vintage + " · published " + r.published_at;
    var prior = i ? ROWS[i - 1] : null;
    el("delta").innerHTML = prior && r.value !== null && prior.value !== null
      ? "restated <b>" + sfmt(r.value - prior.value) + "</b> vs the belief before " + r.published_at
      : "the first stated value \\u2014 nothing to restate yet";
    btns.forEach(function (b, k) { b.setAttribute("aria-pressed", String(k === i)); });
    trs.forEach(function (t, k) { t.className = k === i ? "on" : ""; });
  }
  render(ROWS.length - 1); // default: the newest belief, past ones one press away
</script>
</body>
</html>
""")


def fact_page_filename(entity_id: str, indicator_id: str, valid_from: object) -> str:
    """The one place the fact-page filename scheme lives — the site index
    builds hrefs with it, so a drifting copy would mean broken links.
    ``valid_from`` arrives as a date from compile_page and as a string from
    the site's curated list; normalize both to one canonical form so the two
    call sites can never name the same page differently."""
    valid = date.fromisoformat(str(valid_from)).isoformat()
    return f"{entity_id}.{indicator_id}.{valid}".replace("/", "-").replace(":", "-") + ".html"


def compile_page(
    data_dir: Path,
    entity_id: str,
    indicator_id: str,
    valid_from_text: str,
    out_dir: Path | None = None,
) -> Path:
    try:
        valid_from = date.fromisoformat(valid_from_text)
    except ValueError:
        raise SystemExit(f"valid_from {valid_from_text!r} is not a YYYY-MM-DD date") from None
    facts_pq, entities_pq = data_dir / "facts.parquet", data_dir / "entities.parquet"
    if not facts_pq.exists():
        raise SystemExit(f"{facts_pq} not found — run `bluedot-atlas build-facts` first")
    con = duckdb.connect()

    rows = con.execute(
        """
        SELECT vintage, published_at, valid_from, valid_to, value, moe, value_annotation,
               moe_annotation, boundary_year, source_dataset, source_url, retrieved_at
        FROM read_parquet(?)
        WHERE entity_id = ? AND indicator_id = ? AND valid_from = ?
        ORDER BY published_at, vintage
        """,
        [str(facts_pq), entity_id, indicator_id, valid_from],
    ).fetchall()
    if not rows:
        # Fail loudly, and name WHICH part of the key is unknown.
        known = lambda sql, arg: con.execute(sql, [str(facts_pq), arg]).fetchone()[0]  # noqa: E731
        if not known("SELECT count(*) FROM read_parquet(?) WHERE entity_id = ?", entity_id):
            raise SystemExit(f"unknown entity {entity_id!r} — not in the fact store")
        if not known("SELECT count(*) FROM read_parquet(?) WHERE indicator_id = ?", indicator_id):
            raise SystemExit(f"unknown indicator {indicator_id!r} — not in the fact store")
        avail = con.execute(
            "SELECT DISTINCT valid_from FROM read_parquet(?) WHERE entity_id=? AND indicator_id=? ORDER BY 1",
            [str(facts_pq), entity_id, indicator_id],
        ).fetchall()
        raise SystemExit(
            f"no fact for {entity_id} {indicator_id} at valid_from {valid_from} — "
            f"available: {', '.join(str(a[0]) for a in avail) or 'none for this pair'}"
        )

    name, registry_note = entity_id, ' · <span class="pending">registry pending</span>'
    if entities_pq.exists():
        # "Newest" must be the integer boundary_year, not the vintage label —
        # 'pep-2022' sorts after 'acs5-2026' as a string, which would silently
        # pick a stale name the moment vintages interleave across sources.
        got = con.execute(
            "SELECT name FROM read_parquet(?) WHERE entity_id = ? "
            "ORDER BY boundary_year DESC, source_dataset DESC LIMIT 1",
            [str(entities_pq), entity_id],
        ).fetchone()
        if got:
            name, registry_note = got[0], ""

    meta = INDICATORS.get(indicator_id)
    if meta is None:
        meta = {
            "label": indicator_id,
            "timeframe": "",
            "definition": "No semantic-layer declaration for this indicator yet.",
            "unit": "",
        }
        registry_note += ' · <span class="pending">indicator declaration pending</span>'

    valid_to = rows[0][3]
    one_day = (valid_to - valid_from) == timedelta(days=1)
    valid_label = valid_from.isoformat() if one_day else f"[{valid_from} → {valid_to})"

    payload = [
        {
            "vintage": r[0],
            "published_at": r[1].isoformat(),
            "value": r[4],
            "moe": r[5],
            "value_annotation": r[6],
            "moe_annotation": r[7],
            "boundary_year": r[8],
            "source_dataset": r[9],
            "source_url": r[10],
        }
        for r in rows
    ]
    # Every substituted value is HTML-escaped: names come from live-fetched
    # source data and ids from argv — data, never markup. registry_note is the
    # one exception (our own trusted markup). The payload additionally escapes
    # "<" so a value containing "</script>" can never terminate the tag.
    esc = lambda v: html.escape(str(v), quote=True)  # noqa: E731
    html_out = PAGE.substitute(
        title=esc(f"{name} — {meta['label']}, {valid_label}"),
        name=esc(name),
        entity_id=esc(entity_id),
        indicator_id=esc(indicator_id),
        valid_label=esc(valid_label),
        timeframe=esc(meta["timeframe"]),
        definition=esc(meta["definition"]),
        registry_note=registry_note,
        figcap=esc(f"{meta['unit'] or 'value'} · valid {valid_label}"),
        retrieved=esc(max(r[11] for r in rows).isoformat()),
        payload=json.dumps(payload).replace("<", "\\u003c"),
    )
    out = (out_dir or data_dir / "pages") / fact_page_filename(entity_id, indicator_id, valid_from)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(html_out, encoding="utf-8")
    print(f"compiled {out} — {len(rows)} vintages, name {name!r}")
    return out
