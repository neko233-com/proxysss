//! Aggregate buffered request payload admission, shared across reloads and protocols.
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use bytes::Bytes;

#[derive(Clone)]
pub struct BufferedBodyBudget(Arc<BudgetState>);
struct BudgetState {
    limit: AtomicU64,
    used: AtomicU64,
}

impl BufferedBodyBudget {
    pub fn new(limit: u64) -> Self {
        Self(Arc::new(BudgetState {
            limit: AtomicU64::new(limit),
            used: AtomicU64::new(0),
        }))
    }
    pub fn set_limit(&self, limit: u64) {
        self.0.limit.store(limit, Ordering::Release);
    }
    pub fn reservation(&self) -> BodyReservation {
        BodyReservation {
            budget: self.clone(),
            bytes: 0,
        }
    }
    #[cfg(test)]
    pub fn used(&self) -> u64 {
        self.0.used.load(Ordering::Acquire)
    }
}

pub struct BodyReservation {
    budget: BufferedBodyBudget,
    bytes: u64,
}
impl BodyReservation {
    /// Reserve before copying the next chunk. No queue and no partial reservation.
    pub fn reserve(&mut self, bytes: usize) -> Result<(), ()> {
        if bytes == 0 {
            return Ok(());
        }
        let bytes = u64::try_from(bytes).map_err(|_| ())?;
        let mut used = self.budget.0.used.load(Ordering::Acquire);
        loop {
            let limit = self.budget.0.limit.load(Ordering::Acquire);
            let next = used.checked_add(bytes).ok_or(())?;
            if limit != 0 && next > limit {
                return Err(());
            }
            match self.budget.0.used.compare_exchange_weak(
                used,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    self.bytes += bytes;
                    return Ok(());
                }
                Err(actual) => used = actual,
            }
        }
    }
    /// The owner outlives every Bytes clone, including HTTP client/request buffers.
    pub fn finish(self, data: Bytes) -> Bytes {
        debug_assert_eq!(self.bytes, data.len() as u64);
        if data.is_empty() {
            return data;
        }
        Bytes::from_owner(BodyOwner {
            data,
            _reservation: self,
        })
    }
}
impl Drop for BodyReservation {
    fn drop(&mut self) {
        self.budget.0.used.fetch_sub(self.bytes, Ordering::AcqRel);
    }
}
struct BodyOwner {
    data: Bytes,
    _reservation: BodyReservation,
}
impl AsRef<[u8]> for BodyOwner {
    fn as_ref(&self) -> &[u8] {
        self.data.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clone_owner_keeps_reservation_until_last_bytes_drops() {
        let budget = BufferedBodyBudget::new(10);
        let mut reservation = budget.reservation();
        reservation.reserve(6).unwrap();
        let body = reservation.finish(Bytes::from_static(b"123456"));
        let clone = body.clone();
        drop(body);
        assert_eq!(budget.used(), 6);
        assert!(budget.reservation().reserve(5).is_err());
        drop(clone);
        assert_eq!(budget.used(), 0);
    }
    #[test]
    fn reload_changes_limit_without_resetting_in_flight_or_disabled_counts() {
        let budget = BufferedBodyBudget::new(0);
        let mut old = budget.reservation();
        old.reserve(8).unwrap();
        budget.set_limit(4);
        assert_eq!(budget.used(), 8);
        assert!(old.reserve(1).is_err());
        assert!(budget.reservation().reserve(1).is_err());
        drop(old);
        let mut next = budget.reservation();
        next.reserve(4).unwrap();
        assert!(next.reserve(1).is_err());
        drop(next);
        assert_eq!(budget.used(), 0);
    }
    #[test]
    fn concurrent_admission_never_overbooks_shared_capacity() {
        let budget = BufferedBodyBudget::new(7);
        let threads: Vec<_> = (0..16)
            .map(|_| {
                let budget = budget.clone();
                std::thread::spawn(move || {
                    for _ in 0..1000 {
                        let mut reservation = budget.reservation();
                        if reservation.reserve(1).is_ok() {
                            assert!(budget.used() <= 7);
                        }
                        std::thread::yield_now();
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(budget.used(), 0);
    }
}
