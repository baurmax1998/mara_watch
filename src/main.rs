mod events;
mod manager;

use std::fs;
use std::path::PathBuf;
use events::FileEvent;
use manager::{Manager, SyncProcess};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create directories
    fs::create_dir_all("_mara/a")?;
    fs::create_dir_all("_mara/b")?;
    fs::create_dir_all("_mara/c")?;

    // Initialize manager
    let manager = Manager::new();

    // Sync Process 1: Unidirectional A -> B
    // Filter: .txt files from _mara/a only (prevent loops)
    // Target: _mara/b/
    // Transform: identity (no change)
    let process1 = SyncProcess::new(
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
    );

    // Sync Process 2: Bidirectional A <-> C
    // Filter: all files from _mara/a or _mara/c
    // Target: opposite directory
    // Transform: identity
    let process2 = SyncProcess::new(
        "A<->C (bidirectional)",
        |event: &FileEvent| {
            let path_str = event.path.to_string_lossy();
            path_str.contains("_mara/a") || path_str.contains("_mara/c")
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
    );

    // Sync Process 3: Self-sync with transformation
    // Filter: only README.txt
    // Target: same file (self-overwrite)
    // Transform: convert to uppercase
    // let process3 = SyncProcess::new(
    //     "Transform: README.txt to uppercase",
    //     |event: &FileEvent| {
    //         event.path
    //             .file_name()
    //             .and_then(|n| n.to_str())
    //             .map(|name| name == "README.txt")
    //             .unwrap_or(false)
    //     },
    //     |event: &FileEvent| {
    //         // Self-sync: return same path
    //         Some(event.path.clone())
    //     },
    //     |_event, content| {
    //         let text = String::from_utf8_lossy(content);
    //         Ok(text.to_uppercase().as_bytes().to_vec())
    //     },
    // );

    // Register all processes and watch paths
    let manager = manager
        .register_process(process1)
        .register_process(process2)
        // .register_process(process3)
        .watch_path("/Users/ba22036/RustroverProjects/mara_watch/_mara");

    // Run the manager
    manager.run()?;

    Ok(())
}
