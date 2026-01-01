use tower_lsp::lsp_types::*;

/// Get signature help for functions at cursor position
pub fn get_signature_help(source: &str, position: Position) -> Option<SignatureHelp> {
    let lines: Vec<&str> = source.lines().collect();
    let line = lines.get(position.line as usize)?;
    let char_pos = position.character as usize;

    // Find the function call we're inside
    let before_cursor = &line[..char_pos.min(line.len())];

    // Look backwards for an open paren and function name
    let mut paren_depth = 0;
    let mut func_end = None;

    for (i, c) in before_cursor.chars().rev().enumerate() {
        match c {
            ')' => paren_depth += 1,
            '(' => {
                if paren_depth == 0 {
                    func_end = Some(before_cursor.len() - i - 1);
                    break;
                }
                paren_depth -= 1;
            }
            _ => {}
        }
    }

    let func_end = func_end?;

    // Extract function name
    let before_paren = &before_cursor[..func_end];
    let func_start = before_paren
        .rfind(|c: char| !c.is_ascii_alphanumeric() && c != '$' && c != '_')
        .map(|i| i + 1)
        .unwrap_or(0);

    let func_name = before_paren[func_start..].trim().to_uppercase();
    if func_name.is_empty() {
        return None;
    }

    // Count which parameter we're on
    let after_open = &before_cursor[func_end + 1..];
    let active_param = count_parameters(after_open);

    // Look up function signature
    get_function_signature(&func_name, active_param)
}

fn count_parameters(s: &str) -> u32 {
    let mut count = 0;
    let mut paren_depth = 0;

    for c in s.chars() {
        match c {
            '(' => paren_depth += 1,
            ')' => paren_depth -= 1,
            ',' if paren_depth == 0 => count += 1,
            _ => {}
        }
    }

    count
}

fn get_function_signature(name: &str, active_param: u32) -> Option<SignatureHelp> {
    let (label, params, doc) = match name {
        // String functions
        "CHR$" => (
            "CHR$(code)",
            vec!["code - ASCII code (0-255)"],
            "Returns character for ASCII code",
        ),
        "ASC" => (
            "ASC(string$)",
            vec!["string$ - String to get first character from"],
            "Returns ASCII code of first character",
        ),
        "LEN" => (
            "LEN(string$)",
            vec!["string$ - String to measure"],
            "Returns length of string",
        ),
        "LEFT$" => (
            "LEFT$(string$, count)",
            vec!["string$ - Source string", "count - Number of characters"],
            "Returns leftmost characters",
        ),
        "RIGHT$" => (
            "RIGHT$(string$, count)",
            vec!["string$ - Source string", "count - Number of characters"],
            "Returns rightmost characters",
        ),
        "MID$" => (
            "MID$(string$, start[, length])",
            vec![
                "string$ - Source string",
                "start - Starting position (1-based)",
                "length - Number of characters (optional)",
            ],
            "Returns substring",
        ),
        "STR$" => (
            "STR$(number)",
            vec!["number - Number to convert"],
            "Converts number to string",
        ),
        "VAL" => (
            "VAL(string$)",
            vec!["string$ - String to parse"],
            "Converts string to number",
        ),
        "STRING$" => (
            "STRING$(count, char)",
            vec![
                "count - Number of repetitions",
                "char - Character or ASCII code",
            ],
            "Returns repeated character",
        ),
        "SPACE$" => (
            "SPACE$(count)",
            vec!["count - Number of spaces"],
            "Returns string of spaces",
        ),
        "INSTR" => (
            "INSTR([start,] string$, search$)",
            vec![
                "start - Starting position (optional)",
                "string$ - String to search in",
                "search$ - String to find",
            ],
            "Returns position of substring",
        ),
        "UCASE$" => (
            "UCASE$(string$)",
            vec!["string$ - String to convert"],
            "Converts to uppercase",
        ),
        "LCASE$" => (
            "LCASE$(string$)",
            vec!["string$ - String to convert"],
            "Converts to lowercase",
        ),
        "LTRIM$" => (
            "LTRIM$(string$)",
            vec!["string$ - String to trim"],
            "Removes leading spaces",
        ),
        "RTRIM$" => (
            "RTRIM$(string$)",
            vec!["string$ - String to trim"],
            "Removes trailing spaces",
        ),
        "HEX$" => (
            "HEX$(number)",
            vec!["number - Number to convert"],
            "Converts to hexadecimal string",
        ),
        "OCT$" => (
            "OCT$(number)",
            vec!["number - Number to convert"],
            "Converts to octal string",
        ),

        // Math functions
        "ABS" => (
            "ABS(number)",
            vec!["number - Number to get absolute value of"],
            "Returns absolute value",
        ),
        "SGN" => (
            "SGN(number)",
            vec!["number - Number to check"],
            "Returns sign (-1, 0, or 1)",
        ),
        "INT" => (
            "INT(number)",
            vec!["number - Number to floor"],
            "Returns largest integer <= number",
        ),
        "FIX" => (
            "FIX(number)",
            vec!["number - Number to truncate"],
            "Truncates toward zero",
        ),
        "CINT" => (
            "CINT(number)",
            vec!["number - Number to round"],
            "Rounds to nearest integer",
        ),
        "SQR" => (
            "SQR(number)",
            vec!["number - Non-negative number"],
            "Returns square root",
        ),
        "SIN" => (
            "SIN(angle)",
            vec!["angle - Angle in radians"],
            "Returns sine",
        ),
        "COS" => (
            "COS(angle)",
            vec!["angle - Angle in radians"],
            "Returns cosine",
        ),
        "TAN" => (
            "TAN(angle)",
            vec!["angle - Angle in radians"],
            "Returns tangent",
        ),
        "ATN" => (
            "ATN(number)",
            vec!["number - Value"],
            "Returns arctangent in radians",
        ),
        "LOG" => (
            "LOG(number)",
            vec!["number - Positive number"],
            "Returns natural logarithm",
        ),
        "EXP" => (
            "EXP(number)",
            vec!["number - Exponent"],
            "Returns e raised to power",
        ),
        "RND" => (
            "RND[(seed)]",
            vec!["seed - Optional seed value"],
            "Returns random number 0-1",
        ),

        // Screen/graphics
        "POINT" => (
            "POINT(x, y)",
            vec!["x - X coordinate", "y - Y coordinate"],
            "Returns color at pixel",
        ),
        "CSRLIN" => ("CSRLIN", vec![], "Returns cursor row"),
        "POS" => (
            "POS(dummy)",
            vec!["dummy - Ignored value"],
            "Returns cursor column",
        ),
        "TAB" => (
            "TAB(column)",
            vec!["column - Column to move to"],
            "Moves to column in PRINT",
        ),
        "SPC" => (
            "SPC(count)",
            vec!["count - Number of spaces"],
            "Outputs spaces in PRINT",
        ),

        // I/O
        "EOF" => (
            "EOF(filenum)",
            vec!["filenum - File number"],
            "Returns true if at end of file",
        ),
        "PEEK" => (
            "PEEK(address)",
            vec!["address - Memory address"],
            "Returns byte at address",
        ),
        "TIMER" => ("TIMER", vec![], "Returns seconds since midnight"),

        _ => return None,
    };

    let parameters: Vec<ParameterInformation> = params
        .iter()
        .map(|p| ParameterInformation {
            label: ParameterLabel::Simple(p.split(" - ").next().unwrap_or(p).to_string()),
            documentation: Some(Documentation::String(p.to_string())),
        })
        .collect();

    Some(SignatureHelp {
        signatures: vec![SignatureInformation {
            label: label.to_string(),
            documentation: Some(Documentation::String(doc.to_string())),
            parameters: Some(parameters),
            active_parameter: Some(active_param),
        }],
        active_signature: Some(0),
        active_parameter: Some(active_param),
    })
}
