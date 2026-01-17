use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct FileEvent {
    pub path: PathBuf,
    pub event_kind: EventKind,
}

#[derive(Debug, Clone, Copy)]
pub enum EventKind {
    Create,
    Modify,
    Delete,
}

impl FileEvent {
    pub fn new(path: PathBuf, event_kind: EventKind) -> Self {
        Self { path, event_kind }
    }
}
