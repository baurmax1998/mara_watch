use crate::{FileEvent, EventOrigin, SyncProcess};
use std::process::Command;

/// CommandEntry struct - represents a single command with its result
#[derive(Debug, Clone, PartialEq)]
pub struct CommandEntry {
    pub command: String,
    pub result: Option<String>,
}

impl CommandEntry {
    pub fn new(command: String) -> Self {
        CommandEntry {
            command,
            result: None,
        }
    }

    pub fn with_result(command: String, result: String) -> Self {
        CommandEntry {
            command,
            result: Some(result),
        }
    }
}

/// CommandLog struct - contains a list of command entries
#[derive(Debug, Clone, PartialEq)]
pub struct CommandLog {
    pub entries: Vec<CommandEntry>,
}

impl CommandLog {
    pub fn new() -> Self {
        CommandLog {
            entries: Vec::new(),
        }
    }

    pub fn add_entry(&mut self, entry: CommandEntry) {
        self.entries.push(entry);
    }

    /// Parse content into CommandLog
    /// Format: <command>\n------\n<result>\n-----\n...
    pub fn parse(content: &str) -> Self {
        let mut log = CommandLog::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();

            // Skip empty lines and separators
            if line.is_empty() || line.starts_with("-----") {
                i += 1;
                continue;
            }

            // Collect command lines until separator
            let mut command_lines = Vec::new();
            while i < lines.len() && !lines[i].trim().starts_with("------") {
                command_lines.push(lines[i]);
                i += 1;
            }

            let command = command_lines
                .join("\n")
                .trim()
                .to_string();

            if !command.is_empty() {
                // Skip the ------ separator
                if i < lines.len() && lines[i].trim().starts_with("------") {
                    i += 1;
                }

                // Collect result lines until ----- separator
                let mut result_lines = Vec::new();
                while i < lines.len() && !lines[i].trim().starts_with("-----") {
                    result_lines.push(lines[i]);
                    i += 1;
                }

                let result = if !result_lines.is_empty() {
                    let result_str = result_lines
                        .join("\n")
                        .trim()
                        .to_string();
                    if result_str.is_empty() {
                        None
                    } else {
                        Some(result_str)
                    }
                } else {
                    None
                };

                // Skip the ----- separator
                if i < lines.len() && lines[i].trim().starts_with("-----") {
                    i += 1;
                }

                log.add_entry(CommandEntry::with_result(command, result.unwrap_or_default()));
            }
        }

        log
    }

    /// Render CommandLog back to content string
    pub fn render(&self) -> String {
        self.entries
            .iter()
            .map(|entry| {
                let result = entry.result.as_ref().map(|r| r.as_str()).unwrap_or("");
                format!("{}\n------\n{}\n-----\n", entry.command, result)
            })
            .collect::<Vec<_>>()
            .join("")
    }
}

/// Execute a command and return the output
fn execute_command(command: &str) -> String {
    // Use shell to execute the command
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", command])
            .output()
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
    };

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            if !stderr.is_empty() {
                format!("{}{}", stdout, stderr)
            } else {
                stdout.to_string()
            }
        }
        Err(e) => format!("Error executing command: {}", e),
    }
}

/// Command processor
/// Filter: .command files
/// Target: same file
/// Transform: parse commands, execute new ones, render back
pub fn create_command_processor() -> SyncProcess {
    SyncProcess::new(
        "Command processor",
        |event: &FileEvent| {
            let filename = event.path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|name| name.ends_with(".command"))
                .unwrap_or(false);

            let right_origin = match &event.origin {
                EventOrigin::External => true,
                EventOrigin::Internal { process_name } => {
                    process_name != "Command processor"
                },
            };

            filename && right_origin
        },
        |event: &FileEvent| {
            Some(event.path.clone())
        },
        |_event, content| {
            let content_str = String::from_utf8_lossy(&content);

            // Parse the command log
            let mut log = CommandLog::parse(&content_str);

            // Execute commands that don't have results yet
            for entry in &mut log.entries {
                if entry.result.is_none() || entry.result.as_ref().map(|r| r.is_empty()).unwrap_or(false) {
                    let result = execute_command(&entry.command);
                    entry.result = Some(result);
                }
            }

            // Render back
            let rendered = log.render();
            Ok(rendered.into_bytes())
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_command() {
        let content = "echo hello\n------\nhello\n-----\n";
        let log = CommandLog::parse(content);
        assert_eq!(log.entries.len(), 1);
        assert_eq!(log.entries[0].command, "echo hello");
        assert_eq!(log.entries[0].result, Some("hello".to_string()));
    }

    #[test]
    fn test_parse_multiple_commands() {
        let content = "echo hello\n------\nhello\n-----\necho world\n------\nworld\n-----\n";
        let log = CommandLog::parse(content);
        assert_eq!(log.entries.len(), 2);
        assert_eq!(log.entries[0].command, "echo hello");
        assert_eq!(log.entries[0].result, Some("hello".to_string()));
        assert_eq!(log.entries[1].command, "echo world");
        assert_eq!(log.entries[1].result, Some("world".to_string()));
    }

    #[test]
    fn test_parse_command_without_result() {
        let content = "echo hello\n------\n\n-----\n";
        let log = CommandLog::parse(content);
        assert_eq!(log.entries.len(), 1);
        assert_eq!(log.entries[0].command, "echo hello");
        assert_eq!(log.entries[0].result, Some("".to_string()));
    }

    #[test]
    fn test_render_commands() {
        let mut log = CommandLog::new();
        log.add_entry(CommandEntry::with_result("echo hello".to_string(), "hello".to_string()));
        log.add_entry(CommandEntry::with_result("echo world".to_string(), "world".to_string()));
        let rendered = log.render();
        assert_eq!(rendered, "echo hello\n------\nhello\n-----\necho world\n------\nworld\n-----\n");
    }

    #[test]
    fn test_round_trip() {
        let original = "ls\n------\nfile1.txt\nfile2.txt\n-----\n";
        let log = CommandLog::parse(original);
        let rendered = log.render();
        assert_eq!(rendered, original);
    }

    #[test]
    fn test_multiline_result() {
        let content = "ls\n------\nfile1.txt\nfile2.txt\nfile3.txt\n-----\n";
        let log = CommandLog::parse(content);
        assert_eq!(log.entries.len(), 1);
        assert_eq!(log.entries[0].command, "ls");
        assert_eq!(log.entries[0].result, Some("file1.txt\nfile2.txt\nfile3.txt".to_string()));
    }

    #[test]
    fn test_add_entry() {
        let mut log = CommandLog::new();
        log.add_entry(CommandEntry::new("echo test".to_string()));
        assert_eq!(log.entries.len(), 1);
        assert_eq!(log.entries[0].command, "echo test");
        assert_eq!(log.entries[0].result, None);
    }
}
