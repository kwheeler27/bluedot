"""Blue Dot analysis/glue package (distribution ``bluedot-atlas``, import ``bluedot_atlas``).

Entry point for the ``bluedot-atlas`` console script. Argument handling is
deliberately plain ``sys.argv`` — two subcommands don't justify a CLI framework.
"""

import sys
from importlib.metadata import version
from pathlib import Path

USAGE = """usage: bluedot-atlas <command> [options]

commands:
  build-facts [--data DIR]     load DIR/facts/*.jsonl and DIR/entities/*.jsonl
                               (written by the Rust engine) into DIR/facts.parquet
                               and DIR/entities.parquet, then run the demo queries
  page <entity_id> <indicator_id> <valid_from> [--data DIR]
                               compile a static fact page (the vintage ladder) to
                               DIR/pages/, e.g.:
                               page geoId/06037 pep:POPESTIMATE 2022-07-01

DIR defaults to ./data — run from the repo root.
"""


def _data_dir(rest: list[str]) -> tuple[Path, list[str]]:
    """Pop a trailing ``--data DIR`` pair; everything else passes through."""
    if "--data" in rest:
        i = rest.index("--data")
        if i + 1 >= len(rest):
            sys.exit(f"--data needs a value\n{USAGE}")
        return Path(rest[i + 1]), rest[:i] + rest[i + 2 :]
    return Path("data"), rest


def main() -> None:
    args = sys.argv[1:]
    if not args:
        print(f"Blue Dot atlas v{version('bluedot-atlas')}")
        print(USAGE, end="")
        return

    command, *rest = args
    data_dir, rest = _data_dir(rest)

    if command == "build-facts":
        if rest:
            sys.exit(f"bluedot-atlas build-facts: unexpected arguments {rest!r}\n{USAGE}")
        from .facts import build_facts  # imported lazily so usage output never loads duckdb

        build_facts(data_dir)
        return

    if command == "page":
        if len(rest) != 3:
            sys.exit(f"bluedot-atlas page needs <entity_id> <indicator_id> <valid_from>\n{USAGE}")
        from .page import compile_page

        compile_page(data_dir, *rest)
        return

    sys.exit(f"bluedot-atlas: unknown command {command!r}\n{USAGE}")
