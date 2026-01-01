use std::collections::HashSet;
use tower_lsp::lsp_types::*;

/// Get completion items at the cursor position
pub fn get_completions(source: &str, _position: Position) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    // Add keywords
    for (keyword, detail) in KEYWORDS {
        items.push(CompletionItem {
            label: keyword.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(detail.to_string()),
            ..Default::default()
        });
    }

    // Add built-in functions
    for (func, detail) in FUNCTIONS {
        items.push(CompletionItem {
            label: func.to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some(detail.to_string()),
            ..Default::default()
        });
    }

    // Add variables found in the document
    let variables = extract_variables(source);
    for var in variables {
        let kind = if var.ends_with('$') {
            CompletionItemKind::VARIABLE
        } else if var.contains('(') {
            CompletionItemKind::FIELD // Array
        } else {
            CompletionItemKind::VARIABLE
        };

        items.push(CompletionItem {
            label: var,
            kind: Some(kind),
            detail: Some("Variable".to_string()),
            ..Default::default()
        });
    }

    items
}

/// Extract variable names from source
fn extract_variables(source: &str) -> Vec<String> {
    let mut vars = HashSet::new();

    for line in source.lines() {
        let upper = line.to_uppercase();

        // Skip line number
        let content = skip_line_number(&upper);

        // Look for assignments: VAR = or LET VAR =
        for part in content.split(':') {
            let part = part.trim();

            // LET VAR = ...
            if let Some(rest) = part.strip_prefix("LET ") {
                if let Some(var) = extract_var_name(rest) {
                    vars.insert(var);
                }
            }
            // VAR = ... (implicit LET)
            else if let Some(eq_pos) = part.find('=') {
                let before_eq = part[..eq_pos].trim();
                // Make sure it's not a comparison (inside IF)
                if !part.starts_with("IF ") && !before_eq.contains(' ') {
                    if let Some(var) = extract_var_name(before_eq) {
                        vars.insert(var);
                    }
                }
            }

            // DIM VAR(...)
            if let Some(rest) = part.strip_prefix("DIM ") {
                for dim_part in rest.split(',') {
                    if let Some(var) = extract_var_name(dim_part.trim()) {
                        vars.insert(var);
                    }
                }
            }

            // FOR VAR = ...
            if let Some(rest) = part.strip_prefix("FOR ") {
                if let Some(var) = extract_var_name(rest) {
                    vars.insert(var);
                }
            }

            // INPUT VAR or INPUT "prompt"; VAR
            if let Some(rest) = part.strip_prefix("INPUT ") {
                let vars_part = if let Some(semi) = rest.find(';') {
                    &rest[semi + 1..]
                } else {
                    rest
                };
                for input_var in vars_part.split(',') {
                    if let Some(var) = extract_var_name(input_var.trim()) {
                        vars.insert(var);
                    }
                }
            }

            // READ VAR1, VAR2
            if let Some(rest) = part.strip_prefix("READ ") {
                for read_var in rest.split(',') {
                    if let Some(var) = extract_var_name(read_var.trim()) {
                        vars.insert(var);
                    }
                }
            }
        }
    }

    vars.into_iter().collect()
}

/// Extract variable name from start of string (handles arrays)
fn extract_var_name(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    // Find the variable name (letters, digits, underscore, and optional $)
    let mut end = 0;
    let mut chars = s.chars().peekable();

    // First char must be letter
    if let Some(c) = chars.next() {
        if !c.is_ascii_alphabetic() {
            return None;
        }
        end += 1;
    }

    // Rest can be alphanumeric or underscore
    while let Some(&c) = chars.peek() {
        if c.is_ascii_alphanumeric() || c == '_' {
            end += 1;
            chars.next();
        } else if c == '$' {
            end += 1;
            chars.next();
            break;
        } else {
            break;
        }
    }

    if end > 0 {
        Some(s[..end].to_string())
    } else {
        None
    }
}

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

const KEYWORDS: &[(&str, &str)] = &[
    ("IF", "Conditional execution"),
    ("THEN", "Part of IF statement"),
    ("ELSE", "Alternative branch"),
    ("FOR", "Counted loop"),
    ("TO", "Loop end value"),
    ("STEP", "Loop increment"),
    ("NEXT", "End of FOR loop"),
    ("WHILE", "Conditional loop"),
    ("WEND", "End of WHILE"),
    ("DO", "DO...LOOP block"),
    ("LOOP", "End of DO block"),
    ("UNTIL", "Loop exit condition"),
    ("EXIT", "Exit loop early"),
    ("GOTO", "Jump to line"),
    ("GOSUB", "Call subroutine"),
    ("RETURN", "Return from subroutine"),
    ("ON", "Computed GOTO/GOSUB"),
    ("SELECT", "Multi-way branch"),
    ("CASE", "Branch option"),
    ("END", "End program"),
    ("STOP", "Halt execution"),
    ("LET", "Variable assignment"),
    ("DIM", "Declare array"),
    ("PRINT", "Output to screen"),
    ("LPRINT", "Output to printer"),
    ("INPUT", "Read user input"),
    ("LINE", "LINE INPUT statement"),
    ("READ", "Read from DATA"),
    ("DATA", "Define data values"),
    ("RESTORE", "Reset DATA pointer"),
    ("REM", "Comment"),
    ("OPEN", "Open file"),
    ("CLOSE", "Close file"),
    ("KILL", "Delete file"),
    ("NAME", "Rename file"),
    ("MKDIR", "Create directory"),
    ("RMDIR", "Remove directory"),
    ("CHDIR", "Change directory"),
    ("FILES", "List files"),
    ("SCREEN", "Set screen mode"),
    ("COLOR", "Set colors"),
    ("CLS", "Clear screen"),
    ("LOCATE", "Position cursor"),
    ("WIDTH", "Set screen width"),
    ("CIRCLE", "Draw circle"),
    ("LINE", "Draw line"),
    ("PAINT", "Flood fill"),
    ("PSET", "Set pixel"),
    ("PRESET", "Clear pixel"),
    ("DRAW", "Turtle graphics"),
    ("GET", "Capture sprite"),
    ("PUT", "Draw sprite"),
    ("PLAY", "Play music"),
    ("SOUND", "Play tone"),
    ("BEEP", "System beep"),
    ("DEF", "Define function"),
    ("SWAP", "Exchange variables"),
    ("RANDOMIZE", "Seed RNG"),
    ("CLEAR", "Clear variables"),
    ("POKE", "Write to memory"),
    ("AND", "Logical AND"),
    ("OR", "Logical OR"),
    ("XOR", "Logical XOR"),
    ("NOT", "Logical NOT"),
    ("MOD", "Modulo operator"),
];

const FUNCTIONS: &[(&str, &str)] = &[
    ("CHR$", "Character from ASCII code"),
    ("ASC", "ASCII code of character"),
    ("LEN", "String length"),
    ("LEFT$", "Leftmost characters"),
    ("RIGHT$", "Rightmost characters"),
    ("MID$", "Substring"),
    ("STR$", "Number to string"),
    ("VAL", "String to number"),
    ("STRING$", "Repeat character"),
    ("SPACE$", "String of spaces"),
    ("INSTR", "Find substring"),
    ("UCASE$", "Uppercase"),
    ("LCASE$", "Lowercase"),
    ("LTRIM$", "Trim left spaces"),
    ("RTRIM$", "Trim right spaces"),
    ("HEX$", "Hexadecimal string"),
    ("OCT$", "Octal string"),
    ("ABS", "Absolute value"),
    ("SGN", "Sign of number"),
    ("INT", "Integer part (floor)"),
    ("FIX", "Truncate to integer"),
    ("CINT", "Round to integer"),
    ("SQR", "Square root"),
    ("SIN", "Sine"),
    ("COS", "Cosine"),
    ("TAN", "Tangent"),
    ("ATN", "Arctangent"),
    ("LOG", "Natural logarithm"),
    ("EXP", "Exponential"),
    ("RND", "Random number"),
    ("PEEK", "Read memory"),
    ("TIMER", "Seconds since midnight"),
    ("DATE$", "Current date"),
    ("TIME$", "Current time"),
    ("INKEY$", "Read key (no wait)"),
    ("EOF", "End of file check"),
    ("CSRLIN", "Cursor row"),
    ("POS", "Cursor column"),
    ("POINT", "Pixel color"),
    ("TAB", "Move to column"),
    ("SPC", "Output spaces"),
    ("FN", "User-defined function"),
];
