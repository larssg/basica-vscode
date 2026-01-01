use basica::lexer::Lexer;
use basica::parser::Parser;
use tower_lsp::lsp_types::*;

/// Check source code for parse errors and return diagnostics
pub fn check(source: &str) -> Vec<Diagnostic> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();

    let mut parser = Parser::new(tokens);
    match parser.parse() {
        Ok(_) => vec![],
        Err(msg) => {
            // Try to extract line number from error message
            // Format is typically "Line X: error message"
            let (line, message) = parse_error_message(&msg);

            // Find the line in source to get the range
            let range = if line > 0 {
                let source_line = find_source_line_for_basic_line(source, line);
                Range {
                    start: Position {
                        line: source_line,
                        character: 0,
                    },
                    end: Position {
                        line: source_line,
                        character: 1000,
                    },
                }
            } else {
                // Default to first line if we can't determine location
                Range::default()
            };

            vec![Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("basica".to_string()),
                message,
                ..Default::default()
            }]
        }
    }
}

/// Parse error message to extract line number and clean message
fn parse_error_message(msg: &str) -> (u32, String) {
    // Try to match "Line X:" pattern
    if let Some(rest) = msg.strip_prefix("Line ") {
        if let Some(colon_pos) = rest.find(':') {
            if let Ok(line) = rest[..colon_pos].trim().parse::<u32>() {
                let message = rest[colon_pos + 1..].trim().to_string();
                return (line, message);
            }
        }
    }

    // Also try "at line X" pattern
    if let Some(pos) = msg.find("at line ") {
        let after = &msg[pos + 8..];
        if let Some(end) = after.find(|c: char| !c.is_ascii_digit()) {
            if let Ok(line) = after[..end].parse::<u32>() {
                return (line, msg.to_string());
            }
        } else if let Ok(line) = after.trim().parse::<u32>() {
            return (line, msg.to_string());
        }
    }

    (0, msg.to_string())
}

/// Find the source file line (0-indexed) containing a BASIC line number
fn find_source_line_for_basic_line(source: &str, basic_line: u32) -> u32 {
    for (idx, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(first_word) = trimmed.split_whitespace().next() {
            if let Ok(num) = first_word.parse::<u32>() {
                if num == basic_line {
                    return idx as u32;
                }
            }
        }
    }
    0
}
