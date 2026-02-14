//! CSS Engine for the JS interpreter
//! Parses CSS rules, calculates specificity, and computes styles for elements.

use std::collections::HashMap;

/// CSS specificity: (inline, id, class, element)
/// Higher tuple values = higher priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Specificity(pub u32, pub u32, pub u32, pub u32);

impl Specificity {
    /// Calculate specificity for a single selector string.
    ///
    /// Rules (simplified CSS specificity):
    ///   - Each `#id`   component adds (0, 1, 0, 0)
    ///   - Each `.class` / `[attr]` / `:pseudo-class` adds (0, 0, 1, 0)
    ///   - Each element / `::pseudo-element` adds (0, 0, 0, 1)
    ///   - `*` (universal) adds nothing
    pub fn calculate(selector: &str) -> Self {
        let mut id_count: u32 = 0;
        let mut class_count: u32 = 0;
        let mut element_count: u32 = 0;

        // Tokenise the selector by splitting on combinators while keeping simple selectors.
        let parts = Self::split_selector(selector);

        for part in &parts {
            let part = part.trim();
            if part.is_empty() || part == "*" {
                continue;
            }

            // Walk through the simple selector fragment character by character.
            let chars: Vec<char> = part.chars().collect();
            let len = chars.len();
            let mut i = 0;

            // If the fragment starts with a letter or '-' it is an element name.
            if i < len && (chars[i].is_ascii_alphabetic() || chars[i] == '-') {
                element_count += 1;
                // Skip until we hit '#', '.', '[', ':' or end.
                while i < len && chars[i] != '#' && chars[i] != '.' && chars[i] != '[' && chars[i] != ':' {
                    i += 1;
                }
            }

            while i < len {
                match chars[i] {
                    '#' => {
                        id_count += 1;
                        i += 1;
                        while i < len && chars[i] != '#' && chars[i] != '.' && chars[i] != '[' && chars[i] != ':' {
                            i += 1;
                        }
                    }
                    '.' => {
                        class_count += 1;
                        i += 1;
                        while i < len && chars[i] != '#' && chars[i] != '.' && chars[i] != '[' && chars[i] != ':' {
                            i += 1;
                        }
                    }
                    '[' => {
                        class_count += 1;
                        i += 1;
                        while i < len && chars[i] != ']' {
                            i += 1;
                        }
                        if i < len {
                            i += 1; // skip ']'
                        }
                    }
                    ':' => {
                        i += 1;
                        if i < len && chars[i] == ':' {
                            // pseudo-element  ::before etc.
                            element_count += 1;
                            i += 1;
                        } else {
                            // pseudo-class  :hover etc.
                            class_count += 1;
                        }
                        while i < len && chars[i] != '#' && chars[i] != '.' && chars[i] != '[' && chars[i] != ':' {
                            i += 1;
                        }
                    }
                    _ => {
                        i += 1;
                    }
                }
            }
        }

        Specificity(0, id_count, class_count, element_count)
    }

    /// Split a compound selector on descendant / child / sibling combinators
    /// so that each "simple selector" chunk can be analysed independently.
    fn split_selector(selector: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let mut current = String::new();

        for ch in selector.chars() {
            match ch {
                ' ' | '>' | '+' | '~' => {
                    let trimmed = current.trim().to_string();
                    if !trimmed.is_empty() {
                        parts.push(trimmed);
                    }
                    current.clear();
                }
                _ => current.push(ch),
            }
        }
        let trimmed = current.trim().to_string();
        if !trimmed.is_empty() {
            parts.push(trimmed);
        }
        parts
    }
}

/// A single CSS rule: one or more selectors sharing a declaration block.
#[derive(Debug, Clone)]
pub struct CssRule {
    pub selectors: Vec<String>,
    pub declarations: HashMap<String, String>,
}

/// The CSS engine stores parsed rules and can compute styles for selectors.
#[derive(Debug, Clone)]
pub struct CssEngine {
    rules: Vec<CssRule>,
}

impl CssEngine {
    /// Create a new, empty CSS engine.
    pub fn new() -> Self {
        CssEngine { rules: Vec::new() }
    }

    /// Parse a full CSS stylesheet string and add all rules found.
    ///
    /// Supports basic CSS: `selector { prop: value; }` with multiple
    /// selectors separated by commas.  Does not handle `@`-rules, comments
    /// within values, or nested blocks beyond what the simple parser can do.
    pub fn add_stylesheet(&mut self, css_text: &str) {
        let cleaned = Self::strip_comments(css_text);
        let mut chars = cleaned.chars().peekable();

        loop {
            // Skip whitespace
            while chars.peek().map_or(false, |c| c.is_whitespace()) {
                chars.next();
            }
            if chars.peek().is_none() {
                break;
            }

            // Skip @-rules (e.g. @media, @import) — consume until matching '}'
            if chars.peek() == Some(&'@') {
                let mut depth = 0;
                loop {
                    match chars.next() {
                        None => break,
                        Some('{') => depth += 1,
                        Some('}') => {
                            depth -= 1;
                            if depth <= 0 {
                                break;
                            }
                        }
                        Some(';') if depth == 0 => break,
                        _ => {}
                    }
                }
                continue;
            }

            // Read the selector part (everything up to '{')
            let mut selector_text = String::new();
            loop {
                match chars.peek() {
                    None => break,
                    Some(&'{') => {
                        chars.next();
                        break;
                    }
                    _ => {
                        selector_text.push(chars.next().unwrap());
                    }
                }
            }

            let selector_text = selector_text.trim().to_string();
            if selector_text.is_empty() {
                // Possibly stray characters — skip to avoid infinite loop.
                if chars.peek().is_some() {
                    chars.next();
                }
                continue;
            }

            // Read the declaration block (everything up to '}')
            let mut decl_text = String::new();
            loop {
                match chars.peek() {
                    None => break,
                    Some(&'}') => {
                        chars.next();
                        break;
                    }
                    _ => {
                        decl_text.push(chars.next().unwrap());
                    }
                }
            }

            let selectors: Vec<String> = selector_text
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            let declarations = Self::parse_declarations(&decl_text);

            if !selectors.is_empty() && !declarations.is_empty() {
                self.rules.push(CssRule {
                    selectors,
                    declarations,
                });
            }
        }
    }

    /// Convenience method to add a single rule from a selector string and a
    /// declarations string (e.g. `"color: red; font-size: 14px"`).
    pub fn add_rule(&mut self, selector: &str, declarations: &str) {
        let selectors: Vec<String> = selector
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let decls = Self::parse_declarations(declarations);

        if !selectors.is_empty() && !decls.is_empty() {
            self.rules.push(CssRule {
                selectors,
                declarations: decls,
            });
        }
    }

    /// Compute the merged style for a given selector.
    ///
    /// This walks through **all** stored rules and collects every rule whose
    /// selector list contains a selector that matches `target`.  Matching is
    /// done by simple string equality (case-insensitive) — this keeps the
    /// engine lightweight while still being useful for the interpreter.
    ///
    /// When the same property is declared by multiple matching rules the one
    /// with higher specificity wins; ties are broken by source order (last
    /// rule wins).
    pub fn compute_style(&self, target: &str) -> HashMap<String, String> {
        // Collect (specificity, source_index, property, value)
        let mut property_origins: HashMap<String, (Specificity, usize, String)> = HashMap::new();

        for (rule_idx, rule) in self.rules.iter().enumerate() {
            for selector in &rule.selectors {
                if Self::selector_matches(selector, target) {
                    let spec = Specificity::calculate(selector);
                    for (prop, val) in &rule.declarations {
                        let dominated = match property_origins.get(prop) {
                            None => true,
                            Some((prev_spec, prev_idx, _)) => {
                                spec > *prev_spec || (spec == *prev_spec && rule_idx >= *prev_idx)
                            }
                        };
                        if dominated {
                            property_origins.insert(
                                prop.clone(),
                                (spec, rule_idx, val.clone()),
                            );
                        }
                    }
                }
            }
        }

        property_origins
            .into_iter()
            .map(|(prop, (_spec, _idx, val))| (prop, val))
            .collect()
    }

    /// Return a reference to all stored rules (useful for inspection / tests).
    pub fn rules(&self) -> &[CssRule] {
        &self.rules
    }

    // ------------------------------------------------------------------
    // Private helpers
    // ------------------------------------------------------------------

    /// Strip C-style comments (`/* ... */`) from CSS source text.
    fn strip_comments(text: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let bytes = text.as_bytes();
        let len = bytes.len();
        let mut i = 0;
        while i < len {
            if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
                i += 2;
                while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                if i + 1 < len {
                    i += 2; // skip */
                }
            } else {
                result.push(bytes[i] as char);
                i += 1;
            }
        }
        result
    }

    /// Parse a declaration block string like `"color: red; font-size: 14px;"`.
    fn parse_declarations(text: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();

        for decl in text.split(';') {
            let decl = decl.trim();
            if decl.is_empty() {
                continue;
            }
            if let Some(colon_pos) = decl.find(':') {
                let prop = decl[..colon_pos].trim().to_lowercase();
                let val = decl[colon_pos + 1..].trim().to_string();
                if !prop.is_empty() && !val.is_empty() {
                    map.insert(prop, val);
                }
            }
        }

        map
    }

    /// Simple selector matching: checks whether `selector` matches `target`.
    ///
    /// Matching strategy (intentionally simple for the browser engine):
    ///   - Case-insensitive exact match
    ///   - If `target` starts with `#` or `.`, compare against the last simple
    ///     selector component of `selector` so `div .foo` matches `.foo`.
    fn selector_matches(selector: &str, target: &str) -> bool {
        let sel = selector.trim().to_lowercase();
        let tgt = target.trim().to_lowercase();

        if sel == tgt {
            return true;
        }

        // Compare only the last simple-selector part of a compound selector.
        let last_part = sel
            .rsplit(|c: char| c == ' ' || c == '>' || c == '+' || c == '~')
            .next()
            .unwrap_or("")
            .trim();

        if !last_part.is_empty() && last_part == tgt {
            return true;
        }

        false
    }
}

impl Default for CssEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ======================================================================
// Tests
// ======================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- Specificity tests ---

    #[test]
    fn specificity_universal() {
        assert_eq!(Specificity::calculate("*"), Specificity(0, 0, 0, 0));
    }

    #[test]
    fn specificity_element() {
        assert_eq!(Specificity::calculate("div"), Specificity(0, 0, 0, 1));
        assert_eq!(Specificity::calculate("h1"), Specificity(0, 0, 0, 1));
    }

    #[test]
    fn specificity_class() {
        assert_eq!(Specificity::calculate(".active"), Specificity(0, 0, 1, 0));
    }

    #[test]
    fn specificity_id() {
        assert_eq!(Specificity::calculate("#main"), Specificity(0, 1, 0, 0));
    }

    #[test]
    fn specificity_compound() {
        // div.active  -> 1 element + 1 class
        assert_eq!(Specificity::calculate("div.active"), Specificity(0, 0, 1, 1));
        // div#main.active  -> 1 element + 1 id + 1 class
        assert_eq!(Specificity::calculate("div#main.active"), Specificity(0, 1, 1, 1));
    }

    #[test]
    fn specificity_descendant() {
        // "div p"  -> 2 elements
        assert_eq!(Specificity::calculate("div p"), Specificity(0, 0, 0, 2));
        // "#nav .item a" -> 1 id + 1 class + 1 element
        assert_eq!(Specificity::calculate("#nav .item a"), Specificity(0, 1, 1, 1));
    }

    #[test]
    fn specificity_pseudo_class() {
        // a:hover -> 1 element + 1 pseudo-class
        assert_eq!(Specificity::calculate("a:hover"), Specificity(0, 0, 1, 1));
    }

    #[test]
    fn specificity_pseudo_element() {
        // p::before -> 1 element + 1 pseudo-element
        assert_eq!(Specificity::calculate("p::before"), Specificity(0, 0, 0, 2));
    }

    #[test]
    fn specificity_ordering() {
        let s1 = Specificity::calculate("div");        // (0,0,0,1)
        let s2 = Specificity::calculate(".cls");        // (0,0,1,0)
        let s3 = Specificity::calculate("#id");         // (0,1,0,0)
        assert!(s1 < s2);
        assert!(s2 < s3);
    }

    // --- CssEngine tests ---

    #[test]
    fn engine_add_rule() {
        let mut engine = CssEngine::new();
        engine.add_rule("body", "margin: 0; padding: 0");
        assert_eq!(engine.rules().len(), 1);
        assert_eq!(engine.rules()[0].selectors, vec!["body"]);
        assert_eq!(engine.rules()[0].declarations.get("margin"), Some(&"0".to_string()));
    }

    #[test]
    fn engine_add_stylesheet_basic() {
        let mut engine = CssEngine::new();
        engine.add_stylesheet("body { margin: 0; } h1 { color: red; }");
        assert_eq!(engine.rules().len(), 2);
    }

    #[test]
    fn engine_add_stylesheet_with_comments() {
        let mut engine = CssEngine::new();
        engine.add_stylesheet(
            "/* reset */ body { margin: 0; } /* heading */ h1 { color: blue; }"
        );
        assert_eq!(engine.rules().len(), 2);
    }

    #[test]
    fn engine_add_stylesheet_multi_selector() {
        let mut engine = CssEngine::new();
        engine.add_stylesheet("h1, h2, h3 { font-weight: bold; }");
        assert_eq!(engine.rules().len(), 1);
        assert_eq!(engine.rules()[0].selectors.len(), 3);
    }

    #[test]
    fn engine_compute_style_simple() {
        let mut engine = CssEngine::new();
        engine.add_rule("div", "color: red; font-size: 14px");
        let style = engine.compute_style("div");
        assert_eq!(style.get("color"), Some(&"red".to_string()));
        assert_eq!(style.get("font-size"), Some(&"14px".to_string()));
    }

    #[test]
    fn engine_compute_style_cascade_source_order() {
        let mut engine = CssEngine::new();
        engine.add_rule("p", "color: red");
        engine.add_rule("p", "color: blue");
        let style = engine.compute_style("p");
        // Last rule wins when specificity is equal.
        assert_eq!(style.get("color"), Some(&"blue".to_string()));
    }

    #[test]
    fn engine_compute_style_cascade_specificity() {
        let mut engine = CssEngine::new();
        engine.add_rule("p", "color: red");        // specificity (0,0,0,1)
        engine.add_rule(".highlight", "color: green"); // specificity (0,0,1,0) — does not match "p"
        engine.add_rule("p", "color: blue");        // specificity (0,0,0,1)
        let style = engine.compute_style("p");
        // Only the two "p" rules match; last one wins.
        assert_eq!(style.get("color"), Some(&"blue".to_string()));
    }

    #[test]
    fn engine_compute_style_merge_properties() {
        let mut engine = CssEngine::new();
        engine.add_rule("div", "color: red");
        engine.add_rule("div", "font-size: 16px");
        let style = engine.compute_style("div");
        assert_eq!(style.get("color"), Some(&"red".to_string()));
        assert_eq!(style.get("font-size"), Some(&"16px".to_string()));
    }

    #[test]
    fn engine_compute_style_no_match() {
        let mut engine = CssEngine::new();
        engine.add_rule("div", "color: red");
        let style = engine.compute_style("span");
        assert!(style.is_empty());
    }

    #[test]
    fn engine_compute_style_specificity_wins() {
        let mut engine = CssEngine::new();
        // Lower specificity rule added *after* higher specificity rule
        engine.add_rule("#main", "color: green");   // (0,1,0,0)
        engine.add_rule("div", "color: red");       // (0,0,0,1)
        // Query #main — only #main rule matches
        let style = engine.compute_style("#main");
        assert_eq!(style.get("color"), Some(&"green".to_string()));
    }

    #[test]
    fn engine_add_stylesheet_realistic() {
        let mut engine = CssEngine::new();
        engine.add_stylesheet(r#"
            body {
                margin: 0;
                padding: 0;
                font-family: sans-serif;
            }

            .container {
                max-width: 960px;
                margin: 0 auto;
            }

            #header {
                background: #333;
                color: white;
            }

            h1, h2 {
                font-weight: bold;
            }
        "#);
        assert_eq!(engine.rules().len(), 4);

        let body_style = engine.compute_style("body");
        assert_eq!(body_style.get("margin"), Some(&"0".to_string()));
        assert_eq!(body_style.get("font-family"), Some(&"sans-serif".to_string()));
    }

    #[test]
    fn engine_default() {
        let engine = CssEngine::default();
        assert!(engine.rules().is_empty());
    }
}
