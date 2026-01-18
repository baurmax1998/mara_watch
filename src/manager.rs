use crate::events::{FileEvent, EventKind, EventOrigin};
use notify::{Watcher, RecursiveMode, Result as NotifyResult};
use notify::recommended_watcher;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

type FilterFn = fn(&FileEvent) -> bool;
type TargetFn = fn(&FileEvent) -> Option<PathBuf>;
type TransformFn = fn(&FileEvent, &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>>;

pub struct SyncProcess {
    name: String,
    filter: FilterFn,
    target: TargetFn,
    transform: TransformFn,
}

impl SyncProcess {
    pub fn new(name: &str, filter: FilterFn, target: TargetFn, transform: TransformFn) -> Self {
        Self {
            name: name.to_string(),
            filter,
            target,
            transform,
        }
    }

    pub fn execute(
        &self,
        event: &FileEvent,
        sync_map: &Arc<Mutex<HashMap<String, String>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Filter check
        if !(self.filter)(event) {
            return Ok(());
        }

        // 2. Get target path
        let Some(target_path) = (self.target)(event) else {
            return Ok(());
        };

        let event_kind_str = match event.event_kind {
            EventKind::Create => "CREATE",
            EventKind::Modify => "MODIFY",
            EventKind::Delete => "DELETE",
        };

        let origin_str = match &event.origin {
            EventOrigin::External => "EXT".to_string(),
            EventOrigin::Internal { process_name } => format!("INT[{}]", process_name),
        };

        match event.event_kind {
            EventKind::Create | EventKind::Modify => {
                let content = fs::read(&event.path)?;
                let transformed = (self.transform)(event, &content)?;
                fs::write(&target_path, transformed)?;

                // Track that this target was written by this process
                let target_path_str = target_path.to_string_lossy().to_string();
                sync_map
                    .lock()
                    .unwrap()
                    .insert(target_path_str, self.name.clone());

                println!(
                    "[{}] | {} -> {}",
                    self.name,
                    event.path.display(),
                    target_path.display()
                );
            }
            EventKind::Delete => {
                if target_path.exists() {
                    fs::remove_file(&target_path)?;
                }

                // Remove from tracking
                let target_path_str = target_path.to_string_lossy().to_string();
                sync_map.lock().unwrap().remove(&target_path_str);

                println!(
                    "[{}] | {} (target: {})",
                    self.name,
                    event.path.display(),
                    target_path.display()
                );
            }
        }

        Ok(())
    }
}

pub struct Manager {
    watch_paths: Vec<String>,
    processes: Vec<SyncProcess>,
    sync_map: Arc<Mutex<HashMap<String, String>>>, // Mapping: target_path -> process_name
}

impl Manager {
    pub fn new() -> Self {
        Self {
            watch_paths: Vec::new(),
            processes: Vec::new(),
            sync_map: Arc::new(Mutex::new(HashMap::new())),
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
        let sync_map = self.sync_map.clone();
        let sync_map_clone = sync_map.clone();

        let watch_paths = self.watch_paths.clone();

        let mut watcher = recommended_watcher(move |res: NotifyResult<notify::Event>| {
            match res {
                Ok(event) => match event.kind {
                    notify::EventKind::Create(_) => {
                        for path in &event.paths {
                            if path.is_file() {
                                let mut file_event = FileEvent::new(path.clone(), EventKind::Create);
                                // Check if this file was written by a process
                                let path_str = path.to_string_lossy().to_string();
                                if let Some(process_name) = sync_map_clone.lock().unwrap().get(&path_str) {
                                    file_event = file_event.with_origin(EventOrigin::Internal {
                                        process_name: process_name.clone(),
                                    });
                                }
                                Self::dispatch_event(&file_event, &processes_clone, &sync_map_clone);
                            }
                        }
                    }
                    notify::EventKind::Modify(_) => {
                        for path in &event.paths {
                            if path.is_file() {
                                let mut file_event = FileEvent::new(path.clone(), EventKind::Modify);
                                // Check if this file was written by a process
                                let path_str = path.to_string_lossy().to_string();
                                if let Some(process_name) = sync_map_clone.lock().unwrap().get(&path_str) {
                                    file_event = file_event.with_origin(EventOrigin::Internal {
                                        process_name: process_name.clone(),
                                    });
                                }
                                Self::dispatch_event(&file_event, &processes_clone, &sync_map_clone);
                            }
                        }
                    }
                    notify::EventKind::Remove(_) => {
                        for path in &event.paths {
                            let file_event = FileEvent::new(path.clone(), EventKind::Delete);
                            Self::dispatch_event(&file_event, &processes_clone, &sync_map_clone);
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
        event: &FileEvent,
        processes: &std::sync::Arc<Vec<SyncProcess>>,
        sync_map: &Arc<Mutex<HashMap<String, String>>>,
    ) {
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

        for process in processes.iter() {
            if let Err(e) = process.execute(event, sync_map) {
                println!("Error processing event: {}", e);
            }
        }
    }
}
