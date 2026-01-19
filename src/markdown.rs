//! Markdown Renderer
//!
//! Renders Markdown to styled DOM elements.
//! Supports: headings, lists, code blocks, links, emphasis, tables.

#![allow(dead_code)]

use crate::style::Color;

/// Markdown block element
#[derive(Debug, Clone)]
pub enum MdBlock {
    Heading { level: u8, content: Vec<MdInline> },
    Paragraph(Vec<MdInline>),
    CodeBlock { language: Option<String>, code: String },
    UnorderedList(Vec<Vec<MdInline>>),
    OrderedList(Vec<Vec<MdInline>>),
    Blockquote(Vec<MdBlock>),
    HorizontalRule,
    Table { headers: Vec<String>, rows: Vec<Vec<String>> },
}

/// Markdown inline element
#[derive(Debug, Clone)]
pub enum MdInline {
    Text(String),
    Bold(Vec<MdInline>),
    Italic(Vec<MdInline>),
    Code(String),
    Link { text: String, url: String },
    Image { alt: String, url: String },
    Strikethrough(Vec<MdInline>),
}

/// Markdown parser
pub struct MarkdownParser;

impl MarkdownParser {
    pub fn parse(input: &str) -> Vec<MdBlock> {
        let mut blocks = Vec::new();
        let lines: Vec<&str> = input.lines().collect();
        let mut i = 0;
        
        while i < lines.len() {
            let line = lines[i];
            
            // Empty line
            if line.trim().is_empty() {
                i += 1;
                continue;
            }
            
            // Horizontal rule
            if line.trim().chars().all(|c| c == '-' || c == '*' || c == '_') && line.trim().len() >= 3 {
                blocks.push(MdBlock::HorizontalRule);
                i += 1;
                continue;
            }
            
            // Heading
            if line.starts_with('#') {
                let level = line.chars().take_while(|&c| c == '#').count() as u8;
                let content = line.trim_start_matches('#').trim();
                blocks.push(MdBlock::Heading {
                    level: level.min(6),
                    content: Self::parse_inline(content),
                });
                i += 1;
                continue;
            }
            
            // Code block
            if line.starts_with("```") {
                let language = line.trim_start_matches('`').trim();
                let language = if language.is_empty() { None } else { Some(language.to_string()) };
                
                let mut code = String::new();
                i += 1;
                while i < lines.len() && !lines[i].starts_with("```") {
                    if !code.is_empty() {
                        code.push('\n');
                    }
                    code.push_str(lines[i]);
                    i += 1;
                }
                i += 1; // Skip closing ```
                
                blocks.push(MdBlock::CodeBlock { language, code });
                continue;
            }
            
            // Blockquote
            if line.starts_with('>') {
                let mut quote_lines = Vec::new();
                while i < lines.len() && lines[i].starts_with('>') {
                    quote_lines.push(lines[i].trim_start_matches('>').trim());
                    i += 1;
                }
                let quote_text = quote_lines.join("\n");
                let inner = Self::parse(&quote_text);
                blocks.push(MdBlock::Blockquote(inner));
                continue;
            }
            
            // Unordered list
            if line.trim_start().starts_with("- ") || line.trim_start().starts_with("* ") {
                let mut items = Vec::new();
                while i < lines.len() {
                    let l = lines[i].trim_start();
                    if l.starts_with("- ") || l.starts_with("* ") {
                        let content = l[2..].trim();
                        items.push(Self::parse_inline(content));
                        i += 1;
                    } else if lines[i].starts_with("  ") || lines[i].starts_with("\t") {
                        // Continuation
                        if let Some(last) = items.last_mut() {
                            last.push(MdInline::Text(" ".to_string()));
                            last.extend(Self::parse_inline(lines[i].trim()));
                        }
                        i += 1;
                    } else {
                        break;
                    }
                }
                blocks.push(MdBlock::UnorderedList(items));
                continue;
            }
            
            // Ordered list
            if line.trim_start().chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                let first_char = line.trim_start();
                if first_char.contains(". ") {
                    let mut items = Vec::new();
                    while i < lines.len() {
                        let l = lines[i].trim_start();
                        if l.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) && l.contains(". ") {
                            let dot = l.find(". ").unwrap();
                            let content = &l[dot + 2..];
                            items.push(Self::parse_inline(content));
                            i += 1;
                        } else {
                            break;
                        }
                    }
                    blocks.push(MdBlock::OrderedList(items));
                    continue;
                }
            }
            
            // Table
            if line.contains('|') {
                let mut table_lines = Vec::new();
                while i < lines.len() && lines[i].contains('|') {
                    table_lines.push(lines[i]);
                    i += 1;
                }
                
                if table_lines.len() >= 2 {
                    let headers: Vec<String> = table_lines[0]
                        .split('|')
                        .filter(|s| !s.trim().is_empty())
                        .map(|s| s.trim().to_string())
                        .collect();
                    
                    // Skip separator line (line with ---)
                    let rows: Vec<Vec<String>> = table_lines.iter().skip(2)
                        .map(|line| {
                            line.split('|')
                                .filter(|s| !s.trim().is_empty())
                                .map(|s| s.trim().to_string())
                                .collect()
                        })
                        .collect();
                    
                    blocks.push(MdBlock::Table { headers, rows });
                }
                continue;
            }
            
            // Regular paragraph
            let mut para_lines = Vec::new();
            while i < lines.len() && !lines[i].trim().is_empty() 
                && !lines[i].starts_with('#')
                && !lines[i].starts_with("```")
                && !lines[i].starts_with('>')
                && !lines[i].trim_start().starts_with("- ")
                && !lines[i].trim_start().starts_with("* ")
            {
                para_lines.push(lines[i]);
                i += 1;
            }
            
            let text = para_lines.join(" ");
            blocks.push(MdBlock::Paragraph(Self::parse_inline(&text)));
        }
        
        blocks
    }
    
    fn parse_inline(input: &str) -> Vec<MdInline> {
        let mut result = Vec::new();
        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;
        let mut current_text = String::new();
        
        while i < chars.len() {
            // Bold: **text** or __text__
            if i + 1 < chars.len() && ((chars[i] == '*' && chars[i+1] == '*') || (chars[i] == '_' && chars[i+1] == '_')) {
                let marker = chars[i];
                
                if !current_text.is_empty() {
                    result.push(MdInline::Text(current_text.clone()));
                    current_text.clear();
                }
                
                i += 2;
                let start = i;
                while i + 1 < chars.len() && !(chars[i] == marker && chars[i+1] == marker) {
                    i += 1;
                }
                let inner: String = chars[start..i].iter().collect();
                result.push(MdInline::Bold(Self::parse_inline(&inner)));
                i += 2;
                continue;
            }
            
            // Italic: *text* or _text_
            if (chars[i] == '*' || chars[i] == '_') && i + 1 < chars.len() && chars[i+1] != ' ' {
                let marker = chars[i];
                
                if !current_text.is_empty() {
                    result.push(MdInline::Text(current_text.clone()));
                    current_text.clear();
                }
                
                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != marker {
                    i += 1;
                }
                if i < chars.len() {
                    let inner: String = chars[start..i].iter().collect();
                    result.push(MdInline::Italic(Self::parse_inline(&inner)));
                    i += 1;
                } else {
                    // No closing marker, treat as text
                    current_text.push(marker);
                    i = start;
                }
                continue;
            }
            
            // Inline code: `code`
            if chars[i] == '`' {
                if !current_text.is_empty() {
                    result.push(MdInline::Text(current_text.clone()));
                    current_text.clear();
                }
                
                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != '`' {
                    i += 1;
                }
                let code: String = chars[start..i].iter().collect();
                result.push(MdInline::Code(code));
                if i < chars.len() {
                    i += 1;
                }
                continue;
            }
            
            // Strikethrough: ~~text~~
            if i + 1 < chars.len() && chars[i] == '~' && chars[i+1] == '~' {
                if !current_text.is_empty() {
                    result.push(MdInline::Text(current_text.clone()));
                    current_text.clear();
                }
                
                i += 2;
                let start = i;
                while i + 1 < chars.len() && !(chars[i] == '~' && chars[i+1] == '~') {
                    i += 1;
                }
                let inner: String = chars[start..i].iter().collect();
                result.push(MdInline::Strikethrough(Self::parse_inline(&inner)));
                i += 2;
                continue;
            }
            
            // Link: [text](url)
            if chars[i] == '[' {
                let bracket_start = i;
                i += 1;
                let text_start = i;
                let mut bracket_depth = 1;
                
                while i < chars.len() && bracket_depth > 0 {
                    if chars[i] == '[' { bracket_depth += 1; }
                    if chars[i] == ']' { bracket_depth -= 1; }
                    i += 1;
                }
                
                if i < chars.len() && chars[i] == '(' {
                    let text: String = chars[text_start..i-1].iter().collect();
                    i += 1;
                    let url_start = i;
                    while i < chars.len() && chars[i] != ')' {
                        i += 1;
                    }
                    let url: String = chars[url_start..i].iter().collect();
                    
                    if !current_text.is_empty() {
                        result.push(MdInline::Text(current_text.clone()));
                        current_text.clear();
                    }
                    
                    result.push(MdInline::Link { text, url });
                    if i < chars.len() {
                        i += 1;
                    }
                    continue;
                } else {
                    // Not a link, treat as text
                    i = bracket_start;
                }
            }
            
            // Image: ![alt](url)
            if chars[i] == '!' && i + 1 < chars.len() && chars[i+1] == '[' {
                i += 2;
                let alt_start = i;
                while i < chars.len() && chars[i] != ']' {
                    i += 1;
                }
                let alt: String = chars[alt_start..i].iter().collect();
                
                if i + 1 < chars.len() && chars[i] == ']' && chars[i+1] == '(' {
                    i += 2;
                    let url_start = i;
                    while i < chars.len() && chars[i] != ')' {
                        i += 1;
                    }
                    let url: String = chars[url_start..i].iter().collect();
                    
                    if !current_text.is_empty() {
                        result.push(MdInline::Text(current_text.clone()));
                        current_text.clear();
                    }
                    
                    result.push(MdInline::Image { alt, url });
                    if i < chars.len() {
                        i += 1;
                    }
                    continue;
                }
            }
            
            current_text.push(chars[i]);
            i += 1;
        }
        
        if !current_text.is_empty() {
            result.push(MdInline::Text(current_text));
        }
        
        result
    }
}

/// Style information for rendering
pub struct MdStyle {
    pub heading_colors: [Color; 6],
    pub text_color: Color,
    pub link_color: Color,
    pub code_bg: Color,
    pub code_text: Color,
    pub blockquote_border: Color,
    pub blockquote_bg: Color,
}

impl Default for MdStyle {
    fn default() -> Self {
        MdStyle {
            heading_colors: [
                Color::new(229, 192, 123, 255),  // h1 - golden
                Color::new(97, 175, 239, 255),   // h2 - blue
                Color::new(152, 195, 121, 255),  // h3 - green
                Color::new(198, 120, 221, 255),  // h4 - purple
                Color::new(86, 182, 194, 255),   // h5 - cyan
                Color::new(224, 108, 117, 255),  // h6 - red
            ],
            text_color: Color::new(220, 220, 220, 255),
            link_color: Color::new(97, 175, 239, 255),
            code_bg: Color::new(40, 44, 52, 255),
            code_text: Color::new(152, 195, 121, 255),
            blockquote_border: Color::new(100, 100, 100, 255),
            blockquote_bg: Color::new(35, 35, 40, 255),
        }
    }
}

/// Simple plaintext renderer (for testing)
pub fn render_to_text(blocks: &[MdBlock]) -> String {
    let mut output = String::new();
    
    for block in blocks {
        match block {
            MdBlock::Heading { level, content } => {
                output.push_str(&"#".repeat(*level as usize));
                output.push(' ');
                output.push_str(&inline_to_text(content));
                output.push('\n');
            }
            MdBlock::Paragraph(content) => {
                output.push_str(&inline_to_text(content));
                output.push_str("\n\n");
            }
            MdBlock::CodeBlock { language, code } => {
                output.push_str("```");
                if let Some(lang) = language {
                    output.push_str(lang);
                }
                output.push('\n');
                output.push_str(code);
                output.push_str("\n```\n");
            }
            MdBlock::UnorderedList(items) => {
                for item in items {
                    output.push_str("- ");
                    output.push_str(&inline_to_text(item));
                    output.push('\n');
                }
                output.push('\n');
            }
            MdBlock::OrderedList(items) => {
                for (i, item) in items.iter().enumerate() {
                    output.push_str(&format!("{}. ", i + 1));
                    output.push_str(&inline_to_text(item));
                    output.push('\n');
                }
                output.push('\n');
            }
            MdBlock::Blockquote(blocks) => {
                let inner = render_to_text(blocks);
                for line in inner.lines() {
                    output.push_str("> ");
                    output.push_str(line);
                    output.push('\n');
                }
            }
            MdBlock::HorizontalRule => {
                output.push_str("---\n");
            }
            MdBlock::Table { headers, rows } => {
                output.push_str("| ");
                output.push_str(&headers.join(" | "));
                output.push_str(" |\n");
                output.push('|');
                for _ in headers {
                    output.push_str("---|");
                }
                output.push('\n');
                for row in rows {
                    output.push_str("| ");
                    output.push_str(&row.join(" | "));
                    output.push_str(" |\n");
                }
            }
        }
    }
    
    output
}

fn inline_to_text(inlines: &[MdInline]) -> String {
    let mut output = String::new();
    
    for inline in inlines {
        match inline {
            MdInline::Text(t) => output.push_str(t),
            MdInline::Bold(content) => {
                output.push_str(&inline_to_text(content));
            }
            MdInline::Italic(content) => {
                output.push_str(&inline_to_text(content));
            }
            MdInline::Code(code) => {
                output.push('`');
                output.push_str(code);
                output.push('`');
            }
            MdInline::Link { text, .. } => {
                output.push_str(text);
            }
            MdInline::Image { alt, .. } => {
                output.push_str(alt);
            }
            MdInline::Strikethrough(content) => {
                output.push_str(&inline_to_text(content));
            }
        }
    }
    
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_heading() {
        let blocks = MarkdownParser::parse("# Hello World");
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            MdBlock::Heading { level, .. } => assert_eq!(*level, 1),
            _ => panic!("Expected heading"),
        }
    }
    
    #[test]
    fn test_code_block() {
        let blocks = MarkdownParser::parse("```rust\nfn main() {}\n```");
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            MdBlock::CodeBlock { language, code } => {
                assert_eq!(language.as_deref(), Some("rust"));
                assert!(code.contains("fn main"));
            }
            _ => panic!("Expected code block"),
        }
    }
    
    #[test]
    fn test_inline_bold() {
        let inlines = MarkdownParser::parse_inline("Hello **world**!");
        assert_eq!(inlines.len(), 3);
    }
}
