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
- Admin analytics: `https://itgla.com/admin`, backed by `itgla-analytics.service` on loopback only.
- Release owner: `ken-water`

## Deploy contract

1. Verify HTML, internal links, downloads, checksums, accessibility basics, and 390x844, 768x1024, and 1440x900 layouts locally.
2. Build a candidate directory containing the website source and immutable release artifacts.
3. Copy the current production directory to a timestamped backup on the same filesystem.
4. Upload the candidate to a new directory, validate it through a temporary local Nginx server, then atomically switch the document-root directory.
5. Run `nginx -t`, HTTPS health, page, asset, download, checksum, and responsive smoke checks.

Rollback is an atomic Nginx config/root switch back to the recorded backup, followed by the same health and download checks. The deployment must not edit the VeloWrite vhost, PostgreSQL cluster, Redis, DNS, or user data.

## Discovery and legacy URL checks

- Keep every public page listed in `sitemap.xml` in `llms.txt`; the latter also links to the sitemap and official repository. Account and error pages remain outside this page inventory.
- `/llm.txt` permanently redirects to `/llms.txt`.
- `/downloads/v0.3.30/SHA256SUMS` permanently redirects to the canonical lowercase `sha256sums` manifest. Use the exact Nginx location rather than case-colliding files, so Windows checkouts preserve compatibility.
- Validate both redirects through candidate Nginx and production HTTPS, following each to a 200 response. Run `sha256sum -c sha256sums` against all six release packages.
- Offer only implemented sign-in providers. The website currently supports email verification links.

## v0.4.0 publishing

The desktop source is tagged v0.4.0. Platform binaries are immutable GitHub Release assets, fetched by website CI before link/download checks rather than committed to Git. The website deploy includes all six packages and their lowercase sha256sums manifest in downloads/v0.4.0, preserving historical directories. New Linux builds may set ITGLA_APPIMAGE_RUNTIME_SOURCE to the previous verified AppImage if the upstream runtime download is unavailable; only its launcher runtime prefix is reused, with the application filesystem rebuilt from the current binary.
