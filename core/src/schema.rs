//! Data schema v1: the Rust mirror of `schemas/v1/*.schema.json`.
//!
//! Every top-level document carries `schemaVersion`. Field names are camelCase
//! on disk. Unknown fields are rejected: any new field means a schema bump.

use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

use crate::error::{CoreError, Result};

/// The schema version this build reads and writes.
pub const SCHEMA_VERSION: u32 = 1;

/// A top-level JSON document stored in the workspace.
pub trait Document: Serialize + DeserializeOwned {
    /// Parses a document, checking `schemaVersion` before the full parse so a
    /// newer format surfaces as `SchemaMismatch` rather than a parse error.
    fn from_json(bytes: &[u8]) -> Result<Self> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Header {
            schema_version: u32,
        }

        let header: Header = serde_json::from_slice(bytes)?;
        if header.schema_version != SCHEMA_VERSION {
            return Err(CoreError::SchemaMismatch {
                found: header.schema_version,
                supported: SCHEMA_VERSION,
            });
        }
        Ok(serde_json::from_slice(bytes)?)
    }

    fn to_json(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec_pretty(self)?)
    }
}

/// One tracked item. `id` is stable across devices and fetches so duplicate
/// emissions collapse during merge; `updated_at` drives last-writer-wins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Record {
    pub id: String,
    pub updated_at: DateTime<Utc>,
    /// Tombstone, so a delete wins over older updates during merge.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub deleted: bool,
    /// Source-defined payload, described by the emitting extension's manifest.
    #[serde(default)]
    pub data: Value,
}

/// Canonical state of one source. Written only by compaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snapshot {
    pub schema_version: u32,
    pub source_id: String,
    pub compacted_at: DateTime<Utc>,
    /// Sorted by `id` so identical state serializes identically.
    pub records: Vec<Record>,
}

impl Snapshot {
    pub fn new(source_id: impl Into<String>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            source_id: source_id.into(),
            compacted_at: Utc::now(),
            records: Vec::new(),
        }
    }
}

impl Document for Snapshot {}

/// A batch of record changes dropped into `inbox/<source-id>/` by any writer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InboxEvent {
    pub schema_version: u32,
    pub source_id: String,
    /// Who wrote the event: a device id, extension id, or `app`.
    pub origin: String,
    pub created_at: DateTime<Utc>,
    pub records: Vec<Record>,
}

impl InboxEvent {
    pub fn new(
        source_id: impl Into<String>,
        origin: impl Into<String>,
        records: Vec<Record>,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            source_id: source_id.into(),
            origin: origin.into(),
            created_at: Utc::now(),
            records,
        }
    }
}

impl Document for InboxEvent {}

/// `manifest.json`: workspace metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceManifest {
    pub schema_version: u32,
    pub workspace_id: String,
    pub created_at: DateTime<Utc>,
}

impl Document for WorkspaceManifest {}

/// `extensions.json`: installed extensions, synced across devices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtensionsFile {
    pub schema_version: u32,
    pub extensions: Vec<InstalledExtension>,
}

impl Document for ExtensionsFile {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstalledExtension {
    pub id: String,
    pub version: String,
    pub enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn record(id: &str) -> Record {
        Record {
            id: id.into(),
            updated_at: "2026-01-01T00:00:00Z".parse().unwrap(),
            deleted: false,
            data: json!({ "value": 3 }),
        }
    }

    #[test]
    fn snapshot_round_trips() {
        let mut snapshot = Snapshot::new("steps");
        snapshot.records.push(record("2026-01-01"));
        let bytes = snapshot.to_json().unwrap();
        assert_eq!(Snapshot::from_json(&bytes).unwrap(), snapshot);
    }

    #[test]
    fn json_uses_camel_case_and_omits_false_tombstone() {
        let event = InboxEvent::new("steps", "app", vec![record("a")]);
        let value: Value = serde_json::from_slice(&event.to_json().unwrap()).unwrap();
        assert_eq!(value["schemaVersion"], 1);
        assert!(value["records"][0].get("updatedAt").is_some());
        assert!(value["records"][0].get("deleted").is_none());
    }

    #[test]
    fn rejects_other_schema_versions() {
        let bytes = br#"{"schemaVersion":2,"workspaceId":"w","createdAt":"2026-01-01T00:00:00Z"}"#;
        assert!(matches!(
            WorkspaceManifest::from_json(bytes),
            Err(CoreError::SchemaMismatch { found: 2, supported: 1 })
        ));
    }

    #[test]
    fn rejects_unknown_fields() {
        let bytes = br#"{"schemaVersion":1,"workspaceId":"w","createdAt":"2026-01-01T00:00:00Z","extra":0}"#;
        assert!(matches!(
            WorkspaceManifest::from_json(bytes),
            Err(CoreError::InvalidData(_))
        ));
    }
}
