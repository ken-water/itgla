# ITGLA hosting and data-service isolation

## Current release boundary

ITGLA v0.1.1 is a local-first Slint desktop application. The web component is a static product/download site. Neither component connects to a hosted API, PostgreSQL, or Redis. Consequently the current site deployment provisions no ITGLA database credentials, database, cache, or network listener.

The current-domain host is shared with VeloWrite. Its existing PostgreSQL 16 cluster is bound to loopback and contains a separate `velowrite_analytics` database. Redis is not installed. ITGLA must not use VeloWrite's database, role, analytics service, filesystem, credentials, or application port.

## Static-site boundary

- Dedicated static root: `/var/www/itgla-site/releases/<release>`
- Dedicated Nginx file: `/etc/nginx/conf.d/itgla.conf`
- `itgla.com` and `www.itgla.com`: static product/download site only
- `app.itgla.com`: redirects to the website download page; no app service exists in v0.1.1
- `api.itgla.com`: returns an explicit JSON 404; no hosted API exists in v0.1.1
- No location proxies ITGLA traffic to VeloWrite or exposes database/cache listeners.

## Future backend requirements

Do not provision these resources until a reviewed ITGLA backend has an explicit persistence/cache contract and migration. When required:

1. Create a dedicated PostgreSQL role and database (or separate cluster if threat model/availability requires it), with no superuser, replication, or `CREATEDB` privileges. Scope schema ownership and runtime grants; keep the port loopback/private and credentials in a root-readable service environment file.
2. Use a dedicated Redis instance or a verified Redis ACL user with a unique secret and key/channel namespace. Redis logical database numbers are not a security boundary. Bind only to loopback/private networking, disable public access, and define memory/eviction policy.
3. Give ITGLA its own system user, service unit, directories, logs, backups, health checks, and restore test. Never reuse VeloWrite accounts, tables, keys, environment files, ports, or backups.
4. Test schema migrations and rollback compatibility before moving real data. The old host is currently unreachable, so no server-side ITGLA data source has been identified or copied.
