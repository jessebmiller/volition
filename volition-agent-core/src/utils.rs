// volition-agent-core/src/utils.rs
//! General utility functions.

/// Truncates a string to a maximum character count, adding an ellipsis if truncated.
/// Handles multi-byte characters correctly.
pub fn truncate_string(input: &str, max_chars: usize) -> String {
    if input.chars().count() > max_chars {
        // If the limit is too small to include any characters plus "...",
        // just take the first max_chars characters without an ellipsis.
        if max_chars < 3 {
            input.chars().take(max_chars).collect::<String>()
        } else {
            // Otherwise, take max_chars - 3 characters and add "..."
            format!(
                "{}...",
                input.chars().take(max_chars - 3).collect::<String>()
            )
        }
    } else {
        input.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_no_truncation() {
        assert_eq!(truncate_string("hello", 10), "hello");
        assert_eq!(truncate_string("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_with_truncation() {
        assert_eq!(truncate_string("hello world", 10), "hello w...");
        assert_eq!(truncate_string("hello world", 5), "he...");
    }

    #[test]
    fn test_truncate_short_limit() {
        assert_eq!(truncate_string("hello world", 3), "..."); // Correct: 0 chars + ...
        assert_eq!(truncate_string("hello world", 2), "he"); // Correct: 2 chars, no ...
        assert_eq!(truncate_string("hello world", 1), "h"); // Correct: 1 char, no ...
        assert_eq!(truncate_string("hello world", 0), ""); // Correct: 0 chars, no ...
    }

    #[test]
    fn test_truncate_unicode() {
        assert_eq!(truncate_string("你好世界", 10), "你好世界"); // 4 chars
        assert_eq!(truncate_string("你好世界", 4), "你好世界");
        assert_eq!(truncate_string("你好世界", 3), "..."); // Corrected assertion: 0 chars + ...
        assert_eq!(truncate_string("你好世界", 2), "你好"); // Correct: 2 chars, no ...
    }

    #[test]
    fn test_truncate_empty() {
        assert_eq!(truncate_string("", 10), "");
        assert_eq!(truncate_string("", 0), "");
    }
}
