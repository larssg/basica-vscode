use std::collections::HashMap;
use tower_lsp::lsp_types::*;

/// Find definition for GOTO/GOSUB line number targets
pub fn find_definition(source: &str, position: Position, uri: Url) -> Option<GotoDefinitionResponse> {
    let lines: Vec<&str> = source.lines().collect();
    let line = lines.get(position.line as usize)?;

    // Check if we're on or after GOTO/GOSUB keyword
    let line_upper = line.to_uppercase();
    let is_goto_context = line_upper.contains("GOTO") || line_upper.contains("GOSUB");

    if !is_goto_context {
        return None;
    }

    // Extract number at cursor position
    let target_line = get_number_at_position(line, position.character as usize)?;

    // Build line number -> source line mapping
    let line_map = build_line_map(source);

    // Find the source line containing the target BASIC line number
    if let Some(&source_line) = line_map.get(&target_line) {
        return Some(GotoDefinitionResponse::Scalar(Location {
            uri,
            range: Range {
                start: Position {
                    line: source_line,
                    character: 0,
                },
                end: Position {
                    line: source_line,
                    character: 0,
                },
            },
        }));
    }

    None
}

/// Build a map from BASIC line numbers to source file line numbers (0-indexed)
fn build_line_map(source: &str) -> HashMap<u32, u32> {
    let mut map = HashMap::new();
    for (source_line, text) in source.lines().enumerate() {
        let trimmed = text.trim_start();
        if let Some(first_word) = trimmed.split_whitespace().next() {
            if let Ok(line_num) = first_word.parse::<u32>() {
                map.insert(line_num, source_line as u32);
            }
        }
    }
    map
}

/// Get the number at a given character position in a line
fn get_number_at_position(line: &str, char_pos: usize) -> Option<u32> {
    let bytes = line.as_bytes();
    let char_pos = char_pos.min(bytes.len());

    // Find start of number
    let mut start = char_pos;
    while start > 0 && bytes.get(start - 1).map(|b| b.is_ascii_digit()).unwrap_or(false) {
        start -= 1;
    }

    // Find end of number
    let mut end = char_pos;
    while end < bytes.len() && bytes.get(end).map(|b| b.is_ascii_digit()).unwrap_or(false) {
        end += 1;
    }

    if start < end {
        line[start..end].parse().ok()
    } else {
        None
    }
}
