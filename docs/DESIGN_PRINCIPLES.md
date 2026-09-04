# Design principles — Blue Dot

Seeded 2026-09-04 from `~/projects/DESIGN_PRINCIPLES.md` (the cross-project
doctrine, distilled from Basin), then bound to the atlas/almanac domain.
This doc may specialize and extend the shared file; it may not contradict
it. Lessons that generalize get promoted upward (with Kevin's read), never
silently.

Blue Dot binds early and deliberately: few surfaces exist yet, so most
bindings below constrain the **data model and the compiler**, which is
where the product's honesty is actually decided. Sections marked
**[bound at first surface]** are stubs on purpose — inventing a hue
registry or a badge set before a recurring UI exists would be decoration,
not doctrine.

**Rule of use in this repo.** Stage 0 reviews check new work against
**§4–§6** — those three bind the data-model decisions being made right
now (fact schema, claims schema, semantic layer, compilers). The other
sections bind as surfaces appear.

The two failure modes, atlas edition. **Distortion**: a population trend
drawn across a boundary change or an accounting switch — intuitive, and
wrong. **Paralysis**: drowning every figure in caveats until the reader
gives up. The bitemporal store exists to let us be precise *and* legible.

## 1. Headings lead with the finding, not the label

Already practiced (the map: "Plans are outstripping construction six to
one", computed at build time with a refuse-to-ship guard when the claim
can't be recomputed). Binding: a compiled page's headline claim is
**computed from the store, never typed**, and the build fails loudly when
the data stops supporting it. "Self-retracting" in Blue Dot is not a
copywriting discipline — it is a build error.

## 2. Plain language, no prior knowledge assumed

The reader is smart but new to public statistics: "vintage", "5-year
estimate", "margin of error", "boundary year", "postcensal" each get
taught at first use. The single glossary layer is **[bound at first
surface]** — until it exists, definitions live inline where the term
first appears on each compiled page (the fact page's indicator definition
line is the seed of it).

## 3. Neutral register

The atlas states what the record says and attributes everything else —
already load-bearing in the Data Center Atlas, where the NTT VA10 dossier
*shows* two agencies disagreeing without adjudicating them, and the map
annotation states the entitlement gap without imputing motive (stalled
projects, land banking, and demand narratives are the reader's inferences
to draw, not ours to assert). Test unchanged: every sentence survives
being read aloud by the county, the EPA, or the operator it describes.

## 4. Every number carries its provenance — **Blue Dot's spine**

The shared principle says provenance travels with every displayed number.
In Blue Dot this is not a captioning convention; **it is the schema**.
The fact key `(entity_id, indicator_id, valid_time, vintage)` and the
claim key `(entity_id, attribute_id, valid_time, vintage, source_record)`
*are* "every number carries its provenance," enforced at write time —
a number without source, vintage, and geographic definition cannot exist
in the store (ADR-0001, ADR-0010, ADR-0015).

Bindings beyond the store:

- **Any future UI surfaces vintage/as-of, not just the store.** Every
  compiled surface shows which vintage the reader is looking at and
  offers the knowledge-time dimension where it matters (the fact page's
  vintage ladder is the reference implementation; the map's vintage chip
  is the minimum). A surface that hides vintage is wrong even when its
  numbers are right.
- **Epistemic class is visually encoded.** Blue Dot's classes are richer
  than most products': the claims store's confidence tiers
  (`confirmed_by_record` / `reported` / `rumored` / `inferred`,
  ADR-0015/0016) and the fact store's estimate machinery (ACS margins of
  error and annotation codes). Binding now: `inferred` claims (e.g.
  cross-source links) always render visibly distinct from source
  assertions, with their evidence shown — the dossier link-box pattern.
  The full visual grammar (solid/dashed/reference-line equivalents) is
  **[bound at first surface]** for charts, but no future chart may draw
  an estimate and a count in the same undifferentiated line style.
- **Amber is knowledge time.** The design language already reserves
  amber for vintage/as-of chrome; it never colors data.

## 5. Never mix accountings without a declared bridge

The atlas's accountings of "population" are genuinely different numbers:

- **ACS estimates** (`acs:*`): 5-year rolling survey estimates with
  margins of error, published ~1 year after the window closes;
- **Decennial counts** (future `dec:*`): enumeration, no MOE, once a
  decade;
- **PEP postcensal estimates** (`pep:*`): base-plus-components
  bookkeeping, revised back every vintage.

They never share an axis, a sum, a trend line, or an implied comparison
without a **declared bridge**. The store already enforces the first half:
indicators are namespaced by accounting (`acs:B01003_001` vs
`pep:POPESTIMATE` are different indicator_ids that cannot silently
collide). The semantic layer inherits the second half as a hard rule: a
query mixing accountings without a declared bridge **fails loudly**
(ADR-0005) — it is exactly the "plausible number" the layer exists to
refuse. When a bridge is declared, the caption computes the
reconciliation; it never just asserts it.

Margins of error are an **epistemic class to encode visually (§4), not a
footnote**: the fact schema carries `moe` and `moe_annotation` as
first-class columns, and any surface that shows an ACS estimate shows its
uncertainty in the visual grammar, not in fine print.

The Data Center Atlas already lives by this section: four GFA figures per
building (assessed / approved / permitted / taxed) are four attributes
never merged; county stages and EPA stages are bucketed for display but
never merged in the store; the map's entitled-vs-standing claim names its
two accountings (zoning sites vs completed buildings) in the caption.

## 6. Missing data renders as a gap, never as a guess

Suppressed or unavailable ACS cells render as **gaps carrying the
suppression reason, never zeros**. The store is built for this: Census
annotation codes are captured in `value_annotation` / `moe_annotation`
(the sentinel table refuses unknown codes at ingest), so the *reason* a
cell is missing is data, not a shrug. Bindings:

- A fact page whose value is annotated shows the annotation text where
  the number would be (already implemented: the figure slot renders the
  annotation, never 0 or a dash alone).
- A future chart with a suppressed period breaks the line and names why
  in the caption. A summed series with a suppressed member is a gap, not
  a smaller number.
- The county's 0-means-unset convention (PWC GFA fields) is handled at
  ingest — 0-as-null fields never become claims — with documented
  exceptions where 0 is real (`RemainingGFA`). Never re-learn this per
  surface.

## 7. Chart mechanics

Binds as charts appear; the map decision record's principles (one
saturated hue with everything else receding, halo'd labels, honest
aspect, zoom-dependent detail) are the current partial binding. The
**entity hue registry is [bound at first surface]**: today no entity
recurs across enough charts to need a fixed hue; the first surface where
one does (likely the explore canvas or multi-county fact pages) writes
the registry, in CSS, with dark-mode variants — per the shared rule.
Until then: one hero hue per view, amber reserved for knowledge time,
refusal red reserved for semantic-layer errors.

## 8. Information architecture

The compiled site already follows the shared shapes: the front door owns
the pitch; dossiers are the truth surface ("the map navigates; the
dossier is the truth surface" — the map decision record's phrasing of
this section); the atlas index is audit-adjacent; fact pages are
evidence. Binding: **URLs are commitments** — dossier slugs and fact-page
filenames are permanent (the slug scheme and `fact_page_filename` are
single-sourced in code for exactly this reason); restructures redirect,
never break. Surface kinds beyond these are declared before they get a
home.

## 9. One visual grammar per product

The identity is set (Bricolage Grotesque / Newsreader / Spline Sans Mono;
paper ground; amber = knowledge time; light cartographic maps per the map
decision record). The **badge set and teaching affordances are [bound at
first surface]**: the dossier's confidence column and the fact page's
"registry pending" chip are the first two badge-like marks — when a third
appears, the closed set gets written down and these two join it
retroactively. Screenshot test unchanged: any two compiled pages side by
side read as one product.

## 10. The product never claims more authority than it has

Blue Dot is an independent compilation of public records, and says so on
the surfaces where it matters. Already load-bearing: the map's coverage
caveats ("capacity in compute or storage is never public"; sites without
land geometry named as such), the dossier's stance that claims are
per-source assertions never merged, and the linkage tier that marks Blue
Dot's own inferences as `inferred` with evidence attached. Binding:
when two sources disagree, Blue Dot **shows the disagreement** (NTT VA10
is the reference case) — it never picks a winner silently. When Blue Dot
itself computes something (links, buckets, ratios), the method rides
along: in `stated_by`, in the caption, or in the decision record the
page links.
