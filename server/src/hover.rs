use tower_lsp::lsp_types::*;

/// Get hover documentation for keyword/function at cursor position
pub fn get_hover(source: &str, position: Position) -> Option<Hover> {
    let lines: Vec<&str> = source.lines().collect();
    let line = lines.get(position.line as usize)?;
    let word = get_word_at_position(line, position.character as usize)?;

    let doc = get_documentation(&word.to_uppercase())?;

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: doc.to_string(),
        }),
        range: None,
    })
}

/// Get word at cursor position (including $ suffix for string functions)
fn get_word_at_position(line: &str, char_pos: usize) -> Option<&str> {
    let bytes = line.as_bytes();
    let char_pos = char_pos.min(bytes.len());

    // Find start of word
    let mut start = char_pos;
    while start > 0
        && bytes
            .get(start - 1)
            .map(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'$')
            .unwrap_or(false)
    {
        start -= 1;
    }

    // Find end of word
    let mut end = char_pos;
    while end < bytes.len()
        && bytes
            .get(end)
            .map(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'$')
            .unwrap_or(false)
    {
        end += 1;
    }

    if start < end {
        Some(&line[start..end])
    } else {
        None
    }
}

/// Get documentation for a keyword or function
fn get_documentation(keyword: &str) -> Option<&'static str> {
    // Strip $ suffix for lookup
    let key = keyword.trim_end_matches('$');

    match key {
        // Control flow
        "IF" => Some("**IF** condition **THEN** statement [**ELSE** statement]\n\nConditional execution. If the condition is true, executes the THEN clause; otherwise executes the optional ELSE clause."),
        "THEN" => Some("**THEN**\n\nPart of IF...THEN...ELSE statement. Introduces the code to execute when the condition is true."),
        "ELSE" => Some("**ELSE**\n\nPart of IF...THEN...ELSE statement. Introduces the code to execute when the condition is false."),
        "FOR" => Some("**FOR** var **=** start **TO** end [**STEP** step]\n\nBegin a counted loop. The variable is initialized to start and incremented by step (default 1) until it exceeds end."),
        "TO" => Some("**TO**\n\nPart of FOR...TO...STEP statement. Specifies the ending value of the loop."),
        "STEP" => Some("**STEP** value\n\nOptional part of FOR loop. Specifies the increment (can be negative for counting down)."),
        "NEXT" => Some("**NEXT** [var]\n\nEnd of FOR loop. Increments the loop variable and continues if not past the end value."),
        "WHILE" => Some("**WHILE** condition\n\nBegin a conditional loop. Repeats while the condition is true."),
        "WEND" => Some("**WEND**\n\nEnd of WHILE loop. Returns to WHILE to re-check the condition."),
        "DO" => Some("**DO** [**WHILE**|**UNTIL** condition]\n\nBegin a DO...LOOP block. Can have condition at start or end."),
        "LOOP" => Some("**LOOP** [**WHILE**|**UNTIL** condition]\n\nEnd of DO...LOOP block. Can have condition at end."),
        "UNTIL" => Some("**UNTIL** condition\n\nLoop exit condition. Loop continues until the condition becomes true."),
        "EXIT" => Some("**EXIT** **DO** | **EXIT** **FOR**\n\nExit from the innermost DO or FOR loop."),
        "GOTO" => Some("**GOTO** line\n\nUnconditional jump to the specified line number."),
        "GOSUB" => Some("**GOSUB** line\n\nCall subroutine at line number. Use RETURN to come back."),
        "RETURN" => Some("**RETURN**\n\nReturn from subroutine to the statement after GOSUB."),
        "ON" => Some("**ON** expr **GOTO** line1, line2, ... | **ON** expr **GOSUB** line1, line2, ...\n\nComputed GOTO/GOSUB. Jumps to the nth line in the list based on the expression value."),
        "SELECT" => Some("**SELECT CASE** expr\n\nBegin a SELECT CASE block for multi-way branching."),
        "CASE" => Some("**CASE** value | **CASE** v1 **TO** v2 | **CASE IS** op value | **CASE ELSE**\n\nDefines a case in SELECT CASE block."),
        "END" => Some("**END**\n\nTerminate program execution."),
        "STOP" => Some("**STOP**\n\nHalt program execution (can be resumed in some implementations)."),

        // I/O
        "PRINT" => Some("**PRINT** [expr] [; | ,] ...\n\nOutput to screen. Semicolon continues on same line; comma moves to next tab zone."),
        "LPRINT" => Some("**LPRINT** [expr] [; | ,] ...\n\nOutput to printer. Same format as PRINT."),
        "INPUT" => Some("**INPUT** [\"prompt\";] var1 [, var2, ...]\n\nRead input from user. Displays optional prompt and waits for keyboard input."),
        "LINE" => Some("**LINE INPUT** [\"prompt\";] var$\n\nRead entire line of input including commas into string variable."),
        "READ" => Some("**READ** var1 [, var2, ...]\n\nRead values from DATA statements into variables."),
        "DATA" => Some("**DATA** value1, value2, ...\n\nDefine constant data to be read by READ statements."),
        "RESTORE" => Some("**RESTORE** [line]\n\nReset DATA pointer to beginning or to specified line."),

        // Variables and arrays
        "LET" => Some("**LET** var = expr | var = expr\n\nAssign value to variable. LET keyword is optional."),
        "DIM" => Some("**DIM** array(size) [, array2(size), ...]\n\nDeclare array dimensions. Arrays are 0-indexed by default."),
        "SWAP" => Some("**SWAP** var1, var2\n\nExchange values of two variables."),

        // Functions - String
        "CHR" => Some("**CHR$(n)**\n\nReturns the character with ASCII code n.\n\nExample: `CHR$(65)` returns `\"A\"`"),
        "ASC" => Some("**ASC(string$)**\n\nReturns the ASCII code of the first character.\n\nExample: `ASC(\"A\")` returns `65`"),
        "LEN" => Some("**LEN(string$)**\n\nReturns the length of the string.\n\nExample: `LEN(\"Hello\")` returns `5`"),
        "LEFT" => Some("**LEFT$(string$, n)**\n\nReturns the leftmost n characters.\n\nExample: `LEFT$(\"Hello\", 2)` returns `\"He\"`"),
        "RIGHT" => Some("**RIGHT$(string$, n)**\n\nReturns the rightmost n characters.\n\nExample: `RIGHT$(\"Hello\", 2)` returns `\"lo\"`"),
        "MID" => Some("**MID$(string$, start [, length])**\n\nReturns substring starting at position start (1-based).\n\nExample: `MID$(\"Hello\", 2, 3)` returns `\"ell\"`"),
        "STR" => Some("**STR$(n)**\n\nConverts number to string.\n\nExample: `STR$(42)` returns `\" 42\"` (with leading space for positive)"),
        "VAL" => Some("**VAL(string$)**\n\nConverts string to number.\n\nExample: `VAL(\"3.14\")` returns `3.14`"),
        "STRING" => Some("**STRING$(n, char)**\n\nReturns string of n copies of character.\n\nExample: `STRING$(5, 42)` returns `\"*****\"`"),
        "SPACE" => Some("**SPACE$(n)**\n\nReturns string of n spaces.\n\nExample: `SPACE$(5)` returns `\"     \"`"),
        "INSTR" => Some("**INSTR([start,] string1$, string2$)**\n\nReturns position of string2$ in string1$ (1-based, 0 if not found).\n\nExample: `INSTR(\"Hello\", \"ll\")` returns `3`"),
        "UCASE" => Some("**UCASE$(string$)**\n\nConverts string to uppercase.\n\nExample: `UCASE$(\"Hello\")` returns `\"HELLO\"`"),
        "LCASE" => Some("**LCASE$(string$)**\n\nConverts string to lowercase.\n\nExample: `LCASE$(\"Hello\")` returns `\"hello\"`"),
        "LTRIM" => Some("**LTRIM$(string$)**\n\nRemoves leading spaces.\n\nExample: `LTRIM$(\"  Hi\")` returns `\"Hi\"`"),
        "RTRIM" => Some("**RTRIM$(string$)**\n\nRemoves trailing spaces."),
        "HEX" => Some("**HEX$(n)**\n\nReturns hexadecimal representation of number.\n\nExample: `HEX$(255)` returns `\"FF\"`"),
        "OCT" => Some("**OCT$(n)**\n\nReturns octal representation of number."),

        // Functions - Math
        "ABS" => Some("**ABS(n)**\n\nReturns absolute value.\n\nExample: `ABS(-5)` returns `5`"),
        "SGN" => Some("**SGN(n)**\n\nReturns sign: -1 if negative, 0 if zero, 1 if positive."),
        "INT" => Some("**INT(n)**\n\nReturns largest integer not greater than n (floor).\n\nExample: `INT(3.7)` returns `3`, `INT(-3.7)` returns `-4`"),
        "FIX" => Some("**FIX(n)**\n\nReturns integer part (truncates toward zero).\n\nExample: `FIX(-3.7)` returns `-3`"),
        "CINT" => Some("**CINT(n)**\n\nConverts to integer with rounding."),
        "SQR" => Some("**SQR(n)**\n\nReturns square root.\n\nExample: `SQR(16)` returns `4`"),
        "SIN" => Some("**SIN(n)**\n\nReturns sine of angle in radians."),
        "COS" => Some("**COS(n)**\n\nReturns cosine of angle in radians."),
        "TAN" => Some("**TAN(n)**\n\nReturns tangent of angle in radians."),
        "ATN" => Some("**ATN(n)**\n\nReturns arctangent in radians."),
        "LOG" => Some("**LOG(n)**\n\nReturns natural logarithm (base e)."),
        "EXP" => Some("**EXP(n)**\n\nReturns e raised to the power n."),
        "RND" => Some("**RND** [(n)]\n\nReturns random number between 0 and 1.\n\nUse `RANDOMIZE` to seed the generator."),
        "RANDOMIZE" => Some("**RANDOMIZE** [seed]\n\nSeed the random number generator. Without argument, uses system time."),

        // Functions - I/O and System
        "INKEY" => Some("**INKEY$**\n\nReturns key pressed (empty string if none). Non-blocking keyboard input."),
        "TAB" => Some("**TAB(n)**\n\nMove to column n in PRINT statement."),
        "SPC" => Some("**SPC(n)**\n\nOutput n spaces in PRINT statement."),
        "TIMER" => Some("**TIMER**\n\nReturns seconds since midnight as floating-point number."),
        "DATE" => Some("**DATE$**\n\nReturns current date as string."),
        "TIME" => Some("**TIME$**\n\nReturns current time as string."),
        "EOF" => Some("**EOF(n)**\n\nReturns -1 (true) if end of file reached on file #n."),
        "PEEK" => Some("**PEEK(address)**\n\nReturns byte value at memory address."),
        "POKE" => Some("**POKE** address, value\n\nWrite byte value to memory address."),
        "POINT" => Some("**POINT(x, y)**\n\nReturns color of pixel at coordinates."),

        // Graphics
        "SCREEN" => Some("**SCREEN** mode [, colorswitch]\n\nSet graphics mode. Mode 0 is text, higher modes are graphics."),
        "COLOR" => Some("**COLOR** foreground [, background [, border]]\n\nSet text or graphics colors."),
        "CLS" => Some("**CLS**\n\nClear screen."),
        "LOCATE" => Some("**LOCATE** row, col [, cursor]\n\nPosition text cursor. Row and column are 1-based."),
        "CIRCLE" => Some("**CIRCLE** (x, y), radius [, color]\n\nDraw circle centered at (x, y)."),
        "PAINT" => Some("**PAINT** (x, y) [, color [, border]]\n\nFlood fill starting at (x, y)."),
        "PSET" => Some("**PSET** (x, y) [, color]\n\nSet pixel at coordinates."),
        "PRESET" => Some("**PRESET** (x, y)\n\nReset pixel at coordinates to background color."),
        "GET" => Some("**GET** (x1, y1)-(x2, y2), array\n\nCapture screen rectangle into array (sprite capture)."),
        "PUT" => Some("**PUT** (x, y), array\n\nDraw array contents at position (sprite draw)."),
        "DRAW" => Some("**DRAW** command$\n\nTurtle graphics. Commands: U/D/L/R (move), M (move to), C (color), etc."),
        "PLAY" => Some("**PLAY** music$\n\nPlay music notation. Notes: A-G, O (octave), L (length), T (tempo)."),
        "SOUND" => Some("**SOUND** frequency, duration\n\nPlay tone at frequency (Hz) for duration."),
        "BEEP" => Some("**BEEP**\n\nPlay system beep sound."),
        "WIDTH" => Some("**WIDTH** columns\n\nSet screen width (40 or 80 columns typically)."),

        // File I/O
        "OPEN" => Some("**OPEN** filename$ **FOR** mode **AS** #n\n\nOpen file. Modes: INPUT, OUTPUT, APPEND."),
        "CLOSE" => Some("**CLOSE** [#n]\n\nClose file. Without argument, closes all files."),
        "KILL" => Some("**KILL** filename$\n\nDelete file."),
        "NAME" => Some("**NAME** oldname$ **AS** newname$\n\nRename file."),
        "FILES" => Some("**FILES** [pattern$]\n\nList files matching pattern."),
        "MKDIR" => Some("**MKDIR** dirname$\n\nCreate directory."),
        "RMDIR" => Some("**RMDIR** dirname$\n\nRemove directory."),
        "CHDIR" => Some("**CHDIR** dirname$\n\nChange current directory."),

        // Operators
        "AND" => Some("**AND**\n\nLogical AND operator. Returns true if both operands are true.\n\nExample: `IF A > 0 AND B > 0 THEN ...`"),
        "OR" => Some("**OR**\n\nLogical OR operator. Returns true if either operand is true.\n\nExample: `IF A = 1 OR A = 2 THEN ...`"),
        "XOR" => Some("**XOR**\n\nLogical exclusive OR. Returns true if operands differ."),
        "NOT" => Some("**NOT**\n\nLogical NOT operator. Inverts boolean value.\n\nExample: `IF NOT EOF(1) THEN ...`"),
        "MOD" => Some("**MOD**\n\nModulo operator. Returns remainder of integer division.\n\nExample: `10 MOD 3` returns `1`"),

        // Error handling
        "ERROR" => Some("**ON ERROR GOTO** line\n\nSet error trap. When error occurs, jumps to specified line."),
        "RESUME" => Some("**RESUME** [line | **NEXT**]\n\nContinue after error. RESUME retries, RESUME NEXT continues, RESUME line jumps."),

        // Other
        "REM" => Some("**REM** comment\n\nComment. Everything after REM is ignored."),
        "DEF" => Some("**DEF FN**name(params) = expression\n\nDefine user function.\n\nExample: `DEF FNSQUARE(X) = X * X`"),
        "FN" => Some("**FN**name(args)\n\nCall user-defined function.\n\nExample: `Y = FNSQUARE(5)`"),
        "CLEAR" => Some("**CLEAR**\n\nClear all variables and reset program state."),
        "CHAIN" => Some("**CHAIN** filename$ [, line]\n\nLoad and run another BASIC program."),

        _ => None,
    }
}
