use crate::{FileEvent, EventOrigin, SyncProcess};

/// Message struct - represents a single message from a persona
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub persona: String,
    pub content: String,
}

impl Message {
    pub fn new(persona: String, content: String) -> Self {
        Message { persona, content }
    }
}

/// Chat struct - contains a list of messages
#[derive(Debug, Clone, PartialEq)]
pub struct Chat {
    pub messages: Vec<Message>,
}

impl Chat {
    pub fn new() -> Self {
        Chat {
            messages: Vec::new(),
        }
    }

    pub fn add_message(&mut self, persona: String, content: String) {
        self.messages.push(Message::new(persona, content));
    }

    /// Parse content into Chat
    /// Format: <persona>:\n<content>\n------
    /// If content doesn't have a persona, it's treated as "User"
    pub fn parse(content: &str) -> Self {
        let mut chat = Chat::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();

            // Skip empty lines and separators
            if line.is_empty() || line.starts_with("------") {
                i += 1;
                continue;
            }

            // Check if this line has a persona (ends with :)
            if line.ends_with(':') {
                let persona = line.trim_end_matches(':').to_string();
                let mut content_lines = Vec::new();
                i += 1;

                // Collect content until separator or end
                while i < lines.len() && !lines[i].trim().starts_with("------") {
                    content_lines.push(lines[i]);
                    i += 1;
                }

                let msg_content = content_lines
                    .join("\n")
                    .trim()
                    .to_string();

                if !msg_content.is_empty() {
                    chat.add_message(persona, msg_content);
                }

                // Skip separator line
                if i < lines.len() && lines[i].trim().starts_with("------") {
                    i += 1;
                }
            } else {
                // No persona, treat as User message
                let mut content_lines = vec![line];
                i += 1;

                // Collect until separator or end
                while i < lines.len() && !lines[i].trim().starts_with("------") {
                    content_lines.push(lines[i].trim());
                    i += 1;
                }

                let msg_content = content_lines.join("\n").trim().to_string();
                if !msg_content.is_empty() {
                    chat.add_message("User".to_string(), msg_content);
                }

                // Skip separator line
                if i < lines.len() && lines[i].trim().starts_with("------") {
                    i += 1;
                }
            }
        }

        chat
    }

    /// Render Chat back to content string
    pub fn render(&self) -> String {
        self.messages
            .iter()
            .map(|msg| format!("{}:\n{}\n------\n", msg.persona, msg.content))
            .collect::<Vec<_>>()
            .join("")
    }
}

/// Chat processor
/// Filter: .chat files
/// Target: same file
/// Transform: parse chat, add mara message, render back
pub fn create_chat_processor() -> SyncProcess {
    SyncProcess::new(
        "Chat processor",
        |event: &FileEvent| {
            let filename = event.path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|name| name.ends_with(".chat"))
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
            let content_str = String::from_utf8_lossy(&content);

            // Parse the chat
            let mut chat = Chat::parse(&content_str);

            // Add mara message
            chat.add_message("mara".to_string(), "das ist interessant".to_string());

            // Render back
            let rendered = chat.render();
            Ok(rendered.into_bytes())
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_message() {
        let content = "Alice:\nHello world\n------\n";
        let chat = Chat::parse(content);
        assert_eq!(chat.messages.len(), 1);
        assert_eq!(chat.messages[0].persona, "Alice");
        assert_eq!(chat.messages[0].content, "Hello world");
    }

    #[test]
    fn test_user_message() {
        let content = "hallo";
        let chat = Chat::parse(content);
        assert_eq!(chat.messages.len(), 1);
        assert_eq!(chat.messages[0].persona, "User");
        assert_eq!(chat.messages[0].content, "hallo");
    }

    #[test]
    fn test_parse_user_antwort_message() {
        let content = "Alice:\nHello world\n------\nhallo";
        let chat = Chat::parse(content);
        assert_eq!(chat.messages.len(), 2);
        assert_eq!(chat.messages[0].persona, "Alice");
        assert_eq!(chat.messages[0].content, "Hello world");
        assert_eq!(chat.messages[1].persona, "User");
        assert_eq!(chat.messages[1].content, "hallo");
    }

    #[test]
    fn test_parse_multiple_messages() {
        let content = "Alice:\nHello\n------\nBob:\nWorld\n------\n";
        let chat = Chat::parse(content);
        assert_eq!(chat.messages.len(), 2);
        assert_eq!(chat.messages[0].persona, "Alice");
        assert_eq!(chat.messages[0].content, "Hello");
        assert_eq!(chat.messages[1].persona, "Bob");
        assert_eq!(chat.messages[1].content, "World");
    }

    #[test]
    fn test_render_messages() {
        let mut chat = Chat::new();
        chat.add_message("Alice".to_string(), "Hello".to_string());
        chat.add_message("Bob".to_string(), "World".to_string());
        let rendered = chat.render();
        assert_eq!(rendered, "Alice:\nHello\n------\nBob:\nWorld\n------\n");
    }

    #[test]
    fn test_round_trip() {
        let original = "Alice:\nHello world\n------\nBob:\nThis is a test\n------\n";
        let chat = Chat::parse(original);
        let rendered = chat.render();
        assert_eq!(rendered, original);
    }

    #[test]
    fn test_multiline_content() {
        let content = "Alice:\nLine 1\nLine 2\n------\nBob:\nAnother text\n------\n";
        let chat = Chat::parse(content);
        assert_eq!(chat.messages.len(), 2);
        assert_eq!(chat.messages[0].content, "Line 1\nLine 2");
        assert_eq!(chat.messages[1].content, "Another text");
    }

    #[test]
    fn test_add_message() {
        let mut chat = Chat::new();
        chat.add_message("Alice".to_string(), "Hello".to_string());
        chat.add_message("mara".to_string(), "das ist interessant".to_string());
        assert_eq!(chat.messages.len(), 2);
        assert_eq!(chat.messages[1].persona, "mara");
    }
}
