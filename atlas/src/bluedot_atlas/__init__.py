"""Blue Dot analysis/glue package (distribution ``bluedot-atlas``, import ``bluedot_atlas``).

Entry point for the ``bluedot-atlas`` console script. Argument handling is
deliberately plain ``sys.argv`` — one subcommand doesn't justify a CLI framework.
"""

import sys
from importlib.metadata import version
from pathlib import Path

USAGE = """usage: bluedot-atlas <command> [options]

commands:
  build-facts [--data DIR]   load data/facts/*.jsonl (written by the Rust engine)
                             into data/facts.parquet and run the demo queries.
                             DIR defaults to ./data — run from the repo root.
"""


def main() -> None:
    args = sys.argv[1:]
    if not args:
        print(f"Blue Dot atlas v{version('bluedot-atlas')}")
        print(USAGE, end="")
        return

    command, *rest = args
    if command == "build-facts":
        data_dir = Path("data")
        if rest[:1] == ["--data"] and len(rest) == 2:
            data_dir = Path(rest[1])
        elif rest:
            sys.exit(f"bluedot-atlas: unexpected arguments {rest!r}\n{USAGE}")
        from .facts import build_facts  # imported lazily so `--help` never loads duckdb

        build_facts(data_dir)
        return

    sys.exit(f"bluedot-atlas: unknown command {command!r}\n{USAGE}")
