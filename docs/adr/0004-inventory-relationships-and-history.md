# ADR 0004: Inventory relationships, history, and connection map

## Status

Accepted for local implementation.

## Context

Services and Domains need to be scanned and compared as tables. Resource ownership and dependencies also need to be understandable without maintaining a second, disconnected graph model. The existing database stores domain parent names, server-to-service/domain associations, and service-to-service links separately. It has no operation history for inventory records.

## Decisions

- Keep Services and Domains as compact tables. Add explicit Businesses and Connections destinations before Settings; the Connections shortcut in either table opens the same shared map.
- Use compact tables for routine lookup and editing; put longer edits in a dedicated editor state and server inspection in a read-only details dialog.
- Persist business entities and typed resource links in SQLite. Server hosting/origin assignments remain backed by `operational_hosts`; existing service relations are migrated into the common links table.
- Record lifecycle and edit events locally, with human-readable summaries that never include credential values.
- Treat old server timestamps as known metadata and mark the history baseline explicitly. Do not infer unavailable historical imports, edits, hides, or restores for inventory records.
- Render the relationship map from the same saved entities and links as the table. Relationship edits from the map use the same repository operations and validations.
- Keep the map bounded to a filtered local workspace view; no scanning, remote connection, cloud sync, or automatic DNS discovery is included.

## Consequences

- The database schema is additive and includes a pre-migration backup.
- Existing service relations remain visible after migration.
- The map is deterministic and accessible, with lanes and explicit directed links rather than a free-form canvas whose positions could obscure relationship meaning.
- Users can create, edit, and remove business/domain/server/service links without moving records between separate modules.

## Validation

- Migration/reopen/backup tests preserve existing servers, services, domains, endpoints, credentials, and service relations.
- History tests cover create, update, hide, restore, import baseline, and credential redaction.
- Relationship tests cover resource validation, self-links, duplicates, updates, and deletion.
- UI tests verify table/map selection, server details/history, empty states, and keyboard-accessible controls.
