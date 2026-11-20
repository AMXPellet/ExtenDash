//! Workspace folder layout v1 (spec: `docs/storage-layout.md`).
//!
//! Paths are `/`-separated and relative to the workspace root, so every
//! `StorageBackend` can map them onto its own addressing.

use std::fmt;

use chrono::{DateTime, NaiveDateTime, Utc};
use uuid::Uuid;

use crate::error::{CoreError, Result};

pub const MANIFEST: &str = "manifest.json";
pub const EXTENSIONS: &str = "extensions.json";
pub const SNAPSHOTS_DIR: &str = "snapshots";
pub const INBOX_DIR: &str = "inbox";

/// Sortable, filename-safe UTC timestamp with millisecond precision.
const TIMESTAMP_FORMAT: &str = "%Y%m%dT%H%M%S%3fZ";
const SUFFIX_LEN: usize = 8;
const MAX_ID_LEN: usize = 64;

/// Checks a source or origin id: 1-64 chars of `[a-z0-9.-]`, starting with a
/// letter or digit. Keeps ids safe as path segments and free of `_`, which
/// separates inbox filename parts.
pub fn validate_id(kind: &'static str, value: &str) -> Result<()> {
    let invalid = |reason| {
        Err(CoreError::InvalidId {
            kind,
            value: value.to_owned(),
            reason,
        })
    };

    if value.is_empty() || value.len() > MAX_ID_LEN {
        return invalid("must be 1-64 characters");
    }
    if !value.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit()) {
        return invalid("must start with a lowercase letter or digit");
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-')
    {
        return invalid("may only contain a-z, 0-9, '.' and '-'");
    }
    Ok(())
}

/// `snapshots/<source-id>.json`
pub fn snapshot_path(source_id: &str) -> Result<String> {
    validate_id("source", source_id)?;
    Ok(format!("{SNAPSHOTS_DIR}/{source_id}.json"))
}

/// `inbox/<source-id>`
pub fn inbox_dir(source_id: &str) -> Result<String> {
    validate_id("source", source_id)?;
    Ok(format!("{INBOX_DIR}/{source_id}"))
}

/// `inbox/<source-id>/<timestamp>_<origin>_<rand>.json`
pub fn inbox_event_path(source_id: &str, name: &InboxFileName) -> Result<String> {
    Ok(format!("{}/{name}", inbox_dir(source_id)?))
}

/// A unique inbox event filename: `<timestamp>_<origin>_<rand>.json`.
///
/// The random suffix keeps names unique even when two writers share an origin
/// and timestamp. Inbox files are never overwritten by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxFileName {
    pub created_at: DateTime<Utc>,
    pub origin: String,
    pub suffix: String,
}

impl InboxFileName {
    pub fn new(created_at: DateTime<Utc>, origin: &str) -> Result<Self> {
        validate_id("origin", origin)?;
        let suffix = Uuid::new_v4().simple().to_string()[..SUFFIX_LEN].to_owned();
        Ok(Self {
            created_at,
            origin: origin.to_owned(),
            suffix,
        })
    }

    pub fn parse(name: &str) -> Result<Self> {
        let bad = || CoreError::InvalidInboxFileName(name.to_owned());

        let stem = name.strip_suffix(".json").ok_or_else(bad)?;
        let mut parts = stem.splitn(3, '_');
        let (Some(timestamp), Some(origin), Some(suffix)) =
            (parts.next(), parts.next(), parts.next())
        else {
            return Err(bad());
        };

        let created_at = NaiveDateTime::parse_from_str(timestamp, TIMESTAMP_FORMAT)
            .map_err(|_| bad())?
            .and_utc();
        validate_id("origin", origin).map_err(|_| bad())?;
        if suffix.len() != SUFFIX_LEN
            || !suffix.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
        {
            return Err(bad());
        }

        Ok(Self {
            created_at,
            origin: origin.to_owned(),
            suffix: suffix.to_owned(),
        })
    }
}

impl fmt::Display for InboxFileName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}_{}_{}.json",
            self.created_at.format(TIMESTAMP_FORMAT),
            self.origin,
            self.suffix
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time() -> DateTime<Utc> {
        "2026-10-21T20:15:00.123Z".parse().unwrap()
    }

    #[test]
    fn builds_paths() {
        assert_eq!(snapshot_path("steps").unwrap(), "snapshots/steps.json");
        let name = InboxFileName::new(time(), "laptop").unwrap();
        let path = inbox_event_path("steps", &name).unwrap();
        assert!(path.starts_with("inbox/steps/20261021T201500123Z_laptop_"));
        assert!(path.ends_with(".json"));
    }

    #[test]
    fn inbox_file_name_round_trips() {
        let name = InboxFileName::new(time(), "github.commits").unwrap();
        assert_eq!(InboxFileName::parse(&name.to_string()).unwrap(), name);
    }

    #[test]
    fn inbox_file_names_are_unique() {
        let a = InboxFileName::new(time(), "app").unwrap();
        let b = InboxFileName::new(time(), "app").unwrap();
        assert_ne!(a.to_string(), b.to_string());
    }

    #[test]
    fn rejects_bad_ids_and_names() {
        for id in ["", "Steps", "a_b", "a/b", "-a", ".a"] {
            assert!(validate_id("source", id).is_err(), "{id:?} should be rejected");
        }
        for name in [
            "20261021T201500123Z_app_0123abcd.txt",
            "20261021T201500123Z_app.json",
            "not-a-time_app_0123abcd.json",
            "20261021T201500123Z_app_XYZ.json",
        ] {
            assert!(InboxFileName::parse(name).is_err(), "{name:?} should be rejected");
        }
    }
}
