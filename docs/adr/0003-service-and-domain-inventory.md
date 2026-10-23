# ADR 0003: Small service and domain inventory

- Status: accepted for local implementation
- Date: 2026-10-24

## Outcome and scope

Let a single operator record service instances on existing server records, their versions, ports, access addresses, tags and notes; record domains/hostnames with registrar, registration/expiry dates, CDN notes and origins; describe directed service dependencies, replication, disaster recovery and load balancing with notes. Use a list and form, with no discovery agent, DNS changes, automatic topology or additional service/instance abstraction.

The existing project `assets` model is a separate legacy planning model; its server IDs are not `server_records` IDs. Link operational inventory explicitly to the currently visible `server_records` table rather than guessing equivalent assets. One service row represents a named deployment; distinct versions or credentials get distinct rows. Multiple origins can be attached to a domain. The parent domain is a validated DNS suffix, not a separate graph hierarchy.

## Persistence and credentials

Schema 16 adds inventory, hosting and directed relation tables transactionally; existing tables are unchanged. Before upgrading an on-disk database, make `backups/pre-schema-16.db`. Older binaries reject schema 16; rollback uses that pre-upgrade snapshot, losing subsequent edits. Full SQLite backups retain new records and links. Legacy project JSON exchange does not contain this inventory, and the inventory UI says to use the full workspace backup.

Use Windows DPAPI CurrentUser to protect the username/password pair, without an extra dependency. Store only the protected blob; metadata, tags and notes are not secret fields. Never put credentials in search, list rows, errors, normal clipboard notices or JSON. Showing/copying credentials is explicit and requires successfully decrypting. Hide again after 30 seconds and on navigation; clear the editor on record changes. Clipboard clearing is best effort and only when it still contains the copied secret. DPAPI binds credentials to the Windows user profile; copying the database to another user/device does not by itself transfer access. Non-Windows builds support metadata but fail closed on credential writes/reads until an OS credential backend is implemented. This limitation must be visible before entry, never silently store plaintext.

## Gates and recovery

Test schema-15 migration and preservation, host/target integrity, save/reopen/hide, relation direction and inverse display, date/hostname/port validation, bounded fields, password update/keep/remove, plaintext absence, backup/restore and fail-closed credential behavior. Run fmt, check, clippy, tests and locked build with the existing Rust 1.97 environment if Windows Rust 1.89 cannot compile Slint. Run DPAPI tests natively on Windows; no real credentials in fixtures. Inspect the new UI with synthetic data at 1440x900 and a smaller supported window. Do not claim release or publish artifacts from local-only validation.
