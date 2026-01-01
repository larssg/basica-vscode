use basica::lexer::Lexer;
use basica::parser::Parser;
use std::collections::{HashMap, HashSet};
use tower_lsp::lsp_types::*;

/// Check source code for parse errors and warnings
pub fn check(source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // First check for parse errors
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();

    let mut parser = Parser::new(tokens);
    match parser.parse() {
        Ok(_) => {
            // No parse errors, check for warnings
            diagnostics.extend(check_warnings(source));
        }
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

            diagnostics.push(Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("basica".to_string()),
                message,
                ..Default::default()
            });
        }
    }

    diagnostics
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

/// Check for warnings (undefined vars, unused vars, unreachable code)
fn check_warnings(source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Track variable definitions and usages
    let (definitions, usages) = analyze_variables(source);

    // Check for undefined variables (used but never defined)
    for (var, locations) in &usages {
        if !definitions.contains_key(var) && !is_builtin_var(var) {
            for &(line_idx, char_start, char_end) in locations {
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line: line_idx,
                            character: char_start,
                        },
                        end: Position {
                            line: line_idx,
                            character: char_end,
                        },
                    },
                    severity: Some(DiagnosticSeverity::WARNING),
                    source: Some("basica".to_string()),
                    message: format!("Variable '{}' may not be defined", var),
                    ..Default::default()
                });
            }
        }
    }

    // Check for unused variables (defined but never used)
    for (var, locations) in &definitions {
        if !usages.contains_key(var) {
            // Only warn for first definition
            if let Some(&(line_idx, char_start, char_end)) = locations.first() {
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line: line_idx,
                            character: char_start,
                        },
                        end: Position {
                            line: line_idx,
                            character: char_end,
                        },
                    },
                    severity: Some(DiagnosticSeverity::HINT),
                    source: Some("basica".to_string()),
                    message: format!("Variable '{}' is defined but never used", var),
                    tags: Some(vec![DiagnosticTag::UNNECESSARY]),
                    ..Default::default()
                });
            }
        }
    }

    // Check for unreachable code
    diagnostics.extend(check_unreachable_code(source));

    // Check for undefined line numbers in GOTO/GOSUB
    diagnostics.extend(check_undefined_lines(source));

    diagnostics
}

/// Analyze variable definitions and usages
fn analyze_variables(
    source: &str,
) -> (
    HashMap<String, Vec<(u32, u32, u32)>>,
    HashMap<String, Vec<(u32, u32, u32)>>,
) {
    let mut definitions: HashMap<String, Vec<(u32, u32, u32)>> = HashMap::new();
    let mut usages: HashMap<String, Vec<(u32, u32, u32)>> = HashMap::new();

    for (line_idx, line) in source.lines().enumerate() {
        let line_num = line_idx as u32;
        let upper = line.to_uppercase();

        // Skip line number
        let content = skip_line_number(&upper);
        let offset = (upper.len() - content.len()) as u32;

        // Process each statement (separated by :)
        for part in content.split(':') {
            let part = part.trim();

            // Track definitions: LET X = ..., X = ..., DIM X, FOR X = ..., INPUT X, READ X
            if let Some(rest) = part.strip_prefix("LET ") {
                if let Some((var, pos)) = extract_var_with_pos(rest) {
                    definitions.entry(var).or_default().push((
                        line_num,
                        offset + pos,
                        offset + pos + rest.find('=').unwrap_or(rest.len()) as u32,
                    ));
                }
            } else if let Some(rest) = part.strip_prefix("DIM ") {
                for dim_part in rest.split(',') {
                    if let Some((var, _)) = extract_var_with_pos(dim_part.trim()) {
                        let start = upper.find(dim_part).unwrap_or(0) as u32;
                        definitions.entry(var.clone()).or_default().push((
                            line_num,
                            start,
                            start + var.len() as u32,
                        ));
                    }
                }
            } else if let Some(rest) = part.strip_prefix("FOR ") {
                if let Some((var, _)) = extract_var_with_pos(rest) {
                    let start = upper.find(&var).unwrap_or(0) as u32;
                    definitions.entry(var.clone()).or_default().push((
                        line_num,
                        start,
                        start + var.len() as u32,
                    ));
                }
            } else if let Some(rest) = part.strip_prefix("INPUT ") {
                let vars_part = if let Some(semi) = rest.find(';') {
                    &rest[semi + 1..]
                } else {
                    rest
                };
                for input_var in vars_part.split(',') {
                    if let Some((var, _)) = extract_var_with_pos(input_var.trim()) {
                        let start = upper.find(&var).unwrap_or(0) as u32;
                        definitions.entry(var.clone()).or_default().push((
                            line_num,
                            start,
                            start + var.len() as u32,
                        ));
                    }
                }
            } else if let Some(rest) = part.strip_prefix("READ ") {
                for read_var in rest.split(',') {
                    if let Some((var, _)) = extract_var_with_pos(read_var.trim()) {
                        let start = upper.find(&var).unwrap_or(0) as u32;
                        definitions.entry(var.clone()).or_default().push((
                            line_num,
                            start,
                            start + var.len() as u32,
                        ));
                    }
                }
            } else if !part.starts_with("IF ")
                && !part.starts_with("PRINT")
                && !part.starts_with("GOTO")
                && !part.starts_with("GOSUB")
            {
                // Check for implicit LET: VAR = ...
                if let Some(eq_pos) = part.find('=') {
                    let before_eq = part[..eq_pos].trim();
                    if !before_eq.contains(' ') && !before_eq.is_empty() {
                        if let Some((var, _)) = extract_var_with_pos(before_eq) {
                            let start = upper.find(&var).unwrap_or(0) as u32;
                            definitions.entry(var.clone()).or_default().push((
                                line_num,
                                start,
                                start + var.len() as u32,
                            ));
                        }
                    }
                }
            }

            // Track usages (any variable reference that's not a definition site)
            // This is simplified - we look for all variables in the line
            find_variable_usages(&upper, line_num, &definitions, &mut usages);
        }
    }

    (definitions, usages)
}

fn find_variable_usages(
    line: &str,
    line_num: u32,
    definitions: &HashMap<String, Vec<(u32, u32, u32)>>,
    usages: &mut HashMap<String, Vec<(u32, u32, u32)>>,
) {
    let bytes = line.as_bytes();
    let mut pos = 0;

    while pos < bytes.len() {
        // Skip non-alphabetic
        if !bytes[pos].is_ascii_alphabetic() {
            pos += 1;
            continue;
        }

        // Extract identifier
        let start = pos;
        while pos < bytes.len()
            && (bytes[pos].is_ascii_alphanumeric() || bytes[pos] == b'_' || bytes[pos] == b'$')
        {
            pos += 1;
        }

        let word = &line[start..pos];
        if !is_keyword(word) && !is_function(word) && word.len() > 0 {
            // Skip if this position is a definition site
            let is_def_site = definitions.get(word).map_or(false, |locs| {
                locs.iter()
                    .any(|&(l, s, _)| l == line_num && s == start as u32)
            });

            if !is_def_site {
                usages.entry(word.to_string()).or_default().push((
                    line_num,
                    start as u32,
                    pos as u32,
                ));
            }
        }
    }
}

fn extract_var_with_pos(s: &str) -> Option<(String, u32)> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let bytes = s.as_bytes();
    if !bytes[0].is_ascii_alphabetic() {
        return None;
    }

    let mut end = 1;
    while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        end += 1;
    }
    if end < bytes.len() && bytes[end] == b'$' {
        end += 1;
    }

    Some((s[..end].to_string(), 0))
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

fn is_builtin_var(var: &str) -> bool {
    matches!(var, "TIMER" | "DATE$" | "TIME$" | "INKEY$" | "ERR" | "ERL")
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
            | "AS"
            | "OUTPUT"
            | "APPEND"
            | "RANDOM"
            | "BINARY"
            | "OPEN"
            | "CLOSE"
            | "GET"
            | "PUT"
            | "WRITE"
            | "FIELD"
            | "LSET"
            | "RSET"
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
            | "SUB"
            | "USING"
    )
}

fn is_function(word: &str) -> bool {
    matches!(
        word,
        "CHR$"
            | "ASC"
            | "LEN"
            | "LEFT$"
            | "RIGHT$"
            | "MID$"
            | "STR$"
            | "VAL"
            | "STRING$"
            | "SPACE$"
            | "INSTR"
            | "UCASE$"
            | "LCASE$"
            | "LTRIM$"
            | "RTRIM$"
            | "HEX$"
            | "OCT$"
            | "ABS"
            | "SGN"
            | "INT"
            | "FIX"
            | "CINT"
            | "SQR"
            | "SIN"
            | "COS"
            | "TAN"
            | "ATN"
            | "LOG"
            | "EXP"
            | "RND"
            | "PEEK"
            | "TIMER"
            | "DATE$"
            | "TIME$"
            | "INKEY$"
            | "EOF"
            | "CSRLIN"
            | "POS"
            | "POINT"
            | "TAB"
            | "SPC"
            | "LOF"
            | "LOC"
            | "FRE"
            | "VARPTR"
            | "VARPTR$"
            | "SADD"
    )
}

/// Check for unreachable code after END, STOP, or unconditional GOTO
fn check_unreachable_code(source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut unreachable_start: Option<u32> = None;

    // Build set of line numbers that are jump targets
    let jump_targets = find_jump_targets(source);

    for (line_idx, line) in lines.iter().enumerate() {
        let line_num = line_idx as u32;
        let upper = line.to_uppercase();
        let content = skip_line_number(&upper).trim();

        // Check if this line is a jump target - makes it reachable
        if let Some(first_word) = line.trim_start().split_whitespace().next() {
            if let Ok(basic_line) = first_word.parse::<u32>() {
                if jump_targets.contains(&basic_line) {
                    // This line is a jump target, end any unreachable region
                    if let Some(start) = unreachable_start.take() {
                        if line_num > start + 1 {
                            diagnostics.push(Diagnostic {
                                range: Range {
                                    start: Position {
                                        line: start + 1,
                                        character: 0,
                                    },
                                    end: Position {
                                        line: line_num - 1,
                                        character: 1000,
                                    },
                                },
                                severity: Some(DiagnosticSeverity::HINT),
                                source: Some("basica".to_string()),
                                message: "Unreachable code".to_string(),
                                tags: Some(vec![DiagnosticTag::UNNECESSARY]),
                                ..Default::default()
                            });
                        }
                    }
                }
            }
        }

        // Skip empty lines and comments
        if content.is_empty() || content.starts_with("REM") || content.starts_with("'") {
            continue;
        }

        // Check if we're in unreachable code
        if unreachable_start.is_some() {
            continue;
        }

        // Check for statements that make following code unreachable
        // END, STOP, or unconditional GOTO/RETURN at end of line
        let makes_unreachable = content == "END"
            || content == "STOP"
            || content == "RETURN"
            || (content.starts_with("GOTO ") && !upper.contains("IF ") && !upper.contains("ON "));

        if makes_unreachable {
            unreachable_start = Some(line_num);
        }
    }

    diagnostics
}

/// Check for GOTO/GOSUB to undefined line numbers
fn check_undefined_lines(source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Build set of defined line numbers
    let mut defined_lines = HashSet::new();
    for line in source.lines() {
        if let Some(first_word) = line.trim_start().split_whitespace().next() {
            if let Ok(num) = first_word.parse::<u32>() {
                defined_lines.insert(num);
            }
        }
    }

    // Check GOTO/GOSUB targets
    for (line_idx, line) in source.lines().enumerate() {
        let upper = line.to_uppercase();

        for keyword in &["GOTO ", "GOSUB ", "THEN ", "RESTORE "] {
            let mut search_start = 0;
            while let Some(kw_pos) = upper[search_start..].find(keyword) {
                let abs_pos = search_start + kw_pos + keyword.len();
                let after = &line[abs_pos..];

                // Parse line numbers
                for num_part in after.split(',') {
                    let num_str = num_part.trim().split_whitespace().next().unwrap_or("");
                    if let Ok(target) = num_str.parse::<u32>() {
                        if !defined_lines.contains(&target) {
                            let char_start =
                                abs_pos + (num_part.len() - num_part.trim_start().len());
                            diagnostics.push(Diagnostic {
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
                                severity: Some(DiagnosticSeverity::ERROR),
                                source: Some("basica".to_string()),
                                message: format!("Line {} is not defined", target),
                                ..Default::default()
                            });
                        }
                    }
                    // Stop if we hit a non-number
                    if num_str.parse::<u32>().is_err() {
                        break;
                    }
                }

                search_start = abs_pos;
            }
        }
    }

    diagnostics
}

/// Find all line numbers that are jump targets
fn find_jump_targets(source: &str) -> HashSet<u32> {
    let mut targets = HashSet::new();

    for line in source.lines() {
        let upper = line.to_uppercase();

        for keyword in &["GOTO ", "GOSUB ", "THEN ", "RESTORE "] {
            let mut search_start = 0;
            while let Some(kw_pos) = upper[search_start..].find(keyword) {
                let abs_pos = search_start + kw_pos + keyword.len();
                let after = &upper[abs_pos..];

                for num_part in after.split(',') {
                    let num_str = num_part.trim().split_whitespace().next().unwrap_or("");
                    if let Ok(target) = num_str.parse::<u32>() {
                        targets.insert(target);
                    }
                    if num_str.parse::<u32>().is_err() {
                        break;
                    }
                }

                search_start = abs_pos;
            }
        }
    }

    targets
}
