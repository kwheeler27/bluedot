"""Indicator metadata seed — the embryo of the semantic layer (ADR-0004).

Hand-kept while it is three entries. When it grows past a screen it moves to
YAML and gains the full declaration: denominators, allowed operations, margin
handling, comparability rules. Display code treats a missing entry as
"declaration pending", never as an error — the number is the product, the label
is decoration.
"""

INDICATORS: dict[str, dict[str, str]] = {
    "acs:B01003_001": {
        "label": "Total population",
        "unit": "people",
        "universe": "Total population",
        "timeframe": "5-year period",
        "definition": (
            "ACS 5-year period estimate of total population; survey-based, "
            "published with a 90% margin of error (controlled at county level)."
        ),
    },
    "pep:POPESTIMATE": {
        "label": "Resident population",
        "unit": "people",
        "universe": "Total residents",
        "timeframe": "point in time (July 1)",
        "definition": (
            "Population Estimates Program resident population as of July 1; "
            "administrative estimate, restated by later vintages."
        ),
    },
    "pep:ESTIMATESBASE": {
        "label": "Estimates base",
        "unit": "people",
        "universe": "Total residents",
        "timeframe": "point in time (April 1, 2020)",
        "definition": (
            "The April 1, 2020 base the PEP series is benchmarked to; itself "
            "restated across vintages."
        ),
    },
}
