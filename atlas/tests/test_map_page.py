"""build_map_page tests — the review's coverage gap, closed.

A tiny in-memory store + tiny geo fixtures exercise the real function:
the smoke path (all blobs parse, no leftover template variables, hostile
names neutralized, computed claims present) and every fail-loudly guard.
"""

import json
import re
import tempfile
import unittest
from pathlib import Path

import duckdb

from bluedot_atlas.map_page import build_map_page

HOSTILE = '</script><script>alert(1)</script> "DC"'


def _store():
    con = duckdb.connect()
    con.execute(
        """CREATE TABLE entities (entity_id VARCHAR, name VARCHAR, level VARCHAR,
          boundary_year INTEGER, vintage VARCHAR, source_dataset VARCHAR, lat DOUBLE, lon DOUBLE)"""
    )
    con.execute(
        """
        INSERT INTO entities VALUES
          ('frs/1', ?, 'facility', 2026, 'echo-2026-09-02', 'epa/echo/air', 38.7, -77.5),
          ('pwc/bld/b1', 'BUILDING ONE', 'facility', 2026, 'pwc-2026-09-02', 'pwcva/build-out-analysis', 38.71, -77.51),
          ('pwc/site/s1', 'SITE ONE', 'campus', 2026, 'pwc-2026-09-02', 'pwcva/build-out-analysis', 38.72, -77.52),
          ('pwc/campus/c1', 'CAMPUS ONE', 'campus', 2026, 'pwc-2026-09-02', 'pwcva/build-out-analysis', 38.73, -77.53)
        """,
        [HOSTILE],
    )
    con.execute(
        """CREATE TABLE claims (entity_id VARCHAR, attribute_id VARCHAR, valid_from DATE, valid_to DATE,
          vintage VARCHAR, source_record VARCHAR, value_text VARCHAR, value_num DOUBLE, unit VARCHAR,
          stated_by VARCHAR, confidence VARCHAR, published_at DATE, source_dataset VARCHAR,
          source_url VARCHAR, retrieved_at TIMESTAMP)"""
    )
    con.execute(
        """
        INSERT INTO claims VALUES
          ('frs/1', 'dc:stage', DATE '2026-09-02', DATE '2026-09-03', 'echo-2026-09-02', '1',
           'operating', NULL, NULL, 'EPA', 'confirmed_by_record', DATE '2026-09-02',
           'epa/echo/air', 'https://x', TIMESTAMP '2026-09-02 12:00:00'),
          ('pwc/bld/b1', 'dc:stage', DATE '2026-09-02', DATE '2026-09-03', 'pwc-2026-09-02', 'b1',
           'completed', NULL, NULL, 'PWC', 'confirmed_by_record', DATE '2026-09-02',
           'pwcva/build-out-analysis', 'https://x', TIMESTAMP '2026-09-02 12:00:00'),
          ('pwc/bld/b1', 'dc:gfa_sqft', DATE '2026-09-02', DATE '2026-09-03', 'pwc-2026-09-02', 'b1',
           NULL, 200000, 'sqft', 'PWC', 'confirmed_by_record', DATE '2026-09-02',
           'pwcva/build-out-analysis', 'https://x', TIMESTAMP '2026-09-02 12:00:00'),
          ('pwc/site/s1', 'dc:zoning_status', DATE '2026-09-02', DATE '2026-09-03', 'pwc-2026-09-02', 'Z1',
           'approved', NULL, NULL, 'PWC', 'confirmed_by_record', DATE '2026-09-02',
           'pwcva/build-out-analysis', 'https://x', TIMESTAMP '2026-09-02 12:00:00'),
          ('pwc/site/s1', 'dc:gfa_planned_sqft', DATE '2026-09-02', DATE '2026-09-03', 'pwc-2026-09-02', 'Z1',
           NULL, 1200000, 'sqft', 'PWC', 'confirmed_by_record', DATE '2026-09-02',
           'pwcva/build-out-analysis', 'https://x', TIMESTAMP '2026-09-02 12:00:00'),
          ('pwc/campus/c1', 'dc:stage', DATE '2026-09-02', DATE '2026-09-03', 'pwc-2026-09-02', 'c1',
           'planned', NULL, NULL, 'PWC', 'confirmed_by_record', DATE '2026-09-02',
           'pwcva/build-out-analysis', 'https://x', TIMESTAMP '2026-09-02 12:00:00')
        """
    )
    con.execute(
        """CREATE TABLE geometry (entity_id VARCHAR, vintage VARCHAR, source_dataset VARCHAR,
          rings JSON, retrieved_at TIMESTAMP)"""
    )
    con.execute(
        """INSERT INTO geometry VALUES
          ('pwc/campus/c1', 'pwc-2026-09-02', 'pwcva/build-out-analysis',
           '[[[-77.53,38.73],[-77.52,38.73],[-77.52,38.72],[-77.53,38.73]]]',
           TIMESTAMP '2026-09-02 12:00:00')"""
    )
    return con


def _geo(dir_: Path, kx_lat: object = 38.7) -> Path:
    ring = [[-60.5, -38.73], [-60.49, -38.73], [-60.49, -38.72], [-60.5, -38.73]]
    (dir_ / "us-counties-topo.json").write_text(json.dumps(
        {"type": "Topology", "objects": {"nation": {}, "states": {}, "counties": {}}, "arcs": []}))
    region = {"pwc": {"type": "MultiPolygon", "coordinates": [[ring]]}, "neighbors": []}
    if kx_lat is not None:
        region["kx_lat"] = kx_lat
    (dir_ / "pwc-region-planar.json").write_text(json.dumps(region))
    return dir_


SLUGS = {"frs/1": "frs-1", "pwc/bld/b1": "pwc-bld-b1",
         "pwc/site/s1": "pwc-site-s1", "pwc/campus/c1": "pwc-campus-c1"}


class MapPageTests(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.geo = _geo(Path(tmp.name))
        self.con = _store()

    def test_smoke_blobs_parse_and_claims_are_computed(self):
        out = build_map_page(self.con, self.geo, SLUGS, "2026-09-03")
        # no leftover template variables
        self.assertFalse(re.search(r"\$[a-z_]+", out.split("<script")[0]))
        # every baked blob is valid JSON after unescaping
        for var in ("TOPO", "REGION", "CAMPI", "PTS"):
            m = re.search(r"var " + var + r" = (.*?);(?: //|\n)", out)
            self.assertIsNotNone(m, var)
            json.loads(m.group(1).replace("\\u003c", "<"))
        # hostile name can never close the script tag
        self.assertNotIn("</script><script>alert", out)
        # computed frame-02 claims (1.2M entitled / 0.2M standing = ratio six)
        self.assertIn("construction six to one", out)
        self.assertIn("1.2M sqft is entitled", out)
        self.assertIn("0.2M sqft standing", out)
        # kx_lat travels from the fixture, not a hardcoded constant
        self.assertIn("Math.cos(38.7 * Math.PI / 180)", out)

    def test_unknown_stage_stops_the_build(self):
        self.con.execute("UPDATE claims SET value_text = 'vibes' WHERE entity_id = 'frs/1'")
        with self.assertRaisesRegex(SystemExit, "no display bucket"):
            build_map_page(self.con, self.geo, SLUGS, "2026-09-03")

    def test_campus_without_dossier_stops_the_build(self):
        slugs = {k: v for k, v in SLUGS.items() if k != "pwc/campus/c1"}
        with self.assertRaisesRegex(SystemExit, "no dossier slug"):
            build_map_page(self.con, self.geo, slugs, "2026-09-03")

    def test_campus_stage_outside_landbay_vocabulary_stops_the_build(self):
        # 'operating' is a legal display bucket but not a campus land-bay
        # status — the map must refuse, not render class="undefined".
        self.con.execute(
            "UPDATE claims SET value_text = 'operating' WHERE entity_id = 'pwc/campus/c1' AND attribute_id = 'dc:stage'")
        with self.assertRaisesRegex(SystemExit, "land-bay"):
            build_map_page(self.con, self.geo, SLUGS, "2026-09-03")

    def test_missing_geometry_table_stops_the_build(self):
        self.con.execute("DROP TABLE geometry")
        with self.assertRaisesRegex(SystemExit, "geometry"):
            build_map_page(self.con, self.geo, SLUGS, "2026-09-03")

    def test_missing_kx_lat_stops_the_build(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        geo = _geo(Path(tmp.name), kx_lat=None)
        with self.assertRaisesRegex(SystemExit, "kx_lat"):
            build_map_page(self.con, geo, SLUGS, "2026-09-03")

    def test_missing_stats_stop_the_build(self):
        self.con.execute("DELETE FROM claims WHERE attribute_id = 'dc:gfa_planned_sqft'")
        with self.assertRaisesRegex(SystemExit, "can't back"):
            build_map_page(self.con, self.geo, SLUGS, "2026-09-03")


if __name__ == "__main__":
    unittest.main()
