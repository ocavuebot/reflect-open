use std::{fs, sync::mpsc::{self, Receiver}, thread, time::Instant};
fn drain(rx: &Receiver<DebounceEventResult>) -> Vec<PathBuf> {
    let deadline = Instant::now() + Duration::from_millis(1300);
    let mut paths = vec![];
    while Instant::now() < deadline {
        if let Ok(batch) = rx.recv_timeout(Duration::from_millis(100)) {
            for event in batch.unwrap() { paths.extend(event.paths.clone()); }
        }
    }
    paths
}
fn setup() -> (tempfile::TempDir, GraphDebouncer, Receiver<DebounceEventResult>) {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("notes/sub")).unwrap();
    fs::create_dir_all(dir.path().join(".git/objects/aa")).unwrap();
    fs::write(dir.path().join("notes/a.md"), "a").unwrap();
    let (tx, rx) = mpsc::channel();
    let watcher = watch_graph(dir.path(), tx).unwrap();
    (dir, watcher, rx)
}
#[test]
fn normal_mutations_and_recursive_moves() {
    let (dir, _watcher, rx) = setup(); let root = dir.path();
    for (label, path) in [("overwrite", "notes/a.md"), ("create", "notes/b.md")] {
        fs::write(root.join(path), label).unwrap();
        assert!(drain(&rx).contains(&root.join(path)), "{label}");
    }
    fs::write(root.join(".tmp"), "atomic").unwrap();
    fs::rename(root.join(".tmp"), root.join("notes/a.md")).unwrap();
    assert!(drain(&rx).contains(&root.join("notes/a.md")), "atomic save");
    fs::rename(root.join("notes"), root.join("renamed")).unwrap();
    drain(&rx);
    fs::write(root.join("renamed/sub/deep.md"), "deep").unwrap();
    assert!(drain(&rx).contains(&root.join("renamed/sub/deep.md")), "write after directory rename");
    let outside = tempfile::tempdir().unwrap();
    fs::create_dir_all(outside.path().join("incoming/sub")).unwrap();
    fs::rename(outside.path().join("incoming"), root.join("incoming")).unwrap();
    drain(&rx);
    fs::write(root.join("incoming/sub/new.md"), "new").unwrap();
    assert!(drain(&rx).contains(&root.join("incoming/sub/new.md")), "write after directory move in");
    fs::remove_file(root.join("renamed/a.md")).unwrap();
    assert!(drain(&rx).contains(&root.join("renamed/a.md")), "delete");
}
fn resource_counts() -> (usize, usize, usize) {
    let fds = fs::read_dir("/proc/self/fd").unwrap().filter_map(Result::ok)
        .filter(|e| fs::read_link(e.path()).is_ok_and(|p| p.to_string_lossy().contains("inotify"))).count();
    let threads = fs::read_dir("/proc/self/task").unwrap().count();
    let status = fs::read_to_string("/proc/self/status").unwrap();
    let rss = status.lines().find(|s| s.starts_with("VmRSS:")).unwrap().split_whitespace().nth(1).unwrap().parse().unwrap();
    (fds, threads, rss)
}
#[test]
fn hidden_churn_and_repeated_reuse_stay_bounded() {
    let (dir, watcher, rx) = setup(); let root = dir.path();
    let id = get_file_id(root).unwrap();
    let mut active = Some((root.to_path_buf(), id, watcher));
    let (tx, _unused_rx) = mpsc::channel();
    let before = resource_counts();
    for iteration in 0..1000 {
        start(root.to_path_buf(), &mut active, tx.clone()).unwrap();
        fs::write(root.join(".git/objects/aa/object"), format!("{iteration}")).unwrap();
        for entry in WalkDir::new(root) {
            let entry = entry.unwrap(); if entry.file_type().is_file() { fs::read(entry.path()).unwrap(); }
        }
        thread::sleep(Duration::from_millis(10));
    }
    drain(&rx);
    let settled = resource_counts();
    assert!(rx.recv_timeout(Duration::from_secs(2)).is_err(), "hidden churn does not settle");
    let after = resource_counts();
    println!("resources before={before:?}, settled={settled:?}, after={after:?} (inotify fds, threads, RSS KiB)");
    assert_eq!(before.0, after.0); assert_eq!(before.1, after.1);
    assert!(after.2 < before.2 + 20_000);
    drop(active); thread::sleep(Duration::from_secs(1));
    assert_eq!(resource_counts().0, before.0 - 1);
}
#[test]
fn replacement_root_is_rewatched() {
    let parent = tempfile::tempdir().unwrap(); let root = parent.path().join("graph");
    fs::create_dir(&root).unwrap();
    let (tx, rx) = mpsc::channel();
    let id = get_file_id(&root).unwrap();
    let mut watcher = watch_graph(&root, tx.clone()).unwrap();
    fs::rename(&root, parent.path().join("old")).unwrap(); fs::create_dir(&root).unwrap();
    assert_ne!(id, get_file_id(&root).unwrap());
    drop(watcher); watcher = watch_graph(&root, tx).unwrap(); drain(&rx);
    fs::write(root.join("new.md"), "new").unwrap();
    assert!(drain(&rx).contains(&root.join("new.md")));
    drop(watcher);
}
#[test]
fn moved_root_same_id_probe() {
    let parent = tempfile::tempdir().unwrap(); let root = parent.path().join("graph");
    let renamed = parent.path().join("renamed"); fs::create_dir(&root).unwrap();
    let (tx, rx) = mpsc::channel(); let mut active = None;
    start(root.clone(), &mut active, tx.clone()).unwrap();
    let id = get_file_id(&root).unwrap();
    fs::rename(&root, &renamed).unwrap(); drain(&rx);
    assert_eq!(id, get_file_id(&renamed).unwrap());
    start(renamed.clone(), &mut active, tx).unwrap();
    fs::write(renamed.join("new.md"), "new").unwrap();
    let paths = drain(&rx);
    println!("moved graph retains ID; events={paths:?}; expected={:?}", renamed.join("new.md"));
    let effects = collect_changes(&paths, &active.as_ref().unwrap().0);
    println!("production collect_changes emitted {effects:?}");
    assert_eq!(effects.changes.len(), 1);
    assert_eq!(effects.changes[0].kind, ChangeKind::Upsert, "existing file was reported as deleted after graph move");
    drop(active);
}

#[test]
fn moved_root_reinstall_control() {
    let parent = tempfile::tempdir().unwrap(); let root = parent.path().join("graph");
    let renamed = parent.path().join("renamed"); fs::create_dir(&root).unwrap();
    let (tx, rx) = mpsc::channel(); let watcher = watch_graph(&root, tx.clone()).unwrap();
    fs::rename(&root, &renamed).unwrap(); drain(&rx);
    drop(watcher);
    let _watcher = watch_graph(&renamed, tx).unwrap();
    fs::write(renamed.join("new.md"), "new").unwrap();
    let effects = collect_changes(&drain(&rx), &renamed);
    assert_eq!(effects.changes.len(), 1);
    assert_eq!(effects.changes[0].kind, ChangeKind::Upsert);
}

#[test]
fn switch_graph_and_failed_start() {
    let parent = tempfile::tempdir().unwrap();
    let first = parent.path().join("first"); let second = parent.path().join("second");
    fs::create_dir(&first).unwrap(); fs::create_dir(&second).unwrap();
    let (tx, rx) = mpsc::channel(); let mut active = None;
    start(first.clone(), &mut active, tx.clone()).unwrap();
    start(second.clone(), &mut active, tx.clone()).unwrap(); drain(&rx);
    fs::write(first.join("old.md"), "old").unwrap();
    fs::write(second.join("new.md"), "new").unwrap();
    let paths = drain(&rx);
    assert!(paths.contains(&second.join("new.md")));
    assert!(!paths.contains(&first.join("old.md")));
    assert!(start(parent.path().join("missing"), &mut active, tx).is_err());
    assert!(active.is_none());
}
