//! Immutable file snapshots with a process-wide allocation budget.
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use anyhow::Result;
use bytes::Bytes;

// Includes cached bodies, in-flight fills, and old responses retained across
// cache eviction/reload. Cache membership alone cannot bound those lifetimes.
const MAX_SNAPSHOT_BYTES: u64 = 320 * 1024 * 1024;

#[derive(Debug)]
pub(super) struct BudgetFull;

impl std::fmt::Display for BudgetFull {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("static snapshot budget full; use streaming")
    }
}

impl std::error::Error for BudgetFull {}

pub(super) struct Budget {
    used: AtomicU64,
    limit: u64,
}

impl Budget {
    fn new(limit: u64) -> Self {
        Self {
            used: AtomicU64::new(0),
            limit,
        }
    }

    pub(super) fn reserve(self: &Arc<Self>, bytes: u64) -> Result<Reservation> {
        if !try_reserve(&self.used, bytes, self.limit) {
            return Err(BudgetFull.into());
        }
        Ok(Reservation {
            budget: self.clone(),
            bytes,
        })
    }
}

pub(super) fn global_budget() -> &'static Arc<Budget> {
    static BUDGET: OnceLock<Arc<Budget>> = OnceLock::new();
    BUDGET.get_or_init(|| Arc::new(Budget::new(MAX_SNAPSHOT_BYTES)))
}

pub(super) fn try_reserve(used: &AtomicU64, bytes: u64, limit: u64) -> bool {
    used.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
        current.checked_add(bytes).filter(|next| *next <= limit)
    })
    .is_ok()
}

pub(super) struct Reservation {
    budget: Arc<Budget>,
    bytes: u64,
}

impl Drop for Reservation {
    fn drop(&mut self) {
        self.budget.used.fetch_sub(self.bytes, Ordering::Relaxed);
    }
}

struct Snapshot {
    data: Vec<u8>,
    _reservation: Reservation,
}

impl AsRef<[u8]> for Snapshot {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

impl Reservation {
    pub(super) fn into_bytes(self, data: Vec<u8>) -> Bytes {
        debug_assert_eq!(self.bytes, data.len() as u64);
        Bytes::from_owner(Snapshot {
            data,
            _reservation: self,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshots_hold_budget_until_last_slice_drops() {
        let budget = Arc::new(Budget::new(4));
        let body = budget.reserve(4).unwrap().into_bytes(vec![1, 2, 3, 4]);
        let slice = body.slice(1..2);
        drop(body);
        assert!(budget.reserve(1).is_err());
        assert_eq!(slice.as_ref(), &[2]);
        drop(slice);
        assert_eq!(budget.used.load(Ordering::Relaxed), 0);
        assert!(budget.reserve(4).is_ok());
    }

    #[test]
    fn concurrent_reservations_never_overbook_and_cancel_releases() {
        let budget = Arc::new(Budget::new(8));
        let barrier = Arc::new(std::sync::Barrier::new(32));
        let workers: Vec<_> = (0..32)
            .map(|_| {
                let budget = budget.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let reservation = budget.reserve(1).ok();
                    barrier.wait();
                    assert_eq!(budget.used.load(Ordering::Relaxed), 8);
                    barrier.wait();
                    reservation.is_some()
                })
            })
            .collect();
        let admitted = workers
            .into_iter()
            .map(|worker| usize::from(worker.join().unwrap()))
            .sum::<usize>();
        assert_eq!(admitted, 8);
        assert_eq!(budget.used.load(Ordering::Relaxed), 0);
        assert!(budget.reserve(u64::MAX).is_err());
        assert_eq!(budget.used.load(Ordering::Relaxed), 0);
    }
}
