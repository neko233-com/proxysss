//! Linux static-file sendfile reactor.
//!
//! HTTP parsing and keep-alive ownership remain on the CPU-adaptive Tokio HTTP
//! shards. Large response bodies are handed to a bounded set of native epoll
//! workers so a writable socket cannot monopolize the HTTP scheduler while
//! repeatedly draining `sendfile(2)`. Each job owns duplicated descriptors and
//! reports completion before the HTTP task reads the next request.

use std::fs::File;
use std::io;
use std::mem::{self, MaybeUninit};
use std::net::TcpStream;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread;

use bytes::Bytes;
use crossbeam_queue::ArrayQueue;
use rustc_hash::FxHashMap;
use tokio::sync::oneshot;

// Per-worker handoff bursts are bounded independently of active jobs. A full
// queue falls back to Tokio rather than preallocating O(cores * 65k) entries on
// high-core hosts.
const REGISTRATION_QUEUE_CAPACITY: usize = 4_096;
const EVENT_BATCH: usize = 1_024;
const WAKE_TOKEN: u64 = u64::MAX;

struct SendfileJob {
    socket: TcpStream,
    source: StaticBodySource,
    len: u64,
    max_chunk_bytes: u64,
    completion: oneshot::Sender<io::Result<u64>>,
}

struct SendfileState {
    _socket: TcpStream,
    source: StaticBodySource,
    sent: u64,
    len: u64,
    max_chunk_bytes: u64,
    completion: Option<oneshot::Sender<io::Result<u64>>>,
}

enum StaticBodySource {
    File { file: File, offset: libc::off_t },
    Bytes(Bytes),
}

struct ReactorWorker {
    registrations: ArrayQueue<SendfileJob>,
    wake_fd: RawFd,
}

struct Reactors {
    workers: Vec<Arc<ReactorWorker>>,
    next: AtomicUsize,
}

enum DriveResult {
    Pending,
    Complete(io::Result<u64>),
}

static REACTORS: OnceLock<Reactors> = OnceLock::new();

pub(crate) fn dispatch(
    socket_fd: RawFd,
    file_fd: RawFd,
    offset: u64,
    len: u64,
    max_chunk_bytes: u64,
    requested_workers: usize,
    scheduler_nice: i32,
) -> io::Result<oneshot::Receiver<io::Result<u64>>> {
    if len == 0 {
        let (sender, receiver) = oneshot::channel();
        let _ = sender.send(Ok(0));
        return Ok(receiver);
    }
    let reactors = REACTORS.get_or_init(|| Reactors::start(requested_workers, scheduler_nice));
    let socket = duplicate_stream(socket_fd)?;
    let file = duplicate_file(file_fd)?;
    let (sender, receiver) = oneshot::channel();
    let job = SendfileJob {
        socket,
        source: StaticBodySource::File {
            file,
            offset: offset as libc::off_t,
        },
        len,
        max_chunk_bytes: max_chunk_bytes.max(1),
        completion: sender,
    };
    let index = reactors.next.fetch_add(1, Ordering::Relaxed) % reactors.workers.len();
    let worker = &reactors.workers[index];
    worker
        .registrations
        .push(job)
        .map_err(|_| io::Error::new(io::ErrorKind::WouldBlock, "sendfile reactor queue full"))?;
    wake(worker.wake_fd);
    Ok(receiver)
}

pub(crate) fn dispatch_bytes(
    socket_fd: RawFd,
    body: Bytes,
    max_chunk_bytes: u64,
    requested_workers: usize,
    scheduler_nice: i32,
) -> io::Result<oneshot::Receiver<io::Result<u64>>> {
    if body.is_empty() {
        let (sender, receiver) = oneshot::channel();
        let _ = sender.send(Ok(0));
        return Ok(receiver);
    }
    let reactors = REACTORS.get_or_init(|| Reactors::start(requested_workers, scheduler_nice));
    let socket = duplicate_stream(socket_fd)?;
    let len = body.len() as u64;
    let (sender, receiver) = oneshot::channel();
    let job = SendfileJob {
        socket,
        source: StaticBodySource::Bytes(body),
        len,
        max_chunk_bytes: max_chunk_bytes.max(1),
        completion: sender,
    };
    let index = reactors.next.fetch_add(1, Ordering::Relaxed) % reactors.workers.len();
    let worker = &reactors.workers[index];
    worker
        .registrations
        .push(job)
        .map_err(|_| io::Error::new(io::ErrorKind::WouldBlock, "static body reactor queue full"))?;
    wake(worker.wake_fd);
    Ok(receiver)
}

impl Reactors {
    fn start(requested_workers: usize, scheduler_nice: i32) -> Self {
        let worker_count = requested_workers.max(1);
        let allowed_cpus = allowed_cpu_ids();
        let mut workers = Vec::with_capacity(worker_count);
        for index in 0..worker_count {
            let wake_fd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
            assert!(wake_fd >= 0, "failed creating sendfile reactor eventfd");
            let worker = Arc::new(ReactorWorker {
                registrations: ArrayQueue::new(REGISTRATION_QUEUE_CAPACITY),
                wake_fd,
            });
            let reactor_worker = worker.clone();
            let cpu = allowed_cpus.get(index % allowed_cpus.len()).copied();
            thread::Builder::new()
                .name(format!("proxysss-sendfile-epoll-{index}"))
                .spawn(move || run_reactor(reactor_worker, cpu, scheduler_nice))
                .expect("failed spawning sendfile epoll reactor");
            workers.push(worker);
        }
        Self {
            workers,
            next: AtomicUsize::new(0),
        }
    }
}

fn duplicate_stream(fd: RawFd) -> io::Result<TcpStream> {
    let duplicated = duplicate_fd(fd)?;
    let stream = unsafe { TcpStream::from_raw_fd(duplicated) };
    stream.set_nonblocking(true)?;
    Ok(stream)
}

fn duplicate_file(fd: RawFd) -> io::Result<File> {
    let duplicated = duplicate_fd(fd)?;
    Ok(unsafe { File::from_raw_fd(duplicated) })
}

fn duplicate_fd(fd: RawFd) -> io::Result<RawFd> {
    let duplicated = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 0) };
    if duplicated < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(duplicated)
    }
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
    assert!(epoll_fd >= 0, "failed creating sendfile epoll instance");
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
    assert_eq!(add_wake, 0, "failed registering sendfile reactor eventfd");

    let mut jobs = FxHashMap::<RawFd, SendfileState>::default();
    let mut events = vec![libc::epoll_event { events: 0, u64: 0 }; EVENT_BATCH];
    loop {
        let ready =
            unsafe { libc::epoll_wait(epoll_fd, events.as_mut_ptr(), events.len() as i32, -1) };
        if ready < 0 {
            if io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                continue;
            }
            fail_all_jobs(epoll_fd, &mut jobs, "sendfile epoll_wait failed");
            while let Some(job) = worker.registrations.pop() {
                let _ = job
                    .completion
                    .send(Err(io::Error::other("sendfile epoll_wait failed")));
            }
            break;
        }

        for event in events.iter().take(ready as usize) {
            let token = event.u64;
            if token == WAKE_TOKEN {
                drain_wake(worker.wake_fd);
                while let Some(job) = worker.registrations.pop() {
                    register_job(epoll_fd, &mut jobs, job);
                }
                continue;
            }

            let fd = token as RawFd;
            if !jobs.contains_key(&fd) {
                continue;
            }
            let flags = event.events as i32;
            let result = if flags & (libc::EPOLLERR | libc::EPOLLHUP) != 0 {
                DriveResult::Complete(Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "sendfile client socket closed",
                )))
            } else if flags & libc::EPOLLOUT != 0 {
                drive_job(&mut jobs, fd)
            } else {
                DriveResult::Pending
            };
            if let DriveResult::Complete(result) = result {
                complete_job(epoll_fd, &mut jobs, fd, result);
            }
        }
    }

    unsafe {
        libc::close(epoll_fd);
    }
}

fn register_job(epoll_fd: RawFd, jobs: &mut FxHashMap<RawFd, SendfileState>, job: SendfileJob) {
    let fd = job.socket.as_raw_fd();
    let mut event = libc::epoll_event {
        events: (libc::EPOLLOUT | libc::EPOLLERR | libc::EPOLLHUP) as u32,
        u64: fd as u64,
    };
    if unsafe { libc::epoll_ctl(epoll_fd, libc::EPOLL_CTL_ADD, fd, &mut event) } != 0 {
        let _ = job.completion.send(Err(io::Error::last_os_error()));
        return;
    }
    jobs.insert(
        fd,
        SendfileState {
            _socket: job.socket,
            source: job.source,
            sent: 0,
            len: job.len,
            max_chunk_bytes: job.max_chunk_bytes,
            completion: Some(job.completion),
        },
    );
}

fn drive_job(jobs: &mut FxHashMap<RawFd, SendfileState>, fd: RawFd) -> DriveResult {
    let Some(state) = jobs.get_mut(&fd) else {
        return DriveResult::Complete(Err(io::Error::new(
            io::ErrorKind::NotFound,
            "sendfile reactor job disappeared",
        )));
    };
    let event_budget = (state.len - state.sent).min(state.max_chunk_bytes);
    let event_start = state.sent;
    while state.sent - event_start < event_budget {
        let remaining_budget = event_budget - (state.sent - event_start);
        let remaining_file = state.len - state.sent;
        let count = remaining_budget.min(remaining_file) as usize;
        let written = match &mut state.source {
            StaticBodySource::File { file, offset } => unsafe {
                libc::sendfile(fd, file.as_raw_fd(), offset, count)
            },
            StaticBodySource::Bytes(body) => {
                let start = state.sent as usize;
                unsafe {
                    libc::send(
                        fd,
                        body.as_ptr().add(start).cast(),
                        count,
                        libc::MSG_NOSIGNAL,
                    )
                }
            }
        };
        if written > 0 {
            state.sent = state.sent.saturating_add(written as u64);
            if state.sent >= state.len {
                return DriveResult::Complete(Ok(state.sent));
            }
            continue;
        }
        if written == 0 {
            return DriveResult::Complete(Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "static response source ended before configured length",
            )));
        }

        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::Interrupted {
            continue;
        }
        if error.kind() == io::ErrorKind::WouldBlock {
            return DriveResult::Pending;
        }
        return DriveResult::Complete(Err(error));
    }
    DriveResult::Pending
}

fn complete_job(
    epoll_fd: RawFd,
    jobs: &mut FxHashMap<RawFd, SendfileState>,
    fd: RawFd,
    result: io::Result<u64>,
) {
    unsafe {
        libc::epoll_ctl(epoll_fd, libc::EPOLL_CTL_DEL, fd, std::ptr::null_mut());
    }
    if let Some(mut state) = jobs.remove(&fd) {
        if let Some(completion) = state.completion.take() {
            let _ = completion.send(result);
        }
    }
}

fn fail_all_jobs(epoll_fd: RawFd, jobs: &mut FxHashMap<RawFd, SendfileState>, message: &str) {
    let fds = jobs.keys().copied().collect::<Vec<_>>();
    for fd in fds {
        complete_job(
            epoll_fd,
            jobs,
            fd,
            Err(io::Error::other(message.to_string())),
        );
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

fn set_current_thread_nice(scheduler_nice: i32) {
    if scheduler_nice <= 0 {
        return;
    }
    // Linux applies setpriority to the calling task (native thread). Positive
    // nice values need no elevated capability and preserve full idle-CPU use;
    // they only give HTTP shards more CFS weight when both paths are runnable.
    unsafe {
        let _ = libc::setpriority(libc::PRIO_PROCESS, 0, scheduler_nice);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::Duration;

    #[test]
    fn reactor_sends_exact_file_bytes() {
        let name = CString::new("proxysss-sendfile-test").unwrap();
        let file_fd = unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC) };
        assert!(file_fd >= 0);
        let mut file = unsafe { File::from_raw_fd(file_fd) };
        let expected = (0..64 * 1024)
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>();
        file.write_all(&expected).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        let start = 1024_usize;
        let completion = dispatch(
            server.as_raw_fd(),
            file.as_raw_fd(),
            start as u64,
            (expected.len() - start) as u64,
            16 * 1024,
            1,
            0,
        )
        .unwrap();
        drop(server);
        drop(file);

        let reader = thread::spawn(move || {
            let mut received = vec![0_u8; expected.len() - start];
            client.read_exact(&mut received).unwrap();
            (received, expected[start..].to_vec())
        });
        assert_eq!(
            completion.blocking_recv().unwrap().unwrap(),
            (64 * 1024 - start) as u64
        );
        let (received, expected) = reader.join().unwrap();
        assert_eq!(received, expected);
    }

    #[test]
    fn reactor_sends_exact_cached_bytes() {
        let expected = Bytes::from(
            (0..64 * 1024)
                .map(|index| (index % 239) as u8)
                .collect::<Vec<_>>(),
        );
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        let completion =
            dispatch_bytes(server.as_raw_fd(), expected.clone(), 8 * 1024, 1, 0).unwrap();
        drop(server);

        let reader = thread::spawn(move || {
            let mut received = vec![0_u8; expected.len()];
            client.read_exact(&mut received).unwrap();
            (received, expected.to_vec())
        });
        assert_eq!(completion.blocking_recv().unwrap().unwrap(), 64 * 1024);
        let (received, expected) = reader.join().unwrap();
        assert_eq!(received, expected);
    }
}
