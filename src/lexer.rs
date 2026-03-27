use crate::ast::{Token, TokenKind};
use crate::error::CompileError;

pub struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
            line: 1,
            column: 1,
        }
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.chars.next()?;
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, CompileError> {
        let mut tokens = Vec::new();
        while let Some(ch) = self.peek() {
            match ch {
                c if c.is_whitespace() => {
                    self.bump();
                }
                '(' => {
                    let (line, column) = (self.line, self.column);
                    self.bump();
                    tokens.push(Token { kind: TokenKind::LParen, line, column });
                }
                ')' => {
                    let (line, column) = (self.line, self.column);
                    self.bump();
                    tokens.push(Token { kind: TokenKind::RParen, line, column });
                }
                '{' => {
                    let (line, column) = (self.line, self.column);
                    self.bump();
                    tokens.push(Token { kind: TokenKind::LBrace, line, column });
                }
                '}' => {
                    let (line, column) = (self.line, self.column);
                    self.bump();
                    tokens.push(Token { kind: TokenKind::RBrace, line, column });
                }
                '[' => {
                    let (line, column) = (self.line, self.column);
                    self.bump();
                    tokens.push(Token { kind: TokenKind::LBracket, line, column });
                }
                ']' => {
                    let (line, column) = (self.line, self.column);
                    self.bump();
                    tokens.push(Token { kind: TokenKind::RBracket, line, column });
                }
                ';' => {
                    let (line, column) = (self.line, self.column);
                    self.bump();
                    tokens.push(Token { kind: TokenKind::Semicolon, line, column });
                }
                ',' => {
                    let (line, column) = (self.line, self.column);
                    self.bump();
                    tokens.push(Token { kind: TokenKind::Comma, line, column });
                }
                ':' => {
                    let (line, column) = (self.line, self.column);
                    self.bump();
                    tokens.push(Token { kind: TokenKind::Colon, line, column });
                }
                '?' => {
                    let (line, column) = (self.line, self.column);
                    self.bump();
                    tokens.push(Token { kind: TokenKind::Question, line, column });
                }
                '+' => {
                    let (line, column) = (self.line, self.column);
                    self.bump();
                    if self.peek() == Some('+') {
                        self.bump();
                        tokens.push(Token { kind: TokenKind::PlusPlus, line, column });
                    } else {
                        tokens.push(Token { kind: TokenKind::Plus, line, column });
                    }
                }
                '-' => {
                    let (line, column) = (self.line, self.column);
                    self.bump();
                    if self.peek() == Some('-') {
                        self.bump();
                        tokens.push(Token { kind: TokenKind::MinusMinus, line, column });
                    } else {
                        tokens.push(Token { kind: TokenKind::Minus, line, column });
                    }
                }
                '*' => {
                    let (line, column) = (self.line, self.column);
                    self.bump();
                    tokens.push(Token { kind: TokenKind::Star, line, column });
                }
                '/' => {
                    let (line, column) = (self.line, self.column);
                    self.bump();
                    if self.peek() == Some('/') {
                        while self.peek() != Some('\n') && self.peek().is_some() {
                            self.bump();
                        }
                    } else {
                        tokens.push(Token { kind: TokenKind::Slash, line, column });
                    }
                }
                '#' => {
                    while self.peek() != Some('\n') && self.peek().is_some() {
                        self.bump();
                    }
                }
                '%' => {
                    let (line, column) = (self.line, self.column);
                    self.bump();
                    tokens.push(Token { kind: TokenKind::Percent, line, column });
                }
                '"' => {
                    let (line, column) = (self.line, self.column);
                    let lit = self.lex_string()?;
                    tokens.push(Token { kind: lit, line, column });
                }
                '\'' => {
                    let (line, column) = (self.line, self.column);
                    let lit = self.lex_char()?;
                    tokens.push(Token { kind: lit, line, column });
                }
                '=' => {
                    let (line, column) = (self.line, self.column);
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        tokens.push(Token { kind: TokenKind::EqualEqual, line, column });
                    } else {
                        tokens.push(Token { kind: TokenKind::Equal, line, column });
                    }
                }
                '!' => {
                    let (line, column) = (self.line, self.column);
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        tokens.push(Token { kind: TokenKind::BangEqual, line, column });
                    } else {
                        tokens.push(Token { kind: TokenKind::Bang, line, column });
                    }
                }
                '<' => {
                    let (line, column) = (self.line, self.column);
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        tokens.push(Token { kind: TokenKind::LessEqual, line, column });
                    } else {
                        tokens.push(Token { kind: TokenKind::Less, line, column });
                    }
                }
                '>' => {
                    let (line, column) = (self.line, self.column);
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        tokens.push(Token { kind: TokenKind::GreaterEqual, line, column });
                    } else {
                        tokens.push(Token { kind: TokenKind::Greater, line, column });
                    }
                }
                '&' => {
                    let (line, column) = (self.line, self.column);
                    self.bump();
                    if self.peek() == Some('&') {
                        self.bump();
                        tokens.push(Token { kind: TokenKind::AndAnd, line, column });
                    } else {
                        return Err(CompileError::new("unexpected '&'", line, column));
                    }
                }
                '|' => {
                    let (line, column) = (self.line, self.column);
                    self.bump();
                    if self.peek() == Some('|') {
                        self.bump();
                        tokens.push(Token { kind: TokenKind::OrOr, line, column });
                    } else {
                        return Err(CompileError::new("unexpected '|'", line, column));
                    }
                }
                c if c.is_ascii_digit() => {
                    let (line, column) = (self.line, self.column);
                    let kind = self.lex_number_or_float()?;
                    tokens.push(Token { kind, line, column });
                }
                c if c.is_ascii_alphabetic() || c == '_' => {
                    let (line, column) = (self.line, self.column);
                    let kind = self.lex_ident_or_keyword();
                    tokens.push(Token { kind, line, column });
                }
                _ => {
                    let (line, column) = (self.line, self.column);
                    return Err(CompileError::new(format!("unexpected character '{ch}'"), line, column));
                }
            }
        }
        tokens.push(Token { kind: TokenKind::EOF, line: self.line, column: self.column });
        Ok(tokens)
    }

    fn lex_number_or_float(&mut self) -> Result<TokenKind, CompileError> {
        let mut s = String::new();
        let mut has_dot = false;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                s.push(ch);
                self.bump();
            } else if ch == '.' && !has_dot {
                has_dot = true;
                s.push(ch);
                self.bump();
            } else {
                break;
            }
        }
        if has_dot {
            let v: f64 = s.parse().map_err(|_| CompileError::new("invalid float literal", self.line, self.column))?;
            Ok(TokenKind::FloatLit(v))
        } else {
            let v: i64 = s.parse().map_err(|_| CompileError::new("invalid integer literal", self.line, self.column))?;
            Ok(TokenKind::Number(v))
        }
    }

    fn lex_string(&mut self) -> Result<TokenKind, CompileError> {
        self.bump();
        let mut s = String::new();
        while let Some(ch) = self.bump() {
            if ch == '"' {
                return Ok(TokenKind::StringLit(s));
            }
            if ch == '\\' {
                if let Some(esc) = self.bump() {
                    match esc {
                        'n' => s.push('\n'),
                        'r' => s.push('\r'),
                        't' => s.push('\t'),
                        '\\' => s.push('\\'),
                        '"' => s.push('"'),
                        _ => s.push(esc),
                    }
                }
            } else {
                s.push(ch);
            }
        }
        Err(CompileError::new("unterminated string literal", self.line, self.column))
    }

    fn lex_char(&mut self) -> Result<TokenKind, CompileError> {
        self.bump();
        let ch = match self.bump() {
            Some('\\') => match self.bump() {
                Some('n') => '\n',
                Some('r') => '\r',
                Some('t') => '\t',
                Some('\\') => '\\',
                Some('\'') => '\'',
                Some(c) => c,
                None => return Err(CompileError::new("unterminated char literal", self.line, self.column)),
            },
            Some('"') => '"',
            Some(c) => c,
            None => return Err(CompileError::new("unterminated char literal", self.line, self.column)),
        };
        if self.bump() != Some('\'') {
            return Err(CompileError::new("unterminated char literal", self.line, self.column));
        }
        Ok(TokenKind::CharLit(ch))
    }

    fn lex_ident_or_keyword(&mut self) -> TokenKind {
        let mut s = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                s.push(ch);
                self.bump();
            } else {
                break;
            }
        }

        match s.as_str() {
            "public" => TokenKind::Public,
            "private" => TokenKind::Private,
            "protected" => TokenKind::Protected,
            "funct" => TokenKind::Funct,
            "var" => TokenKind::Var,
            "int" => TokenKind::Int,
            "bool" => TokenKind::Bool,
            "void" => TokenKind::Void,
            "string" => TokenKind::StringType,
            "float" => TokenKind::Float,
            "double" => TokenKind::Double,
            "char" => TokenKind::CharType,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "do" => TokenKind::DoWhile,
            "for" => TokenKind::For,
            "switch" => TokenKind::Switch,
            "case" => TokenKind::Case,
            "default" => TokenKind::Default,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "return" => TokenKind::Return,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            _ => TokenKind::Ident(s),
        }
    }
}
