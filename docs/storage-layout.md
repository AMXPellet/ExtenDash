# Workspace Folder Layout (v1)

The layout every `StorageBackend` exposes. Paths are `/`-separated and relative to the workspace root. Code mirror: `core/src/layout.rs`. Schemas: `schemas/v1/`.

```
extendash/
  manifest.json                                    # workspace metadata
  extensions.json                                  # installed extensions, synced across devices
  snapshots/<source-id>.json                       # canonical state per source
  inbox/<source-id>/<timestamp>_<origin>_<rand>.json  # pending record changes
```

| Path | Schema | Written by |
|---|---|---|
| `manifest.json` | `manifest.schema.json` | App, on workspace creation |
| `extensions.json` | `extensions.schema.json` | App, on install/remove |
| `snapshots/<source-id>.json` | `snapshot.schema.json` | Compaction only |
| `inbox/<source-id>/…` | `inbox-event.schema.json` | Every writer (app edits, extensions, external scripts) |

## Ids

Source ids and origins match `^[a-z0-9][a-z0-9.-]{0,63}$`: safe as path segments, and free of `_`, which separates inbox filename parts. Record ids are opaque non-empty strings.

## Inbox filenames

`<timestamp>_<origin>_<rand>.json`

- `timestamp`: UTC, millisecond precision, `YYYYMMDDTHHMMSSmmmZ` (e.g. `20261021T201500123Z`). Sorts lexically in time order.
- `origin`: the writer (device id, extension id, or `app`).
- `rand`: 8 lowercase hex characters.

Inbox files are never overwritten by name. Backends that allow duplicate names (Google Drive) address files by their native id.

## Versioning

Every file carries `schemaVersion` (currently `1`). Readers reject versions they don't support with `SchemaMismatch` rather than guessing. Adding a field is a schema bump.

## Compaction

snapshot + inbox events → deterministic last-writer-wins merge per record id (by `updatedAt`, tombstones included) → write new snapshot → delete consumed inbox files. Implemented in Milestone 2.
