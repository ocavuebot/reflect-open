from pathlib import Path
s = Path('source/apps/desktop/src-tauri/src/watcher.rs').read_text()
imports = '''use std::{collections::HashMap, path::{Path, PathBuf}, time::Duration};
use file_id::{FileId, get_file_id};
use notify::{EventKindMask, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventHandler, DebounceEventResult, Debouncer, FileIdCache, new_debouncer_opt};
use reflect_graph_paths::{GraphPathKind, classify, evicted_logical_path, eviction_placeholder, has_pruned_component, is_pruned_dir_name, to_slash_lossy, wire_path};
use reflect_types::{ChangeKind, FileChange};
use walkdir::WalkDir;
type GraphDebouncer = Debouncer<RecommendedWatcher, PrunedFileIdMap>;
'''
cache = s[s.index('#[derive(Debug)]\npub struct PrunedFileIdMap'):s.index('/// What one debounced batch')]
effects = s[s.index('/// What one debounced batch'):s.index('fn lock_watcher') ]
io = Path('source/crates/graph-io/src/io.rs').read_text()
modified = io[io.index('pub fn modified_ms('):io.index('pub fn modified_ms(')+io[io.index('pub fn modified_ms('):].index('\n}')+2]
shim = '\nmod fs { use std::{fs, time::UNIX_EPOCH}; pub use reflect_graph_paths::is_dataless; ' + modified + '\n}\n'
watch = s[s.index('fn watch_graph('):s.index('/// Start (or restart)')]
original_test = s[s.index('    #[cfg(target_os = "linux")]'):s.index('    #[test]\n    fn hidden_churn')]
Path('probe/src').mkdir(parents=True, exist_ok=True)
Path('probe/src/lib.rs').write_text(imports + cache + effects + shim + watch + '\n#[cfg(test)] mod tests { use super::*;\n' + original_test + Path('tests.rs').read_text() + '\n}')
