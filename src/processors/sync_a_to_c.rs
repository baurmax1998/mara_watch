use std::path::PathBuf;
use crate::{FileEvent, EventOrigin, SyncProcess};

/// Bidirectional sync A <-> C
/// Filter: Only external events (ignore events from internal syncs)
/// Target: opposite directory
/// Transform: identity
pub fn create_sync_a_to_c() -> SyncProcess {
    SyncProcess::new(
        "A<->C (bidirectional)",
        |event: &FileEvent| {
            // Only process external events - ignore internal ones!
            let path_str = event.path.to_string_lossy();
            let right_path = path_str.contains("_mara/a") || path_str.contains("_mara/c");

            let right_origin = match &event.origin {
                EventOrigin::External => true,
                EventOrigin::Internal { process_name } => {
                    process_name != "A<->C (bidirectional)"
                }, // Ignore internal events
            };

            right_path && right_origin
        },
        |event: &FileEvent| {
            let path_str = event.path.to_string_lossy();
            let filename = event.path.file_name()?.to_str()?.to_string();

            if path_str.contains("_mara/a") {
                Some(PathBuf::from("_mara/c").join(filename))
            } else if path_str.contains("_mara/c") {
                Some(PathBuf::from("_mara/a").join(filename))
            } else {
                None
            }
        },
        |_event, content| Ok(content.to_vec()),
    )
}
