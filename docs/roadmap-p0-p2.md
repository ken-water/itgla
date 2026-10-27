# P0-P2 Delivery Roadmap

## Delivered in the test build

- Schema 18 stores one endpoint per server access point and migrates numeric legacy ports without guessing a protocol.
- Endpoint rows support transport, protocol, purpose, copy, edit, remove and direct HTTP/HTTPS/SSH open actions.
- Direct open actions never include usernames or passwords. Other protocols remain copy-only.
- Services and Domains have separate navigation, with service versions, hosting links, domain registration/CDN fields and origin servers.
- Directed service relations cover dependency, replication, failover and balancing.
- Appearance has five live font-size steps, dark theme support and consistent typography. Guide is modal and blocks background navigation.

## P0 acceptance

- Keep a whole-workspace backup before opening a real database with the test build.
- Reopen the database after migration and confirm endpoint metadata, relations and domain records remain intact.
- Exercise invalid port input, duplicate transport entries, IPv6 copy, last-entry protection and rollback from the pre-schema backup.
- Report UI issues with the test build identifier `0.5.0-test-20261025` and the affected screen.

## P1 next increment

- Add credential-aware connection profiles for SSH and database clients without putting secrets in URLs, logs or exports.
- Add connection history and last-success timestamps, with explicit opt-in for any network check.
- Add focused filters for protocol, transport, service version, domain expiry and relation type.
- Add a compact topology view for a selected server, service or domain.

## P2 release hardening

- Produce signed Windows installers and the two macOS DMGs; build Linux packages on the Linux builder.
- Publish SHA256 manifests and verify every download URL, sitemap entry and `llm.txt` URL against the same release manifest.
- Add migration, UI accessibility, screenshot regression and package smoke tests to release CI.
- Add expiry reminders, backup restore drills and a documented rollback procedure before production release.
