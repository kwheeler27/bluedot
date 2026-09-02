//! One place for the "don't be helpful" HTTP agent configuration, shared by
//! every source client: no redirects (a redirect from a data API is a condition
//! to inspect, not to follow), no status-to-error conversion (we read what the
//! server actually sent), one global timeout, one body-size ceiling.

use std::time::Duration;

/// Refuse response bodies over this size. The largest source file today is ~2 MB.
pub const BODY_LIMIT_BYTES: u64 = 64 << 20;

pub fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .max_redirects(0)
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(60)))
        .build()
        .new_agent()
}
