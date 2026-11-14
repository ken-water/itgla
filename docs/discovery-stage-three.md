# Stage 3: authorized checks

Here, authorization means the operator's permission to inspect the selected
targets. It is separate from subscription entitlements. The proposed Free,
Pro and Team boundaries are documented in `discovery-packaging.md`; no paid
entitlement enforcement or purchase is currently available.

Stage 3 adds a small active-check kernel. It requires the operator to provide
the exact target scope and ports. It accepts hostnames, IP addresses, and
explicit CIDR ranges of at most 64 expanded addresses. It rejects URLs,
credentials, empty scopes, port zero, and scopes over 64 targets, 16 ports, or
256 target-port pairs.

The first profile is TCP connect. It uses a bounded per-connection timeout of
1–30 seconds and runs sequentially. The transport is injected, so UI and
integration tests do not need a live network. `TcpTransport` is a preliminary
network implementation; hostname resolution is not yet bounded by the TCP
timeout and must be fixed before exposing it to users.

Each result records target, operator label, port, profile, status, detail, and
elapsed time. The report records requested scope, start/end audit timestamps,
progress, and whether the operator cancelled. Results remain observations;
the module does not create or update servers, services, domains, or links.

Cancellation is checked before every pair. The caller owns the `Arc<AtomicBool>`
and should expose Start, Stop, and Cancel controls. UI must display the exact
target list, port list, profile, total pair count, and partial results.

Not included: password attempts, banner grabbing, exploit checks, stealth,
redirect following, automatic origin inference, or scanning discovered names
without explicit selection. TLS certificate and selected HTTP checks remain
the next bounded profiles and must use the same explicit scope and audit path.

## Acceptance tests

- A selected IP and two selected ports produce exactly two observations.
- A pre-set cancellation produces a stopped report with no checks.
- URL-shaped input, oversized CIDR, empty targets, empty ports, port zero, and
  oversized pair counts fail before transport calls.
- Duplicate targets and ports are normalized; labels and requested ports remain
  visible in the report.
- A fake transport proves status and progress without network access.

