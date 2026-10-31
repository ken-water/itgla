# ADR 0005: shared asset responsibility context

Date: 2026-10-29. Status: accepted for local implementation of phase one.

Business, server, service and domain records share lightweight maintenance context: technical contact, backup contact, environment, documentation availability/link, information source, and manual verification date/person. Contacts are local text references, not product accounts or legal ownership.

Store context in a schema-21 table keyed by resource type and stable ID. Existing records remain valid with empty context; no field becomes mandatory. Documentation has explicit Not recorded, Unknown, Not applicable and Link states. Empty contact values are Not recorded; literal Unknown and Not applicable are retained and shown explicitly. No data is silently inherited from a business or host.

Save context in the same transaction as its owning record. Invalid context rolls back record and automatic SSH creation. Preserve context in existing calls that do not edit it. Metadata/hosting changes invalidate manual verification; no-op saves do not. Hidden resources retain context, hard deletion removes it. Full SQLite backups preserve context; legacy project JSON is not a full inventory exchange. Schema 20 cannot open schema 21.

Before migrating, save backups/pre-schema-21.db. Migration is additive and transactional. Rollback requires preserving the current file, then using the schema-20 application with that snapshot; later edits would otherwise be lost. Test only isolated databases, never the user's active workspace.

UI: reuse a vertical, token-based shared component in the existing scrollable editors, with a fixed primary save action. Optional detail fields and verification controls remain keyboard accessible at maximum type size. Search includes responsibility/context; records preserve their original editable values. HTTP(S) documentation links only, no embedded credentials; clicking Open is explicit and never executes a command. Errors retain draft input. Manual checks are not uptime monitoring.

Scope excludes cloud sync, account permissions, legal ownership transfer, a contact directory, formal handover acceptance, new dependencies and public release. Success is creating a business/server/domain/service, locating its endpoint and responsible person, then reloading/restoring the same context without data loss. Real-user timed validation remains unmeasured.
