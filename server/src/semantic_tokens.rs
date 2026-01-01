use tower_lsp::lsp_types::*;

/// Token types for semantic highlighting
pub const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::KEYWORD,
    SemanticTokenType::FUNCTION,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::STRING,
    SemanticTokenType::NUMBER,
    SemanticTokenType::COMMENT,
    SemanticTokenType::OPERATOR,
];

/// Token modifiers
pub const TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[
    SemanticTokenModifier::DECLARATION,
    SemanticTokenModifier::DEFINITION,
];

const TYPE_KEYWORD: u32 = 0;
const TYPE_FUNCTION: u32 = 1;
const TYPE_VARIABLE: u32 = 2;
const TYPE_STRING: u32 = 3;
const TYPE_NUMBER: u32 = 4;
const TYPE_COMMENT: u32 = 5;
const TYPE_OPERATOR: u32 = 6;

/// Get semantic tokens for a document
pub fn get_semantic_tokens(source: &str) -> SemanticTokensResult {
    let mut tokens = Vec::new();
    let mut prev_line = 0u32;
    let mut prev_char = 0u32;

    for (line_idx, line) in source.lines().enumerate() {
        let line_num = line_idx as u32;
        let upper = line.to_uppercase();

        // Skip leading whitespace
        let trimmed_start = line.len() - line.trim_start().len();
        let mut char_pos = trimmed_start;

        // Check for line number at start
        let trimmed = line.trim_start();
        if let Some(first_word) = trimmed.split_whitespace().next() {
            if first_word.parse::<u32>().is_ok() {
                // Line number token
                add_token(
                    &mut tokens,
                    &mut prev_line,
                    &mut prev_char,
                    line_num,
                    char_pos as u32,
                    first_word.len() as u32,
                    TYPE_NUMBER,
                    0,
                );
                char_pos += first_word.len();
            }
        }

        // Check for REM comment
        let after_linenum = &upper[char_pos..];
        if after_linenum.trim_start().starts_with("REM")
            || after_linenum.trim_start().starts_with("'")
        {
            let comment_start = char_pos + (after_linenum.len() - after_linenum.trim_start().len());
            add_token(
                &mut tokens,
                &mut prev_line,
                &mut prev_char,
                line_num,
                comment_start as u32,
                (line.len() - comment_start) as u32,
                TYPE_COMMENT,
                0,
            );
            continue;
        }

        // Tokenize the rest of the line
        tokenize_line(
            line,
            &upper,
            char_pos,
            line_num,
            &mut tokens,
            &mut prev_line,
            &mut prev_char,
        );
    }

    SemanticTokensResult::Tokens(SemanticTokens {
        result_id: None,
        data: tokens,
    })
}

fn tokenize_line(
    line: &str,
    upper: &str,
    start_pos: usize,
    line_num: u32,
    tokens: &mut Vec<SemanticToken>,
    prev_line: &mut u32,
    prev_char: &mut u32,
) {
    let bytes = line.as_bytes();
    let mut pos = start_pos;

    while pos < line.len() {
        let b = bytes[pos];

        // Skip whitespace
        if b.is_ascii_whitespace() {
            pos += 1;
            continue;
        }

        // String literal
        if b == b'"' {
            let start = pos;
            pos += 1;
            while pos < line.len() && bytes[pos] != b'"' {
                pos += 1;
            }
            if pos < line.len() {
                pos += 1; // Include closing quote
            }
            add_token(
                tokens,
                prev_line,
                prev_char,
                line_num,
                start as u32,
                (pos - start) as u32,
                TYPE_STRING,
                0,
            );
            continue;
        }

        // Number (including hex &H)
        if b.is_ascii_digit() || (b == b'&' && pos + 1 < line.len() && bytes[pos + 1] == b'H') {
            let start = pos;
            if b == b'&' {
                pos += 2; // Skip &H
                while pos < line.len() && bytes[pos].is_ascii_hexdigit() {
                    pos += 1;
                }
            } else {
                while pos < line.len()
                    && (bytes[pos].is_ascii_digit()
                        || bytes[pos] == b'.'
                        || bytes[pos] == b'E'
                        || bytes[pos] == b'e'
                        || bytes[pos] == b'-'
                        || bytes[pos] == b'+')
                {
                    // Handle scientific notation carefully
                    if (bytes[pos] == b'-' || bytes[pos] == b'+')
                        && pos > start
                        && bytes[pos - 1] != b'E'
                        && bytes[pos - 1] != b'e'
                    {
                        break;
                    }
                    pos += 1;
                }
            }
            add_token(
                tokens,
                prev_line,
                prev_char,
                line_num,
                start as u32,
                (pos - start) as u32,
                TYPE_NUMBER,
                0,
            );
            continue;
        }

        // Identifier (keyword, function, or variable)
        if b.is_ascii_alphabetic() || b == b'_' {
            let start = pos;
            while pos < line.len()
                && (bytes[pos].is_ascii_alphanumeric() || bytes[pos] == b'_' || bytes[pos] == b'$')
            {
                pos += 1;
            }

            let word = &upper[start..pos];
            let token_type = if is_keyword(word) {
                TYPE_KEYWORD
            } else if is_function(word) {
                TYPE_FUNCTION
            } else {
                TYPE_VARIABLE
            };

            add_token(
                tokens,
                prev_line,
                prev_char,
                line_num,
                start as u32,
                (pos - start) as u32,
                token_type,
                0,
            );
            continue;
        }

        // Operators
        if is_operator(b) {
            add_token(
                tokens,
                prev_line,
                prev_char,
                line_num,
                pos as u32,
                1,
                TYPE_OPERATOR,
                0,
            );
        }

        pos += 1;
    }
}

fn add_token(
    tokens: &mut Vec<SemanticToken>,
    prev_line: &mut u32,
    prev_char: &mut u32,
    line: u32,
    char_pos: u32,
    length: u32,
    token_type: u32,
    token_modifiers: u32,
) {
    let delta_line = line - *prev_line;
    let delta_start = if delta_line == 0 {
        char_pos - *prev_char
    } else {
        char_pos
    };

    tokens.push(SemanticToken {
        delta_line,
        delta_start,
        length,
        token_type,
        token_modifiers_bitset: token_modifiers,
    });

    *prev_line = line;
    *prev_char = char_pos;
}

fn is_operator(b: u8) -> bool {
    matches!(
        b,
        b'+' | b'-' | b'*' | b'/' | b'^' | b'=' | b'<' | b'>' | b'(' | b')' | b',' | b';' | b':'
    )
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
    )
}
