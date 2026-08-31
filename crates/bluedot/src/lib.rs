//! Blue Dot ingestion/conformance engine.
//!
//! This is the *library* half of the `bluedot` package: all real logic lives
//! here so it can be unit-tested, integration-tested from `tests/`, and later
//! bound into Python. `main.rs` is a thin shell over it.
//!
//! (`//!` is an "inner" doc comment — it documents the thing it is inside of,
//! here the crate. `///` is an "outer" doc comment — it documents the next item.)

/// The greeting printed by the `bluedot` binary.
///
/// Returns an owned `String` rather than printing directly so the function is
/// pure and the test below can check it without capturing stdout.
/// `env!("CARGO_PKG_VERSION")` is expanded at *compile* time from the version
/// in `Cargo.toml`, so the two can never drift apart.
pub fn greeting() -> String {
    format!("Blue Dot engine v{}", env!("CARGO_PKG_VERSION"))
}

// The conventional home for unit tests: a child module compiled only under
// `cargo test` (that is what `#[cfg(test)]` means). `use super::*;` pulls the
// parent module's items — `greeting` — into scope.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greeting_names_the_engine() {
        assert!(greeting().starts_with("Blue Dot engine v"));
    }
}
