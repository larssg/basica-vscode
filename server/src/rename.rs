use std::collections::HashMap;
use tower_lsp::lsp_types::*;

/// Prepare rename - check if symbol can be renamed and return its range
pub fn prepare_rename(source: &str, position: Position) -> Option<PrepareRenameResponse> {
    let lines: Vec<&str> = source.lines().collect();
    let line = lines.get(position.line as usize)?;
    let char_pos = position.character as usize;

    let (start, end, word) = get_word_at_position(line, char_pos)?;

    // Don't allow renaming line numbers (too complex - affects GOTO/GOSUB)
    if word.parse::<u32>().is_ok() {
        return None;
    }

    // Don't allow renaming keywords
    if is_keyword(&word.to_uppercase()) {
        return None;
    }

    Some(PrepareRenameResponse::Range(Range {
        start: Position {
            line: position.line,
            character: start as u32,
        },
        end: Position {
            line: position.line,
            character: end as u32,
        },
    }))
}

/// Rename a variable throughout the document
pub fn rename_symbol(
    source: &str,
    position: Position,
    new_name: &str,
    uri: Url,
) -> Option<WorkspaceEdit> {
    let lines: Vec<&str> = source.lines().collect();
    let line = lines.get(position.line as usize)?;
    let char_pos = position.character as usize;

    let (_, _, word) = get_word_at_position(line, char_pos)?;

    // Don't allow renaming line numbers or keywords
    if word.parse::<u32>().is_ok() || is_keyword(&word.to_uppercase()) {
        return None;
    }

    let var_upper = word.to_uppercase();
    let is_string_var = var_upper.ends_with('$');
    let base_name = if is_string_var {
        &var_upper[..var_upper.len() - 1]
    } else {
        &var_upper
    };

    // Find all occurrences
    let mut edits = Vec::new();

    for (line_idx, line_text) in source.lines().enumerate() {
        let upper = line_text.to_uppercase();
        let mut search_start = 0;

        while let Some(pos) = upper[search_start..].find(base_name) {
            let abs_pos = search_start + pos;

            // Check word boundaries
            let before_ok = abs_pos == 0 || {
                let prev = upper.as_bytes()[abs_pos - 1];
                !prev.is_ascii_alphanumeric() && prev != b'_' && prev != b'$'
            };

            let after_pos = abs_pos + base_name.len();
            let after_ok = after_pos >= upper.len() || {
                let next = upper.as_bytes()[after_pos];
                !next.is_ascii_alphanumeric() && next != b'_'
            };

            if before_ok && after_ok {
                // Check for $ suffix
                let end_pos = if after_pos < upper.len() && upper.as_bytes()[after_pos] == b'$' {
                    after_pos + 1
                } else {
                    after_pos
                };

                // Preserve the $ suffix if original had it
                let replacement = if end_pos > after_pos {
                    if new_name.ends_with('$') {
                        new_name.to_string()
                    } else {
                        format!("{}$", new_name)
                    }
                } else {
                    new_name.trim_end_matches('$').to_string()
                };

                edits.push(TextEdit {
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
                    new_text: replacement,
                });
            }

            search_start = abs_pos + 1;
        }
    }

    if edits.is_empty() {
        return None;
    }

    let mut changes = HashMap::new();
    changes.insert(uri, edits);

    Some(WorkspaceEdit {
        changes: Some(changes),
        document_changes: None,
        change_annotations: None,
    })
}

fn get_word_at_position(line: &str, char_pos: usize) -> Option<(usize, usize, String)> {
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
        Some((start, end, line[start..end].to_string()))
    } else {
        None
    }
}

fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

fn is_keyword(word: &str) -> bool {
    matches!(
        word,
        "REM"
            | "LET"
            | "DIM"
            | "PRINT"
            | "LPRINT"
            | "INPUT"
            | "LINE"
            | "IF"
            | "THEN"
            | "ELSE"
            | "ELSEIF"
            | "END"
            | "ENDIF"
            | "FOR"
            | "TO"
            | "STEP"
            | "NEXT"
            | "WHILE"
            | "WEND"
            | "DO"
            | "LOOP"
            | "UNTIL"
            | "EXIT"
            | "SELECT"
            | "CASE"
            | "GOTO"
            | "GOSUB"
            | "RETURN"
            | "ON"
            | "READ"
            | "DATA"
            | "RESTORE"
            | "DEF"
            | "FN"
            | "OPEN"
            | "CLOSE"
            | "GET"
            | "PUT"
            | "WRITE"
            | "FIELD"
            | "LSET"
            | "RSET"
            | "AS"
            | "OUTPUT"
            | "APPEND"
            | "RANDOM"
            | "BINARY"
            | "SCREEN"
            | "COLOR"
            | "CLS"
            | "LOCATE"
            | "WIDTH"
            | "CIRCLE"
            | "PAINT"
            | "PSET"
            | "PRESET"
            | "DRAW"
            | "PLAY"
            | "SOUND"
            | "BEEP"
            | "SWAP"
            | "RANDOMIZE"
            | "CLEAR"
            | "STOP"
            | "POKE"
            | "PEEK"
            | "OUT"
            | "INP"
            | "WAIT"
            | "AND"
            | "OR"
            | "XOR"
            | "NOT"
            | "MOD"
            | "IMP"
            | "EQV"
            | "KILL"
            | "NAME"
            | "MKDIR"
            | "RMDIR"
            | "CHDIR"
            | "FILES"
            | "CALL"
            | "CHAIN"
            | "COMMON"
            | "SHARED"
            | "STATIC"
    )
}
