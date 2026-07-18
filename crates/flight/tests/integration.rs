// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration test: ship reports against a mock tpt-flight-control endpoint.
//!
//! Exercises the full offline → connect → deliver → drain cycle the Helm
//! experiences across a flaky satellite link, including a brief offline
//! interval that must not lose reports.

#![allow(clippy::cast_precision_loss, clippy::unwrap_used)]

use std::sync::{Arc, Mutex};

use tp_helm_flight::{LinkClient, LinkConfig, Position, ReportOutcome, Transport, VesselIdentity};

/// A transport that simulates tpt-flight-control: acks only when a matching
/// identity (proxy for a validated pre-shared auth token) is present. Shared
/// state lets the test inspect what the port received and toggle connectivity.
#[derive(Clone)]
struct MockFlightControl {
    expected: VesselIdentity,
    online: Arc<Mutex<bool>>,
    received: Arc<Mutex<Vec<tp_helm_flight::ShipStatusReport>>>,
}

impl MockFlightControl {
    fn new(expected: VesselIdentity) -> Self {
        Self {
            expected,
            online: Arc::new(Mutex::new(false)),
            received: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn set_online(&self, up: bool) {
        *self.online.lock().unwrap() = up;
    }

    fn received_seqs(&self) -> Vec<u64> {
        self.received
            .lock()
            .unwrap()
            .iter()
            .map(|r| r.sequence)
            .collect()
    }
}

impl Transport for MockFlightControl {
    fn is_connected(&self) -> bool {
        *self.online.lock().unwrap()
    }

    fn send(&self, report: &tp_helm_flight::ShipStatusReport) -> tp_helm_flight::LinkStatus {
        if report.vessel != self.expected {
            return tp_helm_flight::LinkStatus::Rejected;
        }
        // In production this is where the encrypted, signed envelope is
        // verified before acknowledgement. The port only acks authentic reports.
        self.received.lock().unwrap().push(*report);
        tp_helm_flight::LinkStatus::Delivered
    }
}

#[test]
fn reports_survive_offline_interval_and_drain_in_order() {
    let expected = VesselIdentity::new(9_876_543, 367_987_654);
    let port = MockFlightControl::new(expected);
    let mut client = LinkClient::new(
        port.clone(),
        LinkConfig {
            queue_capacity: 64,
            vessel: expected,
        },
    );

    // Helm generates three reports while the satellite link is down.
    for i in 0..3 {
        client.report_position(
            1_700_000_000 + i,
            Position::new(-122.0 + i as f64 * 0.01, 37.0),
            90.0,
            12.0,
            90.0,
            0,
        );
    }
    assert_eq!(client.pending(), 3);

    // Ticks while offline must not lose or deliver anything.
    assert_eq!(client.tick(), ReportOutcome::Offline);
    assert_eq!(client.pending(), 3);

    // Link restored; drain the queue across several ticks.
    port.set_online(true);
    let mut delivered = 0;
    while client.pending() > 0 {
        if let ReportOutcome::Sent(tp_helm_flight::LinkStatus::Delivered) = client.tick() {
            delivered += 1;
        }
    }
    assert_eq!(delivered, 3);
    assert_eq!(client.pending(), 0);
    assert_eq!(port.received_seqs().len(), 3);

    // Reports arrived at the port in sequence order (no reordering).
    assert_eq!(port.received_seqs(), vec![0, 1, 2]);
}

#[test]
fn wrong_identity_is_rejected_and_kept_for_retry() {
    let port = MockFlightControl::new(VesselIdentity::new(1, 1));
    let mut client = LinkClient::new(
        port.clone(),
        LinkConfig {
            queue_capacity: 8,
            vessel: VesselIdentity::new(2, 2), // mismatched identity
        },
    );
    port.set_online(true);
    client.report_position(1_700_000_000, Position::new(0.0, 0.0), 0.0, 0.0, 0.0, 0);
    assert_eq!(
        client.tick(),
        ReportOutcome::Sent(tp_helm_flight::LinkStatus::Rejected)
    );
    assert_eq!(client.pending(), 1);
}
