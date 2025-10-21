use rmx::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EscapeError {
    InvalidEscape { position: usize, escape: char },
    InvalidUnicodeEscape { position: usize, reason: String },
    UnterminatedUnicodeEscape { position: usize },
}

/// Process escape sequences in a string literal.
///
/// The input should be the content between the quotes (not including the quotes).
/// Returns the processed string with escape sequences converted to their literal values.
pub fn process_escape_sequences(s: &str) -> Result<String, EscapeError> {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.char_indices().peekable();

    while let Some((i, ch)) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some((_, '"')) => result.push('"'),
                Some((_, '\\')) => result.push('\\'),
                Some((_, 'n')) => result.push('\n'),
                Some((_, 'r')) => result.push('\r'),
                Some((_, 't')) => result.push('\t'),
                Some((_, '0')) => result.push('\0'),
                Some((pos, 'u')) => {
                    // Unicode escape: \u{NNNNNN}.
                    match chars.next() {
                        Some((_, '{')) => {
                            let mut hex_str = String::new();
                            let mut found_close = false;

                            while let Some((_, ch)) = chars.next() {
                                if ch == '}' {
                                    found_close = true;
                                    break;
                                }
                                hex_str.push(ch);
                            }

                            if !found_close {
                                return Err(EscapeError::UnterminatedUnicodeEscape { position: pos });
                            }

                            if hex_str.is_empty() || hex_str.len() > 6 {
                                return Err(EscapeError::InvalidUnicodeEscape {
                                    position: pos,
                                    reason: format!("expected 1-6 hex digits, got {}", hex_str.len()),
                                });
                            }

                            let code_point = u32::from_str_radix(&hex_str, 16)
                                .map_err(|_| EscapeError::InvalidUnicodeEscape {
                                    position: pos,
                                    reason: format!("invalid hex digits: {}", hex_str),
                                })?;

                            let ch = char::from_u32(code_point)
                                .ok_or_else(|| EscapeError::InvalidUnicodeEscape {
                                    position: pos,
                                    reason: format!("invalid Unicode code point: U+{:X}", code_point),
                                })?;

                            result.push(ch);
                        }
                        _ => {
                            return Err(EscapeError::InvalidUnicodeEscape {
                                position: pos,
                                reason: "expected '{' after \\u".to_string(),
                            });
                        }
                    }
                }
                Some((_, esc)) => {
                    return Err(EscapeError::InvalidEscape { position: i, escape: esc });
                }
                None => {
                    // Trailing backslash.
                    return Err(EscapeError::InvalidEscape { position: i, escape: '\\' });
                }
            }
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_escapes() {
        assert_eq!(process_escape_sequences(r#"foo"#).unwrap(), "foo");
        assert_eq!(process_escape_sequences(r#"foo\"bar"#).unwrap(), "foo\"bar");
        assert_eq!(process_escape_sequences(r#"foo\\bar"#).unwrap(), "foo\\bar");
        assert_eq!(process_escape_sequences(r#"foo\nbar"#).unwrap(), "foo\nbar");
        assert_eq!(process_escape_sequences(r#"foo\rbar"#).unwrap(), "foo\rbar");
        assert_eq!(process_escape_sequences(r#"foo\tbar"#).unwrap(), "foo\tbar");
        assert_eq!(process_escape_sequences(r#"foo\0bar"#).unwrap(), "foo\0bar");
    }

    #[test]
    fn test_unicode_escapes() {
        assert_eq!(process_escape_sequences(r#"\u{41}"#).unwrap(), "A");
        assert_eq!(process_escape_sequences(r#"\u{1F4A9}"#).unwrap(), "\u{1F4A9}");
        assert_eq!(process_escape_sequences(r#"foo\u{42}ar"#).unwrap(), "fooBar");
    }

    #[test]
    fn test_invalid_escapes() {
        assert!(matches!(
            process_escape_sequences(r#"\q"#),
            Err(EscapeError::InvalidEscape { escape: 'q', .. })
        ));
        assert!(matches!(
            process_escape_sequences(r#"foo\x"#),
            Err(EscapeError::InvalidEscape { escape: 'x', .. })
        ));
    }

    #[test]
    fn test_invalid_unicode_escapes() {
        assert!(matches!(
            process_escape_sequences(r#"\u{110000}"#),
            Err(EscapeError::InvalidUnicodeEscape { .. })
        ));
        assert!(matches!(
            process_escape_sequences(r#"\u{}"#),
            Err(EscapeError::InvalidUnicodeEscape { .. })
        ));
        assert!(matches!(
            process_escape_sequences(r#"\u{1234567}"#),
            Err(EscapeError::InvalidUnicodeEscape { .. })
        ));
        assert!(matches!(
            process_escape_sequences(r#"\u{GGGG}"#),
            Err(EscapeError::InvalidUnicodeEscape { .. })
        ));
        assert!(matches!(
            process_escape_sequences(r#"\u"#),
            Err(EscapeError::InvalidUnicodeEscape { .. })
        ));
        assert!(matches!(
            process_escape_sequences(r#"\u{"#),
            Err(EscapeError::UnterminatedUnicodeEscape { .. })
        ));
    }

    #[test]
    fn test_edge_cases() {
        assert_eq!(process_escape_sequences("").unwrap(), "");
        assert_eq!(process_escape_sequences(r#"\n\r\t\0"#).unwrap(), "\n\r\t\0");
        assert_eq!(process_escape_sequences(r#"\\\\"#).unwrap(), "\\\\");
    }

    #[test]
    fn test_trailing_backslash() {
        assert!(matches!(
            process_escape_sequences(r#"foo\"#),
            Err(EscapeError::InvalidEscape { .. })
        ));
    }
}
