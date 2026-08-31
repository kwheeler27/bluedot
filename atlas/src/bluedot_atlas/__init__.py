"""Blue Dot analysis/glue package (distribution ``bluedot-atlas``, import ``bluedot_atlas``)."""

from importlib.metadata import version


def main() -> None:
    # Read the version from the installed package metadata so pyproject.toml
    # stays the single source of truth — the Python twin of Rust's env!("CARGO_PKG_VERSION").
    print(f"Blue Dot atlas v{version('bluedot-atlas')}")
