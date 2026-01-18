use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct FileEvent {
    pub path: PathBuf,
    pub event_kind: EventKind,
    pub origin: EventOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventKind {
    Create,
    Modify,
    Delete,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventOrigin {
    External,                      // Datei wurde von außen geändert (Editor, Shell, etc.)
    Internal { process_name: String }, // Datei wurde vom Programm geändert (mit Process-Name)
}

impl FileEvent {
    pub fn new(path: PathBuf, event_kind: EventKind) -> Self {
        Self {
            path,
            event_kind,
            origin: EventOrigin::External,
        }
    }

    pub fn from_process(path: PathBuf, event_kind: EventKind, process_name: &str) -> Self {
        Self {
            path,
            event_kind,
            origin: EventOrigin::Internal {
                process_name: process_name.to_string(),
            },
        }
    }

    pub fn with_origin(mut self, origin: EventOrigin) -> Self {
        self.origin = origin;
        self
    }
}
