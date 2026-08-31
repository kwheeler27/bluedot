//! Where the Census API key comes from.
//!
//! Order: the `CENSUS_API_KEY` environment variable, then a `.env` file in the
//! current directory. A dotenv crate would do this too; it is fifteen lines.

use std::fs;

use crate::Error;

pub const KEY_VAR: &str = "CENSUS_API_KEY";

pub fn census_api_key() -> Result<String, Error> {
    if let Ok(key) = std::env::var(KEY_VAR)
        && !key.trim().is_empty()
    {
        return Ok(key.trim().to_owned());
    }
    // `fs::read_to_string` fails if there's no `.env`; `.ok()` turns that
    // `Result` into an `Option` so "no file" and "file without the key" both
    // fall through to the same error.
    fs::read_to_string(".env")
        .ok()
        .and_then(|text| value_from_dotenv(&text, KEY_VAR))
        .ok_or(Error::MissingApiKey)
}

/// Find `NAME=value` in dotenv-style text. Ignores blank lines and `#` comments,
/// strips one layer of matching quotes. Returns `None` for a missing or empty value.
pub fn value_from_dotenv(text: &str, name: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .find(|(k, _)| k.trim() == name)
        .map(|(_, v)| v.trim().trim_matches(|c| c == '"' || c == '\'').to_owned())
        .filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_key_ignoring_comments_and_quotes() {
        let text = "# comment\n\nOTHER=1\nCENSUS_API_KEY=\"abc123\"\n";
        assert_eq!(value_from_dotenv(text, KEY_VAR).as_deref(), Some("abc123"));
    }

    #[test]
    fn missing_or_empty_is_none() {
        assert_eq!(value_from_dotenv("OTHER=1\n", KEY_VAR), None);
        assert_eq!(value_from_dotenv("CENSUS_API_KEY=\n", KEY_VAR), None);
    }
}
