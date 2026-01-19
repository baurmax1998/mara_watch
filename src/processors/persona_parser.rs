use crate::{FileEvent, EventOrigin, SyncProcess};

/// Parse content into persona-text pairs
/// Format: <persona>:\n<text>\n------
fn parse_personas(content: &str) -> Vec<(String, String)> {
    let mut personas = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        // Look for persona line (ends with :)
        if lines[i].ends_with(':') && !lines[i].trim().is_empty() {
            let persona = lines[i].trim_end_matches(':').to_string();
            let mut text_lines = Vec::new();
            i += 1;

            // Collect text until separator or end
            while i < lines.len() && !lines[i].trim().starts_with("------") {
                text_lines.push(lines[i]);
                i += 1;
            }

            let text = text_lines
                .join("\n")
                .trim()
                .to_string();

            if !text.is_empty() {
                personas.push((persona, text));
            }

            // Skip separator line
            if i < lines.len() && lines[i].trim().starts_with("------") {
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    personas
}

/// Render personas back to content
fn render_personas(personas: &[(String, String)]) -> String {
    personas
        .iter()
        .map(|(persona, text)| format!("{}:\n{}", persona, text))
        .collect::<Vec<_>>()
        .join("\n------\n")
        + "\n"
}

/// Persona parser processor
/// Filter: .txt files
/// Target: same file
/// Transform: parse and re-render personas
pub fn create_persona_parser() -> SyncProcess {
    SyncProcess::new(
        "Persona parser",
        |event: &FileEvent| {
            let filename = event.path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|name| name.ends_with(".txt"))
                .unwrap_or(false);

            let right_origin = match &event.origin {
                EventOrigin::External => true,
                EventOrigin::Internal { process_name } => {
                    process_name != "Persona parser"
                },
            };

            filename && right_origin
        },
        |event: &FileEvent| {
            Some(event.path.clone())
        },
        |_event, content| {
            let content_str = String::from_utf8_lossy(&content);
            let personas = parse_personas(&content_str);
            let rendered = render_personas(&personas);
            Ok(rendered.into_bytes())
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_persona() {
        let content = "Alice:\nHello world\n";
        let result = parse_personas(content);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "Alice");
        assert_eq!(result[0].1, "Hello world");
    }

    #[test]
    fn test_parse_multiple_personas() {
        let content = "Alice:\nHello\n------\nBob:\nWorld\n";
        let result = parse_personas(content);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "Alice");
        assert_eq!(result[0].1, "Hello");
        assert_eq!(result[1].0, "Bob");
        assert_eq!(result[1].1, "World");
    }

    #[test]
    fn test_render_personas() {
        let personas = vec![
            ("Alice".to_string(), "Hello".to_string()),
            ("Bob".to_string(), "World".to_string()),
        ];
        let rendered = render_personas(&personas);
        assert_eq!(rendered, "Alice:\nHello\n------\nBob:\nWorld\n");
    }

    #[test]
    fn test_round_trip() {
        let original = "Alice:\nHello world\n------\nBob:\nThis is a test\n";
        let personas = parse_personas(original);
        let rendered = render_personas(&personas);
        assert_eq!(rendered, original);
    }

    #[test]
    fn test_multiline_text() {
        let content = "Alice:\nLine 1\nLine 2\n------\nBob:\nAnother text\n";
        let result = parse_personas(content);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].1, "Line 1\nLine 2");
        assert_eq!(result[1].1, "Another text");
    }
}
