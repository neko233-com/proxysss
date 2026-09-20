//! Per-connection plain TCP relay on the accepting HTTP/TCP Tokio shard.
//! A ready pair gets a bounded turn, not an entire native reactor's batch.
//! Idle pairs retain no payload buffers; only backpressure owns pending bytes.
use super::{latency_relay_buffer_pool, PooledBuffer};
use std::cell::RefCell;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::task::{Context, Poll};
use tokio::net::TcpStream;

const IO_STEPS_PER_DIRECTION: usize = 16;
thread_local! {
    static SCRATCH: RefCell<PooledBuffer> = RefCell::new(latency_relay_buffer_pool().acquire());
}

#[derive(Default)]
struct Direction {
    pending: Option<PooledBuffer>,
    position: usize,
    end: usize,
    copied: u64,
    done: bool,
}

impl Direction {
    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        reader: &TcpStream,
        writer: &TcpStream,
        scratch: &mut [u8],
    ) -> Poll<io::Result<u64>> {
        if self.done {
            return Poll::Ready(Ok(self.copied));
        }
        let mut steps = 0;
        while steps < IO_STEPS_PER_DIRECTION {
            if let Some(pending) = self.pending.as_ref() {
                steps += 1;
                match writer.try_write(&pending[self.position..self.end]) {
                    Ok(0) => return Poll::Ready(Err(io::ErrorKind::WriteZero.into())),
                    Ok(written) => {
                        self.position += written;
                        self.copied += written as u64;
                        if self.position == self.end {
                            self.pending = None;
                            self.position = 0;
                            self.end = 0;
                        }
                        continue;
                    }
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        // try_write clears a stale readiness tick only after a
                        // real EAGAIN. Register this pair's own write waker.
                        match writer.poll_write_ready(cx) {
                            Poll::Ready(Ok(_)) => continue,
                            Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                            Poll::Pending => return Poll::Pending,
                        }
                    }
                    Err(error) => return Poll::Ready(Err(error)),
                }
            }
            steps += 1;
            let read = match reader.try_read(scratch) {
                Ok(0) => {
                    // All pending bytes are flushed before propagating EOF;
                    // the reverse direction continues until its own EOF.
                    let result = unsafe { libc::shutdown(writer.as_raw_fd(), libc::SHUT_WR) };
                    if result != 0 {
                        return Poll::Ready(Err(io::Error::last_os_error()));
                    }
                    self.done = true;
                    return Poll::Ready(Ok(self.copied));
                }
                Ok(read) => read,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    match reader.poll_read_ready(cx) {
                        Poll::Ready(Ok(_)) => continue,
                        Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                        Poll::Pending => return Poll::Pending,
                    }
                }
                Err(error) => return Poll::Ready(Err(error)),
            };
            // At most one extra write after the final read in a turn. Copy only
            // unsent bytes into owned storage; never retain the thread scratch.
            steps += 1;
            let written = match writer.try_write(&scratch[..read]) {
                Ok(0) => return Poll::Ready(Err(io::ErrorKind::WriteZero.into())),
                Ok(written) => written,
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) =>
                {
                    0
                }
                Err(error) => return Poll::Ready(Err(error)),
            };
            self.copied += written as u64;
            if written != read {
                let mut pending = latency_relay_buffer_pool().acquire();
                let remaining = read - written;
                pending[..remaining].copy_from_slice(&scratch[written..read]);
                self.pending = Some(pending);
                self.position = 0;
                self.end = remaining;
            }
        }
        // Self-wake joins the same runnable queue as HTTP, reverse/SSE and
        // downloads. Sparse pairs instead park on their precise I/O interest.
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

pub(super) async fn relay(left: TcpStream, right: TcpStream) -> io::Result<(u64, u64)> {
    let mut outbound = Direction::default();
    let mut inbound = Direction::default();
    std::future::poll_fn(|cx| {
        SCRATCH.with(|scratch| {
            let mut scratch = scratch.borrow_mut();
            let left_result = outbound.poll(cx, &left, &right, &mut scratch);
            if let Poll::Ready(Err(error)) = left_result {
                return Poll::Ready(Err(error));
            }
            let right_result = inbound.poll(cx, &right, &left, &mut scratch);
            match (left_result, right_result) {
                (_, Poll::Ready(Err(error))) => Poll::Ready(Err(error)),
                (Poll::Ready(Ok(left)), Poll::Ready(Ok(right))) => Poll::Ready(Ok((left, right))),
                _ => Poll::Pending,
            }
        })
    })
    .await
}

fn duplicate_registered(fd: RawFd) -> io::Result<TcpStream> {
    let duplicate = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 0) };
    if duplicate < 0 {
        return Err(io::Error::last_os_error());
    }
    let socket = unsafe { std::net::TcpStream::from_raw_fd(duplicate) };
    // The borrowed source is already nonblocking. from_std registers the new
    // descriptor on the caller's current shard, never the startup reactor.
    TcpStream::from_std(socket)
}

pub(super) fn dispatch(left: RawFd, right: RawFd) -> io::Result<()> {
    let left = duplicate_registered(left)?;
    let right = duplicate_registered(right)?;
    std::mem::drop(tokio::spawn(async move {
        if let Err(error) = relay(left, right).await {
            tracing::debug!(?error, "plain connection relay ended");
        }
    }));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    async fn pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let server = listener.accept().await.unwrap().0;
        client.set_nodelay(true).unwrap();
        server.set_nodelay(true).unwrap();
        (client, server)
    }

    #[tokio::test]
    async fn pair_drains_backpressure_and_preserves_both_half_closes() {
        tokio::time::timeout(Duration::from_secs(8), async {
            let (mut client, left) = pair().await;
            let (right, mut backend) = pair().await;
            socket2::SockRef::from(&right)
                .set_send_buffer_size(8192)
                .unwrap();
            let relay_task = tokio::spawn(relay(left, right));
            let payload: Vec<u8> = (0..(2 * 1024 * 1024 + 31))
                .map(|i| (i % 251) as u8)
                .collect();
            let expected = payload.clone();
            let backend_task = tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(20)).await;
                let mut received = Vec::new();
                backend.read_to_end(&mut received).await.unwrap();
                assert_eq!(received, expected);
                backend.write_all(b"reply-after-eof").await.unwrap();
                backend.shutdown().await.unwrap();
            });
            client.write_all(&payload).await.unwrap();
            client.shutdown().await.unwrap();
            let mut reply = Vec::new();
            client.read_to_end(&mut reply).await.unwrap();
            assert_eq!(reply, b"reply-after-eof");
            backend_task.await.unwrap();
            assert_eq!(
                relay_task.await.unwrap().unwrap(),
                (payload.len() as u64, reply.len() as u64)
            );
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn blocked_pair_does_not_hold_scratch_or_prevent_cancellation() {
        tokio::time::timeout(Duration::from_secs(8), async {
            let (mut client, left) = pair().await;
            let (right, mut backend) = pair().await;
            socket2::SockRef::from(&right)
                .set_send_buffer_size(4096)
                .unwrap();
            socket2::SockRef::from(&backend)
                .set_recv_buffer_size(4096)
                .unwrap();
            let blocked = tokio::spawn(relay(left, right));
            let writer =
                tokio::spawn(async move { client.write_all(&vec![b'b'; 8 * 1024 * 1024]).await });
            // Both relay tasks share this single-thread runtime. One blocked
            // socket must release the scratch and let another pair complete.
            let (mut quick_client, quick_left) = pair().await;
            let (quick_right, mut quick_backend) = pair().await;
            let quick_relay = tokio::spawn(relay(quick_left, quick_right));
            let echo = tokio::spawn(async move {
                let mut data = [0; 4];
                quick_backend.read_exact(&mut data).await.unwrap();
                quick_backend.write_all(&data).await.unwrap();
                quick_backend.shutdown().await.unwrap();
            });
            quick_client.write_all(b"ping").await.unwrap();
            quick_client.shutdown().await.unwrap();
            let mut reply = Vec::new();
            quick_client.read_to_end(&mut reply).await.unwrap();
            assert_eq!(reply, b"ping");
            echo.await.unwrap();
            quick_relay.await.unwrap().unwrap();
            blocked.abort();
            assert!(blocked.await.unwrap_err().is_cancelled());
            // EOF may follow already-queued data; no new event from the stopped
            // reader is needed to release the relay-owned socket and pending buf.
            let mut prefix = Vec::new();
            backend.read_to_end(&mut prefix).await.unwrap();
            assert!(prefix.len() < 8 * 1024 * 1024);
            assert!(prefix.iter().all(|byte| *byte == b'b'));
            assert!(writer.await.unwrap().is_err());
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn dispatched_pair_survives_original_registration_drop() {
        tokio::time::timeout(Duration::from_secs(4), async {
            let (mut client, left) = pair().await;
            let (right, mut backend) = pair().await;
            dispatch(left.as_raw_fd(), right.as_raw_fd()).unwrap();
            drop(left);
            drop(right);
            let echo = tokio::spawn(async move {
                let mut data = Vec::new();
                backend.read_to_end(&mut data).await.unwrap();
                assert_eq!(data, b"after-handoff");
                backend.write_all(&data).await.unwrap();
                backend.shutdown().await.unwrap();
            });
            client.write_all(b"after-handoff").await.unwrap();
            client.shutdown().await.unwrap();
            let mut reply = Vec::new();
            client.read_to_end(&mut reply).await.unwrap();
            assert_eq!(reply, b"after-handoff");
            echo.await.unwrap();
        })
        .await
        .unwrap();
    }
}
