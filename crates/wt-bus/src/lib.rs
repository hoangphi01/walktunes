// SPDX-License-Identifier: GPL-3.0-or-later
//! **wt-bus** — typed in-process events (EventBridge; `docs/ARCHITECTURE.md` §3.3).
//!
//! Events are **notifications, not commands**: nothing may depend on an event
//! being delivered to stay correct. The Ledger is always the state of record.

use std::sync::{Arc, Mutex, PoisonError};

use serde::Serialize;
use wt_core::{DeviceId, JobId};

/// Every event published on the bus.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WtEvent {
    /// A volume belonging to a recognized device was mounted.
    DeviceAttached {
        /// The device, if already paired.
        device: Option<DeviceId>,
        /// Human-readable model or family, e.g. "Sony NW-ZX300".
        name: String,
    },
    /// A device's volumes were all unmounted.
    DeviceDetached {
        /// The device, if paired.
        device: Option<DeviceId>,
    },
    /// Progress of a background job (STYLE_RULE §13.6 progress model).
    JobProgress {
        /// The job.
        job: JobId,
        /// Items or bytes done.
        done: u64,
        /// Items or bytes in total.
        total: u64,
    },
    /// A background job finished (successfully or not).
    JobFinished {
        /// The job.
        job: JobId,
        /// Error code if it failed, e.g. `WT-SYNC-0003`.
        error: Option<String>,
    },
}

/// A broadcast bus: every subscriber receives every event published after it subscribed.
#[derive(Clone, Default)]
pub struct Bus {
    subscribers: Arc<Mutex<Vec<flume::Sender<WtEvent>>>>,
}

impl Bus {
    /// Creates an empty bus.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Subscribes to all future events. The receiver works both blocking and `async`.
    #[must_use]
    pub fn subscribe(&self) -> flume::Receiver<WtEvent> {
        let (tx, rx) = flume::unbounded();
        self.subscribers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(tx);
        rx
    }

    /// Publishes an event to every live subscriber; dropped subscribers are removed.
    pub fn publish(&self, event: &WtEvent) {
        self.subscribers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .retain(|tx| tx.send(event.clone()).is_ok());
    }

    /// Number of live subscribers (useful in tests and diagnostics).
    #[must_use]
    pub fn subscriber_count(&self) -> usize {
        self.subscribers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_subscriber_receives_events() {
        let bus = Bus::new();
        let a = bus.subscribe();
        let b = bus.subscribe();
        let event = WtEvent::DeviceDetached { device: None };
        bus.publish(&event);
        assert_eq!(a.try_recv().ok(), Some(event.clone()));
        assert_eq!(b.try_recv().ok(), Some(event));
    }

    #[test]
    fn dropped_subscribers_are_pruned() {
        let bus = Bus::new();
        drop(bus.subscribe());
        let _kept = bus.subscribe();
        bus.publish(&WtEvent::DeviceDetached { device: None });
        assert_eq!(bus.subscriber_count(), 1);
    }
}
