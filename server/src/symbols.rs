use std::collections::HashSet;
use tower_lsp::lsp_types::*;

/// Get document symbols (outline) for a BASIC program
pub fn get_document_symbols(source: &str) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();

    // Find all GOSUB targets to mark as subroutines
    let subroutine_lines = find_gosub_targets(source);

    for (line_idx, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();

        // Extract line number
        if let Some(first_word) = trimmed.split_whitespace().next() {
            if let Ok(line_num) = first_word.parse::<u32>() {
                let rest = trimmed[first_word.len()..].trim_start();

                // Determine symbol kind and name
                let (name, kind, detail) = if subroutine_lines.contains(&line_num) {
                    (
                        format!("{} (SUB)", line_num),
                        SymbolKind::FUNCTION,
                        Some("Subroutine".to_string()),
                    )
                } else if rest.to_uppercase().starts_with("REM") {
                    let comment = rest[3..].trim();
                    let preview = if comment.len() > 30 {
                        format!("{}...", &comment[..30])
                    } else {
                        comment.to_string()
                    };
                    (
                        format!("{} REM {}", line_num, preview),
                        SymbolKind::STRING,
                        Some("Comment".to_string()),
                    )
                } else if rest.to_uppercase().starts_with("DATA") {
                    (
                        format!("{} DATA", line_num),
                        SymbolKind::ARRAY,
                        Some("Data".to_string()),
                    )
                } else if rest.to_uppercase().starts_with("DEF FN") {
                    let fn_part = &rest[6..];
                    let fn_name = fn_part
                        .split('(')
                        .next()
                        .unwrap_or(fn_part)
                        .split('=')
                        .next()
                        .unwrap_or(fn_part)
                        .trim();
                    (
                        format!("{} DEF FN{}", line_num, fn_name),
                        SymbolKind::FUNCTION,
                        Some("User function".to_string()),
                    )
                } else {
                    // Get first statement keyword
                    let keyword = rest
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .to_uppercase();
                    let keyword = keyword.split('(').next().unwrap_or(&keyword);
                    let keyword = keyword.split('=').next().unwrap_or(keyword);

                    // Show meaningful lines
                    let show = matches!(
                        keyword,
                        "FOR" | "WHILE" | "DO" | "SELECT" | "IF" | "GOSUB" | "ON"
                    );

                    if show {
                        let preview = if rest.len() > 40 {
                            format!("{}...", &rest[..40])
                        } else {
                            rest.to_string()
                        };
                        (
                            format!("{} {}", line_num, preview),
                            SymbolKind::KEY,
                            None,
                        )
                    } else {
                        continue; // Skip non-interesting lines
                    }
                };

                let range = Range {
                    start: Position {
                        line: line_idx as u32,
                        character: 0,
                    },
                    end: Position {
                        line: line_idx as u32,
                        character: line.len() as u32,
                    },
                };

                #[allow(deprecated)]
                symbols.push(DocumentSymbol {
                    name,
                    detail,
                    kind,
                    tags: None,
                    deprecated: None,
                    range,
                    selection_range: range,
                    children: None,
                });
            }
        }
    }

    symbols
}

/// Find all line numbers that are targets of GOSUB
fn find_gosub_targets(source: &str) -> HashSet<u32> {
    let mut targets = HashSet::new();

    for line in source.lines() {
        let upper = line.to_uppercase();

        // Find GOSUB targets
        for part in upper.split("GOSUB") {
            let trimmed = part.trim_start();
            if let Some(num_str) = trimmed.split_whitespace().next() {
                if let Ok(num) = num_str.parse::<u32>() {
                    targets.insert(num);
                }
            }
        }

        // Find ON...GOSUB targets
        if let Some(gosub_pos) = upper.find("GOSUB") {
            if upper[..gosub_pos].contains("ON ") {
                let after = &upper[gosub_pos + 5..];
                for num_str in after.split(',') {
                    let num_str = num_str.trim();
                    if let Ok(num) = num_str.parse::<u32>() {
                        targets.insert(num);
                    }
                }
            }
        }
    }

    targets
}
