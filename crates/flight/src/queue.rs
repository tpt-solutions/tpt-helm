// SPDX-License-Identifier: MIT OR Apache-2.0

//! Offline queue for satellite reports.
//!
//! The Starlink link is intermittent. Reports that cannot be delivered
//! immediately are held in a persistent, order-preserving queue and retried on
//! the next successful connection. The queue bounds its memory footprint and
//! preserves report order so the port can detect sequence gaps on replay.

use std::collections::VecDeque;

use crate::schema::ShipStatusReport;

/// A report held for (re)transmission, with its enqueue order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QueuedReport {
    /// The queued report.
    pub report: ShipStatusReport,
    /// Number of delivery attempts so far (0 = never sent).
    pub attempts: u32,
}

impl QueuedReport {
    /// Construct a fresh queued report (no attempts yet).
    #[must_use]
    pub fn new(report: ShipStatusReport) -> Self {
        Self {
            report,
            attempts: 0,
        }
    }
}

/// A bounded, FIFO queue of undelivered reports.
#[derive(Debug, Clone, Default)]
pub struct ReportQueue {
    inner: VecDeque<QueuedReport>,
    max_len: usize,
}

impl ReportQueue {
    /// Construct a queue that holds at most `max_len` reports.
    #[must_use]
    pub fn new(max_len: usize) -> Self {
        Self {
            inner: VecDeque::new(),
            max_len: max_len.max(1),
        }
    }

    /// Enqueue a report. If the queue is full, the oldest report is dropped
    /// (it is the least recent and least likely to be actionable at the port).
    pub fn enqueue(&mut self, report: ShipStatusReport) {
        if self.inner.len() >= self.max_len {
            self.inner.pop_front();
        }
        self.inner.push_back(QueuedReport::new(report));
    }

    /// Peek the next report to send (FIFO), without removing it.
    #[must_use]
    pub fn peek(&self) -> Option<&QueuedReport> {
        self.inner.front()
    }

    /// Mark the head report as attempted and remove it (call after a confirmed
    /// acknowledgement from the port). Returns the report that was removed.
    pub fn ack_head(&mut self) -> Option<ShipStatusReport> {
        self.inner.pop_front().map(|q| q.report)
    }

    /// Record a failed attempt on the head report (without removing it), so a
    /// subsequent connection can retry it.
    pub fn note_attempt(&mut self) {
        if let Some(head) = self.inner.front_mut() {
            head.attempts += 1;
        }
    }

    /// Number of reports waiting for delivery.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// True when the queue is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Drain the queue into a vector (oldest first), clearing it.
    #[must_use]
    pub fn drain_all(&mut self) -> Vec<QueuedReport> {
        self.inner.drain(..).collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::schema::{Position, VesselIdentity};

    fn report(seq: u64) -> ShipStatusReport {
        ShipStatusReport::new(
            seq,
            1_700_000_000 + seq,
            VesselIdentity::new(1, 2),
            Position::new(0.0, 0.0),
            90.0,
            12.0,
            90.0,
            0,
        )
    }

    #[test]
    fn enqueue_preserves_fifo_order() {
        let mut q = ReportQueue::new(10);
        q.enqueue(report(1));
        q.enqueue(report(2));
        q.enqueue(report(3));
        assert_eq!(q.len(), 3);
        assert_eq!(q.peek().unwrap().report.sequence, 1);
        assert_eq!(q.ack_head().unwrap().sequence, 1);
        assert_eq!(q.ack_head().unwrap().sequence, 2);
        assert_eq!(q.ack_head().unwrap().sequence, 3);
        assert!(q.is_empty());
    }

    #[test]
    fn full_queue_drops_oldest() {
        let mut q = ReportQueue::new(2);
        q.enqueue(report(1));
        q.enqueue(report(2));
        q.enqueue(report(3));
        assert_eq!(q.len(), 2);
        // Oldest (seq 1) was dropped; head is now seq 2.
        assert_eq!(q.peek().unwrap().report.sequence, 2);
    }

    #[test]
    fn attempts_are_recorded() {
        let mut q = ReportQueue::new(5);
        q.enqueue(report(7));
        q.note_attempt();
        q.note_attempt();
        assert_eq!(q.peek().unwrap().attempts, 2);
    }
}
