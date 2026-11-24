use regex::Regex;

/// Compiles a Finicky wildcard pattern into a [`Regex`].
///
/// Matches the exact semantics of `packages/config-api/src/wildcard.ts`:
/// - Empty string never matches anything in handler matching (`is_match`),
///   while `match_wildcard("", "")` is handled consistently.
/// - Escaped asterisks `\*` match literal `*`.
/// - Patterns without an explicit scheme (`^\w+:`) and not starting with `*`
///   automatically match `(?:https?:|ftp:|mailto:|file:|tel:|sms:|data:)?(?://)?`.
/// - Unescaped `*` translates to non-greedy `.*?`.
pub fn compile_wildcard(pattern: &str) -> Option<Regex> {
    const ESCAPED_STAR: char = '\u{0000}';
    let with_placeholder = pattern.replace(r"\*", &ESCAPED_STAR.to_string());

    // Escape special regex characters except '*'
    let mut escaped = String::with_capacity(with_placeholder.len() * 2);
    for ch in with_placeholder.chars() {
        match ch {
            '.' | '+' | '?' | '^' | '$' | '{' | '}' | '(' | ')' | '|' | '[' | ']' | '\\' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            _ => escaped.push(ch),
        }
    }

    let has_scheme = {
        let mut chars = pattern.chars();
        let mut saw_word = false;
        let mut valid = false;
        while let Some(c) = chars.next() {
            if c.is_ascii_alphanumeric() || c == '_' {
                saw_word = true;
            } else if c == ':' {
                valid = saw_word;
                break;
            } else {
                break;
            }
        }
        valid
    };

    if !has_scheme {
        if !pattern.starts_with('*') {
            escaped = format!(
                "(?:https?:|ftp:|mailto:|file:|tel:|sms:|data:)?(?://)?{}",
                escaped
            );
        }
    } else if escaped.ends_with("//") {
        escaped.push_str(".*");
    }

    let regex_pattern = escaped
        .replace('*', ".*?")
        .replace(ESCAPED_STAR, r"\*");

    let full = format!("^{}$", regex_pattern);
    Regex::new(&full).ok()
}

/// Evaluates a Finicky wildcard pattern against `target`.
pub fn match_wildcard(pattern: &str, target: &str) -> bool {
    if !pattern.contains('*') {
        if pattern == target {
            return true;
        }
        // Standard WHATWG URLs normalize `https://example.com` to `https://example.com/`.
        // Allow exact patterns without trailing slash to match normalized bare-origin URLs.
        if target.ends_with('/') && !pattern.ends_with('/') && pattern == target.trim_end_matches('/') {
            return true;
        }
        return false;
    }

    let Some(re) = compile_wildcard(pattern) else {
        return false;
    };

    if re.is_match(target) {
        return true;
    }

    // If `target` has a trailing slash added by URL normalization (e.g. `https://sub.example.com/`)
    // and the pattern doesn't end with `/` or `*` (e.g. `*.example.com`), also test without the trailing slash.
    if target.ends_with('/') && !pattern.ends_with('/') && !pattern.ends_with('*') {
        return re.is_match(target.trim_end_matches('/'));
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_wildcard_matching() {
        let pattern = "example.com*";
        assert!(match_wildcard(pattern, "https://example.com"));
        assert!(match_wildcard(pattern, "http://example.com"));
        assert!(match_wildcard(pattern, "mailto:example.com"));
        assert!(match_wildcard(pattern, "ftp://example.com"));
        assert!(match_wildcard(pattern, "https://example.com/path"));
        assert!(match_wildcard(pattern, "https://example.com?query=123"));
        assert!(match_wildcard(pattern, "https://example.com#fragment"));

        assert!(!match_wildcard(pattern, "https://notexample.com"));
        assert!(!match_wildcard(pattern, "https://sub.different.com"));
    }

    #[test]
    fn wildcard_positions() {
        assert!(match_wildcard("*.example.com", "https://sub.example.com"));
        assert!(match_wildcard(
            "*.example.com",
            "https://another.sub.example.com"
        ));
        assert!(!match_wildcard("*.example.com", "https://example.com"));

        assert!(match_wildcard("example.*/path", "https://example.com/path"));
        assert!(match_wildcard("example.*/path", "https://example.org/path"));
        assert!(!match_wildcard(
            "example.*/path",
            "https://example.com/other"
        ));

        assert!(match_wildcard(
            "*.example.*/path/*",
            "https://sub.example.com/path/test"
        ));
        assert!(match_wildcard(
            "*.example.*/path/*",
            "https://sub.example.org/path/foo/bar"
        ));
        assert!(!match_wildcard(
            "*.example.*/path/*",
            "https://sub.example.com/other/test"
        ));
    }

    #[test]
    fn escaped_wildcards() {
        assert!(match_wildcard(r"example\*.com", "https://example*.com"));
        assert!(!match_wildcard(r"example\*.com", "https://example.com"));

        assert!(match_wildcard(
            r"*example\*.com/*",
            "https://test.example*.com/path"
        ));
        assert!(match_wildcard(
            r"*example\*.com/*",
            "https://example*.com/anything"
        ));
        assert!(!match_wildcard(
            r"*example\*.com/*",
            "https://example.com/path"
        ));
    }

    #[test]
    fn special_and_edge_cases() {
        assert!(match_wildcard(
            "example.com/path?*",
            "https://example.com/path?query=123"
        ));
        assert!(!match_wildcard(
            "example.com/path?*",
            "https://example.com/path"
        ));

        assert!(match_wildcard(
            "*.com/*#*",
            "https://example.com/path#fragment"
        ));
        assert!(match_wildcard(
            "*.com/*#*",
            "https://sub.example.com/path?query=123#fragment"
        ));

        assert!(!match_wildcard("[invalid.pattern", "https://example.com"));

        assert!(match_wildcard("https://example.com", "https://example.com"));
        assert!(!match_wildcard("https://example.com", "http://example.com"));

        assert!(match_wildcard("*.google.*", "https://mail.google.com"));
        assert!(match_wildcard("*.google.*", "https://docs.google.co.uk"));
        assert!(!match_wildcard("*.google.*", "https://google.com"));

        assert!(match_wildcard(
            "https://*.github.io/*",
            "https://user1.github.io/repo"
        ));
        assert!(match_wildcard(
            "https://*.github.io/*",
            "https://org.github.io/docs/api"
        ));
        assert!(!match_wildcard(
            "https://*.github.io/*",
            "https://github.io/test"
        ));
    }
}
