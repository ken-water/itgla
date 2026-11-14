# Discovery certificate fixture

`itgla-leaf-20261114.pem` contains only the public HTTPS leaf certificate retrieved from the owned `itgla.com` endpoint on 2026-11-14, using curl certificate output through the local proxy. No private key or credentials are included. The leaf is issued by Let's Encrypt YE2, expires at 2026-12-22 14:11:33 UTC, and has four SANs: itgla.com, www.itgla.com, api.itgla.com, app.itgla.com.

Tests decode this fixed historical certificate to verify metadata, SAN scope and dates. They do not treat fixture parsing as chain validation or as evidence of the live endpoint's current certificate. Production curl verifies trust and hostname before exposing certificate metadata.
