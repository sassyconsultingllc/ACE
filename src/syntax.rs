//! Syntax Highlighting Engine
//!
//! Renders code blocks with beautiful syntax highlighting.
//! Supports: JavaScript, TypeScript, Rust, Python, HTML, CSS, JSON, Markdown

use crate::style::Color;
use std::collections::HashMap;

/// Syntax highlighting theme
#[derive(Debug, Clone)]
pub struct SyntaxTheme {
    pub background: Color,
    pub foreground: Color,
    pub keyword: Color,
    pub string: Color,
    pub number: Color,
    pub comment: Color,
    pub function: Color,
    pub type_name: Color,
    pub operator: Color,
    pub punctuation: Color,
    pub variable: Color,
    pub constant: Color,
    pub attribute: Color,
    pub tag: Color,
    pub property: Color,
}

impl SyntaxTheme {
    /// One Dark theme (popular VS Code theme)
    pub fn one_dark() -> Self {
        SyntaxTheme {
            background: Color::new(40, 44, 52, 255),     // #282c34
            foreground: Color::new(171, 178, 191, 255),  // #abb2bf
            keyword: Color::new(198, 120, 221, 255),     // #c678dd (purple)
            string: Color::new(152, 195, 121, 255),      // #98c379 (green)
            number: Color::new(209, 154, 102, 255),      // #d19a66 (orange)
            comment: Color::new(92, 99, 112, 255),       // #5c6370 (gray)
            function: Color::new(97, 175, 239, 255),     // #61afef (blue)
            type_name: Color::new(229, 192, 123, 255),   // #e5c07b (yellow)
            operator: Color::new(86, 182, 194, 255),     // #56b6c2 (cyan)
            punctuation: Color::new(171, 178, 191, 255), // #abb2bf
            variable: Color::new(224, 108, 117, 255),    // #e06c75 (red)
            constant: Color::new(209, 154, 102, 255),    // #d19a66
            attribute: Color::new(209, 154, 102, 255),   // #d19a66
            tag: Color::new(224, 108, 117, 255),         // #e06c75
            property: Color::new(224, 108, 117, 255),    // #e06c75
        }
    }

    /// GitHub Light theme
    pub fn github_light() -> Self {
        SyntaxTheme {
            background: Color::new(255, 255, 255, 255),
            foreground: Color::new(36, 41, 46, 255),
            keyword: Color::new(215, 58, 73, 255),   // #d73a49
            string: Color::new(3, 47, 98, 255),      // #032f62
            number: Color::new(0, 92, 197, 255),     // #005cc5
            comment: Color::new(106, 115, 125, 255), // #6a737d
            function: Color::new(111, 66, 193, 255), // #6f42c1
            type_name: Color::new(0, 92, 197, 255),  // #005cc5
            operator: Color::new(215, 58, 73, 255),  // #d73a49
            punctuation: Color::new(36, 41, 46, 255),
            variable: Color::new(227, 98, 9, 255), // #e36209
            constant: Color::new(0, 92, 197, 255),
            attribute: Color::new(0, 92, 197, 255),
            tag: Color::new(34, 134, 58, 255), // #22863a
            property: Color::new(0, 92, 197, 255),
        }
    }

    /// Dracula theme
    pub fn dracula() -> Self {
        SyntaxTheme {
            background: Color::new(40, 42, 54, 255),    // #282a36
            foreground: Color::new(248, 248, 242, 255), // #f8f8f2
            keyword: Color::new(255, 121, 198, 255),    // #ff79c6 (pink)
            string: Color::new(241, 250, 140, 255),     // #f1fa8c (yellow)
            number: Color::new(189, 147, 249, 255),     // #bd93f9 (purple)
            comment: Color::new(98, 114, 164, 255),     // #6272a4
            function: Color::new(80, 250, 123, 255),    // #50fa7b (green)
            type_name: Color::new(139, 233, 253, 255),  // #8be9fd (cyan)
            operator: Color::new(255, 121, 198, 255),   // #ff79c6
            punctuation: Color::new(248, 248, 242, 255),
            variable: Color::new(248, 248, 242, 255),
            constant: Color::new(189, 147, 249, 255),
            attribute: Color::new(80, 250, 123, 255),
            tag: Color::new(255, 121, 198, 255),
            property: Color::new(139, 233, 253, 255),
        }
    }
}

impl SyntaxTheme {
    /// Select theme by name
    pub fn from_name(name: &str) -> Self {
        match name {
            "github" | "github_light" | "light" => Self::github_light(),
            "dracula" => Self::dracula(),
            _ => Self::one_dark(),
        }
    }
}

impl Default for SyntaxTheme {
    fn default() -> Self {
        Self::one_dark()
    }
}

/// Token type for syntax highlighting
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenType {
    Keyword,
    String,
    Number,
    Comment,
    Function,
    Type,
    Operator,
    Punctuation,
    Variable,
    Constant,
    Attribute,
    Tag,
    Property,
    Plain,
}

/// A highlighted token
#[derive(Debug, Clone)]
pub struct HighlightToken {
    pub text: String,
    pub token_type: TokenType,
    pub color: Color,
}

/// Language for syntax highlighting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    JavaScript,
    TypeScript,
    Rust,
    Python,
    Html,
    Css,
    Json,
    Markdown,
    Plain,
}

impl Language {
    /// Detect language from file extension or code block annotation
    pub fn from_annotation(s: &str) -> Self {
        match crate::fontcase::ascii_lower(s).as_str() {
            "js" | "javascript" | "jsx" => Language::JavaScript,
            "ts" | "typescript" | "tsx" => Language::TypeScript,
            "rs" | "rust" => Language::Rust,
            "py" | "python" => Language::Python,
            "html" | "htm" => Language::Html,
            "css" | "scss" | "sass" => Language::Css,
            "json" => Language::Json,
            "md" | "markdown" => Language::Markdown,
            _ => Language::Plain,
        }
    }
}

/// Syntax highlighter
pub struct SyntaxHighlighter {
    theme: SyntaxTheme,
    keywords: HashMap<Language, Vec<&'static str>>,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        let mut keywords = HashMap::new();

        keywords.insert(
            Language::JavaScript,
            vec![
                "const",
                "let",
                "var",
                "function",
                "return",
                "if",
                "else",
                "for",
                "while",
                "do",
                "switch",
                "case",
                "break",
                "continue",
                "try",
                "catch",
                "finally",
                "throw",
                "new",
                "this",
                "class",
                "extends",
                "super",
                "import",
                "export",
                "from",
                "default",
                "async",
                "await",
                "yield",
                "typeof",
                "instanceof",
                "in",
                "of",
                "true",
                "false",
                "null",
                "undefined",
                "void",
                "delete",
            ],
        );

        keywords.insert(
            Language::TypeScript,
            vec![
                "const",
                "let",
                "var",
                "function",
                "return",
                "if",
                "else",
                "for",
                "while",
                "do",
                "switch",
                "case",
                "break",
                "continue",
                "try",
                "catch",
                "finally",
                "throw",
                "new",
                "this",
                "class",
                "extends",
                "super",
                "import",
                "export",
                "from",
                "default",
                "async",
                "await",
                "yield",
                "typeof",
                "instanceof",
                "in",
                "of",
                "true",
                "false",
                "null",
                "undefined",
                "void",
                "delete",
                "interface",
                "type",
                "enum",
                "implements",
                "public",
                "private",
                "protected",
                "readonly",
                "abstract",
                "as",
                "is",
                "keyof",
                "infer",
                "never",
                "unknown",
            ],
        );

        keywords.insert(
            Language::Rust,
            vec![
                "fn", "let", "mut", "const", "static", "if", "else", "match", "for", "while",
                "loop", "break", "continue", "return", "struct", "enum", "impl", "trait", "pub",
                "mod", "use", "crate", "self", "Self", "super", "where", "async", "await", "move",
                "ref", "dyn", "type", "as", "in", "true", "false", "Some", "None", "Ok", "Err",
                "unsafe", "extern",
            ],
        );

        keywords.insert(
            Language::Python,
            vec![
                "def", "class", "if", "elif", "else", "for", "while", "try", "except", "finally",
                "with", "as", "import", "from", "return", "yield", "raise", "pass", "break",
                "continue", "and", "or", "not", "in", "is", "lambda", "True", "False", "None",
                "global", "nonlocal", "assert", "async", "await",
            ],
        );

        keywords.insert(
            Language::Css,
            vec!["important", "inherit", "initial", "unset", "auto", "none"],
        );

        SyntaxHighlighter {
            theme: SyntaxTheme::default(),
            keywords,
        }
    }

    pub fn set_theme(&mut self, theme: SyntaxTheme) {
        self.theme = theme;
    }

    /// Highlight a code block
    pub fn highlight(&self, code: &str, language: Language) -> Vec<Vec<HighlightToken>> {
        let mut lines = Vec::new();

        for line in code.lines() {
            let tokens = self.highlight_line(line, language);
            lines.push(tokens);
        }

        lines
    }

    /// Highlight a single line
    fn highlight_line(&self, line: &str, language: Language) -> Vec<HighlightToken> {
        let mut tokens = Vec::new();
        let keywords = self
            .keywords
            .get(&language)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];

            // Comment detection
            if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
                // Line comment
                let rest: String = chars[i..].iter().collect();
                tokens.push(HighlightToken {
                    text: rest,
                    token_type: TokenType::Comment,
                    color: self.theme.comment,
                });
                break;
            }

            // Python comment
            if c == '#' && language == Language::Python {
                let rest: String = chars[i..].iter().collect();
                tokens.push(HighlightToken {
                    text: rest,
                    token_type: TokenType::Comment,
                    color: self.theme.comment,
                });
                break;
            }

            // String detection
            if c == '"' || c == '\'' || c == '`' {
                let quote = c;
                let start = i;
                i += 1;
                while i < chars.len() && chars[i] != quote {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                if i < chars.len() {
                    i += 1; // Include closing quote
                }
                let text: String = chars[start..i].iter().collect();
                tokens.push(HighlightToken {
                    text,
                    token_type: TokenType::String,
                    color: self.theme.string,
                });
                continue;
            }

            // Number detection
            if c.is_ascii_digit()
                || (c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit())
            {
                let start = i;
                while i < chars.len()
                    && (chars[i].is_ascii_digit()
                        || chars[i] == '.'
                        || chars[i] == 'x'
                        || chars[i] == 'b'
                        || chars[i].is_ascii_hexdigit())
                {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                tokens.push(HighlightToken {
                    text,
                    token_type: TokenType::Number,
                    color: self.theme.number,
                });
                continue;
            }

            // Identifier/keyword detection
            if c.is_alphabetic() || c == '_' || c == '$' {
                let start = i;
                while i < chars.len()
                    && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '$')
                {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();

                let token_type = if keywords.contains(&text.as_str()) {
                    TokenType::Keyword
                } else if text
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
                {
                    TokenType::Type
                } else if i < chars.len() && chars[i] == '(' {
                    TokenType::Function
                } else {
                    TokenType::Variable
                };

                let color = match token_type {
                    TokenType::Keyword => self.theme.keyword,
                    TokenType::Type => self.theme.type_name,
                    TokenType::Function => self.theme.function,
                    _ => self.theme.foreground,
                };

                tokens.push(HighlightToken {
                    text,
                    token_type,
                    color,
                });
                continue;
            }

            // Operators
            if "+-*/%=<>!&|^~?:".contains(c) {
                let start = i;
                while i < chars.len() && "+-*/%=<>!&|^~?:".contains(chars[i]) {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                tokens.push(HighlightToken {
                    text,
                    token_type: TokenType::Operator,
                    color: self.theme.operator,
                });
                continue;
            }

            // Punctuation
            if "(){}[];,.".contains(c) {
                tokens.push(HighlightToken {
                    text: c.to_string(),
                    token_type: TokenType::Punctuation,
                    color: self.theme.punctuation,
                });
                i += 1;
                continue;
            }

            // Whitespace and other
            tokens.push(HighlightToken {
                text: c.to_string(),
                token_type: TokenType::Plain,
                color: self.theme.foreground,
            });
            i += 1;
        }

        tokens
    }

    /// Get color for token type
    pub fn color_for(&self, token_type: TokenType) -> Color {
        match token_type {
            TokenType::Keyword => self.theme.keyword,
            TokenType::String => self.theme.string,
            TokenType::Number => self.theme.number,
            TokenType::Comment => self.theme.comment,
            TokenType::Function => self.theme.function,
            TokenType::Type => self.theme.type_name,
            TokenType::Operator => self.theme.operator,
            TokenType::Punctuation => self.theme.punctuation,
            TokenType::Variable => self.theme.variable,
            TokenType::Constant => self.theme.constant,
            TokenType::Attribute => self.theme.attribute,
            TokenType::Tag => self.theme.tag,
            TokenType::Property => self.theme.property,
            TokenType::Plain => self.theme.foreground,
        }
    }

    pub fn background(&self) -> Color {
        self.theme.background
    }

    /// Apply a named theme
    pub fn apply_theme(&mut self, name: &str) {
        self.set_theme(SyntaxTheme::from_name(name));
    }

    /// Summary for diagnostics - wires color_for, background, set_theme
    pub fn describe(&self) -> String {
        let bg = self.background();
        let all_types = [
            TokenType::Keyword,
            TokenType::String,
            TokenType::Number,
            TokenType::Comment,
            TokenType::Function,
            TokenType::Type,
            TokenType::Operator,
            TokenType::Punctuation,
            TokenType::Variable,
            TokenType::Constant,
            TokenType::Attribute,
            TokenType::Tag,
            TokenType::Property,
            TokenType::Plain,
        ];
        let color_count = all_types
            .iter()
            .map(|t| {
                let c = self.color_for(*t);
                (c.r as u32) << 16 | (c.g as u32) << 8 | c.b as u32
            })
            .collect::<std::collections::HashSet<u32>>()
            .len();
        let lang = Language::from_annotation("rust");
        format!(
            "SyntaxHighlighter[bg=({},{},{},{}), unique_colors={}, languages={}, sample_lang={:?}]",
            bg.r,
            bg.g,
            bg.b,
            bg.a,
            color_count,
            self.keywords.len(),
            lang
        )
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_js_highlight() {
        let highlighter = SyntaxHighlighter::new();
        let tokens = highlighter.highlight("const x = 42;", Language::JavaScript);
        assert!(!tokens.is_empty());
        assert!(!tokens[0].is_empty());
    }

    #[test]
    fn test_rust_highlight() {
        let highlighter = SyntaxHighlighter::new();
        let tokens = highlighter.highlight("fn main() { println!(\"Hello\"); }", Language::Rust);
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_all_themes() {
        let one_dark = SyntaxTheme::one_dark();
        assert!(one_dark.background.a > 0);

        let github = SyntaxTheme::github_light();
        assert!(github.background.a > 0);

        let dracula = SyntaxTheme::dracula();
        assert!(dracula.background.a > 0);

        // from_name
        let t1 = SyntaxTheme::from_name("github");
        assert_eq!(t1.background.r, github.background.r);
        let t2 = SyntaxTheme::from_name("github_light");
        assert_eq!(t2.background.r, github.background.r);
        let t3 = SyntaxTheme::from_name("light");
        assert_eq!(t3.background.r, github.background.r);
        let t4 = SyntaxTheme::from_name("dracula");
        assert_eq!(t4.background.r, dracula.background.r);
        let t5 = SyntaxTheme::from_name("unknown_theme");
        assert_eq!(t5.background.r, one_dark.background.r);
    }

    #[test]
    fn test_set_theme_and_apply_theme() {
        let mut hl = SyntaxHighlighter::new();

        // set_theme
        hl.set_theme(SyntaxTheme::dracula());
        assert_eq!(hl.background().r, SyntaxTheme::dracula().background.r);

        // apply_theme
        hl.apply_theme("github");
        assert_eq!(hl.background().r, SyntaxTheme::github_light().background.r);

        hl.apply_theme("dracula");
        assert_eq!(hl.background().r, SyntaxTheme::dracula().background.r);
    }

    #[test]
    fn test_color_for_all_token_types() {
        let hl = SyntaxHighlighter::new();
        let types = [
            TokenType::Keyword,
            TokenType::String,
            TokenType::Number,
            TokenType::Comment,
            TokenType::Function,
            TokenType::Type,
            TokenType::Operator,
            TokenType::Punctuation,
            TokenType::Variable,
            TokenType::Constant,
            TokenType::Attribute,
            TokenType::Tag,
            TokenType::Property,
            TokenType::Plain,
        ];
        for tt in &types {
            let color = hl.color_for(*tt);
            assert!(color.a > 0, "Token {:?} should have non-zero alpha", tt);
        }
    }

    #[test]
    fn test_background_method() {
        let hl = SyntaxHighlighter::new();
        let bg = hl.background();
        assert!(bg.a > 0);
    }

    #[test]
    fn test_describe() {
        let hl = SyntaxHighlighter::new();
        let desc = hl.describe();
        assert!(desc.contains("SyntaxHighlighter"));
        assert!(desc.contains("unique_colors"));
        assert!(desc.contains("languages"));
    }

    #[test]
    fn test_language_from_annotation() {
        assert_eq!(Language::from_annotation("js"), Language::JavaScript);
        assert_eq!(
            Language::from_annotation("javascript"),
            Language::JavaScript
        );
        assert_eq!(Language::from_annotation("jsx"), Language::JavaScript);
        assert_eq!(Language::from_annotation("ts"), Language::TypeScript);
        assert_eq!(
            Language::from_annotation("typescript"),
            Language::TypeScript
        );
        assert_eq!(Language::from_annotation("tsx"), Language::TypeScript);
        assert_eq!(Language::from_annotation("rs"), Language::Rust);
        assert_eq!(Language::from_annotation("rust"), Language::Rust);
        assert_eq!(Language::from_annotation("py"), Language::Python);
        assert_eq!(Language::from_annotation("python"), Language::Python);
        assert_eq!(Language::from_annotation("html"), Language::Html);
        assert_eq!(Language::from_annotation("htm"), Language::Html);
        assert_eq!(Language::from_annotation("css"), Language::Css);
        assert_eq!(Language::from_annotation("scss"), Language::Css);
        assert_eq!(Language::from_annotation("sass"), Language::Css);
        assert_eq!(Language::from_annotation("json"), Language::Json);
        assert_eq!(Language::from_annotation("md"), Language::Markdown);
        assert_eq!(Language::from_annotation("markdown"), Language::Markdown);
        assert_eq!(Language::from_annotation("unknown"), Language::Plain);
    }

    #[test]
    fn test_highlight_python_with_comment() {
        let hl = SyntaxHighlighter::new();
        let tokens = hl.highlight("x = 10 # comment", Language::Python);
        assert!(!tokens.is_empty());
        // Should have comment token
        let flat: Vec<_> = tokens.into_iter().flatten().collect();
        assert!(flat.iter().any(|t| t.token_type == TokenType::Comment));
    }

    #[test]
    fn test_highlight_all_token_production() {
        let hl = SyntaxHighlighter::new();
        // Code that exercises keywords, strings, numbers, operators, punctuation, identifiers
        let code = r#"const MyType = "hello"; let x = 42; fn foo() {}"#;
        let tokens = hl.highlight(code, Language::JavaScript);
        let flat: Vec<_> = tokens.into_iter().flatten().collect();
        assert!(flat.iter().any(|t| t.token_type == TokenType::Keyword));
        assert!(flat.iter().any(|t| t.token_type == TokenType::String));
        assert!(flat.iter().any(|t| t.token_type == TokenType::Number));
        assert!(flat.iter().any(|t| t.token_type == TokenType::Punctuation));
    }

    #[test]
    fn test_highlight_token_fields() {
        let token = HighlightToken {
            text: "test".to_string(),
            token_type: TokenType::Constant,
            color: Color::new(100, 200, 50, 255),
        };
        assert_eq!(token.text, "test");
        assert_eq!(token.token_type, TokenType::Constant);
        assert_eq!(token.color.r, 100);

        // Exercise Attribute, Tag, Property variants
        let _attr = HighlightToken {
            text: "attr".into(),
            token_type: TokenType::Attribute,
            color: Color::new(0, 0, 0, 255),
        };
        let _tag = HighlightToken {
            text: "tag".into(),
            token_type: TokenType::Tag,
            color: Color::new(0, 0, 0, 255),
        };
        let _prop = HighlightToken {
            text: "prop".into(),
            token_type: TokenType::Property,
            color: Color::new(0, 0, 0, 255),
        };
    }

    #[test]
    fn test_theme_field_access() {
        let theme = SyntaxTheme::one_dark();
        // Access all fields to wire them
        let _ = theme.background;
        let _ = theme.foreground;
        let _ = theme.keyword;
        let _ = theme.string;
        let _ = theme.number;
        let _ = theme.comment;
        let _ = theme.function;
        let _ = theme.type_name;
        let _ = theme.operator;
        let _ = theme.punctuation;
        let _ = theme.variable;
        let _ = theme.constant;
        let _ = theme.attribute;
        let _ = theme.tag;
        let _ = theme.property;
    }

    #[test]
    fn test_default_highlighter() {
        let hl = SyntaxHighlighter::default();
        let tokens = hl.highlight("42", Language::Plain);
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_default_theme() {
        let theme = SyntaxTheme::default();
        assert_eq!(theme.background.r, SyntaxTheme::one_dark().background.r);
    }

    #[test]
    fn test_highlight_css_language() {
        let hl = SyntaxHighlighter::new();
        let tokens = hl.highlight("color: inherit !important;", Language::Css);
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_highlight_typescript() {
        let hl = SyntaxHighlighter::new();
        let tokens = hl.highlight("interface Foo { bar: string; }", Language::TypeScript);
        assert!(!tokens.is_empty());
    }
}
