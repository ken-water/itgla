# Website release

The official website is a dependency-free static site under `website/`. Product claims and download URLs must match an immutable GitHub release before production deployment.

## Production

- URL: `https://itgla.com`
- Host: `186.244.233.223` (shared with VeloWrite; keep separate virtual hosts and roots)
- Document root: `/var/www/itgla-site/current`
- Health check: `https://itgla.com/healthz`
- `www.itgla.com` redirects to the apex; `app.itgla.com` redirects to downloads; `api.itgla.com` returns an explicit JSON 404 because v0.2.0 has no hosted API.
- Dedicated certificate: `/etc/letsencrypt/live/itgla.com/`, covering apex, www, app, and api names.
- Product data services: none for this static/local-first release. Website analytics uses its own least-privilege PostgreSQL role/database; Redis is not required.
- Admin analytics: `https://itgla.com/admin/`, backed by `itgla-analytics.service` on loopback only.
- Release owner: `ken-water`

## Deploy contract

1. Verify HTML, internal links, downloads, checksums, accessibility basics, and 390x844, 768x1024, and 1440x900 layouts locally.
2. Build a candidate directory containing the website source and immutable release artifacts.
3. Copy the current production directory to a timestamped backup on the same filesystem.
4. Upload the candidate to a new directory, validate it through a temporary local Nginx server, then atomically switch the document-root directory.
5. Run `nginx -t`, HTTPS health, page, asset, download, checksum, and responsive smoke checks.

Rollback is an atomic Nginx config/root switch back to the recorded backup, followed by the same health and download checks. The deployment must not edit the VeloWrite vhost, PostgreSQL cluster, Redis, DNS, or user data.
