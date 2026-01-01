use tower_lsp::lsp_types::*;

/// Get folding ranges for control structures
pub fn get_folding_ranges(source: &str) -> Vec<FoldingRange> {
    let mut ranges = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    // Track open structures
    let mut for_stack: Vec<u32> = Vec::new();
    let mut while_stack: Vec<u32> = Vec::new();
    let mut do_stack: Vec<u32> = Vec::new();
    let mut select_stack: Vec<u32> = Vec::new();
    let mut if_stack: Vec<u32> = Vec::new();

    // Track subroutine regions (GOSUB targets to RETURN)
    let gosub_targets = find_gosub_targets(source);
    let mut current_sub_start: Option<u32> = None;

    for (line_idx, line) in lines.iter().enumerate() {
        let line_num = line_idx as u32;
        let upper = line.to_uppercase();
        let trimmed = upper.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Check for subroutine starts
        if let Some(first_word) = trimmed.split_whitespace().next() {
            if let Ok(basic_line) = first_word.parse::<u32>() {
                if gosub_targets.contains(&basic_line) {
                    // End previous subroutine if any
                    if let Some(start) = current_sub_start {
                        if line_num > start + 1 {
                            ranges.push(FoldingRange {
                                start_line: start,
                                start_character: None,
                                end_line: line_num - 1,
                                end_character: None,
                                kind: Some(FoldingRangeKind::Region),
                                collapsed_text: Some("...".to_string()),
                            });
                        }
                    }
                    current_sub_start = Some(line_num);
                }
            }
        }

        // Check for RETURN (ends subroutine)
        if trimmed.contains("RETURN") && !trimmed.contains("GOSUB") {
            if let Some(start) = current_sub_start.take() {
                if line_num > start {
                    ranges.push(FoldingRange {
                        start_line: start,
                        start_character: None,
                        end_line: line_num,
                        end_character: None,
                        kind: Some(FoldingRangeKind::Region),
                        collapsed_text: Some("...".to_string()),
                    });
                }
            }
        }

        // FOR...NEXT
        if contains_keyword(trimmed, "FOR") && contains_keyword(trimmed, "TO") {
            // Check if NEXT is on same line
            if !contains_keyword(trimmed, "NEXT") {
                for_stack.push(line_num);
            }
        }
        if contains_keyword(trimmed, "NEXT") {
            if let Some(start) = for_stack.pop() {
                if line_num > start {
                    ranges.push(FoldingRange {
                        start_line: start,
                        start_character: None,
                        end_line: line_num,
                        end_character: None,
                        kind: Some(FoldingRangeKind::Region),
                        collapsed_text: Some("...".to_string()),
                    });
                }
            }
        }

        // WHILE...WEND
        if contains_keyword(trimmed, "WHILE") && !contains_keyword(trimmed, "WEND") {
            while_stack.push(line_num);
        }
        if contains_keyword(trimmed, "WEND") {
            if let Some(start) = while_stack.pop() {
                if line_num > start {
                    ranges.push(FoldingRange {
                        start_line: start,
                        start_character: None,
                        end_line: line_num,
                        end_character: None,
                        kind: Some(FoldingRangeKind::Region),
                        collapsed_text: Some("...".to_string()),
                    });
                }
            }
        }

        // DO...LOOP
        if contains_keyword(trimmed, "DO") && !contains_keyword(trimmed, "LOOP") {
            do_stack.push(line_num);
        }
        if contains_keyword(trimmed, "LOOP") {
            if let Some(start) = do_stack.pop() {
                if line_num > start {
                    ranges.push(FoldingRange {
                        start_line: start,
                        start_character: None,
                        end_line: line_num,
                        end_character: None,
                        kind: Some(FoldingRangeKind::Region),
                        collapsed_text: Some("...".to_string()),
                    });
                }
            }
        }

        // SELECT CASE...END SELECT
        if contains_keyword(trimmed, "SELECT") && contains_keyword(trimmed, "CASE") {
            select_stack.push(line_num);
        }
        if contains_keyword(trimmed, "END") && contains_keyword(trimmed, "SELECT") {
            if let Some(start) = select_stack.pop() {
                if line_num > start {
                    ranges.push(FoldingRange {
                        start_line: start,
                        start_character: None,
                        end_line: line_num,
                        end_character: None,
                        kind: Some(FoldingRangeKind::Region),
                        collapsed_text: Some("...".to_string()),
                    });
                }
            }
        }

        // Multi-line IF...END IF
        // Only track IF that's not followed by statement on same line (structured IF)
        if contains_keyword(trimmed, "IF") {
            // Check if this looks like a structured IF (no statement after THEN on same line)
            if let Some(then_pos) = trimmed.find("THEN") {
                let after_then = &trimmed[then_pos + 4..].trim();
                // If nothing substantial after THEN, it's multi-line
                if after_then.is_empty() || after_then.parse::<u32>().is_ok() {
                    if_stack.push(line_num);
                }
            }
        }
        if contains_keyword(trimmed, "END") && contains_keyword(trimmed, "IF") {
            if let Some(start) = if_stack.pop() {
                if line_num > start {
                    ranges.push(FoldingRange {
                        start_line: start,
                        start_character: None,
                        end_line: line_num,
                        end_character: None,
                        kind: Some(FoldingRangeKind::Region),
                        collapsed_text: Some("...".to_string()),
                    });
                }
            }
        }

        // REM comment blocks
        if trimmed.starts_with("REM") || trimmed.starts_with("'") {
            // Look for consecutive comment lines
            let mut end_line = line_num;
            for (j, next_line) in lines[line_idx + 1..].iter().enumerate() {
                let next_trimmed = next_line.trim().to_uppercase();
                // Skip line number if present
                let content = skip_line_number(&next_trimmed);
                if content.starts_with("REM") || content.starts_with("'") {
                    end_line = line_num + 1 + j as u32;
                } else {
                    break;
                }
            }
            if end_line > line_num {
                ranges.push(FoldingRange {
                    start_line: line_num,
                    start_character: None,
                    end_line,
                    end_character: None,
                    kind: Some(FoldingRangeKind::Comment),
                    collapsed_text: Some("REM...".to_string()),
                });
            }
        }

        // DATA blocks
        if contains_keyword(trimmed, "DATA") {
            let mut end_line = line_num;
            for (j, next_line) in lines[line_idx + 1..].iter().enumerate() {
                let next_trimmed = next_line.trim().to_uppercase();
                let content = skip_line_number(&next_trimmed);
                if content.starts_with("DATA") {
                    end_line = line_num + 1 + j as u32;
                } else {
                    break;
                }
            }
            if end_line > line_num {
                ranges.push(FoldingRange {
                    start_line: line_num,
                    start_character: None,
                    end_line,
                    end_character: None,
                    kind: Some(FoldingRangeKind::Region),
                    collapsed_text: Some("DATA...".to_string()),
                });
            }
        }
    }

    ranges
}

fn contains_keyword(line: &str, keyword: &str) -> bool {
    // Check for keyword with word boundaries
    for (i, _) in line.match_indices(keyword) {
        let before_ok = i == 0 || !line.as_bytes()[i - 1].is_ascii_alphanumeric();
        let after_ok = i + keyword.len() >= line.len()
            || !line.as_bytes()[i + keyword.len()].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
    }
    false
}

fn skip_line_number(line: &str) -> &str {
    let trimmed = line.trim_start();
    if let Some(first_word) = trimmed.split_whitespace().next() {
        if first_word.parse::<u32>().is_ok() {
            return trimmed[first_word.len()..].trim_start();
        }
    }
    line
}

fn find_gosub_targets(source: &str) -> std::collections::HashSet<u32> {
    let mut targets = std::collections::HashSet::new();

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
