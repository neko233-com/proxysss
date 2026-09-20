//! Drive a bounded native epoll batch on its owning Tokio data shard.
//!
//! The child epoll descriptor is itself pollable. Keep its cached readiness
//! until epoll_wait(0) actually returns no events; clearing it after a partial
//! batch can strand the remaining level-triggered sockets.

use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};

use tokio::io::unix::AsyncFd;
use tokio::runtime::Handle;

pub(crate) struct CooperativeEpoll {
    descriptor: AsyncFd<OwnedFd>,
    events: Vec<libc::epoll_event>,
}

impl CooperativeEpoll {
    pub(crate) fn new(
        runtime: &Handle,
        wake_fd: RawFd,
        wake_token: u64,
        event_capacity: usize,
    ) -> io::Result<Self> {
        let fd = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let descriptor = unsafe { OwnedFd::from_raw_fd(fd) };
        let mut event = libc::epoll_event {
            events: libc::EPOLLIN as u32,
            u64: wake_token,
        };
        if unsafe { libc::epoll_ctl(fd, libc::EPOLL_CTL_ADD, wake_fd, &mut event) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // Registration belongs to the shard which will drive the descriptor,
        // not the startup/control runtime constructing the worker pool.
        let _entered = runtime.enter();
        Ok(Self {
            descriptor: AsyncFd::new(descriptor)?,
            events: vec![libc::epoll_event { events: 0, u64: 0 }; event_capacity.max(1)],
        })
    }

    pub(crate) async fn next(&mut self) -> io::Result<&[libc::epoll_event]> {
        loop {
            let mut readiness = self.descriptor.readable().await?;
            let ready = unsafe {
                libc::epoll_wait(
                    self.descriptor.as_raw_fd(),
                    self.events.as_mut_ptr(),
                    self.events.len() as i32,
                    0,
                )
            };
            if ready < 0 {
                let error = io::Error::last_os_error();
                if error.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(error);
            }
            if ready == 0 {
                readiness.clear_ready();
                continue;
            }
            return Ok(&self.events[..ready as usize]);
        }
    }
}

impl AsRawFd for CooperativeEpoll {
    fn as_raw_fd(&self) -> RawFd {
        self.descriptor.as_raw_fd()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn partial_batches_keep_readiness_and_rearm_after_drain() {
        let descriptors: Vec<_> = (0..3)
            .map(|_| {
                let fd = unsafe { libc::eventfd(0, libc::EFD_NONBLOCK | libc::EFD_CLOEXEC) };
                assert!(fd >= 0);
                unsafe { OwnedFd::from_raw_fd(fd) }
            })
            .collect();
        let mut epoll =
            CooperativeEpoll::new(&Handle::current(), descriptors[0].as_raw_fd(), 0, 1).unwrap();
        for (index, descriptor) in descriptors.iter().enumerate().skip(1) {
            let mut event = libc::epoll_event {
                events: libc::EPOLLIN as u32,
                u64: index as u64,
            };
            assert_eq!(
                unsafe {
                    libc::epoll_ctl(
                        epoll.as_raw_fd(),
                        libc::EPOLL_CTL_ADD,
                        descriptor.as_raw_fd(),
                        &mut event,
                    )
                },
                0
            );
        }
        let wake = |fd| {
            let value = 1_u64;
            assert_eq!(
                unsafe { libc::write(fd, (&value as *const u64).cast(), 8) },
                8
            );
        };
        for descriptor in &descriptors {
            wake(descriptor.as_raw_fd());
        }
        let mut observed = [false; 3];
        for _ in 0..3 {
            let batch = tokio::time::timeout(Duration::from_secs(1), epoll.next())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(batch.len(), 1);
            let index = batch[0].u64 as usize;
            assert!(!observed[index]);
            observed[index] = true;
            let mut value = 0_u64;
            assert_eq!(
                unsafe {
                    libc::read(
                        descriptors[index].as_raw_fd(),
                        (&mut value as *mut u64).cast(),
                        8,
                    )
                },
                8
            );
        }
        assert!(observed.into_iter().all(|seen| seen));
        assert!(
            tokio::time::timeout(Duration::from_millis(10), epoll.next())
                .await
                .is_err()
        );
        wake(descriptors[0].as_raw_fd());
        let batch = tokio::time::timeout(Duration::from_secs(1), epoll.next())
            .await
            .unwrap()
            .unwrap();
        let token = batch[0].u64;
        assert_eq!(token, 0);
    }
}
