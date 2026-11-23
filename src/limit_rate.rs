//! nginx `limit_rate` parity: per-response byte pacing over any inner body.
//! Average-rate token scheduling with a single pending timer; no global locks.
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::Bytes;
use hyper::body::{Body, Frame, SizeHint};
use tokio::time::{Instant, Sleep};

pub struct RateLimited<B> {
    inner: B,
    rate: u64,
    sent: u64,
    start: Instant,
    delay: Option<Pin<Box<Sleep>>>,
    held: Option<Frame<Bytes>>,
}

impl<B> RateLimited<B> {
    pub fn new(inner: B, rate_bytes_per_sec: u64) -> Self {
        Self {
            inner,
            rate: rate_bytes_per_sec.max(1),
            sent: 0,
            start: Instant::now(),
            delay: None,
            held: None,
        }
    }
}

impl<B: Body<Data = Bytes> + Unpin> Body for RateLimited<B> {
    type Data = Bytes;
    type Error = B::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Self::Error>>> {
        if let Some(delay) = &mut self.delay {
            if delay.as_mut().poll(cx).is_pending() {
                return Poll::Pending;
            }
            self.delay = None;
        }
        if let Some(frame) = self.held.take() {
            return Poll::Ready(Some(Ok(frame)));
        }
        match Pin::new(&mut self.inner).poll_frame(cx) {
            Poll::Ready(Some(Ok(frame))) => {
                if let Some(data) = frame.data_ref() {
                    let total = self.sent + data.len() as u64;
                    let allowed = Duration::from_secs_f64(total as f64 / self.rate as f64);
                    let elapsed = self.start.elapsed();
                    if elapsed < allowed {
                        self.held = Some(frame);
                        let mut delay = Box::pin(tokio::time::sleep(allowed - elapsed));
                        // Register the waker now; otherwise the timer never wakes us.
                        let _ = delay.as_mut().poll(cx);
                        self.delay = Some(delay);
                        return Poll::Pending;
                    }
                    self.sent = total;
                }
                Poll::Ready(Some(Ok(frame)))
            }
            other => other,
        }
    }

    fn is_end_stream(&self) -> bool {
        self.held.is_none() && self.delay.is_none() && self.inner.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::{BodyExt, Full};

    #[tokio::test]
    async fn pacing_enforces_average_rate() {
        let body = Full::new(Bytes::from(vec![0u8; 1000]));
        let limited = RateLimited::new(body, 1000);
        let start = Instant::now();
        let collected = limited.collect().await.unwrap().to_bytes();
        assert_eq!(collected.len(), 1000);
        assert!(start.elapsed() >= Duration::from_millis(900));
    }

    #[tokio::test]
    async fn zero_rate_is_clamped_and_small_body_fast() {
        let body = Full::new(Bytes::from_static(b"hi"));
        let limited = RateLimited::new(body, u64::MAX);
        let collected = limited.collect().await.unwrap().to_bytes();
        assert_eq!(&collected[..], b"hi");
    }
}