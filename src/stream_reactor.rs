//! Linux realtime stream relay reactor.
//!
//! Tokio remains responsible for accepting sockets, parsing the HTTP upgrade,
//! upstream selection, and the complete TLS/WSS path. Once a plain WebSocket
//! upgrade is established, this module can take duplicated socket descriptors
//! and relay them with one level-triggered epoll loop per allowed CPU. Small
//! game frames therefore pay one batched epoll wake plus recv/send, without a
//! generic async-future state machine on every direction and every frame.

use std::io;
use std::mem::{self, MaybeUninit};
use std::net::TcpStream;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread;

use crossbeam_queue::ArrayQueue;
use rustc_hash::FxHashMap;
use tokio::sync::oneshot;

const REGISTRATION_QUEUE_CAPACITY: usize = 131_072;
const EVENT_BATCH: usize = 1_024;
const RELAY_BUFFER_BYTES: usize = 16 * 1024;
const RELAY_READ_BATCH: usize = 8;
const PENDING_BUFFER_POOL_CAPACITY: usize = 4_096;
// Low-density realtime traffic gets a very short reply spin. Once each worker
// owns more than four pairs, rely entirely on epoll batching: spinning after
// every event at mixed-load density steals CPU from HTTP/TLS/UDP siblings.
const ACTIVE_SPIN_POLLS: usize = 8;
const ACTIVE_SPIN_MAX_PAIRS_PER_WORKER: usize = 4;
const DENSE_SPIN_POLLS: usize = 4;
const DENSE_SPIN_MAX_PAIRS_PER_WORKER: usize = 128;
const WAKE_TOKEN: u64 = u64::MAX;

struct SocketPair {
    downstream: TcpStream,
    upstream: TcpStream,
    completion: Option<oneshot::Sender<()>>,
}

struct ReactorWorker {
    registrations: ArrayQueue<SocketPair>,
    wake_fd: RawFd,
}

struct Reactors {
    workers: Vec<Arc<ReactorWorker>>,
    next: AtomicUsize,
}

struct SocketState {
    _stream: TcpStream,
    peer_fd: RawFd,
    read_enabled: bool,
    read_closed: bool,
    write_shutdown: bool,
    shutdown_write_after_flush: bool,
    pending: Option<Vec<u8>>,
    pending_offset: usize,
    completion: Option<oneshot::Sender<()>>,
}

static REACTORS: OnceLock<Reactors> = OnceLock::new();
static PENDING_BUFFERS: OnceLock<ArrayQueue<Vec<u8>>> = OnceLock::new();
fn pending_buffer_pool() -> &'static ArrayQueue<Vec<u8>> {
    PENDING_BUFFERS.get_or_init(|| ArrayQueue::new(PENDING_BUFFER_POOL_CAPACITY))
}

fn acquire_pending_buffer() -> Vec<u8> {
    pending_buffer_pool()
        .pop()
        .unwrap_or_else(|| Vec::with_capacity(RELAY_BUFFER_BYTES))
}

fn release_pending_buffer(mut buffer: Vec<u8>) {
    buffer.clear();
    let _ = pending_buffer_pool().push(buffer);
}

/// Duplicate and hand an established plain-WebSocket socket pair to a
/// per-core epoll reactor. The caller may immediately drop its original Tokio
/// descriptors after this succeeds; `dup` keeps the underlying connections
/// alive without adding descriptors for the lifetime beyond the two owned by
/// the reactor.
pub(crate) fn dispatch(
    downstream_fd: RawFd,
    upstream_fd: RawFd,
    requested_workers: usize,
    scheduler_nice: i32,
) -> io::Result<()> {
    dispatch_inner(
        downstream_fd,
        upstream_fd,
        requested_workers,
        scheduler_nice,
        None,
    )
}

/// Like [`dispatch`], but returns a completion receiver for control-plane
/// accounting that must remain active until the tunnel closes.
pub(crate) fn dispatch_with_completion(
    downstream_fd: RawFd,
    upstream_fd: RawFd,
    requested_workers: usize,
    scheduler_nice: i32,
) -> io::Result<oneshot::Receiver<()>> {
    let (sender, receiver) = oneshot::channel();
    dispatch_inner(
        downstream_fd,
        upstream_fd,
        requested_workers,
        scheduler_nice,
        Some(sender),
    )?;
    Ok(receiver)
}

fn dispatch_inner(
    downstream_fd: RawFd,
    upstream_fd: RawFd,
    requested_workers: usize,
    scheduler_nice: i32,
    completion: Option<oneshot::Sender<()>>,
) -> io::Result<()> {
    let reactors = REACTORS.get_or_init(|| Reactors::start(requested_workers, scheduler_nice));
    let downstream = duplicate_stream(downstream_fd)?;
    let upstream = duplicate_stream(upstream_fd)?;
    let pair = SocketPair {
        downstream,
        upstream,
        completion,
    };
    let index = reactors.next.fetch_add(1, Ordering::Relaxed) % reactors.workers.len();
    let worker = &reactors.workers[index];
    worker
        .registrations
        .push(pair)
        .map_err(|_| io::Error::new(io::ErrorKind::WouldBlock, "websocket reactor queue full"))?;
    wake(worker.wake_fd);
    Ok(())
}

impl Reactors {
    fn start(requested_workers: usize, scheduler_nice: i32) -> Self {
        let worker_count = requested_workers.max(1);
        let allowed_cpus = allowed_cpu_ids();
        let mut workers = Vec::with_capacity(worker_count);
        for index in 0..worker_count {
            let wake_fd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
            assert!(wake_fd >= 0, "failed creating WebSocket reactor eventfd");
            let worker = Arc::new(ReactorWorker {
                registrations: ArrayQueue::new(REGISTRATION_QUEUE_CAPACITY),
                wake_fd,
            });
            let reactor_worker = worker.clone();
            let cpu = reactor_worker_cpu(index, worker_count, &allowed_cpus);
            thread::Builder::new()
                .name(format!("proxysss-ws-epoll-{index}"))
                .spawn(move || run_reactor(reactor_worker, cpu, scheduler_nice))
                .expect("failed spawning WebSocket epoll reactor");
            workers.push(worker);
        }
        Self {
            workers,
            next: AtomicUsize::new(0),
        }
    }
}

fn reactor_worker_cpu(index: usize, worker_count: usize, allowed_cpus: &[usize]) -> Option<usize> {
    if allowed_cpus.is_empty() || worker_count < allowed_cpus.len() {
        // A sparse realtime pool must be able to follow whichever data-plane
        // CPU has spare capacity. Hard-pinning its sole owner to the last CPU
        // creates a hotspot beside that CPU's HTTP/UDP shard while another CPU
        // remains idle. Linux will normally keep the thread cache-local and
        // migrate it only when the runnable imbalance warrants doing so.
        return None;
    }
    allowed_cpus
        .get(allowed_cpus.len() - 1 - (index % allowed_cpus.len()))
        .copied()
}

fn duplicate_stream(fd: RawFd) -> io::Result<TcpStream> {
    let duplicated = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 0) };
    if duplicated < 0 {
        return Err(io::Error::last_os_error());
    }
    let stream = unsafe { TcpStream::from_raw_fd(duplicated) };
    stream.set_nonblocking(true)?;
    Ok(stream)
}

fn wake(fd: RawFd) {
    let value = 1_u64;
    let _ = unsafe { libc::write(fd, (&value as *const u64).cast(), mem::size_of::<u64>()) };
}

fn run_reactor(worker: Arc<ReactorWorker>, cpu: Option<usize>, scheduler_nice: i32) {
    if let Some(cpu) = cpu {
        pin_current_thread(cpu);
    }
    set_current_thread_nice(scheduler_nice);
    let epoll_fd = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
    assert!(epoll_fd >= 0, "failed creating WebSocket epoll instance");
    let mut wake_event = libc::epoll_event {
        events: libc::EPOLLIN as u32,
        u64: WAKE_TOKEN,
    };
    let add_wake = unsafe {
        libc::epoll_ctl(
            epoll_fd,
            libc::EPOLL_CTL_ADD,
            worker.wake_fd,
            &mut wake_event,
        )
    };
    assert_eq!(add_wake, 0, "failed registering WebSocket reactor eventfd");

    let mut sockets = FxHashMap::<RawFd, SocketState>::default();
    let mut events = vec![libc::epoll_event { events: 0, u64: 0 }; EVENT_BATCH];
    let mut relay_buffer = [0_u8; RELAY_BUFFER_BYTES];
    let mut active_spin_polls = 0_usize;

    loop {
        let timeout = if active_spin_polls > 0 { 0 } else { -1 };
        let ready = unsafe {
            libc::epoll_wait(epoll_fd, events.as_mut_ptr(), events.len() as i32, timeout)
        };
        if ready < 0 {
            if io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                continue;
            }
            break;
        }
        if ready == 0 {
            active_spin_polls = active_spin_polls.saturating_sub(1);
            std::hint::spin_loop();
            continue;
        }
        for event in events.iter().take(ready as usize) {
            let token = event.u64;
            if token == WAKE_TOKEN {
                drain_wake(worker.wake_fd);
                while let Some(pair) = worker.registrations.pop() {
                    register_pair(epoll_fd, &mut sockets, pair);
                }
                continue;
            }

            let fd = token as RawFd;
            if !sockets.contains_key(&fd) {
                continue;
            }
            let pair_count = sockets.len() / 2;
            active_spin_polls = if pair_count <= ACTIVE_SPIN_MAX_PAIRS_PER_WORKER {
                ACTIVE_SPIN_POLLS
            } else if pair_count <= DENSE_SPIN_MAX_PAIRS_PER_WORKER {
                DENSE_SPIN_POLLS
            } else {
                0
            };
            let flags = event.events as i32;
            if flags & libc::EPOLLERR != 0 {
                close_pair(epoll_fd, &mut sockets, fd);
                continue;
            }
            if flags & libc::EPOLLOUT != 0 && !flush_pending(epoll_fd, &mut sockets, fd) {
                close_pair(epoll_fd, &mut sockets, fd);
                continue;
            }
            if flags & libc::EPOLLIN != 0
                && !relay_read(epoll_fd, &mut sockets, fd, &mut relay_buffer)
            {
                close_pair(epoll_fd, &mut sockets, fd);
                continue;
            }
            // HUP/RDHUP can arrive together with EPOLLIN while the peer's
            // final bytes are still queued. Drain readable data before
            // propagating half-close so tail frames cannot be discarded.
            if flags & (libc::EPOLLRDHUP | libc::EPOLLHUP) != 0
                && flags & libc::EPOLLIN == 0
                && sockets.get(&fd).is_some_and(|state| state.read_enabled)
            {
                if !mark_read_closed(epoll_fd, &mut sockets, fd) {
                    close_pair(epoll_fd, &mut sockets, fd);
                    continue;
                }
            }
            // Both halves must be read-closed before a pair can finish. Avoid
            // two extra hash lookups on every ordinary data event; only enter
            // the full pair check after this side has actually observed EOF.
            if sockets.get(&fd).is_some_and(|state| state.read_closed)
                && pair_finished(&sockets, fd)
            {
                close_pair(epoll_fd, &mut sockets, fd);
            }
        }
    }

    unsafe {
        libc::close(epoll_fd);
    }
}

fn set_current_thread_nice(nice: i32) {
    let nice = nice.clamp(0, 19);
    if nice > 0 {
        unsafe {
            libc::setpriority(libc::PRIO_PROCESS, 0, nice);
        }
    }
}

fn register_pair(epoll_fd: RawFd, sockets: &mut FxHashMap<RawFd, SocketState>, pair: SocketPair) {
    let downstream_fd = pair.downstream.as_raw_fd();
    let upstream_fd = pair.upstream.as_raw_fd();
    let completion = pair.completion;
    sockets.insert(
        downstream_fd,
        SocketState {
            _stream: pair.downstream,
            peer_fd: upstream_fd,
            read_enabled: true,
            read_closed: false,
            write_shutdown: false,
            shutdown_write_after_flush: false,
            pending: None,
            pending_offset: 0,
            completion,
        },
    );
    sockets.insert(
        upstream_fd,
        SocketState {
            _stream: pair.upstream,
            peer_fd: downstream_fd,
            read_enabled: true,
            read_closed: false,
            write_shutdown: false,
            shutdown_write_after_flush: false,
            pending: None,
            pending_offset: 0,
            completion: None,
        },
    );

    if !add_socket(epoll_fd, downstream_fd) || !add_socket(epoll_fd, upstream_fd) {
        close_pair(epoll_fd, sockets, downstream_fd);
    }
}

fn add_socket(epoll_fd: RawFd, fd: RawFd) -> bool {
    let mut event = libc::epoll_event {
        events: base_events() | libc::EPOLLIN as u32,
        u64: fd as u64,
    };
    unsafe { libc::epoll_ctl(epoll_fd, libc::EPOLL_CTL_ADD, fd, &mut event) == 0 }
}

fn modify_socket(epoll_fd: RawFd, fd: RawFd, state: &SocketState) -> bool {
    let mut interests = base_events();
    if state.read_enabled && !state.read_closed {
        interests |= libc::EPOLLIN as u32;
        interests |= libc::EPOLLRDHUP as u32;
    }
    if state
        .pending
        .as_ref()
        .is_some_and(|pending| state.pending_offset < pending.len())
    {
        interests |= libc::EPOLLOUT as u32;
    }
    let mut event = libc::epoll_event {
        events: interests,
        u64: fd as u64,
    };
    unsafe { libc::epoll_ctl(epoll_fd, libc::EPOLL_CTL_MOD, fd, &mut event) == 0 }
}

fn base_events() -> u32 {
    (libc::EPOLLERR | libc::EPOLLHUP) as u32
}

fn relay_read(
    epoll_fd: RawFd,
    sockets: &mut FxHashMap<RawFd, SocketState>,
    fd: RawFd,
    buffer: &mut [u8],
) -> bool {
    let Some(peer_fd) = sockets.get(&fd).map(|state| state.peer_fd) else {
        return false;
    };
    if sockets
        .get(&peer_fd)
        .map(|peer| {
            peer.pending
                .as_ref()
                .is_some_and(|pending| peer.pending_offset < pending.len())
        })
        .unwrap_or(true)
    {
        return pause_reader(epoll_fd, sockets, fd);
    }

    // Level-triggered epoll may coalesce several small WebSocket/game frames
    // behind one readiness notification. Drain a bounded batch so those
    // frames amortize epoll dispatch and hash-table lookups, while the bound
    // preserves fairness between long-lived connections.
    for _ in 0..RELAY_READ_BATCH {
        let read = loop {
            let result = unsafe { libc::recv(fd, buffer.as_mut_ptr().cast(), buffer.len(), 0) };
            if result >= 0 {
                break result as usize;
            }
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            if error.kind() == io::ErrorKind::WouldBlock {
                return true;
            }
            return false;
        };
        if read == 0 {
            return mark_read_closed(epoll_fd, sockets, fd);
        }

        let sent = match send_nonblocking(peer_fd, &buffer[..read]) {
            Ok(sent) => sent,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => 0,
            Err(_) => return false,
        };
        if sent == read {
            continue;
        }

        let Some(peer) = sockets.get_mut(&peer_fd) else {
            return false;
        };
        let mut pending = peer.pending.take().unwrap_or_else(acquire_pending_buffer);
        pending.clear();
        pending.extend_from_slice(&buffer[sent..read]);
        peer.pending = Some(pending);
        peer.pending_offset = 0;
        if !modify_socket(epoll_fd, peer_fd, peer) {
            return false;
        }
        return pause_reader(epoll_fd, sockets, fd);
    }
    true
}

fn flush_pending(epoll_fd: RawFd, sockets: &mut FxHashMap<RawFd, SocketState>, fd: RawFd) -> bool {
    let Some(state) = sockets.get_mut(&fd) else {
        return false;
    };
    let peer_fd = state.peer_fd;
    let pending_len = state.pending.as_ref().map_or(0, Vec::len);
    if state.pending_offset >= pending_len {
        return modify_socket(epoll_fd, fd, state);
    }
    let sent = match send_nonblocking(
        fd,
        &state.pending.as_ref().expect("pending buffer present")[state.pending_offset..],
    ) {
        Ok(sent) => sent,
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => return true,
        Err(_) => return false,
    };
    state.pending_offset += sent;
    if state.pending_offset < state.pending.as_ref().map_or(0, Vec::len) {
        return true;
    }
    if let Some(pending) = state.pending.take() {
        release_pending_buffer(pending);
    }
    state.pending_offset = 0;
    if state.shutdown_write_after_flush && !state.write_shutdown {
        let _ = unsafe { libc::shutdown(fd, libc::SHUT_WR) };
        state.write_shutdown = true;
    }
    if !modify_socket(epoll_fd, fd, state) {
        return false;
    }

    let Some(source) = sockets.get_mut(&peer_fd) else {
        return false;
    };
    source.read_enabled = true;
    modify_socket(epoll_fd, peer_fd, source)
}

fn mark_read_closed(
    epoll_fd: RawFd,
    sockets: &mut FxHashMap<RawFd, SocketState>,
    fd: RawFd,
) -> bool {
    let Some(peer_fd) = sockets.get(&fd).map(|state| state.peer_fd) else {
        return false;
    };
    let Some(source) = sockets.get_mut(&fd) else {
        return false;
    };
    source.read_closed = true;
    source.read_enabled = false;
    if !modify_socket(epoll_fd, fd, source) {
        return false;
    }

    let Some(peer) = sockets.get_mut(&peer_fd) else {
        return false;
    };
    peer.shutdown_write_after_flush = true;
    if peer.pending.as_ref().map_or(0, Vec::len) == peer.pending_offset && !peer.write_shutdown {
        let _ = unsafe { libc::shutdown(peer_fd, libc::SHUT_WR) };
        peer.write_shutdown = true;
    }
    modify_socket(epoll_fd, peer_fd, peer)
}

fn pair_finished(sockets: &FxHashMap<RawFd, SocketState>, fd: RawFd) -> bool {
    let Some(state) = sockets.get(&fd) else {
        return true;
    };
    let Some(peer) = sockets.get(&state.peer_fd) else {
        return true;
    };
    state.read_closed
        && peer.read_closed
        && state.pending.as_ref().map_or(0, Vec::len) == state.pending_offset
        && peer.pending.as_ref().map_or(0, Vec::len) == peer.pending_offset
}

fn pause_reader(epoll_fd: RawFd, sockets: &mut FxHashMap<RawFd, SocketState>, fd: RawFd) -> bool {
    let Some(source) = sockets.get_mut(&fd) else {
        return false;
    };
    source.read_enabled = false;
    modify_socket(epoll_fd, fd, source)
}

fn send_nonblocking(fd: RawFd, buffer: &[u8]) -> io::Result<usize> {
    loop {
        let sent =
            unsafe { libc::send(fd, buffer.as_ptr().cast(), buffer.len(), libc::MSG_NOSIGNAL) };
        if sent >= 0 {
            return Ok(sent as usize);
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}

fn close_pair(epoll_fd: RawFd, sockets: &mut FxHashMap<RawFd, SocketState>, fd: RawFd) {
    let Some(mut state) = sockets.remove(&fd) else {
        return;
    };
    let peer_fd = state.peer_fd;
    unsafe {
        libc::epoll_ctl(epoll_fd, libc::EPOLL_CTL_DEL, fd, std::ptr::null_mut());
    }
    let mut completion = state.completion.take();
    if let Some(pending) = state.pending.take() {
        release_pending_buffer(pending);
    }
    drop(state);
    if let Some(mut peer) = sockets.remove(&peer_fd) {
        unsafe {
            libc::epoll_ctl(epoll_fd, libc::EPOLL_CTL_DEL, peer_fd, std::ptr::null_mut());
        }
        if completion.is_none() {
            completion = peer.completion.take();
        }
        if let Some(pending) = peer.pending.take() {
            release_pending_buffer(pending);
        }
        drop(peer);
    }
    if let Some(completion) = completion {
        let _ = completion.send(());
    }
}

fn drain_wake(fd: RawFd) {
    let mut value = MaybeUninit::<u64>::uninit();
    loop {
        let read = unsafe { libc::read(fd, value.as_mut_ptr().cast(), mem::size_of::<u64>()) };
        if read < 0 && io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
            continue;
        }
        break;
    }
}

fn allowed_cpu_ids() -> Vec<usize> {
    let mut set = unsafe { mem::zeroed::<libc::cpu_set_t>() };
    let result = unsafe {
        libc::sched_getaffinity(
            0,
            mem::size_of::<libc::cpu_set_t>(),
            &mut set as *mut libc::cpu_set_t,
        )
    };
    if result != 0 {
        return vec![0];
    }
    let mut cpus = Vec::new();
    for cpu in 0..libc::CPU_SETSIZE as usize {
        if unsafe { libc::CPU_ISSET(cpu, &set) } {
            cpus.push(cpu);
        }
    }
    if cpus.is_empty() {
        cpus.push(0);
    }
    cpus
}

fn pin_current_thread(cpu: usize) {
    let mut set = unsafe { mem::zeroed::<libc::cpu_set_t>() };
    unsafe {
        libc::CPU_SET(cpu, &mut set);
        let _ = libc::sched_setaffinity(
            0,
            mem::size_of::<libc::cpu_set_t>(),
            &set as *const libc::cpu_set_t,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{Shutdown, TcpListener};
    use std::time::Duration;

    #[test]
    fn sparse_reactor_workers_keep_soft_cpu_ownership() {
        assert_eq!(reactor_worker_cpu(0, 1, &[]), None);
        assert_eq!(reactor_worker_cpu(0, 1, &[2, 4]), None);
    }

    #[test]
    fn per_cpu_reactor_workers_pin_in_reverse_order() {
        assert_eq!(reactor_worker_cpu(0, 2, &[2, 4]), Some(4));
        assert_eq!(reactor_worker_cpu(1, 2, &[2, 4]), Some(2));
    }

    #[test]
    fn dense_spin_budget_is_bounded() {
        assert!(ACTIVE_SPIN_POLLS > DENSE_SPIN_POLLS);
        assert_eq!(DENSE_SPIN_MAX_PAIRS_PER_WORKER, 128);
    }

    #[test]
    fn reactor_preserves_bidirectional_half_close() {
        let downstream_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut client = TcpStream::connect(downstream_listener.local_addr().unwrap()).unwrap();
        let (downstream, _) = downstream_listener.accept().unwrap();

        let backend_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let upstream = TcpStream::connect(backend_listener.local_addr().unwrap()).unwrap();
        let (mut backend, _) = backend_listener.accept().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        backend
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        dispatch(downstream.as_raw_fd(), upstream.as_raw_fd(), 1, 0).unwrap();
        drop(downstream);
        drop(upstream);

        client.write_all(b"ping").unwrap();
        let mut ping = [0_u8; 4];
        backend.read_exact(&mut ping).unwrap();
        assert_eq!(&ping, b"ping");

        client.shutdown(Shutdown::Write).unwrap();
        let mut eof = [0_u8; 1];
        assert_eq!(backend.read(&mut eof).unwrap(), 0);

        let pong = vec![0x5a_u8; RELAY_BUFFER_BYTES * 2 + 17];
        backend.write_all(&pong).unwrap();
        backend.shutdown(Shutdown::Write).unwrap();
        let mut received = vec![0_u8; pong.len()];
        client.read_exact(&mut received).unwrap();
        assert_eq!(received, pong);
        assert_eq!(client.read(&mut eof).unwrap(), 0);
    }
}
