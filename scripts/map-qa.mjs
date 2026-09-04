/**
 * Map QA — render the compiled map and fail loudly if it stops working.
 *
 * Basin's practice, adopted (its CLAUDE.md: "any change to the story map
 * requires pnpm qa:map ... before deploying"). Blue Dot needs it more, not
 * less: deploys are automatic now (decision 2026-09-04), so a map that
 * renders nothing would publish unattended. The compilers already fail
 * loudly on bad data; this is the equivalent check for bad *rendering*.
 *
 *   node scripts/map-qa.mjs [site-dir]     # default: ./site
 *
 * Writes screenshots to .qa/ for a human to look at, and exits non-zero on
 * a console error, a missing mark class, or a count below its floor.
 */
import { chromium } from "playwright";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const siteDir = process.argv[2] ?? "site";
const page_url = pathToFileURL(resolve(siteDir, "dc/map.html")).href;
const OUT = ".qa";

// Floors, not exact counts: the store grows, and a QA harness that fails on
// every new facility teaches people to ignore it. These catch the failure
// that matters — a frame rendering nothing.
// Floors, not exact counts: the store grows, and a harness that fails on
// every new facility teaches people to ignore it. `painted` floors catch the
// failure that matters — a frame that renders nothing — and `spread` floors
// catch marks that render collapsed onto one spot.
const FLOORS = {
  natDots: { painted: 400, spread: 400 },
  natGround: { painted: 1, spread: 600 },
  pwcLand: { painted: 40, spread: 300 },
  pwcBlds: { painted: 100, spread: 300 },
  pwcGround: { painted: 1, spread: 400 },
  chips: { count: 5 },
};

mkdirSync(OUT, { recursive: true });
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1280, height: 950 } });
const errors = [];
page.on("pageerror", (e) => errors.push(`pageerror: ${e.message}`));
page.on("console", (m) => m.type() === "error" && errors.push(`console: ${m.text()}`));

await page.goto(page_url, { waitUntil: "networkidle" });
await page.waitForTimeout(1000);

// Counting elements is not enough: a mark can exist in the DOM and draw
// nothing (a path that lost its `d`, a circle with no radius, geometry
// projected off-canvas). Verified by breaking a real map — the count-only
// version passed it. So every mark class is also measured: how many of
// them actually paint a non-empty box, and how big the painted region is.
const measure = () => {
  const seen = (sel) => {
    const els = Array.from(document.querySelectorAll(sel));
    let painted = 0;
    let box = null;
    for (const el of els) {
      let b;
      try {
        b = el.getBBox();
      } catch {
        continue; // not rendered at all
      }
      if (b.width <= 0 && b.height <= 0) continue;
      painted++;
      // The accumulator carries w/h (not width/height): mixing the two
      // names made every span NaN, and `NaN < floor` is false, so this
      // check silently never fired. Caught by testing the harness against
      // a deliberately broken map.
      const x0 = box ? Math.min(box.x, b.x) : b.x;
      const y0 = box ? Math.min(box.y, b.y) : b.y;
      const x1 = box ? Math.max(box.x + box.w, b.x + b.width) : b.x + b.width;
      const y1 = box ? Math.max(box.y + box.h, b.y + b.height) : b.y + b.height;
      box = { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
    }
    return { count: els.length, painted, w: Math.round(box?.w ?? 0), h: Math.round(box?.h ?? 0) };
  };
  return {
    natDots: seen("#nat .m-dot"),
    natGround: seen("#nat .m-county"),
    pwcLand: seen("#pwc [class^='m-land']"),
    pwcBlds: seen("#pwc .m-bld"),
    pwcGround: seen("#pwc .m-study"),
    chips: { count: document.querySelectorAll(".chip").length, painted: 0, w: 0, h: 0 },
  };
};
const counts = await page.evaluate(measure);

await page.locator("#stage1").screenshot({ path: `${OUT}/national.png` });
await page.locator("#stage2").screenshot({ path: `${OUT}/county.png` });

// The interactions the map exists for: a tooltip on hover, a working filter.
await page.locator("#pwc .m-bld").last().hover({ force: true });
await page.waitForTimeout(250);
const tipText = await page.locator("#stage2 .map-tip").innerText().catch(() => "");
await page.locator("#stage2").screenshot({ path: `${OUT}/county-hover.png` });

await page.locator(".chip", { hasText: "paper" }).click();
await page.waitForTimeout(200);
const faded = await page.evaluate(() => document.querySelectorAll("#nat .m-dot.faded").length);
await page.locator("#stage1").screenshot({ path: `${OUT}/national-hero.png` });

await browser.close();

const failures = [];
for (const [key, floor] of Object.entries(FLOORS)) {
  const m = counts[key];
  if (floor.count !== undefined && m.count < floor.count) {
    failures.push(`${key}: ${m.count} elements, expected at least ${floor.count}`);
  }
  if (floor.painted !== undefined && m.painted < floor.painted) {
    failures.push(
      `${key}: only ${m.painted} of ${m.count} marks actually paint anything ` +
        `(expected at least ${floor.painted}) — present in the DOM, drawing nothing`
    );
  }
  const span = Math.max(m.w, m.h);
  if (floor.spread !== undefined && !(span >= floor.spread)) {
    failures.push(`${key}: painted region is ${m.w}x${m.h}, collapsed or unmeasurable (expected a span of ${floor.spread}+)`);
  }
}
if (!tipText.trim()) failures.push("hovering a building produced no tooltip");
if (faded === 0) failures.push("the hero chip faded nothing — the filter is not wired");
failures.push(...errors);

const summary = Object.entries(counts)
  .map(([k, m]) => `${k}=${m.painted}/${m.count}`)
  .join(" ");
console.log(`map QA: ${summary} (painted/present), tooltip=${tipText ? "yes" : "no"}, faded=${faded}`);
if (failures.length) {
  console.error("map QA FAILED:\n  " + failures.join("\n  "));
  process.exit(1);
}
console.log(`map QA passed — screenshots in ${OUT}/`);
