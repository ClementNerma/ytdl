pub fn sanitize_filename(filename: &str) -> String {
    filename
        .replace('"', "'")
        .replace(": ", " - ")
        .chars()
        .map(|c| {
            if c.is_alphanumeric()
                || c == ' '
                || c == '-'
                || c == '_'
                || c == '.'
                || c == '('
                || c == ')'
                || c == '['
                || c == ']'
                || c == '{'
                || c == '}'
                || c == '!'
                || c == '\''
                || c == '’'
                || c == '°'
                || c == '#'
                || c == '&'
                || c == '$'
                || c == '^'
                || c == '@'
            {
                c
            } else {
                '_'
            }
        })
        .collect()
}
