use crate::{FileEvent, EventOrigin, SyncProcess};
use std::fs;
use std::path::{Path, PathBuf};

/// DokuEntry struct - represents a single documentation file entry
#[derive(Debug, Clone, PartialEq)]
pub struct DokuEntry {
    pub path: String,
    pub summary: String,
    pub last_updated: String,
}

impl DokuEntry {
    pub fn new(path: String, summary: String, last_updated: String) -> Self {
        DokuEntry {
            path,
            summary,
            last_updated,
        }
    }
}

/// DokuIndex struct - contains documentation index
#[derive(Debug, Clone, PartialEq)]
pub struct DokuIndex {
    pub entries: Vec<DokuEntry>,
}

impl DokuIndex {
    pub fn new() -> Self {
        DokuIndex {
            entries: Vec::new(),
        }
    }

    pub fn add_entry(&mut self, entry: DokuEntry) {
        self.entries.push(entry);
    }

    /// Parse content from .doku file
    /// Format:
    /// # Documentation Index
    ///
    /// ## File: path/to/file.md
    /// **Path:** path/to/file.md
    /// **Last Updated:** 2025-01-19 10:30:00
    /// **Summary:**
    /// First 300 chars of content...
    ///
    /// ---
    pub fn parse(content: &str) -> Self {
        let mut index = DokuIndex::new();
        let lines: Vec<&str> = content.lines().collect();

        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();

            // Look for entry markers
            if line.starts_with("## File:") {
                let mut path = String::new();
                let mut last_updated = String::new();
                let mut summary = String::new();

                // Parse path line
                if let Some(pos) = line.find("## File:") {
                    path = line[pos + 8..].trim().to_string();
                }

                i += 1;

                // Parse subsequent lines for Path, Last Updated, Summary
                let mut in_summary = false;
                while i < lines.len() {
                    let current = lines[i].trim();

                    if current.starts_with("---") {
                        break;
                    }

                    if current.starts_with("**Path:**") {
                        path = current[9..].trim().to_string();
                    } else if current.starts_with("**Last Updated:**") {
                        last_updated = current[17..].trim().to_string();
                    } else if current.starts_with("**Summary:**") {
                        in_summary = true;
                        i += 1;
                        continue;
                    } else if in_summary && !current.is_empty() && !current.starts_with("**") {
                        if !summary.is_empty() {
                            summary.push('\n');
                        }
                        summary.push_str(current);
                    }

                    i += 1;
                }

                if !path.is_empty() && !summary.is_empty() {
                    index.add_entry(DokuEntry::new(path, summary, last_updated));
                }

                continue;
            }

            i += 1;
        }

        index
    }

    /// Scan for markdown files in a directory
    pub fn scan_markdown_files(root_path: &Path) -> Vec<(PathBuf, String)> {
        let mut files = Vec::new();

        if let Ok(entries) = fs::read_dir(root_path) {
            for entry in entries.flatten() {
                let path = entry.path();

                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "md" {
                            if let Ok(content) = fs::read_to_string(&path) {
                                files.push((path, content));
                            }
                        }
                    }
                } else if path.is_dir() {
                    // Recursively scan subdirectories
                    let mut subfiles = Self::scan_markdown_files(&path);
                    files.append(&mut subfiles);
                }
            }
        }

        files
    }

    /// Create a summary from markdown content (first 300 chars, clean)
    pub fn create_summary(content: &str) -> String {
        let mut summary = String::new();
        let max_length = 300;

        for line in content.lines() {
            let trimmed = line.trim();
            // Skip markdown headers and empty lines
            if trimmed.is_empty() || trimmed.starts_with("#") || trimmed.starts_with("---") {
                continue;
            }

            // Remove markdown formatting
            let clean_line = trimmed
                .replace("**", "")
                .replace("_", "")
                .replace("`", "")
                .replace("[", "")
                .replace("]", "");

            if summary.is_empty() {
                summary = clean_line;
            } else if summary.len() < max_length {
                summary.push(' ');
                summary.push_str(&clean_line);
            }

            if summary.len() >= max_length {
                summary.truncate(max_length);
                summary.push_str("...");
                break;
            }
        }

        if summary.is_empty() {
            "[No content]".to_string()
        } else {
            summary
        }
    }

    /// Render DokuIndex back to content string
    pub fn render(&self) -> String {
        let mut output = String::from("# Documentation Index\n\n");

        for entry in &self.entries {
            output.push_str(&format!("## File: {}\n", entry.path));
            output.push_str(&format!("**Path:** {}\n", entry.path));
            output.push_str(&format!("**Last Updated:** {}\n", entry.last_updated));
            output.push_str("**Summary:**\n");
            output.push_str(&format!("{}\n", entry.summary));
            output.push_str("\n---\n\n");
        }

        // Add metadata
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let days_since_epoch = now / 86400;
        let seconds_today = now % 86400;
        let hours = seconds_today / 3600;
        let minutes = (seconds_today % 3600) / 60;
        let secs = seconds_today % 60;

        // Simple date approximation (not perfect but works)
        let year = 1970 + (days_since_epoch / 365) as u32;
        let day_of_year = (days_since_epoch % 365) as u32;
        let month = (day_of_year / 30).min(12).max(1);
        let day = (day_of_year % 30).max(1);

        output.push_str(&format!(
            "Last Updated: {}-{:02}-{:02} {:02}:{:02}:{:02}\n",
            year, month, day, hours, minutes, secs
        ));
        output.push_str(&format!("Total Files: {}\n", self.entries.len()));

        output
    }
}

/// Doku processor - scans markdown files and creates documentation index
/// Filter: .md files or .doku files
/// Target: .doku file in the same directory
/// Transform: scan markdown files, create index, render back
pub fn create_doku_processor() -> SyncProcess {
    SyncProcess::new(
        "Doku processor",
        |event: &FileEvent| {
            let filename = event.path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|name| name.ends_with(".md"))
                .unwrap_or(false);

            let right_origin = match &event.origin {
                EventOrigin::External => true,
                EventOrigin::Internal { process_name } => {
                    process_name != "Doku processor"
                },
            };

            filename && right_origin
        },
        |event: &FileEvent| {
            // Find the directory containing the file
            let dir = event.path.parent()?;

            // Look for a .doku file in the same directory
            let doku_path = dir.join("index.doku");
            Some(doku_path)
        },
        |event, _content| {
            // Get the directory to scan
            let dir = match event.path.parent() {
                Some(d) => d.to_path_buf(),
                None => return Ok(Vec::new()),
            };

            // Scan for all markdown files
            let md_files = DokuIndex::scan_markdown_files(&dir);

            if md_files.is_empty() {
                return Ok(Vec::new());
            }

            // Create entries for each markdown file
            let mut index = DokuIndex::new();

            // Generate current timestamp
            let now_ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let days_since_epoch = now_ts / 86400;
            let seconds_today = now_ts % 86400;
            let hours = seconds_today / 3600;
            let minutes = (seconds_today % 3600) / 60;
            let secs = seconds_today % 60;
            let year = 1970 + (days_since_epoch / 365) as u32;
            let day_of_year = (days_since_epoch % 365) as u32;
            let month = (day_of_year / 30).min(12).max(1);
            let day = (day_of_year % 30).max(1);
            let now = format!("{}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, day, hours, minutes, secs);

            for (path, content) in md_files {
                // Get relative path
                let relative_path = match path.strip_prefix(&dir) {
                    Ok(p) => p.to_string_lossy().to_string(),
                    Err(_) => path.to_string_lossy().to_string(),
                };

                let summary = DokuIndex::create_summary(&content);
                index.add_entry(DokuEntry::new(relative_path, summary, now.clone()));
            }

            // Sort entries by path
            index.entries.sort_by(|a, b| a.path.cmp(&b.path));

            // Render back
            let rendered = index.render();
            Ok(rendered.into_bytes())
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_summary_simple() {
        let content = "# Title\n\nThis is a test file with some content that should be summarized.";
        let summary = DokuIndex::create_summary(content);
        assert!(summary.contains("test file"));
        assert!(!summary.contains("#"));
    }

    #[test]
    fn test_create_summary_with_formatting() {
        let content = "# Title\n\nThis is **bold** and _italic_ text with `code`.";
        let summary = DokuIndex::create_summary(content);
        assert!(summary.contains("bold"));
        assert!(!summary.contains("**"));
        assert!(!summary.contains("_"));
    }

    #[test]
    fn test_create_summary_truncate() {
        let long_content = "# Title\n\n".to_string() + &"word ".repeat(100);
        let summary = DokuIndex::create_summary(&long_content);
        assert!(summary.len() <= 305); // 300 + "..."
        assert!(summary.ends_with("..."));
    }

    #[test]
    fn test_doku_entry_creation() {
        let entry = DokuEntry::new(
            "test.md".to_string(),
            "Test summary".to_string(),
            "2025-01-19 10:30:00".to_string(),
        );
        assert_eq!(entry.path, "test.md");
        assert_eq!(entry.summary, "Test summary");
    }

    #[test]
    fn test_doku_index_add_entry() {
        let mut index = DokuIndex::new();
        let entry = DokuEntry::new(
            "test.md".to_string(),
            "Summary".to_string(),
            "2025-01-19 10:30:00".to_string(),
        );
        index.add_entry(entry.clone());
        assert_eq!(index.entries.len(), 1);
        assert_eq!(index.entries[0], entry);
    }

    #[test]
    fn test_doku_index_render() {
        let mut index = DokuIndex::new();
        index.add_entry(DokuEntry::new(
            "test.md".to_string(),
            "Test summary".to_string(),
            "2025-01-19 10:30:00".to_string(),
        ));

        let rendered = index.render();
        assert!(rendered.contains("# Documentation Index"));
        assert!(rendered.contains("## File: test.md"));
        assert!(rendered.contains("**Path:** test.md"));
        assert!(rendered.contains("Test summary"));
        assert!(rendered.contains("Total Files: 1"));
    }

    #[test]
    fn test_parse_single_entry() {
        let content = r#"# Documentation Index

## File: test.md
**Path:** test.md
**Last Updated:** 2025-01-19 10:30:00
**Summary:**
This is a test summary of the documentation.

---

Last Updated: 2025-01-19 10:30:00
Total Files: 1
"#;

        let index = DokuIndex::parse(content);
        assert_eq!(index.entries.len(), 1);
        assert_eq!(index.entries[0].path, "test.md");
        assert!(index.entries[0].summary.contains("test summary"));
    }

    #[test]
    fn test_parse_multiple_entries() {
        let content = r#"# Documentation Index

## File: file1.md
**Path:** file1.md
**Last Updated:** 2025-01-19 10:30:00
**Summary:**
First file summary

---

## File: file2.md
**Path:** file2.md
**Last Updated:** 2025-01-19 10:30:00
**Summary:**
Second file summary

---

Last Updated: 2025-01-19 10:30:00
Total Files: 2
"#;

        let index = DokuIndex::parse(content);
        assert_eq!(index.entries.len(), 2);
        assert_eq!(index.entries[0].path, "file1.md");
        assert_eq!(index.entries[1].path, "file2.md");
    }

    #[test]
    fn test_round_trip() {
        let mut index = DokuIndex::new();
        index.add_entry(DokuEntry::new(
            "docs/api.md".to_string(),
            "API documentation for the system.".to_string(),
            "2025-01-19 10:30:00".to_string(),
        ));
        index.add_entry(DokuEntry::new(
            "docs/setup.md".to_string(),
            "Setup instructions for installation.".to_string(),
            "2025-01-19 10:30:00".to_string(),
        ));

        let rendered = index.render();
        let parsed = DokuIndex::parse(&rendered);

        assert_eq!(parsed.entries.len(), 2);
        assert_eq!(parsed.entries[0].path, "docs/api.md");
        assert_eq!(parsed.entries[1].path, "docs/setup.md");
    }
}
