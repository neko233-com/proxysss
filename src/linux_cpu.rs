//! Startup CPU resources shared by every Linux data-plane owner.
//!
//! The process leader retains launch affinity, including taskset and cpuset.
//! Never expand it to the enclosing cgroup or discover CPUs from a pinned
//! worker. Cache quota-aware parallelism before creating data-plane threads.

use std::io;
use std::mem;
use std::sync::OnceLock;

#[derive(Debug, serde::Serialize)]
pub(crate) struct CpuResources {
    pub(crate) allowed_cpu_ids: Vec<usize>,
    pub(crate) data_plane_workers: usize,
}

static RESOURCES: OnceLock<CpuResources> = OnceLock::new();
// Discovery allocation bound, not a worker cap. Unlike cpu_set_t this supports
// CPU IDs above 1023, without allocating unbounded memory on a bad input.
const MAX_MASK_BYTES: usize = 128 * 1024;
const WORD_BITS: usize = libc::c_ulong::BITS as usize;

pub(crate) fn resources() -> &'static CpuResources {
    RESOURCES.get_or_init(|| {
        let allowed_cpu_ids = affinity_for(unsafe { libc::getpid() })
            .or_else(|_| affinity_for(0))
            .unwrap_or_else(|_| {
                let cpu = unsafe { libc::sched_getcpu() };
                usize::try_from(cpu).ok().into_iter().collect()
            });
        // main initializes this before any data worker is pinned. Rust also
        // accounts for accessible cgroup CPU quotas, including parent limits.
        let parallelism = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(allowed_cpu_ids.len());
        CpuResources {
            data_plane_workers: parallelism.max(1).min(allowed_cpu_ids.len().max(1)),
            allowed_cpu_ids,
        }
    })
}

pub(crate) fn allowed_cpu_ids() -> &'static [usize] {
    &resources().allowed_cpu_ids
}

pub(crate) fn data_plane_workers() -> usize {
    resources().data_plane_workers
}

fn affinity_for(pid: libc::pid_t) -> io::Result<Vec<usize>> {
    let mut words = vec![
        0 as libc::c_ulong;
        mem::size_of::<libc::cpu_set_t>() / mem::size_of::<libc::c_ulong>()
    ];
    loop {
        let bytes = mem::size_of_val(words.as_slice());
        let result = unsafe { libc::sched_getaffinity(pid, bytes, words.as_mut_ptr().cast()) };
        if result == 0 {
            let mut cpus = Vec::new();
            for (word_index, word) in words.iter().copied().enumerate() {
                for bit in 0..WORD_BITS {
                    if word & (1 << bit) != 0 {
                        cpus.push(word_index * WORD_BITS + bit);
                    }
                }
            }
            return if cpus.is_empty() {
                Err(io::Error::other("Linux returned an empty CPU affinity"))
            } else {
                Ok(cpus)
            };
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::EINVAL) || bytes >= MAX_MASK_BYTES {
            return Err(error);
        }
        words.resize(words.len() * 2, 0);
    }
}

fn affinity_mask(cpus: &[usize]) -> io::Result<Vec<libc::c_ulong>> {
    let last = cpus
        .iter()
        .copied()
        .max()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "empty CPU affinity"))?;
    if last >= MAX_MASK_BYTES * 8 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "CPU affinity mask is too large",
        ));
    }
    let mut words = vec![0; last / WORD_BITS + 1];
    for &cpu in cpus {
        words[cpu / WORD_BITS] |= 1 << (cpu % WORD_BITS);
    }
    Ok(words)
}

pub(crate) fn set_current_thread_affinity(cpus: &[usize]) -> io::Result<()> {
    let words = affinity_mask(cpus)?;
    if unsafe {
        libc::sched_setaffinity(0, mem::size_of_val(words.as_slice()), words.as_ptr().cast())
    } == 0
    {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    #[test]
    fn affinity_masks_support_sparse_cpu_ids_above_cpu_set_t() {
        let mask = affinity_mask(&[1, 63, 1024, 8191]).unwrap();
        let cpus: Vec<_> = mask
            .iter()
            .enumerate()
            .flat_map(|(index, &word)| {
                (0..WORD_BITS).filter_map(move |bit| {
                    (word & (1 << bit) != 0).then_some(index * WORD_BITS + bit)
                })
            })
            .collect();
        assert_eq!(cpus, [1, 63, 1024, 8191]);
        assert!(affinity_mask(&[]).is_err());
        assert!(affinity_mask(&[usize::MAX]).is_err());
    }

    #[test]
    fn launch_affinity_survives_later_worker_pinning() {
        const CHILD: &str = "PROXYSSS_CPU_AFFINITY_TEST_CHILD";
        if std::env::var_os(CHILD).is_some() {
            let launch = affinity_for(unsafe { libc::getpid() }).unwrap();
            let cpus = resources();
            assert_eq!(cpus.allowed_cpu_ids, launch);
            let expected_workers = std::thread::available_parallelism()
                .unwrap()
                .get()
                .min(launch.len());
            assert_eq!(cpus.data_plane_workers, expected_workers);
            set_current_thread_affinity(&launch[..1]).unwrap();
            assert_eq!(affinity_for(0).unwrap(), launch[..1]);
            std::thread::spawn(move || {
                assert_eq!(resources().allowed_cpu_ids, launch);
                assert_eq!(data_plane_workers(), expected_workers);
                set_current_thread_affinity(allowed_cpu_ids()).unwrap();
                assert_eq!(affinity_for(0).unwrap(), launch);
            })
            .join()
            .unwrap();
            return;
        }
        let allowed = affinity_for(unsafe { libc::getpid() }).unwrap();
        // Restrict the exec'ed process while its enclosing cgroup stays wide.
        let launch: Vec<_> = allowed.iter().step_by(2).take(2).copied().collect();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "linux_cpu::tests::launch_affinity_survives_later_worker_pinning",
                "--nocapture",
            ])
            .env(CHILD, "1");
        // Precompute the mask: pre_exec must not allocate or take locks.
        let mask = affinity_mask(&launch).unwrap();
        unsafe {
            command.pre_exec(move || {
                if libc::sched_setaffinity(
                    0,
                    mem::size_of_val(mask.as_slice()),
                    mask.as_ptr().cast(),
                ) == 0
                {
                    Ok(())
                } else {
                    Err(io::Error::last_os_error())
                }
            });
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "child failed: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
