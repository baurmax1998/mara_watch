use std::path::PathBuf;
use crate::FileEvent;
use crate::SyncProcess;

/// Unidirectional sync A -> B
/// Filter: .txt files from _mara/a only (prevent loops)
/// Target: _mara/b/
/// Transform: identity (no change)
pub fn create_sync_a_to_b() -> SyncProcess {
    SyncProcess::new(
        "A->B (txt files)",
        |event: &FileEvent| {
            let path_str = event.path.to_string_lossy();
            let is_from_a = path_str.contains("_mara/a");
            let is_txt = event.path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|name| name.ends_with(".txt"))
                .unwrap_or(false);
            is_from_a && is_txt
        },
        |event: &FileEvent| {
            let filename = event.path.file_name()?.to_str()?.to_string();
            Some(PathBuf::from("_mara/b").join(filename))
        },
        |_event, content| Ok(content.to_vec()),
    )
}
