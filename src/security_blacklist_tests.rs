use super::*;
use crate::verify::harness;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

fn fixture(name: &str) -> (std::path::PathBuf, DynamicBlacklistConfig) {
    let root = harness::temp_root(name);
    let config = DynamicBlacklistConfig {
        enabled: true,
        path: root.join("state/blacklist.json"),
    };
    (root, config)
}

#[test]
fn blacklist_restart_preserves_original_clock_and_fractional_lifetime() {
    let (root, config) = fixture("blacklist-clock");
    let blacklist = DynamicBlacklist::default();
    let ip: IpAddr = "2001:db8::41".parse().unwrap();
    let maximum: IpAddr = "203.0.113.42".parse().unwrap();
    let expired: IpAddr = "203.0.113.43".parse().unwrap();
    let mut entry = BlacklistEntry::new(60);
    entry.created_at -= Duration::from_millis(10_750);
    entry.expiry.started_at -= Duration::from_millis(10_750);
    blacklist.entries.insert(ip, entry);
    blacklist.add(maximum, u64::MAX);
    let mut old = BlacklistEntry::new(1);
    old.created_at -= Duration::from_secs(2);
    old.expiry.started_at -= Duration::from_secs(2);
    let bytes = serde_json::to_vec(&BlacklistSnapshot {
        version: 1,
        entries: vec![
            entry.stored(ip),
            blacklist.entries.get(&maximum).unwrap().stored(maximum),
            old.stored(expired),
        ],
    })
    .unwrap();
    write_blacklist_snapshot(&config.path, &bytes).unwrap();
    let loaded = DynamicBlacklist::load_from_disk(&config).unwrap();
    assert_eq!(loaded.tracked_entries(), 2);
    assert!(!loaded.is_blocked(expired));
    let restored = *loaded.entries.get(&ip).unwrap();
    assert!(restored.expiry.valid_for <= Duration::from_millis(49_250));
    assert!(restored.expiry.valid_for > Duration::from_secs(40));
    assert_eq!(restored.created_at, entry.created_at);
    assert_eq!(restored.ttl_secs, 60);
    assert!(loaded.is_blocked(maximum));
    assert_eq!(loaded.entries.get(&maximum).unwrap().ttl_secs, u64::MAX);

    // A subsequent snapshot must preserve the original timestamp rather than
    // adding the rounded remaining seconds to a fresh wall-clock origin.
    loaded
        .apply_admin_sync(
            &config,
            &DdosGuard::default(),
            BlacklistMutation::Remove { ip: expired },
        )
        .unwrap();
    let again = DynamicBlacklist::load_from_disk(&config).unwrap();
    let second = *again.entries.get(&ip).unwrap();
    assert_eq!(second.created_at, entry.created_at);
    assert_eq!(second.ttl_secs, 60);
    assert!(second.expiry.valid_for <= restored.expiry.valid_for);
    assert_eq!(again.tracked_entries(), 2);
    harness::cleanup(&root);
}

#[test]
fn blacklist_corrupt_or_unreadable_state_cannot_silently_clear_bans() {
    let (root, config) = fixture("blacklist-invalid");
    assert_eq!(
        DynamicBlacklist::load_from_disk(&config)
            .unwrap()
            .tracked_entries(),
        0
    );
    std::fs::create_dir_all(config.path.parent().unwrap()).unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let valid = serde_json::json!({"ip":"2001:db8::1","created_at_unix_secs":now,"ttl_secs":30});
    let mut zero = valid.clone();
    zero["ttl_secs"] = 0.into();
    let mut nanos = valid.clone();
    nanos["created_at_nanos"] = 1_000_000_000_u64.into();
    let mut invalid_ip = valid.clone();
    invalid_ip["ip"] = "invalid".into();
    let mut duplicate = valid.clone();
    duplicate["ip"] = "2001:0db8:0:0:0:0:0:1".into();
    let invalid = vec![
        "{truncated".to_string(),
        "[\"203.0.113.1\"]".to_string(),
        serde_json::json!({"version":2,"entries":[]}).to_string(),
        serde_json::json!({"version":1,"entries":[valid.clone()],"unknown":true}).to_string(),
        serde_json::json!({"version":1,"entries":[zero]}).to_string(),
        serde_json::json!({"version":1,"entries":[nanos]}).to_string(),
        serde_json::json!({"version":1,"entries":[invalid_ip]}).to_string(),
        serde_json::json!({"version":1,"entries":[valid,duplicate]}).to_string(),
    ];
    for contents in invalid {
        std::fs::write(&config.path, &contents).unwrap();
        assert!(
            DynamicBlacklist::load_from_disk(&config).is_err(),
            "accepted {contents}"
        );
        assert_eq!(std::fs::read_to_string(&config.path).unwrap(), contents);
    }
    let disabled = DynamicBlacklistConfig {
        enabled: false,
        path: config.path.clone(),
    };
    assert_eq!(
        DynamicBlacklist::load_from_disk(&disabled)
            .unwrap()
            .tracked_entries(),
        0
    );
    std::fs::remove_file(&config.path).unwrap();
    std::fs::create_dir(&config.path).unwrap();
    assert!(DynamicBlacklist::load_from_disk(&config).is_err());
    #[cfg(unix)]
    {
        std::fs::remove_dir(&config.path).unwrap();
        std::os::unix::fs::symlink(root.join("missing-state.json"), &config.path).unwrap();
        assert!(
            DynamicBlacklist::load_from_disk(&config).is_err(),
            "a dangling snapshot link is not a missing file"
        );
    }
    harness::cleanup(&root);
}

#[test]
fn concurrent_admin_snapshots_are_complete_and_keep_each_final_update() {
    let (root, config) = fixture("blacklist-concurrent");
    let blacklist = DynamicBlacklist::default();
    let ddos = DdosGuard::default();
    let automatic = "203.0.113.99".parse().unwrap();
    blacklist.add(automatic, 300);
    blacklist
        .apply_admin_sync(
            &config,
            &ddos,
            BlacklistMutation::Remove {
                ip: "192.0.2.1".parse().unwrap(),
            },
        )
        .unwrap();
    let reading = Arc::new(AtomicBool::new(true));
    struct StopReader(Arc<AtomicBool>);
    impl Drop for StopReader {
        fn drop(&mut self) {
            self.0.store(false, Ordering::Release);
        }
    }
    let _stop_on_error = StopReader(reading.clone());
    let reader = {
        let reading = reading.clone();
        let path = config.path.clone();
        std::thread::spawn(move || {
            let mut snapshots = 0;
            while reading.load(Ordering::Acquire) {
                let data = std::fs::read(&path).unwrap();
                let snapshot: BlacklistSnapshot = serde_json::from_slice(&data)
                    .expect("readers must never see a truncated snapshot");
                assert_eq!(snapshot.version, 1);
                snapshots += 1;
                std::thread::yield_now();
            }
            snapshots
        })
    };
    std::thread::scope(|scope| {
        for index in 1..=8 {
            let blacklist = &blacklist;
            let config = &config;
            let ddos = &ddos;
            scope.spawn(move || {
                let ip = IpAddr::V4(std::net::Ipv4Addr::new(192, 0, 2, index));
                for _ in 0..4 {
                    blacklist
                        .apply_admin_sync(
                            config,
                            ddos,
                            BlacklistMutation::Add { ip, ban_secs: 300 },
                        )
                        .unwrap();
                    blacklist
                        .apply_admin_sync(config, ddos, BlacklistMutation::Remove { ip })
                        .unwrap();
                }
                blacklist
                    .apply_admin_sync(
                        config,
                        ddos,
                        BlacklistMutation::Add {
                            ip,
                            ban_secs: u64::MAX,
                        },
                    )
                    .unwrap();
            });
        }
    });
    reading.store(false, Ordering::Release);
    assert!(reader.join().unwrap() > 0);
    let restored = DynamicBlacklist::load_from_disk(&config).unwrap();
    assert_eq!(restored.tracked_entries(), 9);
    assert!(restored.is_blocked(automatic));
    for index in 1..=8 {
        let ip = IpAddr::V4(std::net::Ipv4Addr::new(192, 0, 2, index));
        assert!(blacklist.is_blocked(ip) && restored.is_blocked(ip));
        assert!(ddos.windows.contains_key(&ip));
    }
    assert_eq!(
        std::fs::read_dir(config.path.parent().unwrap())
            .unwrap()
            .count(),
        1,
        "temporary files must be retired"
    );
    harness::cleanup(&root);
}

#[tokio::test]
async fn cancelled_caller_keeps_accepted_writer_owned_and_hot_checks_independent() {
    let (root, config) = fixture("blacklist-cancel");
    let blacklist = DynamicBlacklist::default();
    let ddos = DdosGuard::default();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let writer = blacklist.writer.clone();
    let holder = std::thread::spawn(move || {
        let _lock = writer.lock().unwrap();
        ready_tx.send(()).unwrap();
        release_rx.recv().unwrap();
    });
    ready_rx.recv().unwrap();
    let ip = "203.0.113.70".parse().unwrap();
    let caller = {
        let blacklist = blacklist.clone();
        let ddos = ddos.clone();
        let config = config.clone();
        tokio::spawn(async move {
            blacklist
                .apply_admin(config, ddos, BlacklistMutation::Add { ip, ban_secs: 300 })
                .await
        })
    };
    tokio::time::timeout(Duration::from_secs(2), async {
        while blacklist.worker_permit.available_permits() != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    assert_eq!(
        blacklist.worker_permit.available_permits(),
        0,
        "worker still owns the permit after caller cancellation"
    );

    let (lookup_tx, lookup_rx) = std::sync::mpsc::channel();
    let lookup = {
        let blacklist = blacklist.clone();
        std::thread::spawn(move || {
            let automatic = "203.0.113.71".parse().unwrap();
            blacklist.add(automatic, 300);
            lookup_tx.send(blacklist.is_blocked(automatic)).unwrap();
        })
    };
    let immediate = lookup_rx.recv_timeout(Duration::from_secs(1));
    release_tx.send(()).unwrap();
    holder.join().unwrap();
    lookup.join().unwrap();
    assert!(
        immediate.unwrap(),
        "ordinary checks/automatic bans cannot wait for the admin writer lock"
    );
    tokio::time::timeout(Duration::from_secs(3), async {
        while blacklist.worker_permit.available_permits() == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(blacklist.is_blocked(ip));
    assert!(ddos.windows.contains_key(&ip));
    assert!(DynamicBlacklist::load_from_disk(&config)
        .unwrap()
        .is_blocked(ip));
    harness::cleanup(&root);
}
