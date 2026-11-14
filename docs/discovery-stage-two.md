# Discovery Stage 2: review workflow

## Scope

Promote sample data and public URL discovery as the two first-use choices in the guide and Overview. Keep manual entry/import available as secondary actions. The guide can always be closed. Discovery entry exits sample mode before opening the real-workspace URL form.

Deliver review by run, status/type/search filters, bounded pages, selected-run warnings, retry using the run's original URL, and an explicit preview for associating observations with matching existing domains/sites. Association preserves names, notes, credentials, visibility, host links and other user-owned fields. It never promotes DNS addresses to servers. Persist the evidence-to-asset association atomically with confirmation, and retain review history.

Acceptance covers independent run filtering and pagination beyond the old 200-observation window, duplicate identity matching, hidden assets, invalid target rejection, atomic rollback, schema-24 upgrade, sample isolation, guide shortcuts and layout across three font sizes.

## Data and compatibility

Schema 25 adds only discovery-to-asset links and a run/page index. Automatic pre-schema-25 backup precedes migration. Existing observations, inventory and review history are retained. Older clients cannot open schema 25. Rollback requires closing the client and restoring the matching backup, preserving the newer file separately.

Only matching domain names or normalized web URL identities can be associated in this iteration. Linking observations cannot overwrite asset fields. Stale/ignored observations do not remove associated assets. Deleted targets are reported as unavailable instead of presenting a broken link as current data.

## Subsequent enrichment

Typed DNS/TTL/CNAME data, public/private suffix scope, TLS SANs, opt-in certificate transparency and bounded recursive sitemap discovery are now implemented in the follow-up enrichment increment. See [discovery-enrichment.md](discovery-enrichment.md) for contracts, limits, dependency decisions and validation. Field-level conflict resolution remains deferred; existing user-owned fields are preserved.
