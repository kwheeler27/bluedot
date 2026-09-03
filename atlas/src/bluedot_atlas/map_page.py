"""Compile dc/map.html — the atlas map, in Basin's story-map idiom.

Decision record docs/decisions/2026-09-02-map-rendering-stack.md, design
approved through the interactive mockup (five review rounds): d3-geo +
topojson from pinned CDN scripts over committed display geometry (geo/),
one saturated hue, graduated marks, halo'd labels, on-map annotations.
Two views: national (AlbersUsa, county-whisper ground, chips choose the
hero lifecycle bucket) and county (Prince William land bays shaded by
project status, buildings as dots). Every mark links to its dossier.

The page is self-contained at view time: no tile service, no runtime
queries — data is baked here at compile time from the claims store, and
the geometry files in geo/ are display fixtures, not claims.
"""

import html
import json
from pathlib import Path
from string import Template

# Lifecycle buckets for display. Source vocabularies are never merged in
# the store (ADR-0015); this mapping is declared, and an unknown stage
# fails the build instead of being silently bucketed.
BUCKET = {
    "operating": "built",
    "completed": "built",
    "under_construction": "cons",
    "planned": "paper",
    "planned_facility": "paper",
    "pending": "paper",
    "approved": "paper",
    "by_right": "paper",
    "permanently_closed": "closed",
    "temporarily_closed": "closed",
    "no_operating_status_in_icis": "closed",
}

PAGE = Template("""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Map — Data Center Atlas</title>
<meta name="description" content="Every US data center the record can prove, on the map — with provenance.">
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Bricolage+Grotesque:opsz,wght@12..96,600;12..96,800&family=Newsreader:ital,opsz,wght@0,6..72,400;1,6..72,400&family=Spline+Sans+Mono:wght@400;500;600&display=swap">
<style>
  :root{ --paper:#F5F4EE; --card:#FFFFFF; --hair:#D8D5C8; --ink:#22303C; --soft:#5A6B79;
         --faint:#8A94A0; --amber:#C69A45; --amber-deep:#8F6D2A; --link:#0F4C7A;
         --hero:#7b5ea7; --recede:#B9C2CB; --border:#DFE3E1;
         --disp:'Bricolage Grotesque',system-ui,sans-serif; --serif:'Newsreader',Georgia,serif;
         --mono:'Spline Sans Mono',ui-monospace,Menlo,monospace; }
  *{box-sizing:border-box}
  body{margin:0; background:var(--paper); color:var(--ink); font-family:var(--serif); font-size:16px; line-height:1.55}
  .wrap{max-width:1020px; margin:0 auto; padding:48px 24px 64px}
  .eyebrow{font-family:var(--mono); font-size:11px; font-weight:600; letter-spacing:.18em; color:var(--amber-deep)}
  .eyebrow a{color:inherit; text-decoration:none}
  h1{font-family:var(--disp); font-weight:800; font-size:clamp(28px,5vw,44px); line-height:1.05; margin:10px 0 6px; text-wrap:balance}
  .lede{color:var(--soft); max-width:66ch; font-size:17px}
  .lede a{color:var(--link)}
  .frame{background:var(--card); border:1px solid var(--hair); border-radius:8px; margin-top:30px; overflow:hidden}
  .fhead{display:flex; flex-wrap:wrap; gap:10px; align-items:baseline; justify-content:space-between;
         padding:16px 20px 10px; border-bottom:1px solid var(--hair)}
  .fnum{font-family:var(--mono); font-size:11px; letter-spacing:.14em; color:var(--amber-deep)}
  .ftitle{font-family:var(--disp); font-weight:600; font-size:19px}
  .prov{font-family:var(--mono); font-size:10.5px; color:var(--amber-deep);
        border:1px solid var(--amber); border-radius:2px; padding:2px 8px; background:#FBF6EA}
  .chips{display:flex; flex-wrap:wrap; gap:8px; padding:12px 20px 4px}
  .chip{font-family:var(--mono); font-size:12px; background:var(--card); color:var(--soft);
        border:1px solid var(--hair); border-radius:3px; padding:6px 11px; cursor:pointer}
  .chip[aria-pressed="true"]{border-color:var(--hero); color:var(--hero); background:#F3EFF9; font-weight:600}
  .chip:focus-visible{outline:2px solid #5FA3D6; outline-offset:2px}
  .stage{position:relative; padding:8px 12px 14px}
  svg.map{display:block; width:100%; height:auto; background:#FDFDFB; border:1px solid #ECEAE2; border-radius:4px; cursor:grab}
  svg.map:active{cursor:grabbing}
  .zoomctl{position:absolute; top:18px; right:22px; display:flex; flex-direction:column; gap:5px}
  .zoomctl button{font-family:var(--mono); font-size:15px; width:30px; height:30px; border:1px solid var(--hair);
                  border-radius:4px; background:var(--card); color:var(--ink); cursor:pointer; line-height:1}
  .zoomctl button:hover{border-color:var(--hero); color:var(--hero)}
  .map-tip{position:absolute; z-index:5; max-width:280px; background:var(--card); border:1px solid var(--border);
           border-radius:8px; padding:10px 12px; box-shadow:0 6px 24px rgba(0,0,0,.14);
           pointer-events:none; font-size:12px; line-height:1.5; font-family:var(--mono)}
  .map-tip-head{font-weight:600; margin-bottom:4px; font-size:12.5px; color:var(--ink); font-family:var(--disp)}
  .map-tip-line{color:var(--soft); margin-top:2px}
  .map-tip .go{color:var(--link)}
  .m-nation{fill:var(--card); stroke:none}
  .m-state{fill:none; stroke:#CFD4D9; stroke-width:.9; vector-effect:non-scaling-stroke}
  .m-county{fill:none; stroke:#E4E7EA; stroke-width:.5; vector-effect:non-scaling-stroke}
  .m-study{fill:color-mix(in srgb, var(--hero) 7%, transparent); stroke:color-mix(in srgb, var(--hero) 50%, var(--card)); stroke-width:1.2; vector-effect:non-scaling-stroke}
  .m-neighbor{fill:#F8F8F5; stroke:#D6DADD; stroke-width:1}
  .m-dot{fill:color-mix(in srgb, var(--hero) 55%, transparent); stroke:var(--hero)}
  .m-dot.faded{fill:color-mix(in srgb, var(--recede) 30%, transparent); stroke:color-mix(in srgb, var(--recede) 75%, transparent)}
  .m-hit{fill:transparent; cursor:pointer}
  .m-town{fill:color-mix(in srgb, var(--faint) 75%, transparent)}
  .m-land-built{fill:color-mix(in srgb, var(--hero) 42%, transparent); stroke:var(--hero); stroke-width:1; cursor:pointer; vector-effect:non-scaling-stroke}
  .m-land-pend{fill:color-mix(in srgb, var(--hero) 20%, transparent); stroke:color-mix(in srgb, var(--hero) 70%, transparent); stroke-width:1; cursor:pointer; vector-effect:non-scaling-stroke}
  .m-land-plan{fill:color-mix(in srgb, var(--hero) 9%, transparent); stroke:color-mix(in srgb, var(--hero) 55%, transparent); stroke-width:1; stroke-dasharray:3 2; cursor:pointer; vector-effect:non-scaling-stroke}
  .m-bld{fill:var(--hero); stroke:#FFFFFF; stroke-width:.8; vector-effect:non-scaling-stroke}
  .m-label{paint-order:stroke; stroke:var(--card); stroke-width:3px; stroke-linejoin:round;
           font-family:var(--mono); font-weight:600; text-anchor:middle; pointer-events:none}
  .m-label.city{fill:var(--faint); font-weight:500; text-anchor:start}
  .m-label.county{fill:#B3BAC1; font-weight:500; letter-spacing:.08em}
  .m-label.anno{fill:var(--ink); font-weight:600; text-anchor:start}
  .m-leader{stroke:var(--faint); stroke-width:1; fill:none}
  .legend{display:flex; flex-wrap:wrap; gap:16px; padding:10px 20px 14px; font-family:var(--mono); font-size:11.5px; color:var(--soft)}
  .legend b{color:var(--ink); font-weight:500}
  .sw{display:inline-block; width:11px; height:11px; border-radius:50%; margin-right:6px; vertical-align:-2px}
  .sw.fill{background:color-mix(in srgb, var(--hero) 55%, transparent); border:1.5px solid var(--hero)}
  .sw.grey{background:color-mix(in srgb, var(--recede) 30%, transparent); border:1.5px solid var(--recede)}
  .swq{display:inline-block; width:12px; height:12px; margin-right:6px; vertical-align:-2px; border-radius:2px}
  .swq.built{background:color-mix(in srgb, var(--hero) 42%, transparent); border:1.5px solid var(--hero)}
  .swq.pend{background:color-mix(in srgb, var(--hero) 20%, transparent); border:1.5px solid color-mix(in srgb, var(--hero) 70%, transparent)}
  .swq.plan{background:color-mix(in srgb, var(--hero) 9%, transparent); border:1.5px dashed color-mix(in srgb, var(--hero) 55%, transparent)}
  .cap{font-family:var(--mono); font-size:10.5px; color:var(--faint); padding:0 20px 16px; line-height:1.7}
  .foot{font-family:var(--mono); font-size:10.5px; color:var(--faint); margin-top:36px; line-height:1.8}
  .foot a{color:var(--link)}
  @media (prefers-reduced-motion: reduce){ *{transition:none !important} }
</style>
</head>
<body>
<div class="wrap">
  <p class="eyebrow"><a href="../index.html">BLUE DOT</a> · <a href="index.html">DATA CENTER ATLAS</a> · MAP</p>
  <h1>$n_entities facilities — and the pipeline crowds one county.</h1>
  <p class="lede">Every mark is a facility the record can prove, drawn at its recorded
  coordinates. Hover for the record, click for the <a href="index.html">dossier</a>.
  No tile service, no runtime queries — the map is compiled from the fact store like
  every other page.</p>

  <section class="frame">
    <div class="fhead">
      <span><span class="fnum">01</span> &nbsp;<span class="ftitle">Where the record puts them</span></span>
      <span class="prov">vintage $echo_vintage · $pwc_vintage · $n_entities entities</span>
    </div>
    <div class="chips" id="chips" role="group" aria-label="Choose which lifecycle bucket is the hero"></div>
    <div class="stage" id="stage1">
      <svg id="nat" class="map" viewBox="0 0 975 610" aria-label="Interactive map of US data center facilities"></svg>
      <div class="zoomctl"><button id="zin" aria-label="Zoom in">+</button><button id="zout" aria-label="Zoom out">−</button><button id="zreset" aria-label="Reset view">⌂</button></div>
    </div>
    <div class="legend">
      <span><span class="sw fill"></span><b>hero bucket</b> (chip above)</span>
      <span><span class="sw grey"></span>everything else, receded</span>
      <span>scroll to zoom · drag to pan · hover for the record · click for the dossier</span>
    </div>
    <p class="cap" id="natcap"></p>
  </section>

  <section class="frame">
    <div class="fhead">
      <span><span class="fnum">02</span> &nbsp;<span class="ftitle">Plans are outstripping construction six to one</span></span>
      <span class="prov">vintage $pwc_vintage · Prince William County, VA</span>
    </div>
    <div class="stage" id="stage2">
      <svg id="pwc" class="map" viewBox="0 0 900 640" aria-label="Prince William County campus land bays by status, with recorded buildings"></svg>
    </div>
    <div class="legend">
      <span><b>shaded land = campus, by status</b></span>
      <span><span class="swq built"></span>completed</span>
      <span><span class="swq pend"></span>pending</span>
      <span><span class="swq plan"></span>planned</span>
      <span><span class="sw fill"></span>building on record</span>
      <span>scroll to zoom into the corridor · hover any shape</span>
    </div>
    <p class="cap">Each shaded shape is a campus land bay from the county GIS, colored by
    its project status — darker is more real — with the recorded buildings as dots on top.
    85.6M sqft is entitled through zoning against 13.2M standing. Coverage caveat, honestly
    held: zoning-application sites are point records; where one lacks a campus polygon it
    appears in the <a href="index.html">atlas directory</a> rather than as land here.</p>
  </section>

  <p class="foot">Marks from data/claims.parquet + entities.parquet (latest vintage per
  entity); land and boundary geometry are display fixtures with their own provenance
  (<a href="https://github.com/kwheeler27/bluedot/tree/main/geo">geo/</a> — Census
  cartographic boundaries, county GIS land bays), never the source of any figure.
  Store as of $as_of.</p>
</div>
<script src="https://cdnjs.cloudflare.com/ajax/libs/d3/7.9.0/d3.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/topojson/3.0.2/topojson.min.js"></script>
<script>
var TOPO = $topo;
var REGION = $region;
var CAMPI = $campi;
var PTS = $pts; // [lon, lat, bucket, stage, src, name, slug, gfa]
var TOWNS = [[-77.4753,38.7509,"Manassas"],[-77.6155,38.7959,"Gainesville"],
  [-77.6361,38.8123,"Haymarket"],[-77.2611,38.6582,"Woodbridge"],
  [-77.3711,38.6371,"Dale City"],[-77.3277,38.5679,"Dumfries"],[-77.5786,38.6998,"Nokesville"]];
var CITIES = [[-118.24,34.05,"Los Angeles",1],[-87.63,41.88,"Chicago",1],[-95.36,29.76,"Houston",1],
  [-112.07,33.45,"Phoenix",1],[-96.80,32.78,"Dallas",1],[-122.42,37.77,"San Francisco",1],
  [-122.33,47.61,"Seattle",1],[-104.99,39.74,"Denver",1],[-77.04,38.91,"Washington",1],
  [-84.39,33.75,"Atlanta",1],[-80.19,25.76,"Miami",1],[-74.01,40.71,"New York",1],
  [-71.06,42.36,"Boston",2],[-93.27,44.98,"Minneapolis",2],[-115.14,36.17,"Las Vegas",2],
  [-111.89,40.76,"Salt Lake City",2],[-122.68,45.52,"Portland",2],[-82.99,39.96,"Columbus",2],
  [-90.20,38.63,"St. Louis",2],[-86.16,39.77,"Indianapolis",2],[-97.52,35.47,"Oklahoma City",2],
  [-106.65,35.08,"Albuquerque",2],[-121.49,38.58,"Sacramento",2],[-78.64,35.78,"Raleigh",2],
  [-94.58,39.10,"Kansas City",2]];

// The counties TopoJSON is unprojected lon/lat — boundaries and points both
// go through this projection (the familiar AlbersUsa frame, AK/HI inset).
var albers = d3.geoAlbersUsa().scale(1300).translate([487.5, 305]);
var path = d3.geoPath(albers);
var nation = topojson.feature(TOPO, TOPO.objects.nation);
var countyMesh = topojson.mesh(TOPO, TOPO.objects.counties, function (a, b) { return a !== b; });

function tip(stageEl) {
  var el = document.createElement("div");
  el.className = "map-tip"; el.hidden = true;
  stageEl.appendChild(el);
  return {
    show: function (evt, title, lines) {
      var r = stageEl.getBoundingClientRect();
      el.innerHTML = "";
      var h = document.createElement("div"); h.className = "map-tip-head"; h.textContent = title;
      el.appendChild(h);
      lines.forEach(function (t, i) {
        var d = document.createElement("div");
        d.className = "map-tip-line" + (i === lines.length - 1 ? " go" : "");
        d.textContent = t; el.appendChild(d);
      });
      el.hidden = false;
      var x = evt.clientX - r.left + 14, y = evt.clientY - r.top + 8;
      if (x > r.width - 300) x -= 320;
      el.style.left = x + "px"; el.style.top = y + "px";
    },
    hide: function () { el.hidden = true; }
  };
}
function fmtSqft(n) { return n >= 1e6 ? (n / 1e6).toFixed(1) + "M sqft" : Math.round(n / 1e3) + "k sqft"; }
function stageLine(p) {
  return p[3].replace(/_/g, " ") + " · " + (p[4] === "EPA" ? "EPA air permit" : "county record") +
         (p[7] > 0 ? " · " + fmtSqft(p[7]) : "");
}
function openDossier(slug) { window.location.href = slug + ".html"; }

// ---------- frame 1: national ----------
(function () {
  var svg = d3.select("#nat");
  var tp = tip(document.getElementById("stage1"));
  var root = svg.append("g");
  root.append("path").datum(nation).attr("class", "m-nation").attr("d", path);
  root.append("path").datum(countyMesh).attr("class", "m-county").attr("d", path);
  root.append("path").datum(topojson.mesh(TOPO, TOPO.objects.states, function (a, b) { return a !== b; }))
      .attr("class", "m-state").attr("d", path);
  root.append("path").datum(topojson.mesh(TOPO, TOPO.objects.nation))
      .attr("class", "m-state").attr("d", path);

  var cityG = root.append("g");
  var proj = PTS.map(function (p) { return { p: p, xy: albers([p[0], p[1]]) }; });
  var off = proj.filter(function (d) { return !d.xy; }).length;
  var dots = root.append("g");
  var mode = "all";
  var counts = { all: 0, built: 0, cons: 0, paper: 0, closed: 0 };
  PTS.forEach(function (p) { counts.all++; counts[p[2]]++; });

  var marks = dots.selectAll("g").data(proj.filter(function (d) { return d.xy; })).enter()
    .append("g")
    .on("mousemove", function (evt, d) { tp.show(evt, d.p[5], [stageLine(d.p), "open dossier ↗"]); })
    .on("mouseleave", tp.hide)
    .on("click", function (evt, d) { openDossier(d.p[6]); });
  marks.append("circle").attr("class", "m-hit")
    .attr("cx", function (d) { return d.xy[0]; }).attr("cy", function (d) { return d.xy[1]; });
  var dotEls = marks.append("circle").attr("class", "m-dot")
    .attr("cx", function (d) { return d.xy[0]; }).attr("cy", function (d) { return d.xy[1]; });

  var nova = albers([-77.55, 38.8]);
  var anno = root.append("g");
  anno.append("path").attr("class", "m-leader")
    .attr("d", "M" + (nova[0] + 68) + "," + (nova[1] + 48) + " L" + (nova[0] + 10) + "," + (nova[1] + 10));
  var annoText = anno.append("text").attr("class", "m-label anno")
    .attr("x", nova[0] + 92).attr("y", nova[1] + 64).style("font-size", "11.5px").style("text-anchor", "end");
  annoText.append("tspan").text("Northern Virginia —");
  annoText.append("tspan").attr("x", nova[0] + 92).attr("dy", 13)
    .text(counts.all - PTS.filter(function (p) { return p[4] === "EPA"; }).length + " county records in one county");

  var k = 1;
  function restyle() {
    var rBase = (mode === "all" ? 2.5 : 3) / Math.sqrt(k);
    dotEls
      .attr("class", function (d) { return "m-dot" + (mode !== "all" && d.p[2] !== mode ? " faded" : ""); })
      .attr("r", function (d) {
        return mode !== "all" && d.p[2] !== mode ? 1.8 / Math.sqrt(k) : rBase * (d.p[2] === mode ? 1.25 : 1);
      })
      .style("stroke-width", 1 / k);
    marks.select(".m-hit").attr("r", Math.max(7 / k, rBase * 1.6));
    cityG.selectAll("*").remove();
    CITIES.forEach(function (c) {
      if (c[3] === 2 && k < 2.4) return;
      var xy = albers([c[0], c[1]]);
      if (!xy) return;
      cityG.append("circle").attr("class", "m-town").attr("cx", xy[0]).attr("cy", xy[1]).attr("r", 1.7 / k);
      cityG.append("text").attr("class", "m-label city").attr("x", xy[0] + 4.5 / k).attr("y", xy[1] + 3 / k)
        .style("font-size", (k >= 2.4 ? 10.5 : 9.5) / k + "px").text(c[2]);
    });
    anno.style("opacity", k < 1.8 ? 1 : 0);
  }
  var zoom = d3.zoom().scaleExtent([1, 16])
    .on("zoom", function (e) { root.attr("transform", e.transform); k = e.transform.k; restyle(); });
  svg.call(zoom);
  d3.select("#zin").on("click", function () { svg.transition().duration(250).call(zoom.scaleBy, 1.7); });
  d3.select("#zout").on("click", function () { svg.transition().duration(250).call(zoom.scaleBy, 1 / 1.7); });
  d3.select("#zreset").on("click", function () { svg.transition().duration(350).call(zoom.transform, d3.zoomIdentity); });

  var LABEL = { built: "built & running", cons: "under construction", paper: "paper pipeline", closed: "closed / inactive" };
  var chipbox = d3.select("#chips");
  ["all", "built", "cons", "paper", "closed"].forEach(function (kkey) {
    chipbox.append("button").attr("class", "chip").attr("type", "button")
      .attr("aria-pressed", String(kkey === mode))
      .text((kkey === "all" ? "all" : LABEL[kkey]) + " · " + (counts[kkey] || 0))
      .on("click", function () {
        mode = kkey;
        chipbox.selectAll(".chip").attr("aria-pressed", "false");
        d3.select(this).attr("aria-pressed", "true");
        restyle();
      });
  });
  restyle();
  document.getElementById("natcap").textContent =
    "Census county boundaries as whisper ground, states one step darker — the Albers frame, Alaska and " +
    "Hawaii in their insets. " + off + " facilities fall outside the projection (Puerto Rico) and are in " +
    "the atlas directory. Zoom in: city labels arrive, the annotation steps aside, dots keep their size " +
    "so the clusters open up.";
})();

// ---------- frame 2: county land bays ----------
(function () {
  var svg = d3.select("#pwc");
  var svgg = svg.append("g");
  var tp = tip(document.getElementById("stage2"));
  var W = 900, Hh = 640;
  var KX = Math.cos(38.7 * Math.PI / 180);
  var pwcF = { type: "Feature", geometry: REGION.pwc, properties: {} };
  // fit the frame to the land bays themselves (zoom-dependent detail)
  var cb = [[Infinity, Infinity], [-Infinity, -Infinity]];
  CAMPI.forEach(function (c) { c[4].forEach(function (ring) { ring.forEach(function (pt) {
    cb[0][0] = Math.min(cb[0][0], pt[0]); cb[0][1] = Math.min(cb[0][1], pt[1]);
    cb[1][0] = Math.max(cb[1][0], pt[0]); cb[1][1] = Math.max(cb[1][1], pt[1]); }); }); });
  var padx = (cb[1][0] - cb[0][0]) * 0.10, pady = (cb[1][1] - cb[0][1]) * 0.10;
  var bb = [[cb[0][0] - padx, cb[0][1] - pady], [cb[1][0] + padx, cb[1][1] + pady]], mrg = 8;
  var s2 = Math.min((W - 2 * mrg) / (bb[1][0] - bb[0][0]), (Hh - 2 * mrg) / (bb[1][1] - bb[0][1]));
  var tx2 = (W - s2 * (bb[0][0] + bb[1][0])) / 2, ty2 = (Hh - s2 * (bb[0][1] + bb[1][1])) / 2;
  var t2 = d3.geoTransform({ point: function (x, y) { this.stream.point(x * s2 + tx2, y * s2 + ty2); } });
  var path2 = d3.geoPath(t2);
  var proj2 = function (ll) { return [ll[0] * KX * s2 + tx2, -ll[1] * s2 + ty2]; };
  var ENCLAVES = { "MANASSAS": true, "MANASSAS PARK": true };

  REGION.neighbors.forEach(function (n) {
    if (ENCLAVES[n[0]]) return;
    svgg.append("path").datum({ type: "Feature", geometry: n[1] })
      .attr("class", "m-neighbor").attr("d", path2);
  });
  svgg.append("path").datum(pwcF).attr("class", "m-study").attr("d", path2);
  REGION.neighbors.forEach(function (n) {
    if (!ENCLAVES[n[0]]) return;
    svgg.append("path").datum({ type: "Feature", geometry: n[1] })
      .attr("class", "m-neighbor").attr("d", path2);
  });

  [["LOUDOUN", -77.63, 38.945], ["FAIRFAX", -77.27, 38.87], ["FAUQUIER", -77.685, 38.64],
   ["STAFFORD", -77.44, 38.525], ["CHARLES (MD)", -77.17, 38.555]].forEach(function (n) {
    var xy = proj2([n[1], n[2]]);
    svgg.append("text").attr("class", "m-label county").attr("x", xy[0]).attr("y", xy[1])
      .style("font-size", "10.5px").text(n[0]);
  });
  TOWNS.forEach(function (tw) {
    var xy = proj2([tw[0], tw[1]]);
    svgg.append("rect").attr("class", "m-town")
      .attr("x", xy[0] - 1.6).attr("y", xy[1] - 1.6).attr("width", 3.2).attr("height", 3.2);
    svgg.append("text").attr("class", "m-label city").attr("x", xy[0] + 5).attr("y", xy[1] + 3.5)
      .style("font-size", "10px").text(tw[2]);
  });

  // campus land bays, least real first so completed land sits on top
  var ORDER = { planned: 0, pending: 1, completed: 2 };
  var CLS = { completed: "m-land-built", pending: "m-land-pend", planned: "m-land-plan" };
  var campi = CAMPI.slice().sort(function (a, b) { return ORDER[a[1]] - ORDER[b[1]]; });
  campi.forEach(function (c) {
    svgg.append("path")
      .datum({ type: "Feature", geometry: { type: "Polygon", coordinates: c[4] } })
      .attr("class", CLS[c[1]]).attr("d", path2)
      .on("mousemove", function (evt) {
        tp.show(evt, c[0], ["campus land bay · " + c[1] + (c[2] > 0 ? " · " + fmtSqft(c[2]) + " planned" : ""), "open dossier ↗"]);
      })
      .on("mouseleave", tp.hide)
      .on("click", function () { openDossier("pwc-campus-" + c[3]); });
  });
  // the buildings the county has on record, as dots over the land
  PTS.filter(function (p) { return p[6].indexOf("pwc-bld-") === 0; }).forEach(function (p) {
    var xy = proj2([p[0], p[1]]);
    svgg.append("circle").attr("class", "m-bld").attr("cx", xy[0]).attr("cy", xy[1]).attr("r", 2)
      .style("cursor", "pointer")
      .on("mousemove", function (evt) { tp.show(evt, p[5], [stageLine(p), "open dossier ↗"]); })
      .on("mouseleave", tp.hide)
      .on("click", function () { openDossier(p[6]); });
  });
  // annotation computed from the data: the biggest planned campus
  var big = campi.filter(function (c) { return c[1] === "planned"; })
    .sort(function (a, b) { return b[2] - a[2]; })[0];
  if (big) {
    var bc = path2.centroid({ type: "Feature", geometry: { type: "Polygon", coordinates: big[4] } });
    svgg.append("path").attr("class", "m-leader")
      .attr("d", "M" + (bc[0] + 52) + "," + (bc[1] - 40) + " L" + (bc[0] + 8) + "," + (bc[1] - 8));
    var a = svgg.append("text").attr("class", "m-label anno")
      .attr("x", bc[0] + 56).attr("y", bc[1] - 44).style("font-size", "11.5px");
    a.append("tspan").text(big[0] + " —");
    a.append("tspan").attr("x", bc[0] + 56).attr("dy", 13)
      .text(fmtSqft(big[2]) + " planned on this land");
  }
  svg.call(d3.zoom().scaleExtent([1, 10])
    .on("zoom", function (e) { svgg.attr("transform", e.transform); }));
})();
</script>
</body>
</html>
""")


def _js(value: object) -> str:
    # "<" escaped so no string can close the script tag (page.py convention)
    return json.dumps(value, separators=(",", ":")).replace("<", "\\u003c")


def build_map_page(con, geo_dir: Path, slugs: dict[str, str], as_of: str) -> str:
    """Render dc/map.html. `con` has the claims/entities views; `slugs` is the
    dossier slug per entity (the map must never link to a page that does not
    exist this build)."""
    for name in ("us-counties-topo.json", "pwc-region-planar.json", "pwc-campus-planar.json"):
        if not (geo_dir / name).exists():
            raise SystemExit(f"{geo_dir / name} not found — display geometry fixtures are required for the map page")

    rows = con.execute(
        """
        WITH e AS (
          SELECT entity_id, name, lat, lon, source_dataset,
                 row_number() OVER (PARTITION BY entity_id ORDER BY vintage DESC) rn
          FROM entities WHERE entity_id IN (SELECT DISTINCT entity_id FROM claims)),
        s AS (
          SELECT entity_id, value_text AS stage,
                 row_number() OVER (PARTITION BY entity_id ORDER BY vintage DESC) rn
          FROM claims WHERE attribute_id IN ('dc:stage','dc:zoning_status')),
        g AS (SELECT entity_id, max(value_num) gfa FROM claims
              WHERE attribute_id IN ('dc:gfa_sqft','dc:gfa_planned_sqft') GROUP BY 1)
        SELECT e.entity_id, e.name, e.lat, e.lon, s.stage, e.source_dataset, coalesce(g.gfa, 0)
        FROM e LEFT JOIN s ON e.entity_id = s.entity_id AND s.rn = 1
        LEFT JOIN g ON e.entity_id = g.entity_id
        WHERE e.rn = 1 AND e.lat IS NOT NULL ORDER BY e.entity_id
        """
    ).fetchall()
    pts = []
    for entity_id, name, lat, lon, stage, src, gfa in rows:
        if stage is None or stage not in BUCKET:
            raise SystemExit(
                f"{entity_id}: stage {stage!r} has no display bucket — extend BUCKET deliberately"
            )
        pts.append([
            round(lon, 4), round(lat, 4), BUCKET[stage], stage,
            "EPA" if src == "epa/echo/air" else "county",
            name[:52], slugs[entity_id], int(gfa),
        ])

    campi = json.loads((geo_dir / "pwc-campus-planar.json").read_text())
    missing = [c[3] for c in campi if f"pwc/campus/{c[3]}" not in slugs]
    if missing:
        raise SystemExit(
            f"{len(missing)} campus land bays have no dossier in this build (e.g. {missing[:3]}) — "
            "re-run `bluedot ingest pwc` + build-facts so the store matches geo/pwc-campus-planar.json"
        )

    def vintage(source: str) -> str:
        got = con.execute(
            "SELECT max(vintage) FROM claims WHERE source_dataset = ?", [source]
        ).fetchone()[0]
        if got is None:
            raise SystemExit(f"no claims for source {source!r} — refusing to bake the map")
        return got

    esc = lambda v: html.escape(str(v), quote=True)  # noqa: E731
    return PAGE.substitute(
        n_entities=esc(f"{len(pts):,}"),
        echo_vintage=esc(vintage("epa/echo/air")),
        pwc_vintage=esc(vintage("pwcva/build-out-analysis")),
        as_of=esc(as_of),
        topo=(geo_dir / "us-counties-topo.json").read_text().replace("<", "\\u003c"),
        region=(geo_dir / "pwc-region-planar.json").read_text().replace("<", "\\u003c"),
        campi=_js(campi),
        pts=_js(pts),
    )
