use std::path::PathBuf;
use crate::events::FileEvent;

pub type FilterFn = fn(&FileEvent) -> bool;
pub type TargetFn = fn(&FileEvent) -> Option<PathBuf>;
pub type TransformFn = fn(&FileEvent, &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>>;

pub struct SyncProcess {
    pub name: String,
    pub filter: FilterFn,
    pub target: TargetFn,
    pub transform: TransformFn,
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

    pub fn should_process(&self, event: &FileEvent) -> bool {
        (self.filter)(event)
    }

    pub fn get_target(&self, event: &FileEvent) -> Option<PathBuf> {
        (self.target)(event)
    }

    pub fn transform_content(&self, event: &FileEvent, content: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        (self.transform)(event, content)
    }
}