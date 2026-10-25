//! Runtime fault injection, shared by every mock.
//!
//! `CASE-CHAOS-001..003` all need the same capability: make a dependency fail
//! *while the system under test is mid-request*, then observe recovery. A
//! config file read at startup cannot express "fail on the third call"; an HTTP
//! API can.

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

/// How a fault behaves once injected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaultMode {
    /// Fail every call until cleared.
    Always,
    /// Fail the next `count` calls, then behave normally. This is how a
    /// transient outage is expressed — and it is what proves the system
    /// actually recovers rather than merely failing.
    NextN,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fault {
    /// A named switch, e.g. `psp.authorize`, `broker.publish`, `db.write`.
    pub target: String,
    #[serde(default = "default_mode")]
    pub mode: FaultMode,
    /// For [`FaultMode::NextN`]. Defaults to 1.
    #[serde(default = "default_count")]
    pub count: u32,
    /// Artificial latency before responding, in milliseconds. Models a
    /// provider that is slow rather than down.
    #[serde(default)]
    pub latency_ms: u64,
    /// For an HTTP mock: status to return. 503 models an outage.
    #[serde(default)]
    pub http_status: u16,
    /// For a payload-level fault: drop the response *after* recording the side
    /// effect. This is the important one — it models "the charge succeeded but
    /// the answer never arrived", which is exactly the `payment_unknown` case.
    #[serde(default)]
    pub drop_response: bool,
}

fn default_mode() -> FaultMode {
    FaultMode::Always
}
fn default_count() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize)]
pub struct ActiveFault {
    pub target: String,
    pub mode: FaultMode,
    pub remaining: u32,
    pub latency_ms: u64,
    pub http_status: u16,
    pub drop_response: bool,
    /// How many times this fault has fired. Lets a test assert that recovery
    /// actually happened rather than assuming it.
    pub fired: u64,
}

#[derive(Clone, Default, Debug)]
pub struct FaultController {
    inner: Arc<RwLock<Inner>>,
}

#[derive(Default, Debug)]
struct Inner {
    faults: HashMap<String, ActiveFault>,
}

impl FaultController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inject(&self, fault: Fault) {
        let mut g = self.inner.write().expect("fault lock");
        g.faults.insert(
            fault.target.clone(),
            ActiveFault {
                target: fault.target.clone(),
                mode: fault.mode,
                remaining: match fault.mode {
                    FaultMode::Always => u32::MAX,
                    FaultMode::NextN => fault.count.max(1),
                },
                latency_ms: fault.latency_ms,
                http_status: if fault.http_status == 0 {
                    503
                } else {
                    fault.http_status
                },
                drop_response: fault.drop_response,
                fired: 0,
            },
        );
    }

    pub fn clear(&self) {
        self.inner.write().expect("fault lock").faults.clear();
    }

    pub fn list(&self) -> Vec<ActiveFault> {
        let g = self.inner.read().expect("fault lock");
        let mut v: Vec<_> = g.faults.values().cloned().collect();
        v.sort_by(|a, b| a.target.cmp(&b.target));
        v
    }

    /// Check whether `target` should fail right now, consuming one use of a
    /// `NextN` fault. Returns the fault to apply, or `None` to behave normally.
    pub fn take(&self, target: &str) -> Option<ActiveFault> {
        let mut g = self.inner.write().expect("fault lock");
        let f = g.faults.get_mut(target)?;
        if f.remaining == 0 {
            return None;
        }
        if f.remaining != u32::MAX {
            f.remaining -= 1;
        }
        f.fired += 1;
        Some(f.clone())
    }

    /// Read without consuming. Used by `/__control/faults` assertions.
    pub fn peek(&self, target: &str) -> Option<ActiveFault> {
        self.inner
            .read()
            .expect("fault lock")
            .faults
            .get(target)
            .cloned()
    }

    pub fn is_firing(&self, target: &str) -> bool {
        self.peek(target).map(|f| f.remaining > 0).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn always_fault_never_expires() {
        let c = FaultController::new();
        c.inject(Fault {
            target: "psp.authorize".into(),
            mode: FaultMode::Always,
            count: 1,
            latency_ms: 0,
            http_status: 0,
            drop_response: false,
        });
        for _ in 0..50 {
            assert!(c.is_firing("psp.authorize"));
            assert!(c.take("psp.authorize").is_some());
        }
    }

    #[test]
    fn nextn_fault_expires_and_then_clears_itself() {
        let c = FaultController::new();
        c.inject(Fault {
            target: "broker.publish".into(),
            mode: FaultMode::NextN,
            count: 3,
            latency_ms: 0,
            http_status: 0,
            drop_response: false,
        });
        for _ in 0..3 {
            assert!(c.take("broker.publish").is_some(), "first 3 must fail");
        }
        // Fourth call must succeed — this is what proves recovery.
        assert!(c.take("broker.publish").is_none());
        assert!(!c.is_firing("broker.publish"));
    }

    #[test]
    fn faults_are_independent_per_target() {
        let c = FaultController::new();
        for target in ["psp.authorize", "psp.reconcile"] {
            c.inject(Fault {
                target: target.into(),
                mode: FaultMode::Always,
                count: 1,
                latency_ms: 0,
                http_status: 0,
                drop_response: false,
            });
        }
        assert!(c.is_firing("psp.authorize"));
        assert!(c.is_firing("psp.reconcile"));
        assert!(!c.is_firing("db.write"));
    }

    #[test]
    fn fired_count_lets_a_test_prove_recovery() {
        let c = FaultController::new();
        c.inject(Fault {
            target: "x".into(),
            mode: FaultMode::NextN,
            count: 1,
            latency_ms: 0,
            http_status: 0,
            drop_response: false,
        });
        assert!(c.take("x").is_some());
        assert!(c.take("x").is_none());
        assert_eq!(c.peek("x").unwrap().fired, 1);
    }

    #[test]
    fn clear_removes_everything() {
        let c = FaultController::new();
        c.inject(Fault {
            target: "a".into(),
            mode: FaultMode::Always,
            count: 1,
            latency_ms: 0,
            http_status: 0,
            drop_response: false,
        });
        c.clear();
        assert!(c.list().is_empty());
        assert!(!c.is_firing("a"));
    }

    #[test]
    fn default_status_is_503_not_zero() {
        let c = FaultController::new();
        c.inject(Fault {
            target: "a".into(),
            mode: FaultMode::Always,
            count: 1,
            latency_ms: 0,
            http_status: 0,
            drop_response: false,
        });
        assert_eq!(c.peek("a").unwrap().http_status, 503);
    }
}
