use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Number(f64),
    String(String),
    Identifier(String),
    Boolean(bool),
    Null,
    Undefined,

    // Keywords
    Var,
    Let,
    Const,
    Function,
    Return,
    If,
    Else,
    While,
    For,
    Break,
    Continue,
    New,
    This,
    Typeof,
    Instanceof,
    Async,
    Await,
    Try,
    Catch,
    Finally,
    Throw,
    Class,
    Extends,
    Super,
    Static,
    Get,
    Set,
    Import,
    Export,
    Default,
    From,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    StarStar,
    PlusPlus,
    MinusMinus,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    Eq,
    EqEq,
    EqEqEq,
    NotEq,
    NotEqEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
    Not,
    NullishCoalesce,
    BitAnd,
    BitOr,
    BitXor,
    BitNot,
    ShiftLeft,
    ShiftRight,
    ShiftRightUnsigned,
    Question,
    Colon,
    OptionalChain,
    Spread,

    // Punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Dot,
    Semicolon,
    Arrow,

    // Template literals
    TemplateStart,
    TemplateMiddle,
    TemplateEnd,
    TemplateLiteral,

    // Special
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}

pub struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
    current: char,
    line: usize,
    column: usize,
    eof: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        let mut chars = source.chars().peekable();
        let current = chars.next().unwrap_or('\0');
        Lexer {
            chars,
            current,
            line: 1,
            column: 1,
            eof: source.is_empty(),
        }
    }

    fn advance(&mut self) -> char {
        let prev = self.current;
        self.current = self.chars.next().unwrap_or('\0');
        if self.current == '\0' {
            self.eof = true;
        }
        if prev == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        prev
    }

    fn peek(&mut self) -> char {
        *self.chars.peek().unwrap_or(&'\0')
    }

    fn skip_whitespace(&mut self) {
        while self.current.is_whitespace() {
            self.advance();
        }
    }

    fn skip_comment(&mut self) {
        if self.current == '/' {
            if self.peek() == '/' {
                while self.current != '\n' && !self.eof {
                    self.advance();
                }
            } else if self.peek() == '*' {
                self.advance();
                self.advance();
                while !self.eof {
                    if self.current == '*' && self.peek() == '/' {
                        self.advance();
                        self.advance();
                        break;
                    }
                    self.advance();
                }
            }
        }
    }

    fn read_number(&mut self) -> Token {
        let line = self.line;
        let column = self.column;
        let mut num_str = String::new();

        // Handle hex, octal, binary
        if self.current == '0' {
            num_str.push(self.advance());
            match self.current {
                'x' | 'X' => {
                    num_str.push(self.advance());
                    while self.current.is_ascii_hexdigit() {
                        num_str.push(self.advance());
                    }
                    let value = i64::from_str_radix(&num_str[2..], 16).unwrap_or(0) as f64;
                    return Token {
                        kind: TokenKind::Number(value),
                        line,
                        column,
                    };
                }
                'b' | 'B' => {
                    num_str.push(self.advance());
                    while self.current == '0' || self.current == '1' {
                        num_str.push(self.advance());
                    }
                    let value = i64::from_str_radix(&num_str[2..], 2).unwrap_or(0) as f64;
                    return Token {
                        kind: TokenKind::Number(value),
                        line,
                        column,
                    };
                }
                'o' | 'O' => {
                    num_str.push(self.advance());
                    while self.current >= '0' && self.current <= '7' {
                        num_str.push(self.advance());
                    }
                    let value = i64::from_str_radix(&num_str[2..], 8).unwrap_or(0) as f64;
                    return Token {
                        kind: TokenKind::Number(value),
                        line,
                        column,
                    };
                }
                _ => {}
            }
        }

        while self.current.is_ascii_digit() {
            num_str.push(self.advance());
        }

        if self.current == '.' && self.peek().is_ascii_digit() {
            num_str.push(self.advance());
            while self.current.is_ascii_digit() {
                num_str.push(self.advance());
            }
        }

        // Scientific notation
        if self.current == 'e' || self.current == 'E' {
            num_str.push(self.advance());
            if self.current == '+' || self.current == '-' {
                num_str.push(self.advance());
            }
            while self.current.is_ascii_digit() {
                num_str.push(self.advance());
            }
        }

        let value = num_str.parse::<f64>().unwrap_or(0.0);
        Token {
            kind: TokenKind::Number(value),
            line,
            column,
        }
    }

    fn read_string(&mut self) -> Token {
        let line = self.line;
        let column = self.column;
        let quote = self.advance();
        let mut value = String::new();

        while self.current != quote && !self.eof {
            if self.current == '\\' {
                self.advance();
                match self.current {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    'r' => value.push('\r'),
                    '\\' => value.push('\\'),
                    '"' => value.push('"'),
                    '\'' => value.push('\''),
                    '0' => value.push('\0'),
                    'x' => {
                        self.advance();
                        let hex: String = [self.advance(), self.advance()].iter().collect();
                        if let Ok(n) = u8::from_str_radix(&hex, 16) {
                            value.push(n as char);
                        }
                        continue;
                    }
                    'u' => {
                        self.advance();
                        if self.current == '{' {
                            self.advance();
                            let mut hex = String::new();
                            while self.current != '}' && !self.eof {
                                hex.push(self.advance());
                            }
                            self.advance();
                            if let Ok(n) = u32::from_str_radix(&hex, 16) {
                                if let Some(c) = char::from_u32(n) {
                                    value.push(c);
                                }
                            }
                        } else {
                            let hex: String = (0..4).map(|_| self.advance()).collect();
                            if let Ok(n) = u16::from_str_radix(&hex, 16) {
                                if let Some(c) = char::from_u32(n as u32) {
                                    value.push(c);
                                }
                            }
                        }
                        continue;
                    }
                    _ => value.push(self.current),
                }
                self.advance();
            } else {
                value.push(self.advance());
            }
        }

        if self.current == quote {
            self.advance();
        }

        Token {
            kind: TokenKind::String(value),
            line,
            column,
        }
    }

    fn read_template_string(&mut self) -> Token {
        let line = self.line;
        let column = self.column;
        self.advance(); // `
        let mut value = String::new();

        while self.current != '`' && !self.eof {
            if self.current == '$' && self.peek() == '{' {
                // Template expression - for now just include literally
                value.push(self.advance());
                value.push(self.advance());
            } else if self.current == '\\' {
                self.advance();
                match self.current {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    '`' => value.push('`'),
                    '$' => value.push('$'),
                    '\\' => value.push('\\'),
                    _ => value.push(self.current),
                }
                self.advance();
            } else {
                value.push(self.advance());
            }
        }

        if self.current == '`' {
            self.advance();
        }

        Token {
            kind: TokenKind::TemplateLiteral,
            line,
            column,
        }
    }

    fn read_identifier(&mut self) -> Token {
        let line = self.line;
        let column = self.column;
        let mut name = String::new();

        while self.current.is_alphanumeric() || self.current == '_' || self.current == '$' {
            name.push(self.advance());
        }

        let kind = match name.as_str() {
            "var" => TokenKind::Var,
            "let" => TokenKind::Let,
            "const" => TokenKind::Const,
            "function" => TokenKind::Function,
            "return" => TokenKind::Return,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "new" => TokenKind::New,
            "this" => TokenKind::This,
            "typeof" => TokenKind::Typeof,
            "instanceof" => TokenKind::Instanceof,
            "async" => TokenKind::Async,
            "await" => TokenKind::Await,
            "try" => TokenKind::Try,
            "catch" => TokenKind::Catch,
            "finally" => TokenKind::Finally,
            "throw" => TokenKind::Throw,
            "class" => TokenKind::Class,
            "extends" => TokenKind::Extends,
            "super" => TokenKind::Super,
            "static" => TokenKind::Static,
            "get" => TokenKind::Get,
            "set" => TokenKind::Set,
            "import" => TokenKind::Import,
            "export" => TokenKind::Export,
            "default" => TokenKind::Default,
            "from" => TokenKind::From,
            "true" => TokenKind::Boolean(true),
            "false" => TokenKind::Boolean(false),
            "null" => TokenKind::Null,
            "undefined" => TokenKind::Undefined,
            _ => TokenKind::Identifier(name),
        };

        Token { kind, line, column }
    }

    pub fn next_token(&mut self) -> Token {
        loop {
            self.skip_whitespace();

            if self.current == '/' && (self.peek() == '/' || self.peek() == '*') {
                self.skip_comment();
                continue;
            }

            break;
        }

        let line = self.line;
        let column = self.column;

        if self.eof {
            return Token {
                kind: TokenKind::Eof,
                line,
                column,
            };
        }

        if self.current.is_ascii_digit() {
            return self.read_number();
        }

        if self.current == '"' || self.current == '\'' {
            return self.read_string();
        }

        if self.current == '`' {
            return self.read_template_string();
        }

        if self.current.is_alphabetic() || self.current == '_' || self.current == '$' {
            return self.read_identifier();
        }

        let kind = match self.current {
            '+' => {
                self.advance();
                if self.current == '+' {
                    self.advance();
                    TokenKind::PlusPlus
                } else if self.current == '=' {
                    self.advance();
                    TokenKind::PlusEq
                } else {
                    TokenKind::Plus
                }
            }
            '-' => {
                self.advance();
                if self.current == '-' {
                    self.advance();
                    TokenKind::MinusMinus
                } else if self.current == '=' {
                    self.advance();
                    TokenKind::MinusEq
                } else {
                    TokenKind::Minus
                }
            }
            '*' => {
                self.advance();
                if self.current == '*' {
                    self.advance();
                    TokenKind::StarStar
                } else if self.current == '=' {
                    self.advance();
                    TokenKind::StarEq
                } else {
                    TokenKind::Star
                }
            }
            '/' => {
                self.advance();
                if self.current == '=' {
                    self.advance();
                    TokenKind::SlashEq
                } else {
                    TokenKind::Slash
                }
            }
            '%' => {
                self.advance();
                if self.current == '=' {
                    self.advance();
                    TokenKind::PercentEq
                } else {
                    TokenKind::Percent
                }
            }
            '=' => {
                self.advance();
                if self.current == '=' {
                    self.advance();
                    if self.current == '=' {
                        self.advance();
                        TokenKind::EqEqEq
                    } else {
                        TokenKind::EqEq
                    }
                } else if self.current == '>' {
                    self.advance();
                    TokenKind::Arrow
                } else {
                    TokenKind::Eq
                }
            }
            '!' => {
                self.advance();
                if self.current == '=' {
                    self.advance();
                    if self.current == '=' {
                        self.advance();
                        TokenKind::NotEqEq
                    } else {
                        TokenKind::NotEq
                    }
                } else {
                    TokenKind::Not
                }
            }
            '<' => {
                self.advance();
                if self.current == '=' {
                    self.advance();
                    TokenKind::LtEq
                } else if self.current == '<' {
                    self.advance();
                    TokenKind::ShiftLeft
                } else {
                    TokenKind::Lt
                }
            }
            '>' => {
                self.advance();
                if self.current == '=' {
                    self.advance();
                    TokenKind::GtEq
                } else if self.current == '>' {
                    self.advance();
                    if self.current == '>' {
                        self.advance();
                        TokenKind::ShiftRightUnsigned
                    } else {
                        TokenKind::ShiftRight
                    }
                } else {
                    TokenKind::Gt
                }
            }
            '&' => {
                self.advance();
                if self.current == '&' {
                    self.advance();
                    TokenKind::And
                } else {
                    TokenKind::BitAnd
                }
            }
            '|' => {
                self.advance();
                if self.current == '|' {
                    self.advance();
                    TokenKind::Or
                } else {
                    TokenKind::BitOr
                }
            }
            '?' => {
                self.advance();
                if self.current == '?' {
                    self.advance();
                    TokenKind::NullishCoalesce
                } else if self.current == '.' {
                    self.advance();
                    TokenKind::OptionalChain
                } else {
                    TokenKind::Question
                }
            }
            '^' => {
                self.advance();
                TokenKind::BitXor
            }
            '~' => {
                self.advance();
                TokenKind::BitNot
            }
            ':' => {
                self.advance();
                TokenKind::Colon
            }
            '(' => {
                self.advance();
                TokenKind::LParen
            }
            ')' => {
                self.advance();
                TokenKind::RParen
            }
            '{' => {
                self.advance();
                TokenKind::LBrace
            }
            '}' => {
                self.advance();
                TokenKind::RBrace
            }
            '[' => {
                self.advance();
                TokenKind::LBracket
            }
            ']' => {
                self.advance();
                TokenKind::RBracket
            }
            ',' => {
                self.advance();
                TokenKind::Comma
            }
            '.' => {
                self.advance();
                if self.current == '.' && self.peek() == '.' {
                    self.advance();
                    self.advance();
                    TokenKind::Spread
                } else {
                    TokenKind::Dot
                }
            }
            ';' => {
                self.advance();
                TokenKind::Semicolon
            }
            _ => {
                self.advance();
                return self.next_token();
            }
        };

        Token { kind, line, column }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            let is_eof = matches!(token.kind, TokenKind::Eof);
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        tokens
    }
}
