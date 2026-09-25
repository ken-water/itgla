# ITGLA hosting and data-service isolation

## Current release boundary

ITGLA v0.2.0 is a local-first Slint desktop application. The public web component remains a static product/download site. A separate website-analytics service reads the dedicated ITGLA Nginx log and exposes a protected `/admin/` dashboard; it does not provide product sync or asset-management APIs.

The current-domain host is shared with VeloWrite and OpsProbe. PostgreSQL 16 is bound to loopback and contains separate `itgla_analytics` and `velowrite_analytics` databases. A Redis service used by OpsProbe is also bound to loopback. ITGLA does not connect to that Redis instance and must not use another product's database, role, analytics service, cache, filesystem, credentials, or application port.

## Static-site boundary

- Dedicated static root: `/var/www/itgla-site/releases/<release>`
- Dedicated Nginx file: `/etc/nginx/conf.d/itgla.conf`
- Dedicated service/account: `itgla-analytics.service` running as the `itgla` system user on `127.0.0.1:3420`
- Dedicated PostgreSQL role/database: `itgla_analytics`; the role owns only its database and has no superuser, database-creation, role-creation, or replication privileges
- Dedicated log: `/var/log/itgla/nginx-access.log`; original IP addresses are converted to salted visitor hashes during ingestion and are not stored in PostgreSQL
- Dedicated root-only secrets: `/etc/itgla/analytics.env`
- `itgla.com` and `www.itgla.com`: static product/download site only
- `app.itgla.com`: redirects to the website download page; no app service exists in v0.2.0
- `api.itgla.com`: returns an explicit JSON 404; no hosted API exists in v0.2.0
- `/admin/` alone proxies to ITGLA analytics. No location proxies ITGLA traffic to VeloWrite or exposes database/cache listeners.
- ITGLA intentionally has no Redis configuration: the single-instance dashboard keeps durable sessions in its isolated PostgreSQL database, so sharing OpsProbe's Redis would weaken isolation without providing an availability benefit.

## Future backend requirements

Do not provision these resources until a reviewed ITGLA backend has an explicit persistence/cache contract and migration. When required:

1. Create a dedicated PostgreSQL role and database (or separate cluster if threat model/availability requires it), with no superuser, replication, or `CREATEDB` privileges. Scope schema ownership and runtime grants; keep the port loopback/private and credentials in a root-readable service environment file.
2. Use a dedicated Redis instance or a verified Redis ACL user with a unique secret and key/channel namespace. Redis logical database numbers are not a security boundary. Bind only to loopback/private networking, disable public access, and define memory/eviction policy.
3. Give ITGLA its own system user, service unit, directories, logs, backups, health checks, and restore test. Never reuse VeloWrite accounts, tables, keys, environment files, ports, or backups.
4. Test schema migrations and rollback compatibility before moving real data. The old host is currently unreachable, so no server-side ITGLA data source has been identified or copied.
