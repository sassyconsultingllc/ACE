//! Syntax Highlighting Engine
//!
//! Renders code blocks with beautiful syntax highlighting.
//! Supports: JavaScript, TypeScript, Rust, Python, HTML, CSS, JSON, Markdown

#![allow(dead_code)]

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
            background: Color::new(40, 44, 52, 255),      // #282c34
            foreground: Color::new(171, 178, 191, 255),   // #abb2bf
            keyword: Color::new(198, 120, 221, 255),      // #c678dd (purple)
            string: Color::new(152, 195, 121, 255),       // #98c379 (green)
            number: Color::new(209, 154, 102, 255),       // #d19a66 (orange)
            comment: Color::new(92, 99, 112, 255),        // #5c6370 (gray)
            function: Color::new(97, 175, 239, 255),      // #61afef (blue)
            type_name: Color::new(229, 192, 123, 255),    // #e5c07b (yellow)
            operator: Color::new(86, 182, 194, 255),      // #56b6c2 (cyan)
            punctuation: Color::new(171, 178, 191, 255),  // #abb2bf
            variable: Color::new(224, 108, 117, 255),     // #e06c75 (red)
            constant: Color::new(209, 154, 102, 255),     // #d19a66
            attribute: Color::new(209, 154, 102, 255),    // #d19a66
            tag: Color::new(224, 108, 117, 255),          // #e06c75
            property: Color::new(224, 108, 117, 255),     // #e06c75
        }
    }
    
    /// GitHub Light theme
    pub fn github_light() -> Self {
        SyntaxTheme {
            background: Color::new(255, 255, 255, 255),
            foreground: Color::new(36, 41, 46, 255),
            keyword: Color::new(215, 58, 73, 255),        // #d73a49
            string: Color::new(3, 47, 98, 255),           // #032f62
            number: Color::new(0, 92, 197, 255),          // #005cc5
            comment: Color::new(106, 115, 125, 255),      // #6a737d
            function: Color::new(111, 66, 193, 255),      // #6f42c1
            type_name: Color::new(0, 92, 197, 255),       // #005cc5
            operator: Color::new(215, 58, 73, 255),       // #d73a49
            punctuation: Color::new(36, 41, 46, 255),
            variable: Color::new(227, 98, 9, 255),        // #e36209
            constant: Color::new(0, 92, 197, 255),
            attribute: Color::new(0, 92, 197, 255),
            tag: Color::new(34, 134, 58, 255),            // #22863a
            property: Color::new(0, 92, 197, 255),
        }
    }
    
    /// Dracula theme
    pub fn dracula() -> Self {
        SyntaxTheme {
            background: Color::new(40, 42, 54, 255),      // #282a36
            foreground: Color::new(248, 248, 242, 255),   // #f8f8f2
            keyword: Color::new(255, 121, 198, 255),      // #ff79c6 (pink)
            string: Color::new(241, 250, 140, 255),       // #f1fa8c (yellow)
            number: Color::new(189, 147, 249, 255),       // #bd93f9 (purple)
            comment: Color::new(98, 114, 164, 255),       // #6272a4
            function: Color::new(80, 250, 123, 255),      // #50fa7b (green)
            type_name: Color::new(139, 233, 253, 255),    // #8be9fd (cyan)
            operator: Color::new(255, 121, 198, 255),     // #ff79c6
            punctuation: Color::new(248, 248, 242, 255),
            variable: Color::new(248, 248, 242, 255),
            constant: Color::new(189, 147, 249, 255),
            attribute: Color::new(80, 250, 123, 255),
            tag: Color::new(255, 121, 198, 255),
            property: Color::new(139, 233, 253, 255),
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
        match s.to_lowercase().as_str() {
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
        
        keywords.insert(Language::JavaScript, vec![
            "const", "let", "var", "function", "return", "if", "else", "for", "while",
            "do", "switch", "case", "break", "continue", "try", "catch", "finally",
            "throw", "new", "this", "class", "extends", "super", "import", "export",
            "from", "default", "async", "await", "yield", "typeof", "instanceof",
            "in", "of", "true", "false", "null", "undefined", "void", "delete",
        ]);
        
        keywords.insert(Language::TypeScript, vec![
            "const", "let", "var", "function", "return", "if", "else", "for", "while",
            "do", "switch", "case", "break", "continue", "try", "catch", "finally",
            "throw", "new", "this", "class", "extends", "super", "import", "export",
            "from", "default", "async", "await", "yield", "typeof", "instanceof",
            "in", "of", "true", "false", "null", "undefined", "void", "delete",
            "interface", "type", "enum", "implements", "public", "private", "protected",
            "readonly", "abstract", "as", "is", "keyof", "infer", "never", "unknown",
        ]);
        
        keywords.insert(Language::Rust, vec![
            "fn", "let", "mut", "const", "static", "if", "else", "match", "for", "while",
            "loop", "break", "continue", "return", "struct", "enum", "impl", "trait",
            "pub", "mod", "use", "crate", "self", "Self", "super", "where", "async",
            "await", "move", "ref", "dyn", "type", "as", "in", "true", "false", "Some",
            "None", "Ok", "Err", "unsafe", "extern",
        ]);
        
        keywords.insert(Language::Python, vec![
            "def", "class", "if", "elif", "else", "for", "while", "try", "except",
            "finally", "with", "as", "import", "from", "return", "yield", "raise",
            "pass", "break", "continue", "and", "or", "not", "in", "is", "lambda",
            "True", "False", "None", "global", "nonlocal", "assert", "async", "await",
        ]);
        
        keywords.insert(Language::Css, vec![
            "important", "inherit", "initial", "unset", "auto", "none",
        ]);
        
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
        let keywords = self.keywords.get(&language).map(|v| v.as_slice()).unwrap_or(&[]);
        
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
            if c.is_ascii_digit() || (c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit()) {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == 'x' || chars[i] == 'b' || chars[i].is_ascii_hexdigit()) {
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
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '$') {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                
                let token_type = if keywords.contains(&text.as_str()) {
                    TokenType::Keyword
                } else if text.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
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
}
