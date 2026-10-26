from pathlib import Path
s = Path('source/apps/desktop/src-tauri/src/watcher.rs').read_text()
imports = '''use std::{collections::HashMap, path::{Path, PathBuf}, time::Duration};
use file_id::{FileId, get_file_id};
use notify::{EventKindMask, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventHandler, DebounceEventResult, Debouncer, FileIdCache, new_debouncer_opt};
use reflect_graph_paths::is_pruned_dir_name;
use walkdir::WalkDir;
type GraphDebouncer = Debouncer<RecommendedWatcher, PrunedFileIdMap>;
'''
cache = s[s.index('#[derive(Debug)]\npub struct PrunedFileIdMap'):s.index('/// What one debounced batch')]
watch = s[s.index('fn watch_graph('):s.index('/// Start (or restart)')]
original_test = s[s.index('    #[cfg(target_os = "linux")]'):s.index('    #[test]\n    fn hidden_churn')]
Path('probe/src/lib.rs').write_text(imports + cache + watch + '\n#[cfg(test)] mod tests { use super::*;\n' + original_test + Path('tests.rs').read_text() + '\n}')
