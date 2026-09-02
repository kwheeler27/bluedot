"""Unit tests for the pure parts of linkage v0 — stdlib unittest, no new deps."""

import unittest

from bluedot_atlas.linkage import CLOSE_METERS, MAX_METERS, haversine_m, is_match, name_tokens


class NameTokens(unittest.TestCase):
    def test_stop_words_and_numbers_drop(self):
        self.assertEqual(
            name_tokens("AMAZON DATA SERVICES, INC. IAD-6 IAD-13"), {"AMAZON", "IAD"}
        )
        self.assertEqual(name_tokens("21110 RIDGETOP CIRCLE LLC"), {"RIDGETOP", "CIRCLE"})

    def test_distinctive_overlap(self):
        a = name_tokens("Iron Mountain Data Center VA-1")
        b = name_tokens("IRON MOUNTAIN INFORMATION MANAGEMENT")
        self.assertIn("IRON", a & b)
        self.assertIn("MOUNTAIN", a & b)


class Haversine(unittest.TestCase):
    def test_zero_distance(self):
        self.assertAlmostEqual(haversine_m(38.77, -77.54, 38.77, -77.54), 0.0)

    def test_known_scale(self):
        # one degree of latitude ≈ 111.2 km
        d = haversine_m(38.0, -77.0, 39.0, -77.0)
        self.assertAlmostEqual(d, 111_195, delta=200)


class Decision(unittest.TestCase):
    def test_close_needs_no_name(self):
        self.assertTrue(is_match(CLOSE_METERS - 1, set()))

    def test_medium_needs_a_shared_token(self):
        self.assertFalse(is_match(CLOSE_METERS + 1, set()))
        self.assertTrue(is_match(MAX_METERS - 1, {"IRON"}))

    def test_far_never_matches(self):
        self.assertFalse(is_match(MAX_METERS + 1, {"IRON", "MOUNTAIN"}))


if __name__ == "__main__":
    unittest.main()


class MatchAll(unittest.TestCase):
    def test_one_building_near_two_facilities_refuses(self):
        from bluedot_atlas.linkage import match_all

        pwc = [("pwc/bld/a", "Iron Mountain VA-1", 38.0, -77.0)]
        frs = [
            ("frs/1", "IRON MOUNTAIN ALPHA", 38.0, -77.0),
            ("frs/2", "IRON MOUNTAIN BETA", 38.0005, -77.0),  # ~55 m away
        ]
        links, ambiguous = match_all(pwc, frs)
        self.assertEqual(links, [])
        self.assertEqual(ambiguous, [("pwc/bld/a", ["frs/1", "frs/2"])])

    def test_two_buildings_near_one_facility_links_both_visibly(self):
        from bluedot_atlas.linkage import match_all

        pwc = [
            ("pwc/bld/a", "Iron Mountain VA-1", 38.0, -77.0),
            ("pwc/bld/b", "Iron Mountain VA-2", 38.0008, -77.0),  # ~89 m away
        ]
        frs = [("frs/1", "IRON MOUNTAIN INFORMATION", 38.0004, -77.0)]
        links, ambiguous = match_all(pwc, frs)
        self.assertEqual(ambiguous, [])
        self.assertEqual(len(links), 2)
        self.assertEqual({fid for _, fid, _, _ in links}, {"frs/1"})
