use tower_lsp::lsp_types::*;

/// Find all references to a variable or line number
pub fn find_references(source: &str, position: Position, uri: Url) -> Vec<Location> {
    let lines: Vec<&str> = source.lines().collect();
    let line = match lines.get(position.line as usize) {
        Some(l) => *l,
        None => return vec![],
    };

    let word = match get_word_at_position(line, position.character as usize) {
        Some(w) => w,
        None => return vec![],
    };

    // Check if it's a line number
    if let Ok(target_line) = word.parse::<u32>() {
        return find_line_references(source, target_line, &uri);
    }

    // It's a variable - find all occurrences
    let var_upper = word.to_uppercase();
    find_variable_references(source, &var_upper, &uri)
}

/// Find all references to a BASIC line number (GOTO, GOSUB, THEN, RESTORE, etc.)
fn find_line_references(source: &str, target_line: u32, uri: &Url) -> Vec<Location> {
    let mut refs = Vec::new();
    let target_str = target_line.to_string();

    for (line_idx, line) in source.lines().enumerate() {
        let upper = line.to_uppercase();

        // Check if this line IS the target line (definition)
        let trimmed = line.trim_start();
        if let Some(first_word) = trimmed.split_whitespace().next() {
            if first_word == target_str {
                refs.push(Location {
                    uri: uri.clone(),
                    range: Range {
                        start: Position {
                            line: line_idx as u32,
                            character: 0,
                        },
                        end: Position {
                            line: line_idx as u32,
                            character: first_word.len() as u32,
                        },
                    },
                });
            }
        }

        // Find references in GOTO, GOSUB, THEN, RESTORE, ON...GOTO/GOSUB
        for keyword in &["GOTO ", "GOSUB ", "THEN ", "RESTORE "] {
            let mut search_start = 0;
            while let Some(kw_pos) = upper[search_start..].find(keyword) {
                let abs_pos = search_start + kw_pos + keyword.len();
                let after = &line[abs_pos..];

                // Parse line numbers (comma-separated for ON...GOTO/GOSUB)
                for num_part in after.split(',') {
                    let num_str = num_part.trim().split_whitespace().next().unwrap_or("");
                    if num_str == target_str {
                        let char_start = abs_pos + (num_part.len() - num_part.trim_start().len());
                        refs.push(Location {
                            uri: uri.clone(),
                            range: Range {
                                start: Position {
                                    line: line_idx as u32,
                                    character: char_start as u32,
                                },
                                end: Position {
                                    line: line_idx as u32,
                                    character: (char_start + num_str.len()) as u32,
                                },
                            },
                        });
                    }
                    // Stop if we hit a non-number (end of line number list)
                    if num_str.parse::<u32>().is_err() {
                        break;
                    }
                }

                search_start = abs_pos;
            }
        }
    }

    refs
}

/// Find all references to a variable
fn find_variable_references(source: &str, var_name: &str, uri: &Url) -> Vec<Location> {
    let mut refs = Vec::new();

    for (line_idx, line) in source.lines().enumerate() {
        let upper = line.to_uppercase();
        let mut search_start = 0;

        while let Some(pos) = upper[search_start..].find(var_name) {
            let abs_pos = search_start + pos;

            // Check word boundaries
            let before_ok = abs_pos == 0 || {
                let prev = upper.as_bytes()[abs_pos - 1];
                !prev.is_ascii_alphanumeric() && prev != b'_' && prev != b'$'
            };

            let after_pos = abs_pos + var_name.len();
            let after_ok = after_pos >= upper.len() || {
                let next = upper.as_bytes()[after_pos];
                // Allow $ suffix or non-word char
                !next.is_ascii_alphanumeric() && next != b'_'
            };

            if before_ok && after_ok {
                // Check for $ suffix
                let end_pos = if after_pos < upper.len() && upper.as_bytes()[after_pos] == b'$' {
                    after_pos + 1
                } else {
                    after_pos
                };

                refs.push(Location {
                    uri: uri.clone(),
                    range: Range {
                        start: Position {
                            line: line_idx as u32,
                            character: abs_pos as u32,
                        },
                        end: Position {
                            line: line_idx as u32,
                            character: end_pos as u32,
                        },
                    },
                });
            }

            search_start = abs_pos + 1;
        }
    }

    refs
}

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
