# SPDX-License-Identifier: MIT OR Apache-2.0

# Independent Security Review — GPS Spoofing Detection

**Scope:** The GPS spoofing / interference detection logic in `crates/spoof`
(inertial + celestial cross-checks, confidence scoring, alerting).

**Status:** Internal correctness and adversarial scenario testing complete
(unit + scenario suites in `crates/spoof/tests/spoofing_scenarios.rs`). An
**independent third-party security audit is still required** before operational
use (tracked in `todo.md` Phase 4).

## What is covered

- Cross-check of the GPS fix against two RF-independent references (INS, celestial).
- Confidence scoring via a saturating logistic of residual / combined sigma.
- Actionable alerting (`Warning`/`Alarm`) and graceful handling of missing
  references (e.g. no celestial fix at night).

## Adversarial scenarios validated by tests

| Scenario | Result |
| --- | --- |
| Legitimate operation, references agree | No actionable alert (no false alarm) |
| Gross position offset (~10 km) | `Alarm` raised |
| Slowly drifting spoof | Detected once residual exceeds envelope |
| Step spoof | Detected immediately |
| Spoof fooling only one reference | Still detected by the other |
| Partially coordinated spoof (references disagree) | Detected |
| Fully coordinated spoof (matched velocity, all references spoofed) | **Documented limitation** — not detectable by cross-check alone |

## Known limitations (must be in the safety case)

1. **Fully coordinated spoof.** If an adversary simultaneously spoofs GPS *and*
   the inertial/celestial references with a matched trajectory, the cross-check
   cannot detect it. This requires additional defenses (signal-authenticity /
   RAIM, multi-constellation, trust anchors) outside `crates/spoof`.
2. **Celestial accuracy.** Celestial fixes are deliberately conservative
   (~1 NM). They are a slow, coarse reference — they confirm gross divergence,
   not meter-level integrity.
3. **INS drift.** Long unaligned intervals grow the INS envelope; the detector
   correctly widens tolerance, but a slowly accelerating drift could be masked
   near the saturation threshold.

## Open items before independent audit

- [ ] Engage an accredited maritime-cybersecurity reviewer for the detection logic.
- [ ] Add RAIM / signal-authenticity inputs as a third reference class.
- [ ] Document the confidence thresholds' false-alarm vs. miss trade-off.
- [ ] Add a fuzz/property test asserting "consistent references never alert".
