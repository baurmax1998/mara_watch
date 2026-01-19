mod lib {
    pub mod events;
    pub mod manager;
    pub mod process;
}

pub mod processors;

pub use lib::events::{FileEvent, EventKind, EventOrigin};
pub use lib::manager::Manager;
pub use lib::process::SyncProcess;
pub use processors::{create_sync_a_to_b, create_sync_a_to_c, create_chat_processor, create_persona_parser};
