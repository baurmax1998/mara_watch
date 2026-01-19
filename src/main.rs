use std::fs;
use mara_watch::{Manager, create_sync_a_to_b, create_sync_a_to_c, create_chat_processor};
use mara_watch::processors::{create_command_processor, create_todo_processor};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create directories
    fs::create_dir_all("_mara/a")?;
    fs::create_dir_all("_mara/b")?;
    fs::create_dir_all("_mara/c")?;

    // Initialize manager
    let mut manager = Manager::new();

    // Register all processes and watch paths
    manager = manager
        .register_process(create_sync_a_to_b())
        .register_process(create_sync_a_to_c())
        .register_process(create_chat_processor())
        .register_process(create_command_processor())
        .register_process(create_todo_processor())
        .watch_path("/Users/ba22036/RustroverProjects/mara_watch/_mara");

    // Run the manager
    manager.run()?;

    Ok(())
}
