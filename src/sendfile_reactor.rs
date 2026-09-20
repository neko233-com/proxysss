//! Linux static-file sendfile reactor.
//!
//! HTTP parsing and keep-alive ownership remain on the CPU-adaptive Tokio HTTP
//! shards. Large response bodies are handed to a bounded set of native epoll
//! workers so a writable socket cannot monopolize the HTTP scheduler while
//! repeatedly draining `sendfile(2)`. Each job owns duplicated descriptors and
//! reports completion before the HTTP task reads the next request.

use std::fs::File;
use std::future::Future;
use std::io;
use std::mem::{self, MaybeUninit};
use std::net::TcpStream;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::task::{Context, Poll};
use std::thread;

use crossbeam_queue::ArrayQueue;
use tokio::sync::oneshot;

// Per-worker handoff bursts are bounded independently of active jobs. A full
// queue falls back to Tokio rather than preallocating O(cores * 65k) entries on
// high-core hosts.
const REGISTRATION_QUEUE_CAPACITY: usize = 4_096;
const EVENT_BATCH: usize = 1_024;
const REGISTRATION_BATCH: usize = 256;
const WAKE_TOKEN: u64 = u64::MAX;
pub(crate) const SHARED_SEND_BYTES: u64 = 1024 * 1024;
const SHARED_ADMISSION_BATCH: usize = 32;

pub(crate) struct SendfileCompletion {
    pub(crate) bytes: u64,
    pub(crate) uncorked: bool,
}

pub(crate) struct PendingSendfile {
    receiver: oneshot::Receiver<io::Result<SendfileCompletion>>,
    cancellation: Option<(Arc<ReactorWorker>, RawFd)>,
}

impl Future for PendingSendfile {
    type Output = Result<io::Result<SendfileCompletion>, oneshot::error::RecvError>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let result = Pin::new(&mut self.receiver).poll(context);
        if result.is_ready() {
            self.cancellation = None;
        }
        result
    }
}

impl Drop for PendingSendfile {
    fn drop(&mut self) {
        if let Some((worker, fd)) = self.cancellation.take() {
            // Publish closure before the wake: a blocked socket need never emit
            // another event. Only abandoned jobs are eligible for cancellation,
            // so an old fd token cannot abort a new live job after fd reuse.
            self.receiver.close();
            if worker.cancellations.push(fd).is_err() {
                worker.sweep_cancelled.store(true, Ordering::Release);
            }
            wake(worker.wake_fd);
        }
    }
}

struct SendfileJob {
    socket: TcpStream,
    file: File,
    offset: u64,
    len: u64,
    max_chunk_bytes: u64,
    completion: oneshot::Sender<io::Result<SendfileCompletion>>,
}

struct SendfileState {
    socket: TcpStream,
    file: File,
    offset: libc::off_t,
    sent: u64,
    len: u64,
    max_chunk_bytes: u64,
    completion: Option<oneshot::Sender<io::Result<SendfileCompletion>>>,
    ready: bool,
    ready_flags: i32,
    ready_prev: Option<RawFd>,
    ready_next: Option<RawFd>,
}

#[derive(Default)]
struct SendfileTable {
    slots: Vec<Option<SendfileState>>,
    ready_head: Option<RawFd>,
    ready_tail: Option<RawFd>,
}

impl SendfileTable {
    fn contains(&self, fd: RawFd) -> bool {
        usize::try_from(fd)
            .ok()
            .and_then(|index| self.slots.get(index))
            .is_some_and(Option::is_some)
    }

    fn get_mut(&mut self, fd: RawFd) -> Option<&mut SendfileState> {
        let index = usize::try_from(fd).ok()?;
        self.slots.get_mut(index)?.as_mut()
    }

    fn insert(&mut self, fd: RawFd, state: SendfileState) {
        let index = usize::try_from(fd).expect("sendfile fd must be non-negative");
        if self.slots.len() <= index {
            self.slots.resize_with(index + 1, || None);
        }
        debug_assert!(self.slots[index].is_none());
        self.slots[index] = Some(state);
    }

    fn remove(&mut self, fd: RawFd) -> Option<SendfileState> {
        self.unlink_ready(fd);
        let index = usize::try_from(fd).ok()?;
        self.slots.get_mut(index)?.take()
    }

    fn enqueue_ready(&mut self, fd: RawFd, flags: i32) {
        let tail = self.ready_tail;
        let Some(state) = self.get_mut(fd) else {
            return;
        };
        state.ready_flags |= flags;
        if state.ready {
            return;
        }
        state.ready = true;
        state.ready_prev = tail;
        state.ready_next = None;
        if let Some(tail) = tail {
            self.get_mut(tail).expect("ready tail exists").ready_next = Some(fd);
        } else {
            self.ready_head = Some(fd);
        }
        self.ready_tail = Some(fd);
    }

    fn unlink_ready(&mut self, fd: RawFd) {
        let Some(state) = self.get_mut(fd) else {
            return;
        };
        if !state.ready {
            return;
        }
        let (prev, next) = (state.ready_prev.take(), state.ready_next.take());
        state.ready = false;
        if let Some(prev) = prev {
            self.get_mut(prev)
                .expect("ready predecessor exists")
                .ready_next = next;
        } else {
            self.ready_head = next;
        }
        if let Some(next) = next {
            self.get_mut(next)
                .expect("ready successor exists")
                .ready_prev = prev;
        } else {
            self.ready_tail = prev;
        }
    }

    fn pop_ready(&mut self) -> Option<(RawFd, i32)> {
        let fd = self.ready_head?;
        self.unlink_ready(fd);
        let flags = mem::take(&mut self.get_mut(fd).expect("ready job exists").ready_flags);
        Some((fd, flags))
    }

    fn active_fds(&self) -> Vec<RawFd> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(fd, state)| state.as_ref().map(|_| fd as RawFd))
            .collect()
    }
}

pub(crate) struct ReactorWorker {
    registrations: ArrayQueue<SendfileJob>,
    cancellations: ArrayQueue<RawFd>,
    sweep_cancelled: AtomicBool,
    wake_fd: RawFd,
}

impl ReactorWorker {
    pub(crate) fn shared(wake_fd: RawFd) -> Arc<Self> {
        Arc::new(Self {
            registrations: ArrayQueue::new(REGISTRATION_QUEUE_CAPACITY),
            cancellations: ArrayQueue::new(REGISTRATION_QUEUE_CAPACITY),
            sweep_cancelled: AtomicBool::new(false),
            wake_fd,
        })
    }
}

/// Single-owner download state driven by the realtime epoll loop. Ready jobs
/// form an intrusive FIFO: at most one node per live job, removed in O(1) on
/// completion/cancellation, so fd reuse cannot leave stale queue entries.
#[derive(Default)]
pub(crate) struct SharedDownloads {
    jobs: SendfileTable,
}

impl SharedDownloads {
    pub(crate) fn has_ready(&self) -> bool {
        self.jobs.ready_head.is_some()
    }

    pub(crate) fn observe(&mut self, events: &[libc::epoll_event]) {
        for event in events {
            if event.u64 != WAKE_TOKEN {
                self.jobs
                    .enqueue_ready(event.u64 as RawFd, event.events as i32);
            }
        }
    }

    // Call only after the entire cached epoll batch has been consumed.
    pub(crate) fn on_wake(&mut self, epoll_fd: RawFd, worker: &ReactorWorker) {
        cancel_abandoned_jobs(epoll_fd, &mut self.jobs, worker);
        for _ in 0..SHARED_ADMISSION_BATCH {
            let Some(mut job) = worker.registrations.pop() else {
                break;
            };
            job.max_chunk_bytes = SHARED_SEND_BYTES;
            register_job_inner(epoll_fd, &mut self.jobs, job, false);
        }
        if !worker.registrations.is_empty() {
            wake(worker.wake_fd);
        }
    }

    pub(crate) fn drive_one(&mut self, epoll_fd: RawFd) {
        let Some((fd, flags)) = self.jobs.pop_ready() else {
            return;
        };
        if flags & (libc::EPOLLERR | libc::EPOLLHUP) != 0 {
            complete_job(
                epoll_fd,
                &mut self.jobs,
                fd,
                Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "sendfile client socket closed",
                )),
            );
            return;
        }
        let before = self.jobs.get_mut(fd).expect("ready job exists").sent;
        match drive_job(&mut self.jobs, fd) {
            DriveResult::Complete(result) => complete_job(epoll_fd, &mut self.jobs, fd, result),
            DriveResult::Pending => {
                let state = self.jobs.get_mut(fd).expect("pending job exists");
                // Full quantum remains runnable; real EAGAIN waits for epoll.
                if state.sent - before >= state.max_chunk_bytes {
                    self.jobs.enqueue_ready(fd, libc::EPOLLOUT);
                }
            }
        }
    }

    pub(crate) fn fail(&mut self, epoll_fd: RawFd, worker: &ReactorWorker) {
        fail_all_jobs(epoll_fd, &mut self.jobs, "shared realtime epoll failed");
        while let Some(job) = worker.registrations.pop() {
            let _ = job
                .completion
                .send(Err(io::Error::other("shared realtime epoll failed")));
        }
    }
}

struct Reactors {
    workers: Vec<Arc<ReactorWorker>>,
    worker_cpus: Vec<Option<usize>>,
    next: AtomicUsize,
}

enum DriveResult {
    Pending,
    Complete(io::Result<u64>),
}

static REACTORS: OnceLock<Reactors> = OnceLock::new();

pub(crate) fn warm(requested_workers: usize, scheduler_nice: i32) {
    let _ = REACTORS.get_or_init(|| Reactors::start(requested_workers, scheduler_nice));
}

#[allow(clippy::too_many_arguments)] // Explicit ownership/limits on the existing reactor boundary.
pub(crate) fn dispatch(
    socket_fd: RawFd,
    file_fd: RawFd,
    offset: u64,
    len: u64,
    max_chunk_bytes: u64,
    requested_workers: usize,
    active_workers: usize,
    scheduler_nice: i32,
) -> io::Result<PendingSendfile> {
    let reactors = REACTORS.get_or_init(|| Reactors::start(requested_workers, scheduler_nice));
    let fallback = reactors.next.fetch_add(1, Ordering::Relaxed);
    let index = preferred_worker_index_limited(
        &reactors.worker_cpus,
        current_cpu(),
        fallback,
        active_workers,
    );
    dispatch_to_worker(
        &reactors.workers[index],
        socket_fd,
        file_fd,
        offset,
        len,
        max_chunk_bytes,
    )
}

pub(crate) fn dispatch_to_worker(
    worker: &Arc<ReactorWorker>,
    socket_fd: RawFd,
    file_fd: RawFd,
    offset: u64,
    len: u64,
    max_chunk_bytes: u64,
) -> io::Result<PendingSendfile> {
    if len == 0 {
        let (sender, receiver) = oneshot::channel();
        let _ = sender.send(Ok(SendfileCompletion {
            bytes: 0,
            uncorked: false,
        }));
        return Ok(PendingSendfile {
            receiver,
            cancellation: None,
        });
    }
    let socket = duplicate_stream(socket_fd)?;
    let reactor_fd = socket.as_raw_fd();
    let file = duplicate_file(file_fd)?;
    let (sender, receiver) = oneshot::channel();
    let job = SendfileJob {
        socket,
        file,
        offset,
        len,
        max_chunk_bytes: max_chunk_bytes.max(1),
        completion: sender,
    };
    worker
        .registrations
        .push(job)
        .map_err(|_| io::Error::new(io::ErrorKind::WouldBlock, "sendfile reactor queue full"))?;
    wake(worker.wake_fd);
    Ok(PendingSendfile {
        receiver,
        cancellation: Some((worker.clone(), reactor_fd)),
    })
}

impl Reactors {
    fn start(requested_workers: usize, scheduler_nice: i32) -> Self {
        let worker_count = requested_workers.max(1);
        let allowed_cpus = crate::linux_cpu::allowed_cpu_ids();
        let mut workers = Vec::with_capacity(worker_count);
        let mut worker_cpus = Vec::with_capacity(worker_count);
        for index in 0..worker_count {
            let wake_fd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
            assert!(wake_fd >= 0, "failed creating sendfile reactor eventfd");
            let worker = Arc::new(ReactorWorker {
                registrations: ArrayQueue::new(REGISTRATION_QUEUE_CAPACITY),
                cancellations: ArrayQueue::new(REGISTRATION_QUEUE_CAPACITY),
                sweep_cancelled: AtomicBool::new(false),
                wake_fd,
            });
            let reactor_worker = worker.clone();
            let cpu = allowed_cpus.get(index % allowed_cpus.len().max(1)).copied();
            thread::Builder::new()
                .name(format!("proxysss-sendfile-epoll-{index}"))
                .spawn(move || run_reactor(reactor_worker, cpu, scheduler_nice))
                .expect("failed spawning sendfile epoll reactor");
            workers.push(worker);
            worker_cpus.push(cpu);
        }
        Self {
            workers,
            worker_cpus,
            next: AtomicUsize::new(0),
        }
    }
}

fn current_cpu() -> Option<usize> {
    let cpu = unsafe { libc::sched_getcpu() };
    (cpu >= 0).then_some(cpu as usize)
}

fn preferred_worker_index(
    worker_cpus: &[Option<usize>],
    current_cpu: Option<usize>,
    fallback: usize,
) -> usize {
    current_cpu
        .and_then(|cpu| {
            worker_cpus
                .iter()
                .position(|worker_cpu| *worker_cpu == Some(cpu))
        })
        .unwrap_or(fallback % worker_cpus.len().max(1))
}

fn preferred_worker_index_limited(
    worker_cpus: &[Option<usize>],
    current_cpu: Option<usize>,
    fallback: usize,
    active_workers: usize,
) -> usize {
    let active_workers = active_workers.clamp(1, worker_cpus.len().max(1));
    if active_workers >= worker_cpus.len() {
        return preferred_worker_index(worker_cpus, current_cpu, fallback);
    }
    let candidate = |slot: usize| slot.saturating_mul(worker_cpus.len()) / active_workers;
    if let Some(cpu) = current_cpu {
        for slot in 0..active_workers {
            let index = candidate(slot);
            if worker_cpus.get(index).copied().flatten() == Some(cpu) {
                return index;
            }
        }
    }
    candidate(fallback % active_workers)
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

    let mut jobs = SendfileTable::default();
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

        process_event_batch(epoll_fd, &mut jobs, &events[..ready as usize], &worker);
    }

    unsafe {
        libc::close(epoll_fd);
    }
}

fn process_event_batch(
    epoll_fd: RawFd,
    jobs: &mut SendfileTable,
    events: &[libc::epoll_event],
    worker: &ReactorWorker,
) {
    let mut registrations_ready = false;
    for event in events {
        let token = event.u64;
        if token == WAKE_TOKEN {
            drain_wake(worker.wake_fd);
            registrations_ready = true;
            continue;
        }

        let fd = token as RawFd;
        if !jobs.contains(fd) {
            continue;
        }
        let flags = event.events as i32;
        let result = if flags & (libc::EPOLLERR | libc::EPOLLHUP) != 0 {
            DriveResult::Complete(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "sendfile client socket closed",
            )))
        } else if flags & libc::EPOLLOUT != 0 {
            drive_job(jobs, fd)
        } else {
            DriveResult::Pending
        };
        if let DriveResult::Complete(result) = result {
            complete_job(epoll_fd, jobs, fd, result);
        }
    }
    // Do not reuse slots until every cached epoll event has been consumed.
    // In particular, an old HUP must never fail a newly registered download.
    if registrations_ready {
        cancel_abandoned_jobs(epoll_fd, jobs, worker);
        register_pending_jobs(epoll_fd, jobs, worker);
    }
}

fn register_pending_jobs(epoll_fd: RawFd, jobs: &mut SendfileTable, worker: &ReactorWorker) {
    for _ in 0..REGISTRATION_BATCH {
        let Some(job) = worker.registrations.pop() else {
            break;
        };
        register_job(epoll_fd, jobs, job);
    }
    if !worker.registrations.is_empty() {
        wake(worker.wake_fd);
    }
}

fn register_job(epoll_fd: RawFd, jobs: &mut SendfileTable, job: SendfileJob) {
    register_job_inner(epoll_fd, jobs, job, true);
}

fn register_job_inner(
    epoll_fd: RawFd,
    jobs: &mut SendfileTable,
    job: SendfileJob,
    immediate: bool,
) {
    if job.completion.is_closed() {
        return;
    }
    let fd = job.socket.as_raw_fd();
    jobs.insert(
        fd,
        SendfileState {
            socket: job.socket,
            file: job.file,
            offset: job.offset as libc::off_t,
            sent: 0,
            len: job.len,
            max_chunk_bytes: job.max_chunk_bytes,
            completion: Some(job.completion),
            ready: false,
            ready_flags: 0,
            ready_prev: None,
            ready_next: None,
        },
    );
    if immediate {
        if let DriveResult::Complete(result) = drive_job(jobs, fd) {
            finish_unregistered_job(jobs, fd, result);
            return;
        }
    }

    let mut event = libc::epoll_event {
        events: (libc::EPOLLOUT | libc::EPOLLERR | libc::EPOLLHUP) as u32,
        u64: fd as u64,
    };
    if unsafe { libc::epoll_ctl(epoll_fd, libc::EPOLL_CTL_ADD, fd, &mut event) } != 0 {
        finish_unregistered_job(jobs, fd, Err(io::Error::last_os_error()));
    } else if !immediate {
        jobs.enqueue_ready(fd, libc::EPOLLOUT);
    }
}

fn finish_unregistered_job(jobs: &mut SendfileTable, fd: RawFd, result: io::Result<u64>) {
    if let Some(mut state) = jobs.remove(fd) {
        let result = finish_sendfile_result(&state.socket, result);
        if let Some(completion) = state.completion.take() {
            let _ = completion.send(result);
        }
    }
}

fn drive_job(jobs: &mut SendfileTable, fd: RawFd) -> DriveResult {
    let Some(state) = jobs.get_mut(fd) else {
        return DriveResult::Complete(Err(io::Error::new(
            io::ErrorKind::NotFound,
            "sendfile reactor job disappeared",
        )));
    };
    let event_budget = (state.len - state.sent).min(state.max_chunk_bytes);
    let event_start = state.sent;
    while state.sent - event_start < event_budget {
        if state
            .completion
            .as_ref()
            .is_some_and(oneshot::Sender::is_closed)
        {
            return DriveResult::Complete(Err(cancelled_download()));
        }
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

fn cancelled_download() -> io::Error {
    io::Error::new(io::ErrorKind::Interrupted, "sendfile response cancelled")
}

fn cancel_abandoned_jobs(epoll_fd: RawFd, jobs: &mut SendfileTable, worker: &ReactorWorker) {
    for _ in 0..REGISTRATION_BATCH {
        let Some(fd) = worker.cancellations.pop() else {
            break;
        };
        cancel_abandoned_job(epoll_fd, jobs, fd);
    }
    if !worker.cancellations.is_empty() {
        wake(worker.wake_fd);
    }
    // A cancellation burst cannot grow the queue indefinitely. Only overflow
    // triggers this scan; normal completions and cancellation use indexed slots.
    if worker.sweep_cancelled.swap(false, Ordering::AcqRel) {
        for index in 0..jobs.slots.len() {
            cancel_abandoned_job(epoll_fd, jobs, index as RawFd);
        }
    }
}

fn cancel_abandoned_job(epoll_fd: RawFd, jobs: &mut SendfileTable, fd: RawFd) {
    if jobs
        .get_mut(fd)
        .and_then(|state| state.completion.as_ref())
        .is_some_and(oneshot::Sender::is_closed)
    {
        complete_job(epoll_fd, jobs, fd, Err(cancelled_download()));
    }
}

fn complete_job(epoll_fd: RawFd, jobs: &mut SendfileTable, fd: RawFd, result: io::Result<u64>) {
    unsafe {
        libc::epoll_ctl(epoll_fd, libc::EPOLL_CTL_DEL, fd, std::ptr::null_mut());
    }
    if let Some(mut state) = jobs.remove(fd) {
        let result = finish_sendfile_result(&state.socket, result);
        if let Some(completion) = state.completion.take() {
            let _ = completion.send(result);
        }
    }
}

fn fail_all_jobs(epoll_fd: RawFd, jobs: &mut SendfileTable, message: &str) {
    let fds = jobs.active_fds();
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

fn pin_current_thread(cpu: usize) {
    if let Err(error) = crate::linux_cpu::set_current_thread_affinity(&[cpu]) {
        tracing::warn!(
            cpu,
            ?error,
            "sendfile owner CPU affinity unavailable; retaining inherited mask"
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

fn finish_sendfile_result(
    socket: &TcpStream,
    result: io::Result<u64>,
) -> io::Result<SendfileCompletion> {
    result.map(|bytes| SendfileCompletion {
        bytes,
        uncorked: uncork_sendfile_socket(socket),
    })
}

fn uncork_sendfile_socket(socket: &TcpStream) -> bool {
    let disabled: libc::c_int = 0;
    unsafe {
        libc::setsockopt(
            socket.as_raw_fd(),
            libc::IPPROTO_TCP,
            libc::TCP_CORK,
            (&disabled as *const libc::c_int).cast(),
            mem::size_of_val(&disabled) as libc::socklen_t,
        ) == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::os::fd::OwnedFd;
    use std::time::Duration;

    #[test]
    fn shared_download_turns_rotate_and_cancellation_removes_ready_nodes() {
        let wake_fd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
        assert!(wake_fd >= 0);
        let _wake = unsafe { OwnedFd::from_raw_fd(wake_fd) };
        let epoll_fd = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
        assert!(epoll_fd >= 0);
        let _epoll = unsafe { OwnedFd::from_raw_fd(epoll_fd) };
        let worker = ReactorWorker::shared(wake_fd);
        let mut downloads = SharedDownloads::default();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let name = CString::new("proxysss-shared-ready-test").unwrap();
        let fd = unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC) };
        assert!(fd >= 0);
        let file = unsafe { File::from_raw_fd(fd) };
        file.set_len(4 * SHARED_SEND_BYTES).unwrap();
        let mut clients = Vec::new();
        let mut pending = Vec::new();
        let mut fds = Vec::new();
        for _ in 0..3 {
            let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
            let (server, _) = listener.accept().unwrap();
            let future = dispatch_to_worker(
                &worker,
                server.as_raw_fd(),
                file.as_raw_fd(),
                0,
                4 * SHARED_SEND_BYTES,
                4 * SHARED_SEND_BYTES,
            )
            .unwrap();
            fds.push(future.cancellation.as_ref().unwrap().1);
            pending.push(future);
            clients.push(client);
        }
        // Old HUP entries must be consumed before registering new jobs.
        downloads.observe(&[libc::epoll_event {
            events: libc::EPOLLHUP as u32,
            u64: fds[0] as u64,
        }]);
        downloads.on_wake(epoll_fd, &worker);
        for _ in 0..8 {
            downloads.observe(
                &fds.iter()
                    .map(|&fd| libc::epoll_event {
                        events: libc::EPOLLOUT as u32,
                        u64: fd as u64,
                    })
                    .collect::<Vec<_>>(),
            );
        }
        downloads.drive_one(epoll_fd);
        let first = downloads.jobs.get_mut(fds[0]).unwrap().sent;
        assert!(first > 0 && first <= SHARED_SEND_BYTES);
        assert_eq!(downloads.jobs.get_mut(fds[1]).unwrap().sent, 0);
        downloads.drive_one(epoll_fd);
        assert!(downloads.jobs.get_mut(fds[1]).unwrap().sent > 0);
        assert_eq!(downloads.jobs.get_mut(fds[0]).unwrap().sent, first);
        // Remove middle/head/tail nodes while readiness is duplicated/queued.
        drop(pending.remove(1));
        downloads.on_wake(epoll_fd, &worker);
        assert!(!downloads.jobs.contains(fds[1]));
        let mut queued = Vec::new();
        while let Some((fd, _)) = downloads.jobs.pop_ready() {
            queued.push(fd);
        }
        let unique: std::collections::HashSet<_> = queued.iter().copied().collect();
        assert_eq!(unique.len(), queued.len());
        assert!(!unique.contains(&fds[1]));
        for fd in [fds[0], fds[2]] {
            downloads.jobs.enqueue_ready(fd, libc::EPOLLOUT);
        }
        let mut state = downloads.jobs.remove(fds[0]).unwrap();
        state.ready_flags = 0;
        downloads.jobs.insert(fds[0], state);
        downloads.jobs.enqueue_ready(fds[0], libc::EPOLLOUT);
        assert_eq!(downloads.jobs.pop_ready().unwrap().0, fds[2]);
        assert_eq!(downloads.jobs.pop_ready().unwrap().0, fds[0]);
        assert!(downloads.jobs.pop_ready().is_none());
        drop(pending);
        downloads.on_wake(epoll_fd, &worker);
        assert!(downloads.jobs.active_fds().is_empty());
        assert!(!downloads.has_ready());
    }

    #[test]
    fn admission_ignores_cached_hup_for_reused_descriptor_and_bounds_queued_work() {
        let wake_fd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
        assert!(wake_fd >= 0);
        let _wake_owner = unsafe { OwnedFd::from_raw_fd(wake_fd) };
        let epoll_fd = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
        assert!(epoll_fd >= 0);
        let _epoll_owner = unsafe { OwnedFd::from_raw_fd(epoll_fd) };
        let worker = ReactorWorker {
            registrations: ArrayQueue::new(REGISTRATION_BATCH + 1),
            cancellations: ArrayQueue::new(1),
            sweep_cancelled: AtomicBool::new(false),
            wake_fd,
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let (socket, _) = listener.accept().unwrap();
        socket.set_nonblocking(true).unwrap();
        let name = CString::new("proxysss-sendfile-stale-event-test").unwrap();
        let file_fd = unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC) };
        assert!(file_fd >= 0);
        let mut file = unsafe { File::from_raw_fd(file_fd) };
        file.write_all(b"ok").unwrap();
        let fd = socket.as_raw_fd();
        let (completion, mut receiver) = oneshot::channel();
        assert!(worker
            .registrations
            .push(SendfileJob {
                socket,
                file,
                offset: 0,
                len: 2,
                max_chunk_bytes: 1,
                completion
            })
            .is_ok());
        let events = [
            libc::epoll_event {
                events: libc::EPOLLIN as u32,
                u64: WAKE_TOKEN,
            },
            libc::epoll_event {
                events: libc::EPOLLHUP as u32,
                u64: fd as u64,
            },
        ];
        let mut jobs = SendfileTable::default();
        process_event_batch(epoll_fd, &mut jobs, &events, &worker);
        assert!(jobs.contains(fd), "stale HUP failed a new download");
        assert!(matches!(
            receiver.try_recv(),
            Err(oneshot::error::TryRecvError::Empty)
        ));
        process_event_batch(
            epoll_fd,
            &mut jobs,
            &[libc::epoll_event {
                events: libc::EPOLLOUT as u32,
                u64: fd as u64,
            }],
            &worker,
        );
        assert_eq!(receiver.try_recv().unwrap().unwrap().bytes, 2);
        let mut body = [0; 2];
        client.read_exact(&mut body).unwrap();
        assert_eq!(&body, b"ok");
        assert!(jobs.active_fds().is_empty());

        // Abandoned admissions still consume the same bounded processing
        // budget, so cancellation storms cannot hide an unbounded drain loop.
        for _ in 0..=REGISTRATION_BATCH {
            let (completion, receiver) = oneshot::channel();
            drop(receiver);
            let job = SendfileJob {
                socket: client.try_clone().unwrap(),
                file: File::open("/dev/null").unwrap(),
                offset: 0,
                len: 1,
                max_chunk_bytes: 1,
                completion,
            };
            assert!(worker.registrations.push(job).is_ok());
        }
        process_event_batch(epoll_fd, &mut jobs, &events[..1], &worker);
        assert_eq!(worker.registrations.len(), 1);
        let mut counter = 0_u64;
        assert_eq!(
            unsafe { libc::read(wake_fd, (&mut counter as *mut u64).cast(), 8) },
            8
        );
        assert!(counter > 0);
        process_event_batch(epoll_fd, &mut jobs, &events[..1], &worker);
        assert!(worker.registrations.is_empty());
    }

    #[test]
    fn cancelled_queued_download_never_sends_a_body() {
        let name = CString::new("proxysss-sendfile-queued-cancel-test").unwrap();
        let file_fd = unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC) };
        assert!(file_fd >= 0);
        let mut file = unsafe { File::from_raw_fd(file_fd) };
        file.write_all(b"cancelled body").unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let (socket, _) = listener.accept().unwrap();
        let (completion, receiver) = oneshot::channel();
        drop(receiver);
        let mut jobs = SendfileTable::default();
        register_job(
            -1,
            &mut jobs,
            SendfileJob {
                socket,
                file,
                offset: 0,
                len: 14,
                max_chunk_bytes: 14,
                completion,
            },
        );
        assert!(jobs.active_fds().is_empty());
        assert_eq!(client.read(&mut [0; 32]).unwrap(), 0);
    }

    #[test]
    fn cancellation_overflow_releases_abandoned_jobs_and_preserves_live_fd() {
        // A one-slot queue deterministically exercises overflow without needing
        // thousands of sockets or depending on native owner scheduling.
        let wake_fd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
        assert!(wake_fd >= 0);
        let _wake_owner = unsafe { std::os::fd::OwnedFd::from_raw_fd(wake_fd) };
        let worker = Arc::new(ReactorWorker {
            registrations: ArrayQueue::new(1),
            cancellations: ArrayQueue::new(1),
            sweep_cancelled: AtomicBool::new(false),
            wake_fd,
        });
        let epoll_fd = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
        assert!(epoll_fd >= 0);
        let _epoll_owner = unsafe { std::os::fd::OwnedFd::from_raw_fd(epoll_fd) };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut jobs = SendfileTable::default();
        let mut clients = Vec::new();
        let mut pending = Vec::new();
        for _ in 0..3 {
            clients.push(TcpStream::connect(listener.local_addr().unwrap()).unwrap());
            let (socket, _) = listener.accept().unwrap();
            let fd = socket.as_raw_fd();
            let (completion, receiver) = oneshot::channel();
            jobs.insert(
                fd,
                SendfileState {
                    socket,
                    file: File::open("/dev/null").unwrap(),
                    offset: 0,
                    sent: 0,
                    len: 1,
                    max_chunk_bytes: 1,
                    completion: Some(completion),
                    ready: false,
                    ready_flags: 0,
                    ready_prev: None,
                    ready_next: None,
                },
            );
            pending.push(PendingSendfile {
                receiver,
                cancellation: Some((worker.clone(), fd)),
            });
        }
        let live = pending.pop().unwrap();
        let live_fd = live.cancellation.as_ref().unwrap().1;
        drop(pending);
        assert!(worker.sweep_cancelled.load(Ordering::Acquire));
        cancel_abandoned_jobs(epoll_fd, &mut jobs, &worker);
        assert_eq!(jobs.active_fds(), vec![live_fd]);
        // A delayed token with a reused fd must not cancel a new live receiver.
        worker.cancellations.push(live_fd).unwrap();
        cancel_abandoned_jobs(epoll_fd, &mut jobs, &worker);
        assert_eq!(jobs.active_fds(), vec![live_fd]);
        complete_job(epoll_fd, &mut jobs, live_fd, Ok(0));
        let result = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(live)
            .unwrap()
            .unwrap();
        assert_eq!(result.bytes, 0);
        assert!(
            worker.cancellations.is_empty(),
            "normal completion must not enqueue cancellation"
        );
        assert!(jobs.active_fds().is_empty());
    }

    #[test]
    fn cancelled_blocked_download_releases_descriptors_without_client_disconnect() {
        let name = CString::new("proxysss-sendfile-cancel-test").unwrap();
        let file_fd = unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC) };
        assert!(file_fd >= 0);
        let file = unsafe { File::from_raw_fd(file_fd) };
        let len = 512 * 1024 * 1024;
        file.set_len(len).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        let completion =
            dispatch(server.as_raw_fd(), file.as_raw_fd(), 0, len, len, 1, 1, 0).unwrap();
        drop(server);
        drop(file);
        let retained_files = || {
            std::fs::read_dir("/proc/self/fd")
                .unwrap()
                .filter_map(Result::ok)
                .filter_map(|entry| std::fs::read_link(entry.path()).ok())
                .filter(|path| {
                    path.to_string_lossy()
                        .contains("proxysss-sendfile-cancel-test")
                })
                .count()
        };
        thread::sleep(Duration::from_millis(50));
        assert_eq!(
            retained_files(),
            1,
            "blocked reactor must own the source file"
        );
        drop(completion);
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while retained_files() != 0 && std::time::Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
        let retained = retained_files();
        // Keep the peer alive and unread until after the cancellation observation.
        drop(client);
        assert_eq!(
            retained, 0,
            "cancelled sendfile retained its file descriptor"
        );
    }

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
        let completion = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(completion)
            .unwrap()
            .unwrap();
        assert_eq!(completion.bytes, (64 * 1024 - start) as u64);
        assert!(completion.uncorked);
        let (received, expected) = reader.join().unwrap();
        assert_eq!(received, expected);
    }

    #[test]
    fn sendfile_handoff_prefers_current_cpu_owner() {
        let worker_cpus = vec![Some(2), Some(4), Some(6), Some(8)];
        assert_eq!(preferred_worker_index(&worker_cpus, Some(6), 0), 2);
        assert_eq!(preferred_worker_index(&worker_cpus, Some(7), 3), 3);
        assert_eq!(preferred_worker_index(&worker_cpus, None, 5), 1);
        assert_eq!(
            preferred_worker_index_limited(&worker_cpus, Some(6), 0, 2),
            2
        );
        assert_eq!(
            preferred_worker_index_limited(&worker_cpus, Some(4), 1, 2),
            2
        );
        assert_eq!(preferred_worker_index_limited(&worker_cpus, None, 1, 2), 2);
    }

    #[test]
    fn sendfile_table_indexes_jobs_by_fd_without_hashing() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        let file = File::open("/dev/null").unwrap();
        let fd = server.as_raw_fd();
        let (completion, _receiver) = oneshot::channel();
        let mut jobs = SendfileTable::default();
        jobs.insert(
            fd,
            SendfileState {
                socket: server,
                file,
                offset: 0,
                sent: 0,
                len: 1,
                max_chunk_bytes: 1,
                completion: Some(completion),
                ready: false,
                ready_flags: 0,
                ready_prev: None,
                ready_next: None,
            },
        );
        assert!(jobs.contains(fd));
        assert_eq!(jobs.active_fds(), vec![fd]);
        assert!(jobs.get_mut(fd).is_some());
        assert!(jobs.remove(fd).is_some());
        assert!(!jobs.contains(fd));
        drop(client);
    }
}
