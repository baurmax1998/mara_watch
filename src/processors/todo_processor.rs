use crate::{FileEvent, EventOrigin, SyncProcess};

/// TodoEntry struct - represents a single todo item
#[derive(Debug, Clone, PartialEq)]
pub struct TodoEntry {
    pub text: String,
    pub completed: bool,
}

impl TodoEntry {
    pub fn new(text: String) -> Self {
        TodoEntry {
            text,
            completed: false,
        }
    }

    pub fn with_status(text: String, completed: bool) -> Self {
        TodoEntry {
            text,
            completed,
        }
    }
}

/// TodoLog struct - contains a list of todo entries
#[derive(Debug, Clone, PartialEq)]
pub struct TodoLog {
    pub entries: Vec<TodoEntry>,
}

impl TodoLog {
    pub fn new() -> Self {
        TodoLog {
            entries: Vec::new(),
        }
    }

    pub fn add_entry(&mut self, entry: TodoEntry) {
        self.entries.push(entry);
    }

    /// Parse content into TodoLog
    /// Format:
    /// Neues Todo:
    /// <new_todo_text>
    /// Todos:
    /// [] todo1
    /// [] todo2
    /// -----------------
    /// [x] completed_todo1
    pub fn parse(content: &str) -> Self {
        let mut log = TodoLog::new();
        let lines: Vec<&str> = content.lines().collect();

        let mut active_todos = Vec::new();
        let mut completed_todos = Vec::new();
        let mut new_todos = Vec::new();
        let mut in_new_todo_section = false;
        let mut in_completed_section = false;

        for line in lines {
            let trimmed = line.trim();

            // Check if we're in the "Neues Todo:" section
            if trimmed.starts_with("Neues Todo:") {
                in_new_todo_section = true;
                in_completed_section = false;
                continue;
            }

            // Check if we're starting the "Todos:" section
            if trimmed.starts_with("Todos:") {
                in_new_todo_section = false;
                continue;
            }

            // Skip empty lines
            if trimmed.is_empty() {
                continue;
            }

            // Check for separator between active and completed todos
            if trimmed.starts_with("-----------------") {
                in_completed_section = true;
                continue;
            }

            // If in new todo section and not empty, add as new todo
            if in_new_todo_section && !trimmed.is_empty() {
                new_todos.push(trimmed.to_string());
                continue;
            }

            // Parse todo items
            if trimmed.starts_with("[") {
                if let Some(close_bracket) = trimmed.find("]") {
                    if close_bracket >= 1 {
                        let checkbox = trimmed[1..close_bracket].trim();
                        let text = trimmed[close_bracket + 1..].trim().to_string();

                        let completed = checkbox == "x" || checkbox == "X";
                        let entry = TodoEntry::with_status(text, completed);

                        if in_completed_section {
                            completed_todos.push(entry);
                        } else {
                            active_todos.push(entry);
                        }
                    }
                }
            }
        }

        // Add new todos first (not in completed section)
        for text in new_todos {
            log.add_entry(TodoEntry::new(text));
        }

        // Add active todos
        for entry in active_todos {
            log.add_entry(entry);
        }

        // Add completed todos
        for entry in completed_todos {
            log.add_entry(entry);
        }

        log
    }

    /// Render TodoLog back to content string
    pub fn render(&self) -> String {
        let mut output = String::from("Neues Todo:\n\nTodos:\n");

        // Separate active and completed todos
        let active: Vec<_> = self.entries.iter().filter(|e| !e.completed).collect();
        let completed: Vec<_> = self.entries.iter().filter(|e| e.completed).collect();

        // Add active todos
        for entry in &active {
            output.push_str(&format!("[] {}\n", entry.text));
        }

        // Add separator if there are completed todos
        if !completed.is_empty() {
            output.push_str("-----------------\n");
        }

        // Add completed todos
        for entry in &completed {
            output.push_str(&format!("[x] {}\n", entry.text));
        }

        output
    }
}

/// Todo processor
/// Filter: .todo files
/// Target: same file
/// Transform: parse todos, sort by completion status, render back
pub fn create_todo_processor() -> SyncProcess {
    SyncProcess::new(
        "Todo processor",
        |event: &FileEvent| {
            let filename = event.path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|name| name.ends_with(".todo"))
                .unwrap_or(false);

            let right_origin = match &event.origin {
                EventOrigin::External => true,
                EventOrigin::Internal { process_name } => {
                    process_name != "Todo processor"
                },
            };

            filename && right_origin
        },
        |event: &FileEvent| {
            Some(event.path.clone())
        },
        |_event, content| {
            let content_str = String::from_utf8_lossy(&content);

            // Parse the todo log
            let log = TodoLog::parse(&content_str);

            // Render back (automatically sorts completed todos to the bottom)
            let rendered = log.render();
            Ok(rendered.into_bytes())
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_existing_todo() {
        let content = "Neues Todo:\n\nTodos:\n[] Rasen mähen\n";
        let log = TodoLog::parse(content);
        assert_eq!(log.entries.len(), 1);
        assert_eq!(log.entries[0].text, "Rasen mähen");
        assert_eq!(log.entries[0].completed, false);
    }

    #[test]
    fn test_parse_multiple_existing_todos() {
        let content = "Neues Todo:\n\nTodos:\n[] Rasen mähen\n[] Pflanzen gießen\n";
        let log = TodoLog::parse(content);
        assert_eq!(log.entries.len(), 2);
        assert_eq!(log.entries[0].text, "Rasen mähen");
        assert_eq!(log.entries[1].text, "Pflanzen gießen");
    }

    #[test]
    fn test_parse_new_todo_input() {
        let content = "Neues Todo:\nNeues Item hinzufügen\n\nTodos:\n[] Rasen mähen\n";
        let log = TodoLog::parse(content);
        assert_eq!(log.entries.len(), 2);
        assert_eq!(log.entries[0].text, "Neues Item hinzufügen");
        assert_eq!(log.entries[0].completed, false);
        assert_eq!(log.entries[1].text, "Rasen mähen");
        assert_eq!(log.entries[1].completed, false);
    }

    #[test]
    fn test_parse_completed_todo() {
        let content = "Neues Todo:\n\nTodos:\n-----------------\n[x] Müll runter bringen\n";
        let log = TodoLog::parse(content);
        assert_eq!(log.entries.len(), 1);
        assert_eq!(log.entries[0].text, "Müll runter bringen");
        assert_eq!(log.entries[0].completed, true);
    }

    #[test]
    fn test_parse_mixed_todos() {
        let content = "Neues Todo:\n\nTodos:\n[] Rasen mähen\n[] Pflanzen gießen\n-----------------\n[x] Müll runter bringen\n";
        let log = TodoLog::parse(content);
        assert_eq!(log.entries.len(), 3);
        assert_eq!(log.entries[0].text, "Rasen mähen");
        assert_eq!(log.entries[0].completed, false);
        assert_eq!(log.entries[1].text, "Pflanzen gießen");
        assert_eq!(log.entries[1].completed, false);
        assert_eq!(log.entries[2].text, "Müll runter bringen");
        assert_eq!(log.entries[2].completed, true);
    }

    #[test]
    fn test_render_todos() {
        let mut log = TodoLog::new();
        log.add_entry(TodoEntry::new("Rasen mähen".to_string()));
        log.add_entry(TodoEntry::new("Pflanzen gießen".to_string()));
        let rendered = log.render();
        assert!(rendered.contains("[] Rasen mähen"));
        assert!(rendered.contains("[] Pflanzen gießen"));
    }

    #[test]
    fn test_render_with_completed() {
        let mut log = TodoLog::new();
        log.add_entry(TodoEntry::new("Rasen mähen".to_string()));
        log.add_entry(TodoEntry::with_status("Müll runter bringen".to_string(), true));
        let rendered = log.render();
        assert!(rendered.contains("[] Rasen mähen"));
        assert!(rendered.contains("-----------------"));
        assert!(rendered.contains("[x] Müll runter bringen"));
    }

    #[test]
    fn test_round_trip() {
        let original = "Neues Todo:\n\nTodos:\n[] Rasen mähen\n[] Pflanzen gießen\n-----------------\n[x] Müll runter bringen\n";
        let log = TodoLog::parse(original);
        let rendered = log.render();
        assert_eq!(rendered, original);
    }

    #[test]
    fn test_add_entry() {
        let mut log = TodoLog::new();
        log.add_entry(TodoEntry::new("Test todo".to_string()));
        assert_eq!(log.entries.len(), 1);
        assert_eq!(log.entries[0].text, "Test todo");
        assert_eq!(log.entries[0].completed, false);
    }
}
