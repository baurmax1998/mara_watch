use std::fs;
use std::path::PathBuf;
use crate::events::{EventKind, FileEvent};

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

    pub fn execute(&self, event: &FileEvent) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Filter check
        if !(self.filter)(event) {
            return Ok(());
        }

        // 2. Get target path
        let Some(target_path) = (self.target)(event) else {
            return Ok(());
        };

        match event.event_kind {
            EventKind::Create | EventKind::Modify => {
                let content = fs::read(&event.path)?;
                let transformed = (self.transform)(event, &content)?;
                fs::write(&target_path, transformed)?;

                println!(
                    "[{}] {} -> {}",
                    self.name,
                    event.path.display(),
                    target_path.display()
                );
            }
            EventKind::Delete => {
                if target_path.exists() {
                    fs::remove_file(&target_path)?;
                }

                println!(
                    "[{}] {} (target: {})",
                    self.name,
                    event.path.display(),
                    target_path.display()
                );
            }
        }

        Ok(())
    }
}