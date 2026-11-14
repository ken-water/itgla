# Plan configuration

`plans.json` records the current free-first direction and future packaging
research. ITGLA currently has no asset-count limit, no payment flow and no
license enforcement. The application must remain useful without an account.

The numeric values in `plans.json` are future planning scenarios only. They are
not shown as current limits and must not block creating, editing, importing,
exporting or recovering assets. Prices are intentionally unpublished. The
website may say that paid options could be evaluated later, but must not claim
that a paid tier or checkout exists.

Do not use this file as a license, server authorization source or customer
secret store. If paid plans are ever introduced, add trusted entitlement checks,
a clear upgrade path and a safe transition for existing records first.
