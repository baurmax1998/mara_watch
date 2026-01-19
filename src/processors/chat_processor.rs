use crate::{FileEvent, EventOrigin, SyncProcess};

/// Chat processor
/// Filter: chat.txt files
/// Target: same file (chat.txt)
/// Transform: append chat message
pub fn create_chat_processor() -> SyncProcess {
    SyncProcess::new(
        "Chat processor",
        |event: &FileEvent| {
            let filename = event.path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|name| name == "chat.txt")
                .unwrap_or(false);

            let right_origin = match &event.origin {
                EventOrigin::External => true,
                EventOrigin::Internal { process_name } => {
                    process_name != "Chat processor"
                },
            };

            filename && right_origin
        },
        |event: &FileEvent| {
            Some(event.path.clone())
        },
        |_event, content| {
            let mut new_content = content.to_vec();
            let chat_message = b"\nchat: das ist interessant";
            new_content.extend_from_slice(chat_message);
            Ok(new_content)
        },
    )
}
