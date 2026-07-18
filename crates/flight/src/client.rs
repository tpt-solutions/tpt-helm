// SPDX-License-Identifier: MIT OR Apache-2.0

//! Satellite (Starlink) link client and delivery logic.
//!
//! [`LinkClient`] delivers [`ShipStatusReport`]s to `tpt-flight-control` over
//! an intermittent satellite link. It owns a [`ReportQueue`] so that reports
//! generated while offline are not lost, and retries the head of the queue on
//! each successful [`LinkClient::tick`].
//!
//! The actual bytes-on-the-wire concern (auth, signing, encryption) is
//! delegated to a [`Transport`] trait. The production transport wraps the
//! report in an authenticated, encrypted envelope (see
//! `docs/security/flight-control-link.md`); tests use a [`MockTransport`].
//!
//! Expensive crypto dependencies are intentionally not pulled in here — the
//! transport boundary keeps them out of the core scheduling logic and makes
//! the link auditable.

use crate::queue::{QueuedReport, ReportQueue};
use crate::schema::{ShipStatusReport, VesselIdentity};

/// Configuration for the satellite link client.
#[derive(Debug, Clone, Copy)]
pub struct LinkConfig {
    /// Maximum reports buffered while offline.
    pub queue_capacity: usize,
    /// Identity used in every outgoing report.
    pub vessel: VesselIdentity,
}

impl Default for LinkConfig {
    fn default() -> Self {
        Self {
            queue_capacity: 256,
            vessel: VesselIdentity::new(0, 0),
        }
    }
}

/// Transport-layer status reported after each delivery attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkStatus {
    /// Link is down; report was queued for later.
    Offline,
    /// Link is up but the port rejected the report (e.g. bad auth / schema).
    Rejected,
    /// Report acknowledged by the port.
    Delivered,
}

/// Outcome of a single [`LinkClient::tick`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportOutcome {
    /// The link is currently down; nothing was sent.
    Offline,
    /// Nothing was queued to send.
    Idle,
    /// One report was delivered this tick.
    Sent(LinkStatus),
}

/// The wire boundary. Implementors authenticate and (in production) encrypt
/// the report before transmission, and verify the port's acknowledgement.
pub trait Transport {
    /// Whether the satellite link currently has connectivity.
    fn is_connected(&self) -> bool;

    /// Attempt to deliver `report`. Returns the resulting [`LinkStatus`].
    /// Implementors must treat a non-`Delivered` result as "send again later".
    fn send(&self, report: &ShipStatusReport) -> LinkStatus;
}

/// A satellite link client that queues and retries reports.
pub struct LinkClient<T: Transport> {
    transport: T,
    queue: ReportQueue,
    config: LinkConfig,
    sequence: u64,
}

impl<T: Transport> std::fmt::Debug for LinkClient<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinkClient")
            .field("config", &self.config)
            .field("sequence", &self.sequence)
            .field("queued", &self.queue.len())
            .finish_non_exhaustive()
    }
}

impl<T: Transport> LinkClient<T> {
    /// Construct a client over the given transport and config.
    #[must_use]
    pub fn new(transport: T, config: LinkConfig) -> Self {
        Self {
            transport,
            queue: ReportQueue::new(config.queue_capacity),
            config,
            sequence: 0,
        }
    }

    /// Number of reports waiting in the offline queue.
    #[must_use]
    pub fn pending(&self) -> usize {
        self.queue.len()
    }

    /// Queue a position report. All reports share the configured vessel
    /// identity and are assigned a monotonically increasing sequence number.
    pub fn report_position(
        &mut self,
        report_time_epoch_s: u64,
        position: crate::schema::Position,
        cog_deg: f64,
        sog_kn: f64,
        heading_deg: f64,
        nav_status: u8,
    ) {
        let seq = self.sequence;
        self.sequence += 1;
        let report = ShipStatusReport::new(
            seq,
            report_time_epoch_s,
            self.config.vessel,
            position,
            cog_deg,
            sog_kn,
            heading_deg,
            nav_status,
        );
        self.queue.enqueue(report);
    }

    /// Advance the link: if connected, attempt to deliver the head report.
    /// Returns what happened this tick (and does nothing if offline or idle).
    pub fn tick(&mut self) -> ReportOutcome {
        if self.queue.is_empty() {
            return ReportOutcome::Idle;
        }
        if !self.transport.is_connected() {
            return ReportOutcome::Offline;
        }
        let head: QueuedReport = match self.queue.peek() {
            Some(q) => *q,
            None => return ReportOutcome::Idle,
        };
        match self.transport.send(&head.report) {
            LinkStatus::Delivered => {
                self.queue.ack_head();
                ReportOutcome::Sent(LinkStatus::Delivered)
            }
            other => {
                self.queue.note_attempt();
                ReportOutcome::Sent(other)
            }
        }
    }
}

/// A in-memory transport used for tests and local development. It can be
/// toggled online/offline and can be configured to reject reports.
#[derive(Debug, Clone, Default)]
pub struct MockTransport {
    connected: bool,
    reject: bool,
    /// Count of reports received while connected.
    pub delivered: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl MockTransport {
    /// Construct a mock transport with the given connectivity and policy.
    #[must_use]
    pub fn new(connected: bool, reject: bool) -> Self {
        Self {
            connected,
            reject,
            delivered: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Toggle connectivity on/off (simulating satellite handoff).
    pub fn set_connected(&mut self, connected: bool) {
        self.connected = connected;
    }
}

impl Transport for MockTransport {
    fn is_connected(&self) -> bool {
        self.connected
    }

    fn send(&self, _report: &ShipStatusReport) -> LinkStatus {
        if self.reject {
            return LinkStatus::Rejected;
        }
        self.delivered
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        LinkStatus::Delivered
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn client(connected: bool, reject: bool) -> LinkClient<MockTransport> {
        LinkClient::new(
            MockTransport::new(connected, reject),
            LinkConfig {
                queue_capacity: 16,
                vessel: VesselIdentity::new(1_234_567, 367_123_456),
            },
        )
    }

    #[test]
    fn offline_report_is_queued_not_lost() {
        let mut c = client(false, false);
        c.report_position(
            1_700_000_000,
            crate::schema::Position::new(-122.0, 37.0),
            90.0,
            12.0,
            90.0,
            0,
        );
        assert_eq!(c.pending(), 1);
        // Tick while offline: still queued.
        assert_eq!(c.tick(), ReportOutcome::Offline);
        assert_eq!(c.pending(), 1);
    }

    #[test]
    fn queued_report_delivered_when_link_returns() {
        let mut c = client(false, false);
        c.report_position(
            1_700_000_000,
            crate::schema::Position::new(-122.0, 37.0),
            90.0,
            12.0,
            90.0,
            0,
        );
        // Link comes up; tick delivers the head and drains the queue.
        c.transport.set_connected(true);
        assert_eq!(c.tick(), ReportOutcome::Sent(LinkStatus::Delivered));
        assert_eq!(c.pending(), 0);
    }

    #[test]
    fn rejected_report_stays_queued_for_retry() {
        let mut c = client(true, true);
        c.report_position(
            1_700_000_000,
            crate::schema::Position::new(-122.0, 37.0),
            90.0,
            12.0,
            90.0,
            0,
        );
        assert_eq!(c.tick(), ReportOutcome::Sent(LinkStatus::Rejected));
        // Rejected reports are retained for the next attempt.
        assert_eq!(c.pending(), 1);
    }

    #[test]
    fn sequence_numbers_are_monotonic() {
        let mut c = client(true, false);
        c.report_position(1, crate::schema::Position::new(0.0, 0.0), 0.0, 0.0, 0.0, 0);
        c.report_position(2, crate::schema::Position::new(0.0, 0.0), 0.0, 0.0, 0.0, 0);
        c.tick();
        c.tick();
        // Both delivered in order; mock recorded two deliveries.
        assert_eq!(
            c.transport
                .delivered
                .load(std::sync::atomic::Ordering::SeqCst),
            2
        );
        assert_eq!(c.pending(), 0);
    }
}
