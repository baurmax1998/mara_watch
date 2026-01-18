use crate::events::{EventKind, EventOrigin, FileEvent};
use crate::process::SyncProcess;
use notify::recommended_watcher;
use notify::{RecursiveMode, Result as NotifyResult, Watcher};
use std::fs;
use std::path::Path;

pub struct TargetMapping {
    pub target_path: std::path::PathBuf,
    pub process_name: String,
}

pub struct Manager {
    watch_paths: Vec<String>,
    processes: Vec<SyncProcess>,
    target_mappings: Vec<TargetMapping>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            watch_paths: Vec::new(),
            processes: Vec::new(),
            target_mappings: Vec::new(),
        }
    }

    pub fn watch_path(mut self, path: &str) -> Self {
        self.watch_paths.push(path.to_string());
        self
    }

    pub fn register_process(mut self, process: SyncProcess) -> Self {
        self.processes.push(process);
        self
    }

    pub fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        if self.watch_paths.is_empty() {
            println!("No paths to watch!");
            return Ok(());
        }

        if self.processes.is_empty() {
            println!("No sync processes registered!");
            return Ok(());
        }

        println!(
            "Starting file sync manager with {} processes, watching {} paths",
            self.processes.len(),
            self.watch_paths.len()
        );

        let processes = std::sync::Arc::new(self.processes);
        let processes_clone = processes.clone();

        let target_mappings = std::sync::Arc::new(std::sync::Mutex::new(self.target_mappings));
        let target_mappings_clone = std::sync::Arc::clone(&target_mappings);

        let watch_paths = self.watch_paths.clone();

        let mut watcher = recommended_watcher(move |res: NotifyResult<notify::Event>| {
            match res {
                Ok(event) => match event.kind {
                    notify::EventKind::Create(_) => {
                        for path in &event.paths {
                            if path.is_file() {
                                Self::dispatch_event(path.clone(), EventKind::Create, &processes_clone, &target_mappings_clone);
                            }
                        }
                    }
                    notify::EventKind::Modify(_) => {
                        for path in &event.paths {
                            if path.is_file() {
                                Self::dispatch_event(path.clone(), EventKind::Modify, &processes_clone, &target_mappings_clone);
                            }
                        }
                    }
                    notify::EventKind::Remove(_) => {
                        for path in &event.paths {
                            Self::dispatch_event(path.clone(), EventKind::Delete, &processes_clone, &target_mappings_clone);
                        }
                    }
                    _ => {}
                },
                Err(e) => println!("Watcher error: {}", e),
            }
        })?;

        // Watch all configured paths
        for path in &watch_paths {
            watcher.watch(Path::new(path), RecursiveMode::Recursive)?;
            println!("Watching: {}", path);
        }

        println!("Sync manager running. Press Ctrl+C to stop.");

        // Keep running
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }

    fn dispatch_event(
        path: std::path::PathBuf,
        event_kind: EventKind,
        processes: &std::sync::Arc<Vec<SyncProcess>>,
        target_mappings: &std::sync::Arc<std::sync::Mutex<Vec<TargetMapping>>>,
    ) {
        // Check if this path is a target path from a previous operation (Internal origin)
        let mut mappings = target_mappings.lock().unwrap();
        let mut origin = EventOrigin::External;
        let mut matched_index = None;

        for (index, mapping) in mappings.iter().enumerate() {
            let absolute_target = if mapping.target_path.is_absolute() {
                mapping.target_path.clone()
            } else {
                std::env::current_dir()
                    .ok()
                    .map(|cwd| cwd.join(&mapping.target_path))
                    .unwrap_or_else(|| mapping.target_path.clone())
            };

            if absolute_target == path {
                origin = EventOrigin::Internal {
                    process_name: mapping.process_name.clone(),
                };
                matched_index = Some(index);
                break;
            }
        }

        // Remove the matched mapping to prevent false matches in future events
        if let Some(index) = matched_index {
            mappings.remove(index);
        }
        drop(mappings);

        let event = FileEvent::new_with_origin(path, event_kind, origin);

        // Log the event before processing
        let event_kind_str = match event.event_kind {
            EventKind::Create => "CREATE",
            EventKind::Modify => "MODIFY",
            EventKind::Delete => "DELETE",
        };

        let origin_str = match &event.origin {
            EventOrigin::External => "[EXT]".to_string(),
            EventOrigin::Internal { process_name } => format!("[INT:{}]", process_name),
        };

        println!("EVENT {} {} | {}", event_kind_str, origin_str, event.path.display());

        // Process each sync process
        for process in processes.iter() {
            // 1. Check if process should handle this event
            if !process.should_process(&event) {
                continue;
            }

            // 2. Get target path
            let Some(target_path) = process.get_target(&event) else {
                continue;
            };

            // 3. Add target mapping
            {
                let mut mappings = target_mappings.lock().unwrap();
                mappings.push(TargetMapping {
                    target_path: target_path.clone(),
                    process_name: process.name.clone(),
                });
            }

            // 4. Execute the sync
            match event.event_kind {
                EventKind::Create | EventKind::Modify => {
                    if let Ok(content) = fs::read(&event.path) {
                        if let Ok(transformed) = process.transform_content(&event, &content) {
                            if let Err(e) = fs::write(&target_path, transformed) {
                                println!("[{}] Error writing: {}", process.name, e);
                                continue;
                            }

                            println!(
                                "[{}] {} [{}] | {} -> {}",
                                process.name,
                                event_kind_str,
                                origin_str,
                                event.path.display(),
                                target_path.display()
                            );
                        } else {
                            println!("[{}] Transform error", process.name);
                        }
                    } else {
                        println!("[{}] Read error", process.name);
                    }
                }
                EventKind::Delete => {
                    if target_path.exists() {
                        if let Err(e) = fs::remove_file(&target_path) {
                            println!("[{}] Delete error: {}", process.name, e);
                            continue;
                        }
                    }

                    println!(
                        "[{}] {} [{}] | {} (target: {})",
                        process.name,
                        event_kind_str,
                        origin_str,
                        event.path.display(),
                        target_path.display()
                    );
                }
            }
        }
    }
}
