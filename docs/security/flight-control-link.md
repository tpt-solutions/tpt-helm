# SPDX-License-Identifier: MIT OR Apache-2.0

# Security Review — TPT Helm ↔ tpt-flight-control Satellite Link

**Scope:** The satellite (Starlink) reporting path implemented in `crates/flight`.
This covers the wire schema (`schema`), the offline queue (`queue`), and the
delivery client (`client`).

**Status:** Internal security design review complete (see findings below).
An independent third-party audit is **still required** before operational use
(tracked in `todo.md` Phase 6).

## Threat model

- **Link is untrusted.** Starlink is a shared, internet-routable link. Adversaries
  may observe, replay, modify, or inject traffic between Helm and the port.
- **Adversary goals:** (a) feed the port false vessel positions, (b) learn vessel
  movements from intercepted reports, (c) suppress real reports (DoS).
- **Out of scope here:** physical tampering with the onboard terminal, compromise
  of the port-side `tpt-flight-control` service (owned by that project).

## Design controls (to be enforced by the production `Transport`)

| Threat | Control | Where |
| --- | --- | --- |
| Replay of an old report | Monotonic `sequence` + `report_time_epoch_s`; port rejects stale/out-of-window seqs | `schema::ShipStatusReport` |
| Report injection / impersonation | Authenticated envelope (e.g. HMAC with a pre-shared key or mTLS); port drops unauthenticated reports | production `Transport::send` |
| Interception of vessel track | Confidentiality via TLS 1.3 (or AEAD envelope) | production `Transport` |
| Report suppression (DoS) | Offline queue retries until acknowledged; bounded so memory is safe | `queue::ReportQueue` |
| Queue memory exhaustion | `queue_capacity` bound with oldest-dropped eviction | `queue::ReportQueue::enqueue` |

## Findings from internal review

1. **Queue ordering is preserved.** FIFO with `ack_head` draining guarantees the
   port sees reports in `sequence` order; gaps are detectable. (`queue` tests.)
2. **Rejected reports are retained, not dropped.** A non-`Delivered` status keeps
   the head in the queue for retry (`client::tick`), so transient port rejection
   cannot lose data.
3. **Identity mismatch is the proxy for auth failure.** The mock port rejects any
   report whose `VesselIdentity` does not match the expected partner; the
   production transport must extend this to cryptographic authentication.
4. **No plaintext secrets in the crate.** Auth material lives behind the
   `Transport` boundary; the core logic never handles keys directly, which keeps
   the auditable surface small.

## Open items before independent audit

- [ ] Specify and implement the authenticated+encrypted envelope (the `Transport`
      trait is the integration point; concrete impl lives outside this crate).
- [ ] Define the replay window and sequence-wrap policy at the port.
- [ ] Add a fuzz target for the serialized `ShipStatusReport` envelope.
- [ ] Document key provisioning / rotation for the onboard terminal.
