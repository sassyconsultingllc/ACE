// Case normalization helpers  make comparisons case/format insensitive
//! Normalize strings to a canonical ASCII alphanumeric lowercase form so
//! that CamelCase, snake_case, kebab-case and UPPERCASE compare equal.

/// Normalize an input string by keeping only ASCII alphanumeric characters
/// and converting to ASCII lowercase. Useful for canonicalizing keys and
/// authorization checks where differing case or separators should be ignored.
pub fn normalize_key<S: AsRef<str>>(s: S) -> String {
    s.as_ref().chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Case-insensitive equality after normalization.
pub fn eq_normalized<A: AsRef<str>, B: AsRef<str>>(a: A, b: B) -> bool {
    normalize_key(a) == normalize_key(b)
}

/// Return an ASCII-lowercased version of the input string.
/// This is a lightweight helper for places where simple lowercasing
/// (not full normalization) is desired (e.g. tag or extension matching).
pub fn ascii_lower(s: &str) -> String {
    s.to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_variants() {
        assert_eq!(normalize_key("CamelCase"), "camelcase");
        assert_eq!(normalize_key("camel_case"), "camelcase");
        assert_eq!(normalize_key("CAMEL-CASE"), "camelcase");
        assert!(eq_normalized("SomeValue", "some_value"));
    }
}


