//! Linux data-plane CPU discovery.
//!
//! Native reactors may initialize lazily from an already pinned Tokio worker.
//! Querying that thread's affinity would collapse every new owner onto one
//! CPU, so prefer the process cgroup's effective cpuset.

use std::mem;
use std::path::PathBuf;
use std::sync::OnceLock;

static ALLOWED_CPU_IDS: OnceLock<Vec<usize>> = OnceLock::new();

pub(crate) fn allowed_cpu_ids() -> &'static [usize] {
    ALLOWED_CPU_IDS.get_or_init(discover_allowed_cpu_ids)
}

fn discover_allowed_cpu_ids() -> Vec<usize> {
    for path in cgroup_cpuset_paths() {
        if let Ok(value) = std::fs::read_to_string(path) {
            let cpus = parse_cpu_list(&value);
            if !cpus.is_empty() {
                return cpus;
            }
        }
    }

    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        if let Some(value) = status
            .lines()
            .find_map(|line| line.strip_prefix("Cpus_allowed_list:").map(str::trim))
        {
            let cpus = parse_cpu_list(value);
            if !cpus.is_empty() {
                return cpus;
            }
        }
    }

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
    if cpus.is_empty() {
        cpus.push(0);
    }
    cpus
}

fn cgroup_cpuset_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(cgroups) = std::fs::read_to_string("/proc/self/cgroup") {
        for line in cgroups.lines() {
            let mut fields = line.splitn(3, ':');
            let _hierarchy = fields.next();
            let Some(controllers) = fields.next() else {
                continue;
            };
            let Some(relative) = fields.next() else {
                continue;
            };
            let relative = relative.trim_start_matches('/');
            if controllers.is_empty() {
                let root = PathBuf::from("/sys/fs/cgroup").join(relative);
                paths.push(root.join("cpuset.cpus.effective"));
                paths.push(root.join("cpuset.cpus"));
            } else if controllers.split(',').any(|name| name == "cpuset") {
                let root = PathBuf::from("/sys/fs/cgroup/cpuset").join(relative);
                paths.push(root.join("cpuset.effective_cpus"));
                paths.push(root.join("cpuset.cpus"));
            }
        }
    }
    paths.extend([
        PathBuf::from("/sys/fs/cgroup/cpuset.cpus.effective"),
        PathBuf::from("/sys/fs/cgroup/cpuset/cpuset.effective_cpus"),
        PathBuf::from("/sys/fs/cgroup/cpuset/cpuset.cpus"),
    ]);
    paths
}

fn parse_cpu_list(value: &str) -> Vec<usize> {
    let mut cpus: Vec<usize> = Vec::new();
    for segment in value
        .trim()
        .split(',')
        .filter(|segment| !segment.is_empty())
    {
        let mut bounds = segment.splitn(2, '-');
        let Some(start) = bounds
            .next()
            .and_then(|part| part.trim().parse::<usize>().ok())
        else {
            return Vec::new();
        };
        let end = match bounds.next() {
            Some(part) => match part.trim().parse::<usize>() {
                Ok(end) => end,
                Err(_) => return Vec::new(),
            },
            None => start,
        };
        if end < start || end >= libc::CPU_SETSIZE as usize {
            return Vec::new();
        }
        cpus.extend(start..=end);
    }
    cpus.sort_unstable();
    cpus.dedup();
    cpus
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sparse_effective_cpuset() {
        assert_eq!(parse_cpu_list("0-2,4,7-8\n"), vec![0, 1, 2, 4, 7, 8]);
        assert!(parse_cpu_list("4-2").is_empty());
        assert!(parse_cpu_list("bad").is_empty());
    }
}
