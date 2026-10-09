//! Text handling that matches Python's, so hashes and line numbers stay identical to the ones
//! the Python backend stored.

use std::fs;
use std::io;
use std::path::Path;

/// Reads a file like Python's `Path.read_text()`: UTF-8, with `\r\n` and `\r` turned into `\n`.
///
/// Returns `None` when the file can't be read or isn't valid UTF-8.
pub fn read_text(path: &Path) -> Option<String> {
    read_text_strict(path).ok()
}

/// Like [`read_text`], returning why the file couldn't be read.
pub fn read_text_strict(path: &Path) -> io::Result<String> {
    Ok(universal_newlines(&fs::read_to_string(path)?))
}

/// Python's universal newlines: `\r\n` and `\r` turned into `\n`.
pub fn universal_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Python's `str.isspace()`: Rust's whitespace plus the `\x1c`–`\x1f` separators.
pub fn is_space(c: char) -> bool {
    c.is_whitespace() || ('\x1c'..='\x1f').contains(&c)
}

/// Python's `str.rstrip()`.
pub fn rstrip(s: &str) -> &str {
    s.trim_end_matches(is_space)
}

/// Python's `str.strip()`.
pub fn strip(s: &str) -> &str {
    s.trim_matches(is_space)
}

fn is_line_break(c: char) -> bool {
    matches!(
        c,
        '\n' | '\r'
            | '\x0b'
            | '\x0c'
            | '\x1c'
            | '\x1d'
            | '\x1e'
            | '\u{85}'
            | '\u{2028}'
            | '\u{2029}'
    )
}

/// Python's `str.splitlines()`, without the line breaks.
pub fn split_lines(s: &str) -> Vec<&str> {
    split_lines_keep_ends(s)
        .into_iter()
        .map(|line| {
            line.strip_suffix("\r\n")
                .unwrap_or_else(|| strip_last_break(line))
        })
        .collect()
}

fn strip_last_break(line: &str) -> &str {
    match line.chars().next_back() {
        Some(c) if is_line_break(c) => &line[..line.len() - c.len_utf8()],
        _ => line,
    }
}

/// Python's `str.splitlines(keepends=True)`.
pub fn split_lines_keep_ends(s: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut chars = s.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if !is_line_break(c) {
            continue;
        }
        let mut end = i + c.len_utf8();
        if c == '\r' && chars.peek().is_some_and(|&(_, next)| next == '\n') {
            chars.next();
            end += 1;
        }
        lines.push(&s[start..end]);
        start = end;
    }
    if start < s.len() {
        lines.push(&s[start..]);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_lines_matches_python() {
        assert_eq!(split_lines(""), Vec::<&str>::new());
        assert_eq!(split_lines("\n"), vec![""]);
        assert_eq!(split_lines("a\nb"), vec!["a", "b"]);
        assert_eq!(split_lines("a\r\nb\rc\n"), vec!["a", "b", "c"]);
        assert_eq!(
            split_lines("a\x0cb\x1cc\u{2028}d\u{85}e\x1fz"),
            vec!["a", "b", "c", "d", "e\x1fz"]
        );
    }

    #[test]
    fn split_lines_keep_ends_matches_python() {
        assert_eq!(
            split_lines_keep_ends("a\r\nb\x0cc"),
            vec!["a\r\n", "b\x0c", "c"]
        );
        assert_eq!(split_lines_keep_ends("a\n\n"), vec!["a\n", "\n"]);
    }

    #[test]
    fn rstrip_includes_information_separators() {
        assert_eq!(rstrip("a \t\x1f\x1c\u{a0}"), "a");
        assert_eq!(strip("\x1e a "), "a");
    }

    #[test]
    fn read_text_translates_newlines_and_rejects_non_utf8() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("f.md");
        fs::write(&path, b"a\r\nb\rc\n").unwrap();
        assert_eq!(read_text(&path).as_deref(), Some("a\nb\nc\n"));
        fs::write(&path, b"caf\xe9").unwrap();
        assert_eq!(read_text(&path), None);
        assert_eq!(read_text(&tmp.path().join("missing")), None);
    }
}
