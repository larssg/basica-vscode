use basica::lexer::is_keyword;
use std::collections::HashMap;
use tower_lsp::lsp_types::*;

/// Find definition for GOTO/GOSUB targets or variable first assignments
pub fn find_definition(
    source: &str,
    position: Position,
    uri: Url,
) -> Option<GotoDefinitionResponse> {
    let lines: Vec<&str> = source.lines().collect();
    let line = lines.get(position.line as usize)?;

    // Get word at cursor
    let word = get_word_at_position(line, position.character as usize)?;

    // Check if it's a number (GOTO/GOSUB target)
    if let Ok(target_line) = word.parse::<u32>() {
        let line_upper = line.to_uppercase();
        if line_upper.contains("GOTO")
            || line_upper.contains("GOSUB")
            || line_upper.contains("RESTORE")
            || line_upper.contains("THEN")
        {
            let line_map = build_line_map(source);
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
        }
        return None;
    }

    // It's a variable - find first assignment
    let var_upper = word.to_uppercase();

    // Skip keywords (use the authoritative list from basica)
    if is_keyword(&var_upper) {
        return None;
    }

    // Find first assignment of this variable
    if let Some((def_line, def_char)) = find_variable_definition(source, &var_upper) {
        return Some(GotoDefinitionResponse::Scalar(Location {
            uri,
            range: Range {
                start: Position {
                    line: def_line,
                    character: def_char,
                },
                end: Position {
                    line: def_line,
                    character: def_char + word.len() as u32,
                },
            },
        }));
    }

    None
}

/// Find the first assignment of a variable
fn find_variable_definition(source: &str, var_name: &str) -> Option<(u32, u32)> {
    for (line_idx, line) in source.lines().enumerate() {
        let upper = line.to_uppercase();

        // Skip the line number at the start
        let content = skip_line_number(&upper);

        // Look for patterns like "VAR =" or "LET VAR =" or "VAR(..." for arrays
        // Also handle DIM statements

        // Check for DIM
        if let Some(dim_pos) = content.find("DIM ") {
            let after_dim = &content[dim_pos + 4..];
            if let Some(var_pos) = find_var_in_list(after_dim, var_name) {
                let original_line = skip_line_number(line);
                let offset = line.len() - original_line.len();
                return Some((line_idx as u32, (offset + dim_pos + 4 + var_pos) as u32));
            }
        }

        // Check for LET VAR = or VAR =
        if let Some(pos) = find_assignment(content, var_name) {
            let original_line = skip_line_number(line);
            let offset = line.len() - original_line.len();
            return Some((line_idx as u32, (offset + pos) as u32));
        }

        // Check for FOR VAR =
        if let Some(for_pos) = content.find("FOR ") {
            let after_for = &content[for_pos + 4..];
            if after_for.trim_start().starts_with(var_name) {
                let trimmed = after_for.trim_start();
                if trimmed.len() > var_name.len() {
                    let next_char = trimmed.chars().nth(var_name.len()).unwrap_or(' ');
                    if next_char == ' ' || next_char == '=' || next_char == '(' {
                        let original_line = skip_line_number(line);
                        let offset = line.len() - original_line.len();
                        let var_offset = after_for.len() - trimmed.len();
                        return Some((line_idx as u32, (offset + for_pos + 4 + var_offset) as u32));
                    }
                }
            }
        }

        // Check for INPUT VAR or READ VAR
        for keyword in &["INPUT ", "READ "] {
            if let Some(kw_pos) = content.find(keyword) {
                let after_kw = &content[kw_pos + keyword.len()..];
                // Skip optional prompt in INPUT "prompt"; VAR
                let vars_part = if *keyword == "INPUT " {
                    if let Some(semi_pos) = after_kw.find(';') {
                        &after_kw[semi_pos + 1..]
                    } else {
                        after_kw
                    }
                } else {
                    after_kw
                };

                if let Some(var_pos) = find_var_in_list(vars_part, var_name) {
                    let original_line = skip_line_number(line);
                    let offset = line.len() - original_line.len();
                    let vars_offset = content.len() - vars_part.len();
                    return Some((line_idx as u32, (offset + vars_offset + var_pos) as u32));
                }
            }
        }
    }
    None
}

/// Find a variable in a comma-separated list
fn find_var_in_list(list: &str, var_name: &str) -> Option<usize> {
    let mut pos = 0;
    for part in list.split(',') {
        let trimmed = part.trim_start();
        let var_part = trimmed.split('(').next().unwrap_or(trimmed).trim();
        if var_part == var_name {
            return Some(pos + (part.len() - trimmed.len()));
        }
        pos += part.len() + 1; // +1 for comma
    }
    None
}

/// Find assignment pattern (LET VAR = or VAR =)
fn find_assignment(line: &str, var_name: &str) -> Option<usize> {
    // Try LET VAR =
    if let Some(let_pos) = line.find("LET ") {
        let after_let = &line[let_pos + 4..];
        let trimmed = after_let.trim_start();
        let var_part = trimmed.split('(').next().unwrap_or(trimmed);
        let var_part = var_part.split('=').next().unwrap_or(var_part).trim();
        if var_part == var_name {
            let offset = after_let.len() - trimmed.len();
            return Some(let_pos + 4 + offset);
        }
    }

    // Try VAR = (implicit LET) - look for VAR followed by = or (
    let mut search_start = 0;
    while let Some(pos) = line[search_start..].find(var_name) {
        let abs_pos = search_start + pos;

        // Check it's a word boundary before
        if abs_pos > 0 {
            let prev = line.chars().nth(abs_pos - 1).unwrap_or(' ');
            if prev.is_alphanumeric() || prev == '_' || prev == '$' {
                search_start = abs_pos + 1;
                continue;
            }
        }

        // Check what follows
        let after = &line[abs_pos + var_name.len()..];
        let next = after.trim_start().chars().next().unwrap_or(' ');

        // Should be followed by = or ( for array assignment
        if next == '=' || next == '(' {
            // Make sure it's not == (comparison in some contexts)
            if next == '=' && after.trim_start().starts_with("==") {
                search_start = abs_pos + 1;
                continue;
            }
            return Some(abs_pos);
        }

        search_start = abs_pos + 1;
    }

    None
}

/// Skip the line number at the start of a BASIC line
fn skip_line_number(line: &str) -> &str {
    let trimmed = line.trim_start();
    if let Some(first_word) = trimmed.split_whitespace().next() {
        if first_word.parse::<u32>().is_ok() {
            let after_num = &trimmed[first_word.len()..];
            return after_num.trim_start();
        }
    }
    line
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

/// Get word at cursor position
fn get_word_at_position(line: &str, char_pos: usize) -> Option<&str> {
    let bytes = line.as_bytes();
    let char_pos = char_pos.min(bytes.len());

    let mut start = char_pos;
    while start > 0 && is_word_char(bytes[start - 1]) {
        start -= 1;
    }

    let mut end = char_pos;
    while end < bytes.len() && is_word_char(bytes[end]) {
        end += 1;
    }

    if start < end {
        Some(&line[start..end])
    } else {
        None
    }
}

fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}
