"""Site compiler tests (brief 08) — escaping, linking, slugs, stability.

Builds a miniature Parquet store in a temp dir (two linked DC entities with
deliberately hostile strings, one fact ladder) and compiles the real site
over it, with CURATED_FACTS patched to the mini store's one fact key.
"""

import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import duckdb

from bluedot_atlas import site
from bluedot_atlas.site import build_site, entity_slug

HOSTILE_NAME = '<script>alert(1)</script> "Quoted" DC'
HOSTILE_VALUE = "</td></table><script>x</script>"


def _mini_store(data_dir: Path, include_echo: bool = True, ghost_claim: bool = False) -> None:
    con = duckdb.connect()
    con.execute(
        """
        CREATE TABLE entities (entity_id VARCHAR, name VARCHAR, level VARCHAR,
          boundary_year INTEGER, vintage VARCHAR, source_dataset VARCHAR,
          lat DOUBLE, lon DOUBLE);
        INSERT INTO entities VALUES
          ('pwc/bld/aaa', ?, 'facility', 2026, 'pwc-2026-09-01', 'pwcva/build-out-analysis', 38.7, -77.5),
          ('pwc/bld/aaa', ?, 'facility', 2026, 'pwc-2026-09-02', 'pwcva/build-out-analysis', 38.7, -77.5),
          ('pwc/bld/bbb', 'BUILDING TWO', 'facility', 2026, 'pwc-2026-09-02', 'pwcva/build-out-analysis', 38.71, -77.51),
          ('frs/123', 'EPA FACILITY ONE', 'facility', 2026, 'echo-2026-09-02', 'epa/echo/air', 38.7001, -77.5001),
          ('geoId/99', 'Testonia County', 'county', 2023, 'pep-2024', 'census/pep/co-est', NULL, NULL);
        """,
        ["OLD NAME", HOSTILE_NAME],
    )
    con.execute(
        """
        CREATE TABLE claims (entity_id VARCHAR, attribute_id VARCHAR, valid_from DATE,
          valid_to DATE, vintage VARCHAR, source_record VARCHAR, value_text VARCHAR,
          value_num DOUBLE, unit VARCHAR, stated_by VARCHAR, confidence VARCHAR,
          published_at DATE, source_dataset VARCHAR, source_url VARCHAR, retrieved_at TIMESTAMP);
        INSERT INTO claims VALUES
          ('pwc/bld/aaa', 'dc:recorded_name', DATE '2026-09-02', DATE '2026-09-03', 'pwc-2026-09-02',
           'aaa', ?, NULL, NULL, 'Prince William County Planning GIS', 'confirmed_by_record',
           DATE '2026-09-02', 'pwcva/build-out-analysis', 'https://example.gov/q', TIMESTAMP '2026-09-02 12:00:00'),
          ('pwc/bld/aaa', 'dc:stage', DATE '2026-09-02', DATE '2026-09-03', 'pwc-2026-09-02',
           'aaa', ?, NULL, NULL, 'Prince William County Planning GIS', 'confirmed_by_record',
           DATE '2026-09-02', 'pwcva/build-out-analysis', 'https://example.gov/q', TIMESTAMP '2026-09-02 12:00:00'),
          ('pwc/bld/aaa', 'dc:gfa_sqft', DATE '2026-09-02', DATE '2026-09-03', 'pwc-2026-09-02',
           'aaa', NULL, 165230.0, 'sqft', 'PWC (Real Estate Assessments)', 'confirmed_by_record',
           DATE '2026-09-02', 'pwcva/build-out-analysis', 'https://example.gov/q', TIMESTAMP '2026-09-02 12:00:00'),
          ('pwc/bld/bbb', 'dc:stage', DATE '2026-09-02', DATE '2026-09-03', 'pwc-2026-09-02',
           'bbb', 'completed', NULL, NULL, 'Prince William County Planning GIS', 'confirmed_by_record',
           DATE '2026-09-02', 'pwcva/build-out-analysis', 'https://example.gov/q', TIMESTAMP '2026-09-02 12:00:00');
        """,
        [HOSTILE_NAME, HOSTILE_VALUE],
    )
    if include_echo:
        con.execute(
            """
            INSERT INTO claims VALUES
              ('frs/123', 'dc:stage', DATE '2026-09-02', DATE '2026-09-03', 'echo-2026-09-02',
               '123', 'operating', NULL, NULL, 'EPA ECHO', 'confirmed_by_record',
               DATE '2026-09-02', 'epa/echo/air', 'https://example.gov/echo', TIMESTAMP '2026-09-02 12:00:00'),
              ('frs/123', 'dc:state', DATE '2026-09-02', DATE '2026-09-03', 'echo-2026-09-02',
               '123', 'VA', NULL, NULL, 'EPA ECHO', 'confirmed_by_record',
               DATE '2026-09-02', 'epa/echo/air', 'https://example.gov/echo', TIMESTAMP '2026-09-02 12:00:00'),
              ('pwc/bld/aaa', 'dc:same_as', DATE '2026-09-02', DATE '2026-09-03', 'link-2026-09-02',
               'frs/123', 'frs/123', NULL, NULL, 'bluedot linkage v0: 30m apart', 'inferred',
               DATE '2026-09-02', 'bluedot/linkage-v0', 'https://example.gov/q', TIMESTAMP '2026-09-02 12:00:00'),
              ('pwc/bld/bbb', 'dc:same_as', DATE '2026-09-02', DATE '2026-09-03', 'link-2026-09-02',
               'frs/123', 'frs/123', NULL, NULL, 'bluedot linkage v0: 55m apart', 'inferred',
               DATE '2026-09-02', 'bluedot/linkage-v0', 'https://example.gov/q', TIMESTAMP '2026-09-02 12:00:00');
            """
        )
    if ghost_claim:
        con.execute(
            """
            INSERT INTO claims VALUES
              ('ghost/1', 'dc:stage', DATE '2026-09-02', DATE '2026-09-03', 'echo-2026-09-02',
               'g1', 'operating', NULL, NULL, 'EPA ECHO', 'confirmed_by_record',
               DATE '2026-09-02', 'epa/echo/air', 'https://example.gov/echo', TIMESTAMP '2026-09-02 12:00:00');
            """
        )
    con.execute(
        """
        CREATE TABLE facts (entity_id VARCHAR, indicator_id VARCHAR, valid_from DATE,
          valid_to DATE, vintage VARCHAR, published_at DATE, value DOUBLE, moe DOUBLE,
          value_annotation VARCHAR, moe_annotation VARCHAR, boundary_year INTEGER,
          source_dataset VARCHAR, source_url VARCHAR, retrieved_at TIMESTAMP);
        INSERT INTO facts VALUES
          ('geoId/99', 'pep:POPESTIMATE', DATE '2022-07-01', DATE '2022-07-02', 'pep-2023',
           DATE '2024-03-14', 1000.0, NULL, NULL, NULL, 2023, 'census/pep/co-est',
           'https://example.gov/pep', TIMESTAMP '2026-09-01 12:00:00'),
          ('geoId/99', 'pep:POPESTIMATE', DATE '2022-07-01', DATE '2022-07-02', 'pep-2024',
           DATE '2025-03-13', 1010.0, NULL, NULL, NULL, 2023, 'census/pep/co-est',
           'https://example.gov/pep', TIMESTAMP '2026-09-01 12:00:00');
        """
    )
    for table in ("entities", "claims", "facts"):
        con.execute(f"COPY {table} TO '{data_dir / (table + '.parquet')}' (FORMAT PARQUET)")


class SlugTests(unittest.TestCase):
    def test_slug_replaces_slashes(self):
        self.assertEqual(entity_slug("pwc/bld/ab-12.x"), "pwc-bld-ab-12.x")

    def test_slug_refuses_unknown_characters(self):
        for bad in ("a/b<c", "a b", "geoId/06037'"):
            with self.assertRaises(SystemExit):
                entity_slug(bad)


class SiteBuildTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls._tmp = tempfile.TemporaryDirectory()
        root = Path(cls._tmp.name)
        cls.data, cls.out = root / "data", root / "site"
        cls.data.mkdir()
        _mini_store(cls.data)
        with patch.object(site, "CURATED_FACTS", [("geoId/99", "pep:POPESTIMATE", "2022-07-01")]):
            build_site(cls.data, cls.out, geo_dir=None)

    @classmethod
    def tearDownClass(cls):
        cls._tmp.cleanup()

    def _read(self, rel: str) -> str:
        return (self.out / rel).read_text(encoding="utf-8")

    def test_expected_files_exist(self):
        for rel in (
            "index.html",
            "dc/index.html",
            "dc/pwc-bld-aaa.html",
            "dc/frs-123.html",
            "facts/geoId-99.pep-POPESTIMATE.2022-07-01.html",
        ):
            self.assertTrue((self.out / rel).exists(), rel)

    def test_hostile_strings_are_escaped_everywhere(self):
        for page in self.out.rglob("*.html"):
            text = page.read_text(encoding="utf-8")
            self.assertNotIn("<script>alert", text, page)
            self.assertNotIn("</td></table><script>", text, page)

    def test_dossier_shows_latest_registry_name_and_all_claims(self):
        dossier = self._read("dc/pwc-bld-aaa.html")
        self.assertIn("&lt;script&gt;alert(1)&lt;/script&gt;", dossier)
        self.assertNotIn("OLD NAME", dossier)  # latest vintage wins
        self.assertIn("dc:gfa_sqft", dossier)
        self.assertIn("165,230", dossier)
        self.assertIn("Real Estate Assessments", dossier)

    def test_same_as_cross_links_both_directions(self):
        self.assertIn('href="frs-123.html"', self._read("dc/pwc-bld-aaa.html"))
        self.assertIn('href="pwc-bld-aaa.html"', self._read("dc/frs-123.html"))
        self.assertIn("30m apart", self._read("dc/frs-123.html"))

    def test_many_to_one_renders_multiple_linkboxes(self):
        # Two county buildings link to the same EPA facility (the real
        # campus-permit pattern): its dossier must show BOTH, each way.
        frs = self._read("dc/frs-123.html")
        self.assertEqual(frs.count("same facility (inferred)"), 2)
        self.assertIn('href="pwc-bld-bbb.html"', frs)
        self.assertIn('href="frs-123.html"', self._read("dc/pwc-bld-bbb.html"))

    def test_dc_index_directory_and_fact_index(self):
        dc = self._read("dc/index.html")
        self.assertIn('href="pwc-bld-aaa.html"', dc)
        self.assertIn("EPA facility · VA", dc)
        index = self._read("index.html")
        self.assertIn("facts/geoId-99.pep-POPESTIMATE.2022-07-01.html", index)
        self.assertIn("Testonia County", index)

    def test_rebuild_is_byte_stable_and_prunes_stale_pages(self):
        before = {p: p.read_bytes() for p in self.out.rglob("*.html")}
        # A page left behind by an earlier build (renamed slug, re-baked
        # entity set) must be pruned; non-HTML files must be left alone.
        stale = self.out / "dc" / "some-renamed-entity-old-slug.html"
        stale.write_text("stale", encoding="utf-8")
        keep = self.out / ".vercel-keep.json"
        keep.write_text("{}", encoding="utf-8")
        with patch.object(site, "CURATED_FACTS", [("geoId/99", "pep:POPESTIMATE", "2022-07-01")]):
            build_site(self.data, self.out, geo_dir=None)
        self.assertFalse(stale.exists(), "stale page must be pruned")
        self.assertTrue(keep.exists(), "non-HTML files must survive")
        after = {p: p.read_bytes() for p in self.out.rglob("*.html")}
        self.assertEqual(sorted(after), sorted(before), "page set must match exactly")
        for p, blob in before.items():
            self.assertEqual(after[p], blob, p)


class FailLoudlyTests(unittest.TestCase):
    def _fresh(self, **store_kwargs) -> tuple[Path, Path]:
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        (root / "data").mkdir()
        _mini_store(root / "data", **store_kwargs)
        return root / "data", root / "site"

    def test_claims_entity_without_registry_row_stops_the_build(self):
        data, out = self._fresh(ghost_claim=True)
        with patch.object(site, "CURATED_FACTS", [("geoId/99", "pep:POPESTIMATE", "2022-07-01")]):
            with self.assertRaisesRegex(SystemExit, "ghost/1"):
                build_site(data, out, geo_dir=None)


    def test_missing_geo_fixtures_stop_the_build(self):
        data, out = self._fresh()
        with patch.object(site, "CURATED_FACTS", [("geoId/99", "pep:POPESTIMATE", "2022-07-01")]):
            with self.assertRaisesRegex(SystemExit, "display geometry"):
                build_site(data, out, geo_dir=Path("definitely-not-a-real-geo-dir"))

    def test_source_with_no_claims_stops_the_build(self):
        data, out = self._fresh(include_echo=False)
        with patch.object(site, "CURATED_FACTS", [("geoId/99", "pep:POPESTIMATE", "2022-07-01")]):
            with self.assertRaisesRegex(SystemExit, "epa/echo/air"):
                build_site(data, out, geo_dir=None)


class FilenameTests(unittest.TestCase):
    def test_str_and_date_inputs_name_the_same_page(self):
        from datetime import date

        from bluedot_atlas.page import fact_page_filename

        canonical = fact_page_filename("geoId/99", "pep:POPESTIMATE", "2022-07-01")
        self.assertEqual(fact_page_filename("geoId/99", "pep:POPESTIMATE", date(2022, 7, 1)), canonical)
        # fromisoformat accepts non-canonical ISO 8601 variants — they must
        # normalize to the same filename, not fork it.
        self.assertEqual(fact_page_filename("geoId/99", "pep:POPESTIMATE", "20220701"), canonical)


if __name__ == "__main__":
    unittest.main()
