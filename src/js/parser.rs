#![allow(dead_code, unused_variables, unused_imports)]

use super::lexer::{Token, TokenKind, Lexer};

#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    String(String),
    Boolean(bool),
    Null,
    Undefined,
    Identifier(String),
    Array(Vec<Expr>),
    Object(Vec<(String, Expr)>),
    
    Binary { left: Box<Expr>, op: String, right: Box<Expr> },
    Unary { op: String, expr: Box<Expr> },
    Logical { left: Box<Expr>, op: String, right: Box<Expr> },
    Ternary { condition: Box<Expr>, consequent: Box<Expr>, alternate: Box<Expr> },
    
    Assignment { target: Box<Expr>, value: Box<Expr> },
    Compound { target: Box<Expr>, op: String, value: Box<Expr> },
    
    Member { object: Box<Expr>, property: String },
    Index { object: Box<Expr>, index: Box<Expr> },
    Call { callee: Box<Expr>, args: Vec<Expr> },
    New { callee: Box<Expr>, args: Vec<Expr> },
    
    Function { name: Option<String>, params: Vec<String>, body: Vec<Stmt> },
    Arrow { params: Vec<String>, body: Box<ArrowBody> },
    
    This,
    Typeof(Box<Expr>),
    
    PreIncrement(Box<Expr>),
    PreDecrement(Box<Expr>),
    PostIncrement(Box<Expr>),
    PostDecrement(Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum ArrowBody {
    Expr(Expr),
    Block(Vec<Stmt>),
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expr(Expr),
    Var { name: String, init: Option<Expr> },
    Block(Vec<Stmt>),
    If { condition: Expr, consequent: Box<Stmt>, alternate: Option<Box<Stmt>> },
    While { condition: Expr, body: Box<Stmt> },
    For { init: Option<Box<Stmt>>, condition: Option<Expr>, update: Option<Expr>, body: Box<Stmt> },
    Function { name: String, params: Vec<String>, body: Vec<Stmt> },
    Return(Option<Expr>),
    Break,
    Continue,
    Empty,
    Try { body: Vec<Stmt>, catch_param: Option<String>, catch_body: Option<Vec<Stmt>>, finally_body: Option<Vec<Stmt>> },
    Throw(Expr),
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(source: &str) -> Self {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        Parser { tokens, pos: 0 }
    }
    
    fn current(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token { 
            kind: TokenKind::Eof, line: 0, column: 0 
        })
    }
    
    fn advance(&mut self) -> &Token {
        let token = self.current();
        if !matches!(token.kind, TokenKind::Eof) {
            self.pos += 1;
        }
        self.tokens.get(self.pos - 1).unwrap()
    }
    
    fn expect(&mut self, kind: TokenKind) -> Result<&Token, String> {
        if std::mem::discriminant(&self.current().kind) == std::mem::discriminant(&kind) {
            Ok(self.advance())
        } else {
            Err(format!("Expected {:?}, got {:?}", kind, self.current().kind))
        }
    }
    
    fn is_at_end(&self) -> bool {
        matches!(self.current().kind, TokenKind::Eof)
    }
    
    pub fn parse(&mut self) -> Result<Vec<Stmt>, String> {
        let mut statements = Vec::new();
        while !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }
        Ok(statements)
    }
    
    fn parse_statement(&mut self) -> Result<Stmt, String> {
        match &self.current().kind {
            TokenKind::Var | TokenKind::Let | TokenKind::Const => self.parse_var_declaration(),
            TokenKind::Function => self.parse_function_declaration(),
            TokenKind::If => self.parse_if_statement(),
            TokenKind::While => self.parse_while_statement(),
            TokenKind::For => self.parse_for_statement(),
            TokenKind::Return => self.parse_return_statement(),
            TokenKind::Break => { self.advance(); self.consume_semicolon(); Ok(Stmt::Break) }
            TokenKind::Continue => { self.advance(); self.consume_semicolon(); Ok(Stmt::Continue) }
            TokenKind::LBrace => self.parse_block(),
            TokenKind::Semicolon => { self.advance(); Ok(Stmt::Empty) }
            TokenKind::Try => self.parse_try_statement(),
            TokenKind::Throw => self.parse_throw_statement(),
            _ => self.parse_expression_statement(),
        }
    }
    
    fn consume_semicolon(&mut self) {
        if matches!(self.current().kind, TokenKind::Semicolon) {
            self.advance();
        }
    }
    
    fn parse_var_declaration(&mut self) -> Result<Stmt, String> {
        self.advance();
        
        if let TokenKind::Identifier(name) = self.current().kind.clone() {
            self.advance();
            
            let init = if matches!(self.current().kind, TokenKind::Eq) {
                self.advance();
                Some(self.parse_expression()?)
            } else {
                None
            };
            
            self.consume_semicolon();
            Ok(Stmt::Var { name, init })
        } else {
            Err("Expected identifier".to_string())
        }
    }
    
    fn parse_function_declaration(&mut self) -> Result<Stmt, String> {
        self.advance();
        
        let name = if let TokenKind::Identifier(n) = self.current().kind.clone() {
            self.advance();
            n
        } else {
            return Err("Expected function name".to_string());
        };
        
        self.expect(TokenKind::LParen)?;
        let params = self.parse_parameters()?;
        self.expect(TokenKind::RParen)?;
        
        self.expect(TokenKind::LBrace)?;
        let body = self.parse_statement_list()?;
        self.expect(TokenKind::RBrace)?;
        
        Ok(Stmt::Function { name, params, body })
    }
    
    fn parse_parameters(&mut self) -> Result<Vec<String>, String> {
        let mut params = Vec::new();
        
        while let TokenKind::Identifier(name) = self.current().kind.clone() {
            params.push(name);
            self.advance();
            
            if matches!(self.current().kind, TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        
        Ok(params)
    }
    
    fn parse_statement_list(&mut self) -> Result<Vec<Stmt>, String> {
        let mut statements = Vec::new();
        
        while !matches!(self.current().kind, TokenKind::RBrace | TokenKind::Eof) {
            statements.push(self.parse_statement()?);
        }
        
        Ok(statements)
    }
    
    fn parse_block(&mut self) -> Result<Stmt, String> {
        self.expect(TokenKind::LBrace)?;
        let statements = self.parse_statement_list()?;
        self.expect(TokenKind::RBrace)?;
        Ok(Stmt::Block(statements))
    }
    
    fn parse_if_statement(&mut self) -> Result<Stmt, String> {
        self.advance();
        self.expect(TokenKind::LParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::RParen)?;
        
        let consequent = Box::new(self.parse_statement()?);
        
        let alternate = if matches!(self.current().kind, TokenKind::Else) {
            self.advance();
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };
        
        Ok(Stmt::If { condition, consequent, alternate })
    }
    
    fn parse_while_statement(&mut self) -> Result<Stmt, String> {
        self.advance();
        self.expect(TokenKind::LParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::RParen)?;
        let body = Box::new(self.parse_statement()?);
        
        Ok(Stmt::While { condition, body })
    }
    
    fn parse_for_statement(&mut self) -> Result<Stmt, String> {
        self.advance();
        self.expect(TokenKind::LParen)?;
        
        let init = if matches!(self.current().kind, TokenKind::Semicolon) {
            self.advance();
            None
        } else if matches!(self.current().kind, TokenKind::Var | TokenKind::Let) {
            Some(Box::new(self.parse_var_declaration()?))
        } else {
            let expr = self.parse_expression()?;
            self.consume_semicolon();
            Some(Box::new(Stmt::Expr(expr)))
        };
        
        let condition = if matches!(self.current().kind, TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.consume_semicolon();
        
        let update = if matches!(self.current().kind, TokenKind::RParen) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        
        self.expect(TokenKind::RParen)?;
        let body = Box::new(self.parse_statement()?);
        
        Ok(Stmt::For { init, condition, update, body })
    }
    
    fn parse_return_statement(&mut self) -> Result<Stmt, String> {
        self.advance();
        
        if matches!(self.current().kind, TokenKind::Semicolon | TokenKind::RBrace | TokenKind::Eof) {
            self.consume_semicolon();
            return Ok(Stmt::Return(None));
        }
        
        let value = self.parse_expression()?;
        self.consume_semicolon();
        Ok(Stmt::Return(Some(value)))
    }
    
    fn parse_try_statement(&mut self) -> Result<Stmt, String> {
        self.advance(); // try
        self.expect(TokenKind::LBrace)?;
        let body = self.parse_statement_list()?;
        self.expect(TokenKind::RBrace)?;
        
        let (catch_param, catch_body) = if matches!(self.current().kind, TokenKind::Catch) {
            self.advance();
            let param = if matches!(self.current().kind, TokenKind::LParen) {
                self.advance();
                let p = if let TokenKind::Identifier(n) = self.current().kind.clone() {
                    self.advance();
                    Some(n)
                } else {
                    None
                };
                self.expect(TokenKind::RParen)?;
                p
            } else {
                None
            };
            self.expect(TokenKind::LBrace)?;
            let body = self.parse_statement_list()?;
            self.expect(TokenKind::RBrace)?;
            (param, Some(body))
        } else {
            (None, None)
        };
        
        let finally_body = if matches!(self.current().kind, TokenKind::Finally) {
            self.advance();
            self.expect(TokenKind::LBrace)?;
            let body = self.parse_statement_list()?;
            self.expect(TokenKind::RBrace)?;
            Some(body)
        } else {
            None
        };
        
        Ok(Stmt::Try { body, catch_param, catch_body, finally_body })
    }
    
    fn parse_throw_statement(&mut self) -> Result<Stmt, String> {
        self.advance();
        let expr = self.parse_expression()?;
        self.consume_semicolon();
        Ok(Stmt::Throw(expr))
    }
    
    fn parse_expression_statement(&mut self) -> Result<Stmt, String> {
        let expr = self.parse_expression()?;
        self.consume_semicolon();
        Ok(Stmt::Expr(expr))
    }
    
    fn parse_expression(&mut self) -> Result<Expr, String> {
        // Support comma operator (evaluates operands left-to-right, returns last)
        self.parse_comma()
    }

    fn parse_comma(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_assignment()?;

        while matches!(self.current().kind, TokenKind::Comma) {
            self.advance();
            let right = self.parse_assignment()?;
            expr = Expr::Binary { left: Box::new(expr), op: ",".to_string(), right: Box::new(right) };
        }

        Ok(expr)
    }
    
    fn parse_assignment(&mut self) -> Result<Expr, String> {
        let expr = self.parse_ternary()?;
        
        match &self.current().kind {
            TokenKind::Eq => {
                self.advance();
                let value = self.parse_assignment()?;
                Ok(Expr::Assignment { target: Box::new(expr), value: Box::new(value) })
            }
            TokenKind::PlusEq => {
                self.advance();
                let value = self.parse_assignment()?;
                Ok(Expr::Compound { target: Box::new(expr), op: "+".to_string(), value: Box::new(value) })
            }
            TokenKind::MinusEq => {
                self.advance();
                let value = self.parse_assignment()?;
                Ok(Expr::Compound { target: Box::new(expr), op: "-".to_string(), value: Box::new(value) })
            }
            TokenKind::StarEq => {
                self.advance();
                let value = self.parse_assignment()?;
                Ok(Expr::Compound { target: Box::new(expr), op: "*".to_string(), value: Box::new(value) })
            }
            TokenKind::SlashEq => {
                self.advance();
                let value = self.parse_assignment()?;
                Ok(Expr::Compound { target: Box::new(expr), op: "/".to_string(), value: Box::new(value) })
            }
            _ => Ok(expr)
        }
    }
    
    fn parse_ternary(&mut self) -> Result<Expr, String> {
        let condition = self.parse_or()?;
        
        if matches!(self.current().kind, TokenKind::Question) {
            self.advance();
            let consequent = self.parse_assignment()?;
            self.expect(TokenKind::Colon)?;
            let alternate = self.parse_assignment()?;
            Ok(Expr::Ternary {
                condition: Box::new(condition),
                consequent: Box::new(consequent),
                alternate: Box::new(alternate),
            })
        } else {
            Ok(condition)
        }
    }
    
    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and()?;
        
        while matches!(self.current().kind, TokenKind::Or | TokenKind::NullishCoalesce) {
            let op = if matches!(self.current().kind, TokenKind::NullishCoalesce) { "??" } else { "||" };
            self.advance();
            let right = self.parse_and()?;
            left = Expr::Logical { left: Box::new(left), op: op.to_string(), right: Box::new(right) };
        }
        
        Ok(left)
    }
    
    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_equality()?;
        
        while matches!(self.current().kind, TokenKind::And) {
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::Logical { left: Box::new(left), op: "&&".to_string(), right: Box::new(right) };
        }
        
        Ok(left)
    }
    
    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_comparison()?;
        
        loop {
            let op = match &self.current().kind {
                TokenKind::EqEq => "==",
                TokenKind::EqEqEq => "===",
                TokenKind::NotEq => "!=",
                TokenKind::NotEqEq => "!==",
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::Binary { left: Box::new(left), op: op.to_string(), right: Box::new(right) };
        }
        
        Ok(left)
    }
    
    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_additive()?;
        
        loop {
            let op = match &self.current().kind {
                TokenKind::Lt => "<",
                TokenKind::LtEq => "<=",
                TokenKind::Gt => ">",
                TokenKind::GtEq => ">=",
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            left = Expr::Binary { left: Box::new(left), op: op.to_string(), right: Box::new(right) };
        }
        
        Ok(left)
    }
    
    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplicative()?;
        
        loop {
            let op = match &self.current().kind {
                TokenKind::Plus => "+",
                TokenKind::Minus => "-",
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::Binary { left: Box::new(left), op: op.to_string(), right: Box::new(right) };
        }
        
        Ok(left)
    }
    
    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;
        
        loop {
            let op = match &self.current().kind {
                TokenKind::Star => "*",
                TokenKind::Slash => "/",
                TokenKind::Percent => "%",
                TokenKind::StarStar => "**",
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::Binary { left: Box::new(left), op: op.to_string(), right: Box::new(right) };
        }
        
        Ok(left)
    }
    
    fn parse_unary(&mut self) -> Result<Expr, String> {
        match &self.current().kind {
            TokenKind::Not => {
                self.advance();
                Ok(Expr::Unary { op: "!".to_string(), expr: Box::new(self.parse_unary()?) })
            }
            TokenKind::Minus => {
                self.advance();
                Ok(Expr::Unary { op: "-".to_string(), expr: Box::new(self.parse_unary()?) })
            }
            TokenKind::Plus => {
                self.advance();
                Ok(Expr::Unary { op: "+".to_string(), expr: Box::new(self.parse_unary()?) })
            }
            TokenKind::Typeof => {
                self.advance();
                Ok(Expr::Typeof(Box::new(self.parse_unary()?)))
            }
            TokenKind::PlusPlus => {
                self.advance();
                Ok(Expr::PreIncrement(Box::new(self.parse_unary()?)))
            }
            TokenKind::MinusMinus => {
                self.advance();
                Ok(Expr::PreDecrement(Box::new(self.parse_unary()?)))
            }
            _ => self.parse_postfix(),
        }
    }
    
    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_call()?;
        
        loop {
            match &self.current().kind {
                TokenKind::PlusPlus => {
                    self.advance();
                    expr = Expr::PostIncrement(Box::new(expr));
                }
                TokenKind::MinusMinus => {
                    self.advance();
                    expr = Expr::PostDecrement(Box::new(expr));
                }
                _ => break,
            }
        }
        
        Ok(expr)
    }
    
    fn parse_call(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;
        
        loop {
            match &self.current().kind {
                TokenKind::LParen => {
                    self.advance();
                    let args = self.parse_arguments()?;
                    self.expect(TokenKind::RParen)?;
                    expr = Expr::Call { callee: Box::new(expr), args };
                }
                TokenKind::Dot | TokenKind::OptionalChain => {
                    self.advance();
                    if let TokenKind::Identifier(prop) = self.current().kind.clone() {
                        self.advance();
                        expr = Expr::Member { object: Box::new(expr), property: prop };
                    } else {
                        return Err("Expected property name".to_string());
                    }
                }
                TokenKind::LBracket => {
                    self.advance();
                    let index = self.parse_expression()?;
                    self.expect(TokenKind::RBracket)?;
                    expr = Expr::Index { object: Box::new(expr), index: Box::new(index) };
                }
                _ => break,
            }
        }
        
        Ok(expr)
    }
    
    fn parse_arguments(&mut self) -> Result<Vec<Expr>, String> {
        let mut args = Vec::new();
        
        if !matches!(self.current().kind, TokenKind::RParen) {
            args.push(self.parse_assignment()?);
            
            while matches!(self.current().kind, TokenKind::Comma) {
                self.advance();
                args.push(self.parse_assignment()?);
            }
        }
        
        Ok(args)
    }
    
    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.current().kind.clone() {
            TokenKind::Number(n) => { self.advance(); Ok(Expr::Number(n)) }
            TokenKind::String(s) => { self.advance(); Ok(Expr::String(s)) }
            TokenKind::Boolean(b) => { self.advance(); Ok(Expr::Boolean(b)) }
            TokenKind::Null => { self.advance(); Ok(Expr::Null) }
            TokenKind::Undefined => { self.advance(); Ok(Expr::Undefined) }
            TokenKind::This => { self.advance(); Ok(Expr::This) }
            TokenKind::Identifier(name) => {
                self.advance();
                // Check for arrow function: name =>
                if matches!(self.current().kind, TokenKind::Arrow) {
                    self.advance();
                    return self.parse_arrow_body(vec![name]);
                }
                Ok(Expr::Identifier(name))
            }
            TokenKind::LParen => {
                self.advance();
                
                if matches!(self.current().kind, TokenKind::RParen) {
                    self.advance();
                    if matches!(self.current().kind, TokenKind::Arrow) {
                        self.advance();
                        return self.parse_arrow_body(vec![]);
                    }
                }
                
                let expr = self.parse_expression()?;
                self.expect(TokenKind::RParen)?;
                
                if matches!(self.current().kind, TokenKind::Arrow) {
                    self.advance();
                    let params = self.extract_params(&expr)?;
                    return self.parse_arrow_body(params);
                }
                
                Ok(expr)
            }
            TokenKind::LBracket => self.parse_array(),
            TokenKind::LBrace => self.parse_object(),
            TokenKind::Function => self.parse_function_expression(),
            TokenKind::New => {
                self.advance();
                let callee = self.parse_call()?;
                Ok(Expr::New { callee: Box::new(callee), args: vec![] })
            }
            _ => Err(format!("Unexpected token: {:?}", self.current().kind))
        }
    }
    
    fn extract_params(&self, expr: &Expr) -> Result<Vec<String>, String> {
        match expr {
            Expr::Identifier(name) => Ok(vec![name.clone()]),
            _ => Err("Invalid arrow function parameters".to_string()),
        }
    }
    
    fn parse_arrow_body(&mut self, params: Vec<String>) -> Result<Expr, String> {
        if matches!(self.current().kind, TokenKind::LBrace) {
            self.expect(TokenKind::LBrace)?;
            let body = self.parse_statement_list()?;
            self.expect(TokenKind::RBrace)?;
            Ok(Expr::Arrow { params, body: Box::new(ArrowBody::Block(body)) })
        } else {
            let expr = self.parse_assignment()?;
            Ok(Expr::Arrow { params, body: Box::new(ArrowBody::Expr(expr)) })
        }
    }
    
    fn parse_array(&mut self) -> Result<Expr, String> {
        self.advance();
        let mut elements = Vec::new();
        
        while !matches!(self.current().kind, TokenKind::RBracket | TokenKind::Eof) {
            // Support elisions / leading or consecutive commas: `[,,a]`
            if matches!(self.current().kind, TokenKind::Comma) {
                elements.push(Expr::Undefined);
                self.advance();
                continue;
            }

            elements.push(self.parse_assignment()?);
            
            if matches!(self.current().kind, TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        
        self.expect(TokenKind::RBracket)?;
        Ok(Expr::Array(elements))
    }
    
    fn parse_object(&mut self) -> Result<Expr, String> {
        self.advance();
        let mut properties = Vec::new();
        
        while !matches!(self.current().kind, TokenKind::RBrace | TokenKind::Eof) {
            let key = match self.current().kind.clone() {
                TokenKind::Identifier(s) => { self.advance(); s }
                TokenKind::String(s) => { self.advance(); s }
                TokenKind::Number(n) => { self.advance(); n.to_string() }
                _ => return Err("Expected property name".to_string()),
            };
            
            // Shorthand property { x } == { x: x }
            if matches!(self.current().kind, TokenKind::Comma | TokenKind::RBrace) {
                properties.push((key.clone(), Expr::Identifier(key)));
            } else {
                self.expect(TokenKind::Colon)?;
                let value = self.parse_assignment()?;
                properties.push((key, value));
            }
            
            if matches!(self.current().kind, TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        
        self.expect(TokenKind::RBrace)?;
        Ok(Expr::Object(properties))
    }
    
    fn parse_function_expression(&mut self) -> Result<Expr, String> {
        self.advance();
        
        let name = if let TokenKind::Identifier(n) = self.current().kind.clone() {
            self.advance();
            Some(n)
        } else {
            None
        };
        
        self.expect(TokenKind::LParen)?;
        let params = self.parse_parameters()?;
        self.expect(TokenKind::RParen)?;
        
        self.expect(TokenKind::LBrace)?;
        let body = self.parse_statement_list()?;
        self.expect(TokenKind::RBrace)?;
        
        Ok(Expr::Function { name, params, body })
    }
}
