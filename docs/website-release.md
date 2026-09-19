# Website release

The official website is a dependency-free static site under `website/`. Product claims and download URLs must match an immutable GitHub release before production deployment.

## Production

- URL: `https://itgla.com`
- Host: the Nginx server configured for `itgla.com`
- Document root: `/var/www/itgla-app`
- Health check: `https://itgla.com/healthz`
- Release owner: `ken-water`

## Deploy contract

1. Verify HTML, internal links, downloads, checksums, accessibility basics, and 390x844, 768x1024, and 1440x900 layouts locally.
2. Build a candidate directory containing the website source and immutable release artifacts.
3. Copy the current production directory to a timestamped backup on the same filesystem.
4. Upload the candidate to a new directory, validate it through a temporary local Nginx server, then atomically switch the document-root directory.
5. Run `nginx -t`, HTTPS health, page, asset, download, checksum, and responsive smoke checks.

Rollback is an atomic directory rename back to the recorded backup, followed by the same health and download checks. The deployment changes no database, DNS, certificate, runtime service, or user data.
