//! Linux exact-static HTTP reactor.
//!
//! The ordinary gateway validates the first request and resolves the cached
//! static object. Repeated byte-identical keep-alive GETs can then stay on a
//! small native epoll/sendfile loop. Any request, config epoch, or file
//! metadata change hands the owned socket and every unconsumed byte back to
//! Tokio, so this lane never substitutes for routing or policy evaluation.

use std::fs::File;
use std::io;
use std::mem;
use std::net::TcpStream;
use std::os::fd::{AsRawFd, RawFd};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use bytes::Bytes;
use crossbeam_queue::ArrayQueue;
use memchr::memmem;
use tokio::sync::oneshot;

const REGISTRATION_QUEUE_CAPACITY: usize = 131_072;
const EVENT_BATCH: usize = 1_024;
const READ_BUFFER_BYTES: usize = 8 * 1024;
const MAX_HEAD_BYTES: usize = 64 * 1024;
const MAX_RESPONSES_PER_EVENT: usize = 32;
const REVALIDATE_INTERVAL: Duration = Duration::from_secs(1);
const WAKE_TOKEN: u64 = u64::MAX;

pub(crate) struct DispatchRequest {
    pub stream: TcpStream,
    pub initial_prefix: Bytes,
    pub exact_request_head: Bytes,
    pub response_head: Bytes,
    pub combined_response: Option<Bytes>,
    pub body: Option<Bytes>,
    pub sendfile: Option<Arc<File>>,
    pub file_path: PathBuf,
    pub file_len: u64,
    pub file_modified: Option<SystemTime>,
    pub config_epoch: Arc<AtomicU64>,
    pub expected_epoch: u64,
}

pub(crate) struct Fallback {
    pub stream: TcpStream,
    pub prefix: Bytes,
}

#[derive(Debug)]
pub(crate) struct DispatchFailure {
    pub stream: TcpStream,
    pub prefix: Bytes,
}

struct Registration {
    request: DispatchRequest,
    fallback: oneshot::Sender<Fallback>,
}

struct Worker {
    registrations: ArrayQueue<Registration>,
    wake_fd: RawFd,
}

struct Reactors {
    workers: Vec<Arc<Worker>>,
    next: AtomicUsize,
}

struct Connection {
    stream: TcpStream,
    input: Vec<u8>,
    exact_request_head: Bytes,
    response_head: Bytes,
    combined_response: Option<Bytes>,
    body: Option<Bytes>,
    sendfile: Option<Arc<File>>,
    file_path: PathBuf,
    file_len: u64,
    file_modified: Option<SystemTime>,
    checked_at: Instant,
    config_epoch: Arc<AtomicU64>,
    expected_epoch: u64,
    pending: bool,
    head_offset: usize,
    body_offset: usize,
    file_offset: libc::off_t,
    corked: bool,
    fallback: Option<oneshot::Sender<Fallback>>,
}

#[derive(Default)]
struct ConnectionTable {
    slots: Vec<Option<Box<Connection>>>,
}

impl ConnectionTable {
    fn get_mut(&mut self, fd: RawFd) -> Option<&mut Connection> {
        usize::try_from(fd)
            .ok()
            .and_then(|index| self.slots.get_mut(index))
            .and_then(Option::as_deref_mut)
    }

    fn insert(&mut self, fd: RawFd, connection: Connection) {
        let Ok(index) = usize::try_from(fd) else {
            return;
        };
        if index >= self.slots.len() {
            self.slots.resize_with(index + 1, || None);
        }
        self.slots[index] = Some(Box::new(connection));
    }

    fn remove(&mut self, fd: RawFd) -> Option<Box<Connection>> {
        let index = usize::try_from(fd).ok()?;
        self.slots.get_mut(index)?.take()
    }
}

static REACTORS: OnceLock<Reactors> = OnceLock::new();

pub(crate) fn dispatch(
    request: DispatchRequest,
    requested_workers: usize,
) -> Result<oneshot::Receiver<Fallback>, DispatchFailure> {
    let reactors = REACTORS.get_or_init(|| Reactors::start(requested_workers));
    let (fallback, receiver) = oneshot::channel();
    let index = reactors.next.fetch_add(1, Ordering::Relaxed) % reactors.workers.len();
    let worker = &reactors.workers[index];
    match worker
        .registrations
        .push(Registration { request, fallback })
    {
        Ok(()) => {
            wake(worker.wake_fd);
            Ok(receiver)
        }
        Err(registration) => Err(DispatchFailure {
            stream: registration.request.stream,
            prefix: registration.request.initial_prefix,
        }),
    }
}

impl Reactors {
    fn start(requested_workers: usize) -> Self {
        let worker_count = requested_workers.max(1);
        let cpus = allowed_cpu_ids();
        let mut workers = Vec::with_capacity(worker_count);
        for index in 0..worker_count {
            let wake_fd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
            assert!(wake_fd >= 0, "failed creating static HTTP reactor eventfd");
            let worker = Arc::new(Worker {
                registrations: ArrayQueue::new(REGISTRATION_QUEUE_CAPACITY),
                wake_fd,
            });
            let owner = worker.clone();
            let cpu = cpus.get(index % cpus.len().max(1)).copied();
            thread::Builder::new()
                .name(format!("proxysss-http-epoll-{index}"))
                .spawn(move || run_worker(owner, cpu))
                .expect("failed spawning static HTTP epoll reactor");
            workers.push(worker);
        }
        Self {
            workers,
            next: AtomicUsize::new(0),
        }
    }
}

fn run_worker(worker: Arc<Worker>, cpu: Option<usize>) {
    if let Some(cpu) = cpu {
        pin_current_thread(cpu);
    }
    let epoll_fd = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
    assert!(epoll_fd >= 0, "failed creating static HTTP epoll instance");
    let mut wake_event = libc::epoll_event {
        events: libc::EPOLLIN as u32,
        u64: WAKE_TOKEN,
    };
    assert_eq!(
        unsafe {
            libc::epoll_ctl(
                epoll_fd,
                libc::EPOLL_CTL_ADD,
                worker.wake_fd,
                &mut wake_event,
            )
        },
        0,
        "failed registering static HTTP eventfd"
    );

    let mut connections = ConnectionTable::default();
    let mut events = vec![libc::epoll_event { events: 0, u64: 0 }; EVENT_BATCH];
    let mut read_buffer = [0_u8; READ_BUFFER_BYTES];
    loop {
        let ready =
            unsafe { libc::epoll_wait(epoll_fd, events.as_mut_ptr(), events.len() as i32, -1) };
        if ready < 0 {
            if io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                continue;
            }
            break;
        }
        for event in events.iter().take(ready as usize) {
            if event.u64 == WAKE_TOKEN {
                drain_wake(worker.wake_fd);
                while let Some(registration) = worker.registrations.pop() {
                    register(epoll_fd, &mut connections, registration);
                }
                continue;
            }
            let fd = event.u64 as RawFd;
            let flags = event.events as i32;
            if flags & (libc::EPOLLERR | libc::EPOLLHUP) != 0 {
                close(epoll_fd, &mut connections, fd);
                continue;
            }
            if flags & libc::EPOLLOUT != 0 && !flush(epoll_fd, &mut connections, fd) {
                close(epoll_fd, &mut connections, fd);
                continue;
            }
            if flags & libc::EPOLLIN != 0
                && !read_ready(epoll_fd, &mut connections, fd, &mut read_buffer)
            {
                close(epoll_fd, &mut connections, fd);
                continue;
            }
            if flags & libc::EPOLLRDHUP != 0 {
                close(epoll_fd, &mut connections, fd);
            }
        }
    }
    unsafe { libc::close(epoll_fd) };
}

fn register(epoll_fd: RawFd, table: &mut ConnectionTable, registration: Registration) {
    let request = registration.request;
    let fd = request.stream.as_raw_fd();
    table.insert(
        fd,
        Connection {
            stream: request.stream,
            input: request.initial_prefix.to_vec(),
            exact_request_head: request.exact_request_head,
            response_head: request.response_head,
            combined_response: request.combined_response,
            body: request.body,
            sendfile: request.sendfile,
            file_path: request.file_path,
            file_len: request.file_len,
            file_modified: request.file_modified,
            checked_at: Instant::now(),
            config_epoch: request.config_epoch,
            expected_epoch: request.expected_epoch,
            pending: false,
            head_offset: 0,
            body_offset: 0,
            file_offset: 0,
            corked: false,
            fallback: Some(registration.fallback),
        },
    );
    if !prepare_next(table.get_mut(fd).expect("static connection registered"))
        || !add_or_modify(
            epoll_fd,
            fd,
            table.get_mut(fd).expect("static connection registered"),
            true,
        )
    {
        close(epoll_fd, table, fd);
        return;
    }
    if table
        .get_mut(fd)
        .is_some_and(|connection| connection.pending)
        && !flush(epoll_fd, table, fd)
    {
        close(epoll_fd, table, fd);
    }
}

fn prepare_next(connection: &mut Connection) -> bool {
    if connection.pending {
        return true;
    }
    let Some(head_end) = memmem::find(&connection.input, b"\r\n\r\n").map(|index| index + 4) else {
        return connection.input.len() < MAX_HEAD_BYTES;
    };
    if connection.config_epoch.load(Ordering::Acquire) != connection.expected_epoch
        || connection.input[..head_end] != *connection.exact_request_head
        || !file_is_current(connection)
    {
        return fallback(connection);
    }
    connection.input.drain(..head_end);
    connection.pending = true;
    connection.head_offset = 0;
    connection.body_offset = 0;
    connection.file_offset = 0;
    if connection.sendfile.is_some() && !connection.corked {
        set_tcp_cork(connection.stream.as_raw_fd(), true);
        connection.corked = true;
    }
    true
}

fn file_is_current(connection: &mut Connection) -> bool {
    if connection.checked_at.elapsed() < REVALIDATE_INTERVAL {
        return true;
    }
    connection.checked_at = Instant::now();
    std::fs::metadata(&connection.file_path).is_ok_and(|metadata| {
        metadata.len() == connection.file_len
            && metadata.modified().ok() == connection.file_modified
    })
}

fn fallback(connection: &mut Connection) -> bool {
    let Some(sender) = connection.fallback.take() else {
        return false;
    };
    let replacement = connection.stream.try_clone();
    match replacement {
        Ok(stream) => {
            let original = mem::replace(&mut connection.stream, stream);
            let prefix = Bytes::copy_from_slice(&connection.input);
            let _ = sender.send(Fallback {
                stream: original,
                prefix,
            });
        }
        Err(_) => return false,
    }
    false
}

fn read_ready(epoll_fd: RawFd, table: &mut ConnectionTable, fd: RawFd, buffer: &mut [u8]) -> bool {
    loop {
        let read = unsafe { libc::recv(fd, buffer.as_mut_ptr().cast(), buffer.len(), 0) };
        if read > 0 {
            let Some(connection) = table.get_mut(fd) else {
                return false;
            };
            if connection.pending {
                return true;
            }
            connection.input.extend_from_slice(&buffer[..read as usize]);
            if !prepare_next(connection) {
                return false;
            }
            if connection.pending {
                return flush(epoll_fd, table, fd);
            }
            continue;
        }
        if read == 0 {
            return false;
        }
        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::Interrupted {
            continue;
        }
        return error.kind() == io::ErrorKind::WouldBlock;
    }
}

fn flush(epoll_fd: RawFd, table: &mut ConnectionTable, fd: RawFd) -> bool {
    for _ in 0..MAX_RESPONSES_PER_EVENT {
        let Some(connection) = table.get_mut(fd) else {
            return false;
        };
        if !connection.pending {
            return add_or_modify(epoll_fd, fd, connection, false);
        }
        match flush_one(fd, connection) {
            Ok(true) => {
                if connection.corked {
                    set_tcp_cork(fd, false);
                    connection.corked = false;
                }
                connection.pending = false;
                if !prepare_next(connection) {
                    return false;
                }
            }
            Ok(false) => return add_or_modify(epoll_fd, fd, connection, false),
            Err(_) => return false,
        }
    }
    table
        .get_mut(fd)
        .is_some_and(|connection| add_or_modify(epoll_fd, fd, connection, false))
}

fn flush_one(fd: RawFd, connection: &mut Connection) -> io::Result<bool> {
    if let Some(combined) = connection.combined_response.as_ref() {
        connection.head_offset += send_slice(fd, &combined[connection.head_offset..])?;
        return Ok(connection.head_offset == combined.len());
    }
    if connection.head_offset < connection.response_head.len() {
        connection.head_offset +=
            send_slice(fd, &connection.response_head[connection.head_offset..])?;
        if connection.head_offset < connection.response_head.len() {
            return Ok(false);
        }
    }
    if let Some(body) = connection.body.as_ref() {
        connection.body_offset += send_slice(fd, &body[connection.body_offset..])?;
        return Ok(connection.body_offset == body.len());
    }
    if let Some(file) = connection.sendfile.as_ref() {
        while connection.file_offset < connection.file_len as libc::off_t {
            let remaining = (connection.file_len as libc::off_t - connection.file_offset) as usize;
            let sent = unsafe {
                libc::sendfile(fd, file.as_raw_fd(), &mut connection.file_offset, remaining)
            };
            if sent > 0 {
                continue;
            }
            if sent == 0 {
                break;
            }
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            if error.kind() == io::ErrorKind::WouldBlock {
                return Ok(false);
            }
            return Err(error);
        }
        return Ok(connection.file_offset >= connection.file_len as libc::off_t);
    }
    Ok(true)
}

fn send_slice(fd: RawFd, bytes: &[u8]) -> io::Result<usize> {
    if bytes.is_empty() {
        return Ok(0);
    }
    loop {
        let sent =
            unsafe { libc::send(fd, bytes.as_ptr().cast(), bytes.len(), libc::MSG_NOSIGNAL) };
        if sent >= 0 {
            return Ok(sent as usize);
        }
        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::Interrupted {
            continue;
        }
        if error.kind() == io::ErrorKind::WouldBlock {
            return Ok(0);
        }
        return Err(error);
    }
}

fn set_tcp_cork(fd: RawFd, enabled: bool) {
    let value: libc::c_int = i32::from(enabled);
    unsafe {
        libc::setsockopt(
            fd,
            libc::IPPROTO_TCP,
            libc::TCP_CORK,
            (&value as *const libc::c_int).cast(),
            mem::size_of_val(&value) as libc::socklen_t,
        );
    }
}

fn add_or_modify(epoll_fd: RawFd, fd: RawFd, connection: &Connection, add: bool) -> bool {
    let mut interests = (libc::EPOLLERR | libc::EPOLLHUP | libc::EPOLLRDHUP) as u32;
    if connection.pending {
        interests |= libc::EPOLLOUT as u32;
    } else {
        interests |= libc::EPOLLIN as u32;
    }
    let mut event = libc::epoll_event {
        events: interests,
        u64: fd as u64,
    };
    let operation = if add {
        libc::EPOLL_CTL_ADD
    } else {
        libc::EPOLL_CTL_MOD
    };
    unsafe { libc::epoll_ctl(epoll_fd, operation, fd, &mut event) == 0 }
}

fn close(epoll_fd: RawFd, table: &mut ConnectionTable, fd: RawFd) {
    unsafe { libc::epoll_ctl(epoll_fd, libc::EPOLL_CTL_DEL, fd, std::ptr::null_mut()) };
    drop(table.remove(fd));
}

fn wake(fd: RawFd) {
    let value = 1_u64;
    let _ = unsafe { libc::write(fd, (&value as *const u64).cast(), mem::size_of::<u64>()) };
}

fn drain_wake(fd: RawFd) {
    let mut value = 0_u64;
    loop {
        let result =
            unsafe { libc::read(fd, (&mut value as *mut u64).cast(), mem::size_of::<u64>()) };
        if result >= 0 {
            if result == 0 {
                return;
            }
            continue;
        }
        if io::Error::last_os_error().kind() != io::ErrorKind::Interrupted {
            return;
        }
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
    let mut cpus = Vec::new();
    if result == 0 {
        for cpu in 0..libc::CPU_SETSIZE as usize {
            if unsafe { libc::CPU_ISSET(cpu, &set) } {
                cpus.push(cpu);
            }
        }
    }
    cpus
}

fn pin_current_thread(cpu: usize) {
    let mut set = unsafe { mem::zeroed::<libc::cpu_set_t>() };
    unsafe {
        libc::CPU_SET(cpu, &mut set);
        libc::sched_setaffinity(
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
    use std::net::TcpListener;

    #[test]
    fn exact_static_requests_stay_native_and_epoch_change_falls_back() {
        let path = std::env::temp_dir().join(format!(
            "proxysss-static-reactor-{}-{}.txt",
            std::process::id(),
            Instant::now().elapsed().as_nanos()
        ));
        std::fs::write(&path, b"ok").expect("write reactor fixture");
        let metadata = std::fs::metadata(&path).expect("fixture metadata");
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind reactor fixture");
        let mut client = TcpStream::connect(listener.local_addr().expect("fixture address"))
            .expect("connect reactor fixture");
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set fixture timeout");
        let (server, _) = listener.accept().expect("accept reactor fixture");
        server
            .set_nonblocking(true)
            .expect("set fixture nonblocking");

        let request = Bytes::from_static(b"GET /asset HTTP/1.1\r\nHost: test\r\n\r\n");
        let response = Bytes::from_static(
            b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\nconnection: keep-alive\r\n\r\nok",
        );
        let epoch = Arc::new(AtomicU64::new(7));
        let fallback = dispatch(
            DispatchRequest {
                stream: server,
                initial_prefix: request.clone(),
                exact_request_head: request.clone(),
                response_head: Bytes::new(),
                combined_response: Some(response.clone()),
                body: None,
                sendfile: None,
                file_path: path.clone(),
                file_len: metadata.len(),
                file_modified: metadata.modified().ok(),
                config_epoch: epoch.clone(),
                expected_epoch: 7,
            },
            1,
        )
        .expect("dispatch exact static fixture");

        let mut received = vec![0_u8; response.len()];
        client
            .read_exact(&mut received)
            .expect("read first response");
        assert_eq!(received.as_slice(), response.as_ref());

        client.write_all(&request).expect("write repeated request");
        client
            .read_exact(&mut received)
            .expect("read repeated response");
        assert_eq!(received.as_slice(), response.as_ref());

        epoch.store(8, Ordering::Release);
        client
            .write_all(&request)
            .expect("write request after reload");
        let fallback = fallback.blocking_recv().expect("receive epoch fallback");
        assert_eq!(fallback.prefix, request);
        drop(fallback.stream);
        drop(client);
        let _ = std::fs::remove_file(path);
    }
}
