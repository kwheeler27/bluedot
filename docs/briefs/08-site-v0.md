# Brief 08 — Blue Dot v0 on Vercel

**What.** A public static site compiled from the fact store: a front door,
a curated set of fact pages (the vintage ladders), and the Data Center
Atlas — an overview page plus a dossier page for every one of the 838
facility entities. Built by a new `bluedot-atlas site` command; deployed
to Vercel.

**Why.** Kevin asked "when will this be live on Vercel?" — everything so
far ships to a terminal. Once a URL exists, every future slice lands
somewhere visible, and the compiled-page architecture (mockup 03 made
real) becomes demonstrable instead of describable. Traces to the vision:
the atlas page is generated per question — this ships the generated pages.

**Use cases.** Kevin showing the project to someone ("here's the LA County
ladder — watch the number change as you move the knowledge date"). A
reader landing on the NTT VA10 dossier and seeing two government sources
disagree, with provenance. Future slices (explore canvas, more sources)
deploying onto an existing surface.

**Proposed solution.**
- `atlas/src/bluedot_atlas/site.py` — compiles `site/` from the Parquet
  store: `index.html`, `facts/*.html` (curated list, reusing the existing
  fact-page compiler), `dc/index.html` (baked pipeline tables + a
  client-side-filterable directory of all entities), `dc/<slug>.html`
  (one dossier per entity: registry identity + every claim, all vintages,
  with stated_by/record/confidence; `dc:same_as` links cross-link the two
  dossiers). Same design language as the fact pages (paper ground, mono
  figures, amber = knowledge time).
- **All claims shown, all vintages** — the dossier is a knowledge-time
  record, not a summary. Collapsing identical values across accumulating
  monthly vintages is a later slice, revisited when it hurts.
- Escaping discipline as in page.py: every dynamic string HTML-escaped
  (claim values are scraped government text), DOM-built JS, external
  hrefs only when `https://`. Slugs from entity ids are charset-checked
  and collision-checked — a bad id fails the build loudly.
- Byte-stable like the fact pages: no wall-clock timestamps in output;
  "compiled from" dates come from the store itself.
- **Deploy: local build → Vercel static deploy** (no build on Vercel, no
  framework, no JS toolchain — the explore canvas decides that later, per
  its own brief). `site/` is gitignored; the generator is the source of
  truth. Moving the build+deploy into CI is an explicit follow-up.

**Out of scope.** Maps and the explore canvas (own brief, own toolchain
decision); dossiers for ACS/PEP-only entities (3k counties — the curated
fact pages cover the demo need); search beyond the client-side directory
filter; a custom domain (Bluedot trademark check pending, per the main
brief).

**Deliverables.** site.py + CLI wiring + unittest coverage (escaping,
slug collisions, mini-store build); live build over the real store;
adversarial review; Vercel deploy; URL in the README.
