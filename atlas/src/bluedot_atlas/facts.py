"""Load the engine's JSON Lines fact files into one Parquet file and run the demo queries.

The Rust engine (crates/bluedot) writes data/facts/<vintage>.jsonl. This module is
the Python half of the boundary described in ADR-0006: it reads what the engine
emitted, pins the column types (never inferred — a fact schema is a contract),
refuses duplicate keys, and writes data/facts.parquet for DuckDB/Parquet consumers.
"""

from pathlib import Path

import duckdb

# Fact schema v0 — see docs/briefs/01-acs-facts-v0.md. Order matters: it is the
# column order of the Parquet file. Types are declared so a malformed row fails
# the load instead of silently widening a column to VARCHAR.
FACT_COLUMNS: dict[str, str] = {
    "entity_id": "VARCHAR",
    "indicator_id": "VARCHAR",
    "valid_from": "DATE",
    "valid_to": "DATE",
    "vintage": "VARCHAR",
    "published_at": "DATE",
    "value": "DOUBLE",
    "moe": "DOUBLE",
    "value_annotation": "VARCHAR",
    "moe_annotation": "VARCHAR",
    "boundary_year": "INTEGER",
    "source_dataset": "VARCHAR",
    "source_url": "VARCHAR",
    "retrieved_at": "TIMESTAMP",
}

# The fact key (ADR-0001, ADR-0013). valid_time is the half-open [valid_from, valid_to).
FACT_KEY = ("entity_id", "indicator_id", "valid_from", "valid_to", "vintage")

DEMO_QUERIES: list[tuple[str, str]] = [
    (
        "(a) Los Angeles County across vintages — the vintage axis",
        """
        SELECT vintage, published_at, valid_from, valid_to, value, moe, moe_annotation
        FROM facts
        WHERE entity_id = 'geoId/06037' AND indicator_id = 'acs:B01003_001'
        ORDER BY vintage
        """,
    ),
    (
        "(b) Connecticut county-equivalents per vintage — the boundary axis",
        """
        SELECT vintage, boundary_year, count(*) AS entities,
               min(entity_id) AS first_entity, max(entity_id) AS last_entity
        FROM facts
        WHERE entity_id LIKE 'geoId/09%'
        GROUP BY ALL
        ORDER BY vintage
        """,
    ),
    (
        "(b') Connecticut entities present in BOTH vintages (expected: none)",
        """
        SELECT entity_id, count(DISTINCT vintage) AS vintages
        FROM facts
        WHERE entity_id LIKE 'geoId/09%'
        GROUP BY entity_id
        HAVING count(DISTINCT vintage) > 1
        ORDER BY entity_id
        """,
    ),
    (
        "(c) Annotated rows per vintage — sentinel decoding on real data",
        """
        SELECT vintage,
               coalesce(value_annotation, '(none)') AS value_annotation,
               coalesce(moe_annotation, '(none)') AS moe_annotation,
               count(*) AS rows
        FROM facts
        GROUP BY ALL
        ORDER BY vintage, value_annotation, moe_annotation
        """,
    ),
    (
        "(d) LA County, July 1 2022, as stated by each PEP vintage — a true revision",
        """
        SELECT vintage, published_at, value::BIGINT AS value
        FROM facts
        WHERE entity_id = 'geoId/06037' AND indicator_id = 'pep:POPESTIMATE'
          AND valid_from = DATE '2022-07-01'
        ORDER BY published_at
        """,
    ),
    (
        "(e) ...and what we BELIEVED about it on 2024-01-01 — the as-of-knowledge-date query",
        """
        SELECT vintage, published_at, value::BIGINT AS believed_value
        FROM facts
        WHERE entity_id = 'geoId/06037' AND indicator_id = 'pep:POPESTIMATE'
          AND valid_from = DATE '2022-07-01' AND published_at <= DATE '2024-01-01'
        ORDER BY published_at DESC
        LIMIT 1
        """,
    ),
    (
        "(f) Largest revisions of July-1-2024 populations between Vintage 2024 and Vintage 2025",
        """
        SELECT f24.entity_id,
               f24.value::BIGINT AS vintage_2024_said,
               f25.value::BIGINT AS vintage_2025_says,
               (f25.value - f24.value)::BIGINT AS revision
        FROM facts f24
        JOIN facts f25 USING (entity_id, indicator_id, valid_from)
        WHERE indicator_id = 'pep:POPESTIMATE' AND valid_from = DATE '2024-07-01'
          AND f24.vintage = 'pep-2024' AND f25.vintage = 'pep-2025'
        ORDER BY abs(f25.value - f24.value) DESC
        LIMIT 5
        """,
    ),
    (
        "(g) CAUTION — 'LA County population, 2023' from both sources: a 5-year-period"
        " estimate and a July-1 point are different concepts. Refusing to conflate them"
        " is the semantic layer's future job (ADR-0004/0005).",
        """
        SELECT indicator_id, vintage, valid_from, valid_to, value::BIGINT AS value
        FROM facts
        WHERE entity_id = 'geoId/06037'
          AND ((indicator_id = 'acs:B01003_001' AND vintage = 'acs5-2023')
               OR (indicator_id = 'pep:POPESTIMATE' AND vintage = 'pep-2025'
                   AND valid_from = DATE '2023-07-01'))
        ORDER BY indicator_id
        """,
    ),
]


def build_facts(data_dir: Path) -> Path:
    """Load ``data_dir/facts/*.jsonl`` into ``data_dir/facts.parquet``; print the demo queries."""
    src_dir = data_dir / "facts"
    files = sorted(src_dir.glob("*.jsonl"))
    if not files:
        raise SystemExit(
            f"no fact files in {src_dir}/ — run the engine first:\n"
            "  cargo run -p bluedot -- ingest acs --indicator B01003_001 --vintage 2021 --vintage 2023"
        )
    out = data_dir / "facts.parquet"

    columns_sql = ", ".join(f"'{name}': '{typ}'" for name, typ in FACT_COLUMNS.items())
    con = duckdb.connect()  # in-memory; Parquet is the artifact, not a .duckdb file
    con.execute(
        f"CREATE TABLE facts AS SELECT * FROM read_json(?, format = 'newline_delimited', columns = {{{columns_sql}}})",
        [[str(f) for f in files]],
    )

    # Refuse to write a Parquet file whose key isn't unique: silently keeping a
    # duplicate is exactly the kind of quiet wrongness ADR-0005 forbids.
    key_sql = ", ".join(FACT_KEY)
    (dups,) = con.execute(
        f"SELECT count(*) FROM (SELECT {key_sql} FROM facts GROUP BY ALL HAVING count(*) > 1)"
    ).fetchone()
    if dups:
        raise SystemExit(f"{dups} duplicate fact keys across {len(files)} files — refusing to write {out}")

    # COPY takes a literal path, not a bound parameter; escape single quotes.
    con.execute(f"COPY facts TO '{str(out).replace(chr(39), chr(39) * 2)}' (FORMAT PARQUET)")

    (n_rows,) = con.execute("SELECT count(*) FROM facts").fetchone()
    per_file = con.execute("SELECT vintage, count(*) FROM facts GROUP BY ALL ORDER BY vintage").fetchall()
    print(f"wrote {out} — {n_rows} facts from {len(files)} files: "
          + ", ".join(f"{v}={n}" for v, n in per_file))

    for title, sql in DEMO_QUERIES:
        print(f"\n{title}")
        con.sql(sql).show()
    return out
