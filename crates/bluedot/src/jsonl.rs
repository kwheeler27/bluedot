//! Atomic JSON Lines writer: one JSON object per line, written to a temp file
//! and renamed into place, so a reader never sees a partial file.

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use serde::Serialize;

use crate::Error;

/// Write `rows` to `path`. On any failure the target path is left untouched.
///
/// `T: Serialize` — generic over anything serde can serialize; the compiler
/// stamps out a version of this function per concrete `T` it is used with.
pub fn write_atomic<T: Serialize>(path: &Path, rows: &[T]) -> Result<(), Error> {
    // `io_err` turns a bare io::Error into our Error with the path attached.
    // A closure that captures `path`, so each `?` site stays short.
    let io_err = |source: std::io::Error| Error::Io {
        path: path.to_path_buf(),
        source,
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_err)?;
    }
    let tmp = path.with_extension("jsonl.tmp");

    // Inner block so `writer` is dropped (and the file closed) before the rename.
    let result = (|| {
        let mut writer = BufWriter::new(File::create(&tmp).map_err(io_err)?);
        for row in rows {
            // `to_writer` writes compact JSON; a trailing newline makes it a line.
            serde_json::to_writer(&mut writer, row).map_err(|e| io_err(e.into()))?;
            writer.write_all(b"\n").map_err(io_err)?;
        }
        writer.flush().map_err(io_err)?;
        writer
            .into_inner()
            .map_err(|e| io_err(e.into_error()))?
            .sync_all()
            .map_err(io_err)
    })();

    match result {
        Ok(()) => fs::rename(&tmp, path).map_err(io_err),
        Err(e) => {
            let _ = fs::remove_file(&tmp); // best effort; the original error is what matters
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_one_object_per_line_and_leaves_no_temp_file() {
        let dir = std::env::temp_dir().join(format!("bluedot-jsonl-{}", std::process::id()));
        let path = dir.join("nested").join("out.jsonl");
        write_atomic(
            &path,
            &[serde_json::json!({"a": 1}), serde_json::json!({"a": 2})],
        )
        .unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"a\":1}\n{\"a\":2}\n");
        assert!(!path.with_extension("jsonl.tmp").exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
