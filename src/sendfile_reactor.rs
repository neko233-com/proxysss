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
    file: Arc<File>,
    len: u64,
    max_chunk_bytes: u64,
    completion: oneshot::Sender<io::Result<u64>>,
}

struct SendfileState {
    _socket: TcpStream,
    file: Arc<File>,
    offset: libc::off_t,
    sent: u64,
    len: u64,
    max_chunk_bytes: u64,
    completion: Option<oneshot::Sender<io::Result<u64>>>,
}

struct ReactorWorker {
    registrations: ArrayQueue<SendfileJob>,
    wake_fd: RawFd,
}

struct Reactors {
    workers: Vec<Arc<ReactorWorker>>,
    worker_by_cpu: FxHashMap<usize, usize>,
    next: AtomicUsize,
}

enum DriveResult {
    Pending,
    Complete(io::Result<u64>),
}

static REACTORS: OnceLock<Reactors> = OnceLock::new();

pub(crate) fn dispatch(
    socket_fd: RawFd,
    file: Arc<File>,
    len: u64,
    max_chunk_bytes: u64,
    requested_workers: usize,
) -> io::Result<oneshot::Receiver<io::Result<u64>>> {
    if len == 0 {
        let (sender, receiver) = oneshot::channel();
        let _ = sender.send(Ok(0));
        return Ok(receiver);
    }
    let reactors = REACTORS.get_or_init(|| Reactors::start(requested_workers));
    let socket = duplicate_stream(socket_fd)?;
    let (sender, receiver) = oneshot::channel();
    let job = SendfileJob {
        socket,
        file,
        len,
        max_chunk_bytes: max_chunk_bytes.max(1),
        completion: sender,
    };
    let index = reactors.select_worker();
    let worker = &reactors.workers[index];
    worker
        .registrations
        .push(job)
        .map_err(|_| io::Error::new(io::ErrorKind::WouldBlock, "sendfile reactor queue full"))?;
    wake(worker.wake_fd);
    Ok(receiver)
}

impl Reactors {
    fn start(requested_workers: usize) -> Self {
        let worker_count = requested_workers.max(1);
        let allowed_cpus = allowed_cpu_ids();
        let worker_cpu_groups = build_worker_cpu_groups(&allowed_cpus, worker_count);
        let mut workers = Vec::with_capacity(worker_count);
        let mut worker_by_cpu = FxHashMap::default();
        for index in 0..worker_count {
            let wake_fd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
            assert!(wake_fd >= 0, "failed creating sendfile reactor eventfd");
            let worker = Arc::new(ReactorWorker {
                registrations: ArrayQueue::new(REGISTRATION_QUEUE_CAPACITY),
                wake_fd,
            });
            let reactor_worker = worker.clone();
            let cpu_group = worker_cpu_groups[index].clone();
            for cpu in &cpu_group {
                worker_by_cpu.insert(*cpu, index);
            }
            thread::Builder::new()
                .name(format!("proxysss-sendfile-epoll-{index}"))
                .spawn(move || run_reactor(reactor_worker, &cpu_group))
                .expect("failed spawning sendfile epoll reactor");
            workers.push(worker);
        }
        Self {
            workers,
            worker_by_cpu,
            next: AtomicUsize::new(0),
        }
    }

    fn select_worker(&self) -> usize {
        if let Some(cpu) = current_cpu_id() {
            if let Some(index) = self.worker_by_cpu.get(&cpu) {
                return *index;
            }
        }
        self.next.fetch_add(1, Ordering::Relaxed) % self.workers.len()
    }
}

fn current_cpu_id() -> Option<usize> {
    let cpu = unsafe { libc::sched_getcpu() };
    (cpu >= 0).then_some(cpu as usize)
}

fn build_worker_cpu_groups(allowed_cpus: &[usize], worker_count: usize) -> Vec<Vec<usize>> {
    let worker_count = worker_count.max(1);
    let mut groups = vec![Vec::new(); worker_count];
    for (position, cpu) in allowed_cpus.iter().copied().enumerate() {
        let worker = position.saturating_mul(worker_count) / allowed_cpus.len().max(1);
        groups[worker.min(worker_count - 1)].push(cpu);
    }
    for (index, group) in groups.iter_mut().enumerate() {
        if group.is_empty() {
            group.push(
                allowed_cpus
                    .get(index % allowed_cpus.len().max(1))
                    .copied()
                    .unwrap_or(0),
            );
        }
    }
    groups
}

fn duplicate_stream(fd: RawFd) -> io::Result<TcpStream> {
    let duplicated = duplicate_fd(fd)?;
    let stream = unsafe { TcpStream::from_raw_fd(duplicated) };
    stream.set_nonblocking(true)?;
    Ok(stream)
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

fn run_reactor(worker: Arc<ReactorWorker>, cpu_group: &[usize]) {
    if !cpu_group.is_empty() {
        pin_current_thread(cpu_group);
    }
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
    jobs.insert(
        fd,
        SendfileState {
            _socket: job.socket,
            file: job.file,
            offset: 0,
            sent: 0,
            len: job.len,
            max_chunk_bytes: job.max_chunk_bytes,
            completion: Some(job.completion),
        },
    );

    // The HTTP shard has just completed the response head write, so the socket
    // is commonly writable already. Drain once before EPOLL_CTL_ADD to avoid a
    // redundant readiness round-trip for every keep-alive response. A bounded
    // chunk or EAGAIN still enters the normal level-triggered epoll path.
    if let DriveResult::Complete(result) = drive_job(jobs, fd) {
        complete_job(epoll_fd, jobs, fd, result);
        return;
    }

    let mut event = libc::epoll_event {
        events: (libc::EPOLLOUT | libc::EPOLLERR | libc::EPOLLHUP) as u32,
        u64: fd as u64,
    };
    if unsafe { libc::epoll_ctl(epoll_fd, libc::EPOLL_CTL_ADD, fd, &mut event) } != 0 {
        complete_job(epoll_fd, jobs, fd, Err(io::Error::last_os_error()));
    }
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
        let written =
            unsafe { libc::sendfile(fd, state.file.as_raw_fd(), &mut state.offset, count) };
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
                "sendfile source ended before configured length",
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

fn pin_current_thread(cpus: &[usize]) {
    let mut set = unsafe { mem::zeroed::<libc::cpu_set_t>() };
    unsafe {
        for cpu in cpus {
            libc::CPU_SET(*cpu, &mut set);
        }
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
        let expected = vec![0x5a_u8; 64 * 1024];
        file.write_all(&expected).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        let completion = dispatch(
            server.as_raw_fd(),
            Arc::new(file),
            expected.len() as u64,
            16 * 1024,
            1,
        )
        .unwrap();
        drop(server);

        let reader = thread::spawn(move || {
            let mut received = vec![0_u8; expected.len()];
            client.read_exact(&mut received).unwrap();
            received
        });
        assert_eq!(completion.blocking_recv().unwrap().unwrap(), 64 * 1024);
        assert_eq!(reader.join().unwrap(), vec![0x5a_u8; 64 * 1024]);
    }

    #[test]
    fn sendfile_handoff_groups_allowed_cpus_stably() {
        assert_eq!(
            build_worker_cpu_groups(&[2, 7, 11, 19], 2),
            vec![vec![2, 7], vec![11, 19]]
        );
        assert_eq!(
            build_worker_cpu_groups(&[2, 7, 11, 19], 4),
            vec![vec![2], vec![7], vec![11], vec![19]]
        );
    }
}
