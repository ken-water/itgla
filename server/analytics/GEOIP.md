# Local GeoIP setup

The analytics service performs GeoIP lookups locally. It never calls a remote
geolocation API while ingesting access logs.

Set:

```text
ITGLA_GEOIP_DB_PATH=/var/lib/itgla-analytics/geo.mmdb
ITGLA_GEOIP_PROVIDER=db-ip-lite
```

Place a MaxMind DB-compatible file at that path, for example a DB-IP Lite
City/ASN database. Keep the file readable by the `itgla` service user:

```sh
install -o itgla -g itgla -m 0640 downloaded-city.mmdb /var/lib/itgla-analytics/geo.mmdb
```

If the file is absent or unreadable, analytics continues without geographic
enrichment. Existing events are not sent to a third-party API.

The admin traffic and download views expose source IP and approximate location
only to authorized administrators. IP and detailed GeoIP fields are cleared
after 180 days; aggregate event counters use the same retention period.

Keep the provider's attribution and license notice with the deployed database.
Update the database through a reviewed maintenance job rather than downloading
it during request processing.
