//! `bluedot` binary — a thin shell over the `bluedot` library.
//!
//! A package may contain both a library target (`lib.rs`) and a binary target
//! (`main.rs`) with the same name. From here the library is reached by its
//! crate name, `bluedot::...`, exactly as an outside user of the library would
//! write it. Keep this file small: logic that lives here is unreachable from
//! tests.

fn main() {
    println!("{}", bluedot::greeting());
}
