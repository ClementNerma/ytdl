pub fn sanitize_filename(filename: &str) -> String {
    filename
        .replace('/', "\u{1735}")
        .replace('\\', "\u{29F5}")
        .replace('|', "\u{2223}")
        .replace('<', "\u{02C2}")
        .replace('>', "\u{02C3}")
        .replace(':', "\u{0589}")
        .replace('"', "\u{02BA}")
        .replace('?', "\u{FF1F}")
        .replace('*', "\u{2217}")
}
