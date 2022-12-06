use unicode_width::UnicodeWidthChar;

/// Strip a string until it only takes up to N columns in the terminal
/// Currently uses an inefficient bruteforce algorithm
pub fn ansi_strip(input: &str, max_width: usize) -> &str {
    let mut len = 0;
    let mut cols = 0;

    let mut in_escape_seq = false;

    for char in input.chars() {
        len += char.len_utf8();

        if u64::from(char) == 0x1B {
            in_escape_seq = true;
            continue;
        }

        if in_escape_seq {
            if char == 'm' {
                in_escape_seq = false;
            }

            continue;
        }

        let char_cols = char.width().unwrap_or(0);

        if cols + char_cols > max_width {
            len -= char.len_utf8();
            break;
        }

        cols += char_cols;
    }

    assert!(
        !in_escape_seq,
        "Did not find end of escape sequence in string: {input}"
    );

    &input[..len]
}
