use rayon::prelude::*;
use std::collections::HashSet;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/* =======================
   ERROR HANDLING
   ======================= */

#[derive(Debug)]
struct SourcePos {
    line: usize,
    column: usize,
}

#[derive(Debug)]
struct CompileError {
    message: String,
    pos: SourcePos,
}

impl CompileError {
    fn new(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            message: message.into(),
            pos: SourcePos { line, column },
        }
    }
}

fn print_error(err: &CompileError, filename: &str, src: &str) {
    let line = err.pos.line;
    let col = err.pos.column;
    eprintln!("{filename}:{line}:{col}: error: {}", err.message);

    if let Some(line_str) = src.lines().nth(line - 1) {
        eprintln!("  {line} | {line_str}");
        let mut caret = String::new();
        caret.push_str("    | ");
        for _ in 1..col {
            caret.push(' ');
        }
        caret.push('^');
        eprintln!("{caret}");
    }
}

/* =======================
   CLI / DRIVER
   ======================= */

fn main() {
    let mut args = env::args().skip(1);

    let mut import_dir: Option<PathBuf> = None;
    let mut entry: Option<String> = None;
    let mut output: Option<String> = None;
    let mut opt_level: OptLevel = OptLevel::None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-importdir" => {
                let Some(dir) = args.next() else {
                    eprintln!("error: -importdir requires a path");
                    std::process::exit(1);
                };
                import_dir = Some(PathBuf::from(dir));
            }
            "-o" => {
                let Some(out) = args.next() else {
                    eprintln!("error: -o requires an output file name");
                    std::process::exit(1);
                };
                output = Some(out);
            }
            "-optno" => {
                opt_level = OptLevel::None;
            }
            "-optyes" => {
                opt_level = OptLevel::O2;
            }
            _ => {
                if entry.is_none() {
                    entry = Some(arg);
                } else {
                    eprintln!("usage: snekplusplus <entry.spp> -o <output.exe> [-importdir DIR] [-optno|-optyes]");
                    std::process::exit(1);
                }
            }
        }
    }

    let entry = match entry {
        Some(e) => e,
        None => {
            eprintln!("usage: snekplusplus <entry.spp> -o <output.exe> [-importdir DIR] [-optno|-optyes]");
            std::process::exit(1);
        }
    };

    let output = match output {
        Some(o) => o,
        None => {
            eprintln!("error: missing -o <output.exe>");
            std::process::exit(1);
        }
    };

    let entry_path = PathBuf::from(&entry);
    if !entry_path.exists() {
        eprintln!("error: file not found: {entry}");
        std::process::exit(1);
    }

    let mut visited = HashSet::new();
    let src = match load_snekpp_with_imports(&entry_path, import_dir.as_deref(), &mut visited) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error loading sources: {e}");
            std::process::exit(1);
        }
    };

    let lexer = Lexer::new(&src);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            print_error(&e, &entry, &src);
            std::process::exit(1);
        }
    };

    let mut parser = Parser::new(tokens);
    let program = match parser.parse_program() {
        Ok(p) => p,
        Err(e) => {
            print_error(&e, &entry, &src);
            std::process::exit(1);
        }
    };

    if let Err(e) = check_program(&program) {
        let err = CompileError::new(e, 1, 1);
        print_error(&err, &entry, &src);
        std::process::exit(1);
    }

    let cpp = generate_cpp(&program);

    fs::write("out.cpp", &cpp).expect("failed to write out.cpp");

    let mut cmd = String::from("clang++ out.cpp -std=c++20");
    match opt_level {
        OptLevel::None => {}
        OptLevel::O2 => cmd.push_str(" -O2"),
    }
    cmd.push_str(" -o ");
    cmd.push_str(&output);

    let status = Command::new("cmd")
        .args(["/C", &cmd])
        .status()
        .expect("failed to invoke clang++ via cmd.exe");

    eprintln!("clang++ exit status: {status}");
}

#[derive(Clone, Copy)]
enum OptLevel {
    None,
    O2,
}

/* =======================
   IMPORT HANDLING (.spp)
   ======================= */

fn load_snekpp_with_imports(
    path: &Path,
    import_dir: Option<&Path>,
    visited: &mut HashSet<PathBuf>,
) -> Result<String, String> {
    let canon = fs::canonicalize(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !visited.insert(canon.clone()) {
        return Ok(String::new());
    }

    let content =
        fs::read_to_string(&canon).map_err(|e| format!("{}: {e}", canon.display()))?;

    let mut out = String::new();
    let dir = canon.parent().unwrap_or(Path::new("."));

    for line in content.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("import ") {
            let rest = rest.trim_start();
            if let Some(rest) = rest.strip_prefix('"') {
                if let Some(end) = rest.find('"') {
                    let fname = &rest[..end];

                    let primary = dir.join(fname);
                    let mut tried = Vec::new();

                    let imported = if primary.exists() {
                        tried.push(primary.display().to_string());
                        load_snekpp_with_imports(&primary, import_dir, visited)
                    } else if let Some(extra) = import_dir {
                        let secondary = extra.join(fname);
                        tried.push(primary.display().to_string());
                        tried.push(secondary.display().to_string());
                        if secondary.exists() {
                            load_snekpp_with_imports(&secondary, import_dir, visited)
                        } else {
                            Err(format!(
                                "import \"{fname}\" not found. tried: {}",
                                tried.join(", ")
                            ))
                        }
                    } else {
                        tried.push(primary.display().to_string());
                        Err(format!(
                            "import \"{fname}\" not found. tried: {}",
                            tried.join(", ")
                        ))
                    }?;

                    out.push_str(&imported);
                    out.push('\n');
                    continue;
                }
            }
            return Err(format!("invalid import syntax: {line}"));
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }

    Ok(out)
}

/* =======================
   LEXER
   ======================= */

#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Public,
    Private,
    Protected,
    Funct,
    Var,
    Int,
    Bool,
    Void,
    StringType,
    Float,
    Double,
    CharType,
    If,
    Else,
    While,
    DoWhile,
    For,
    Switch,
    Case,
    Default,
    Break,
    Continue,
    Return,
    True,
    False,
    Ident(String),
    Number(i64),
    FloatLit(f64),
    StringLit(String),
    CharLit(char),
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semicolon,
    Comma,
    Colon,
    Question,
    Equal,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Less,
    Greater,
    Bang,
    EqualEqual,
    BangEqual,
    LessEqual,
    GreaterEqual,
    AndAnd,
    OrOr,
    PlusPlus,
    MinusMinus,
    Comment,
    EOF,
}

#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    line: usize,
    column: usize,
}

struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
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

    fn tokenize(mut self) -> Result<Vec<Token>, CompileError> {
        let mut tokens = Vec::new();
        while let Some(ch) = self.peek() {
            match ch {
                c if c.is_whitespace() => {
                    self.bump();
                }
                '(' => {
                    let (line, col) = (self.line, self.column);
                    self.bump();
                    tokens.push(Token { kind: TokenKind::LParen, line, column: col });
                }
                ')' => {
                    let (line, col) = (self.line, self.column);
                    self.bump();
                    tokens.push(Token { kind: TokenKind::RParen, line, column: col });
                }
                '{' => {
                    let (line, col) = (self.line, self.column);
                    self.bump();
                    tokens.push(Token { kind: TokenKind::LBrace, line, column: col });
                }
                '}' => {
                    let (line, col) = (self.line, self.column);
                    self.bump();
                    tokens.push(Token { kind: TokenKind::RBrace, line, column: col });
                }
                '[' => {
                    let (line, col) = (self.line, self.column);
                    self.bump();
                    tokens.push(Token { kind: TokenKind::LBracket, line, column: col });
                }
                ']' => {
                    let (line, col) = (self.line, self.column);
                    self.bump();
                    tokens.push(Token { kind: TokenKind::RBracket, line, column: col });
                }
                ';' => {
                    let (line, col) = (self.line, self.column);
                    self.bump();
                    tokens.push(Token { kind: TokenKind::Semicolon, line, column: col });
                }
                ',' => {
                    let (line, col) = (self.line, self.column);
                    self.bump();
                    tokens.push(Token { kind: TokenKind::Comma, line, column: col });
                }
                ':' => {
                    let (line, col) = (self.line, self.column);
                    self.bump();
                    tokens.push(Token { kind: TokenKind::Colon, line, column: col });
                }
                '?' => {
                    let (line, col) = (self.line, self.column);
                    self.bump();
                    tokens.push(Token { kind: TokenKind::Question, line, column: col });
                }
                '+' => {
                    let (line, col) = (self.line, self.column);
                    self.bump();
                    if self.peek() == Some('+') {
                        self.bump();
                        tokens.push(Token { kind: TokenKind::PlusPlus, line, column: col });
                    } else {
                        tokens.push(Token { kind: TokenKind::Plus, line, column: col });
                    }
                }
                '-' => {
                    let (line, col) = (self.line, self.column);
                    self.bump();
                    if self.peek() == Some('-') {
                        self.bump();
                        tokens.push(Token { kind: TokenKind::MinusMinus, line, column: col });
                    } else {
                        tokens.push(Token { kind: TokenKind::Minus, line, column: col });
                    }
                }
                '*' => {
                    let (line, col) = (self.line, self.column);
                    self.bump();
                    tokens.push(Token { kind: TokenKind::Star, line, column: col });
                }
                '/' => {
                    let (line, col) = (self.line, self.column);
                    self.bump();
                    tokens.push(Token { kind: TokenKind::Slash, line, column: col });
                }
                '#' => {
                    let (line, col) = (self.line, self.column);
                    self.bump();
                    while let Some(ch) = self.peek() {
                        if ch == '\n' {
                            break;
                        }
                        self.bump();
                    }
                    tokens.push(Token { kind: TokenKind::Comment, line, column: col });
                }
                '%' => {
                    let (line, col) = (self.line, self.column);
                    self.bump();
                    tokens.push(Token { kind: TokenKind::Percent, line, column: col });
                }
                '"' => {
                    let (line, col) = (self.line, self.column);
                    let kind = self.lex_string()?;
                    tokens.push(Token { kind, line, column: col });
                }
                '\'' => {
                    let (line, col) = (self.line, self.column);
                    let kind = self.lex_char()?;
                    tokens.push(Token { kind, line, column: col });
                }
                '=' => {
                    let (line, col) = (self.line, self.column);
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        tokens.push(Token { kind: TokenKind::EqualEqual, line, column: col });
                    } else {
                        tokens.push(Token { kind: TokenKind::Equal, line, column: col });
                    }
                }
                '!' => {
                    let (line, col) = (self.line, self.column);
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        tokens.push(Token { kind: TokenKind::BangEqual, line, column: col });
                    } else {
                        tokens.push(Token { kind: TokenKind::Bang, line, column: col });
                    }
                }
                '<' => {
                    let (line, col) = (self.line, self.column);
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        tokens.push(Token { kind: TokenKind::LessEqual, line, column: col });
                    } else {
                        tokens.push(Token { kind: TokenKind::Less, line, column: col });
                    }
                }
                '>' => {
                    let (line, col) = (self.line, self.column);
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        tokens.push(Token { kind: TokenKind::GreaterEqual, line, column: col });
                    } else {
                        tokens.push(Token { kind: TokenKind::Greater, line, column: col });
                    }
                }
                '&' => {
                    let (line, col) = (self.line, self.column);
                    self.bump();
                    if self.peek() == Some('&') {
                        self.bump();
                        tokens.push(Token { kind: TokenKind::AndAnd, line, column: col });
                    } else {
                        return Err(CompileError::new(
                            "unexpected character '&'",
                            line,
                            col,
                        ));
                    }
                }
                '|' => {
                    let (line, col) = (self.line, self.column);
                    self.bump();
                    if self.peek() == Some('|') {
                        self.bump();
                        tokens.push(Token { kind: TokenKind::OrOr, line, column: col });
                    } else {
                        return Err(CompileError::new(
                            "unexpected character '|'",
                            line,
                            col,
                        ));
                    }
                }
                '0'..='9' => {
                    let (line, col) = (self.line, self.column);
                    let kind = self.lex_number_or_float()?;
                    tokens.push(Token { kind, line, column: col });
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    let (line, col) = (self.line, self.column);
                    let kind = self.lex_ident_or_keyword();
                    tokens.push(Token { kind, line, column: col });
                }
                _ => {
                    return Err(CompileError::new(
                        format!("unexpected character '{ch}'"),
                        self.line,
                        self.column,
                    ));
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
            let v: f64 = s.parse().map_err(|_| {
                CompileError::new("invalid float literal", self.line, self.column)
            })?;
            Ok(TokenKind::FloatLit(v))
        } else {
            let v: i64 = s.parse().map_err(|_| {
                CompileError::new("invalid integer literal", self.line, self.column)
            })?;
            Ok(TokenKind::Number(v))
        }
    }

    fn lex_string(&mut self) -> Result<TokenKind, CompileError> {
        self.bump(); // consume "
        let mut s = String::new();
        while let Some(ch) = self.bump() {
            if ch == '"' {
                return Ok(TokenKind::StringLit(s));
            } else {
                s.push(ch);
            }
        }
        Err(CompileError::new("unterminated string literal", self.line, self.column))
    }

    fn lex_char(&mut self) -> Result<TokenKind, CompileError> {
        self.bump(); // '
        let ch = match self.bump() {
            Some(c) => c,
            None => {
                return Err(CompileError::new(
                    "unterminated char literal",
                    self.line,
                    self.column,
                ))
            }
        };
        if self.bump() != Some('\'') {
            return Err(CompileError::new("invalid char literal", self.line, self.column));
        }
        Ok(TokenKind::CharLit(ch))
    }

    fn lex_ident_or_keyword(&mut self) -> TokenKind {
        let mut s = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
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

/* =======================
   AST
   ======================= */

#[derive(Debug)]
struct Program {
    functions: Vec<FunctionDecl>,
}

#[derive(Debug, Clone, Copy)]
enum Visibility {
    Public,
    Private,
    Protected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeName {
    Int,
    Bool,
    Void,
    String,
    Float,
    Double,
    Char,
}

#[derive(Debug)]
struct FunctionDecl {
    name: String,
    visibility: Visibility,
    return_type: TypeName,
    params: Vec<Param>,
    body: Vec<Stmt>,
}

#[derive(Debug)]
struct Param {
    name: String,
    ty: TypeName,
}

#[derive(Debug)]
enum Stmt {
    VarDecl {
        explicit_type: Option<TypeName>,
        name: String,
        init: Expr,
    },
    Assign {
        name: String,
        expr: Expr,
    },
    Return(Option<Expr>),
    ExprStmt(Expr),
    If {
        cond: Expr,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    DoWhile {
        body: Vec<Stmt>,
        cond: Expr,
    },
    For {
        init: Box<Stmt>,
        cond: Expr,
        step: Box<Stmt>,
        body: Vec<Stmt>,
    },
    Switch {
        expr: Expr,
        cases: Vec<(Option<Expr>, Vec<Stmt>)>,
    },
    Break,
    Continue,
}

#[derive(Debug, Clone)]
enum Expr {
    Number(i64),
    Float(f64),
    BoolLiteral(bool),
    StringLiteral(String),
    CharLiteral(char),
    Var(String),
    Call {
        name: String,
        args: Vec<Expr>,
    },
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Ternary {
        cond: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },
    Paren(Box<Expr>),
}

#[derive(Debug, Clone, Copy)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    Equal,
    NotEqual,
    And,
    Or,
}

#[derive(Debug, Clone, Copy)]
enum UnaryOp {
    Neg,      // -
    Not,      // !
    PostInc,  // ++
    PostDec,  // --
    PreInc,   // ++
    PreDec,   // --
}

/* =======================
   PARSER
   ======================= */

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }
    
    

    fn current(&mut self) -> &Token {
    while self.pos < self.tokens.len()
        && self.tokens[self.pos].kind == TokenKind::Comment
    {
        self.pos += 1;
    }

    // If we hit EOF or the end, return EOF safely
    if self.pos >= self.tokens.len() {
        return &self.tokens[self.tokens.len() - 1]; // EOF
    }

    &self.tokens[self.pos]
    }

    

    fn is_eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn peek_kind(&self) -> Option<&TokenKind> {
        self.tokens.get(self.pos).map(|t| &t.kind)
    }



    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn parse_program(&mut self) -> Result<Program, CompileError> {
    let mut functions = Vec::new();

    loop {
        // STOP before parsing anything
        if self.current().kind == TokenKind::EOF {
            break;
        }

        functions.push(self.parse_function()?);
    }

    Ok(Program { functions })
    }

    fn parse_function(&mut self) -> Result<FunctionDecl, CompileError> {
        let visibility = self.parse_visibility()?;
        self.expect_kind(TokenKind::Funct)?;
        let return_type = self.parse_type()?;
        let name = match &self.current().kind {
            TokenKind::Ident(s) => {
                let s = s.clone();
                self.advance();
                s
            }
            _ => {
                let t = self.current();
                return Err(CompileError::new(
                    "expected function name",
                    t.line,
                    t.column,
                ));
            }
        };
        self.expect_kind(TokenKind::LParen)?;
        let mut params = Vec::new();
        if !self.check_kind(&TokenKind::RParen) {
            loop {
                let ty = self.parse_type()?;
                let pname = match &self.current().kind {
                    TokenKind::Ident(s) => {
                        let s = s.clone();
                        self.advance();
                        s
                    }
                    _ => {
                        let t = self.current();
                        return Err(CompileError::new(
                            "expected parameter name",
                            t.line,
                            t.column,
                        ));
                    }
                };
                params.push(Param { name: pname, ty });
                if self.check_kind(&TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect_kind(TokenKind::RParen)?;
        self.expect_kind(TokenKind::LBrace)?;

        let mut body = Vec::new();
        while !self.check_kind(&TokenKind::RBrace) {
            body.push(self.parse_stmt()?);
        }

        self.expect_kind(TokenKind::RBrace)?;

        Ok(FunctionDecl {
            name,
            visibility,
            return_type,
            params,
            body,
        })
    }

    fn parse_visibility(&mut self) -> Result<Visibility, CompileError> {
        let t = self.current().clone();
        if t.kind == TokenKind::EOF {
            self.advance(); // consume EOF to avoid infinite loop in case of missing function declarations
            return Ok(Visibility::Private) // default visibility if EOF is encountered

        }
        let vis = match t.kind {
            TokenKind::Public => Visibility::Public,
            TokenKind::Private => Visibility::Private,
            TokenKind::Protected => Visibility::Protected,
            _ => {
                return Err(CompileError::new(
                    "expected visibility (public/private/protected)",
                    t.line,
                    t.column,
                ))
            }
        };
        self.advance();
        Ok(vis)
    }

    fn parse_type(&mut self) -> Result<TypeName, CompileError> {
        let t = self.current();
        let ty = match t.kind {
            TokenKind::Int => TypeName::Int,
            TokenKind::Bool => TypeName::Bool,
            TokenKind::Void => TypeName::Void,
            TokenKind::StringType => TypeName::String,
            TokenKind::Float => TypeName::Float,
            TokenKind::Double => TypeName::Double,
            TokenKind::CharType => TypeName::Char,
            _ => {
                return Err(CompileError::new(
                    "expected type",
                    t.line,
                    t.column,
                ))
            }
        };
        self.advance();
        Ok(ty)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, CompileError> {
        match self.peek_kind() {
            Some(TokenKind::Int)
            | Some(TokenKind::Bool)
            | Some(TokenKind::StringType)
            | Some(TokenKind::Float)
            | Some(TokenKind::Double)
            | Some(TokenKind::CharType)
            | Some(TokenKind::Var) => self.parse_var_decl(),
            Some(TokenKind::Return) => {
                self.advance();
                if self.check_kind(&TokenKind::Semicolon) {
                    self.advance();
                    Ok(Stmt::Return(None))
                } else {
                    let expr = self.parse_expr()?;
                    self.expect_kind(TokenKind::Semicolon)?;
                    Ok(Stmt::Return(Some(expr)))
                }
            }
            Some(TokenKind::If) => self.parse_if(),
            Some(TokenKind::While) => self.parse_while(),
            Some(TokenKind::DoWhile) => self.parse_do_while(),
            Some(TokenKind::For) => self.parse_for(),
            Some(TokenKind::Switch) => self.parse_switch(),
            Some(TokenKind::Break) => {
                self.advance();
                self.expect_kind(TokenKind::Semicolon)?;
                Ok(Stmt::Break)
            }
            Some(TokenKind::Continue) => {
                self.advance();
                self.expect_kind(TokenKind::Semicolon)?;
                Ok(Stmt::Continue)
            }
            Some(TokenKind::Ident(_)) => {
                if self.lookahead_is_assign() {
                    self.parse_assign()
                } else {
                    let expr = self.parse_expr()?;
                    self.expect_kind(TokenKind::Semicolon)?;
                    Ok(Stmt::ExprStmt(expr))
                }
            }
            _ => {
                let expr = self.parse_expr()?;
                self.expect_kind(TokenKind::Semicolon)?;
                Ok(Stmt::ExprStmt(expr))
            }
        }
    }

    fn lookahead_is_assign(&self) -> bool {
        if let Some(TokenKind::Ident(_)) = self.peek_kind() {
            if let Some(TokenKind::Equal) = self.tokens.get(self.pos + 1).map(|t| &t.kind) {
                return true;
            }
        }
        false
    }

    fn parse_assign(&mut self) -> Result<Stmt, CompileError> {
        let t = self.current();
        let name = match &t.kind {
            TokenKind::Ident(s) => {
                let s = s.clone();
                self.advance();
                s
            }
            _ => {
                return Err(CompileError::new(
                    "expected identifier in assignment",
                    t.line,
                    t.column,
                ))
            }
        };
        self.expect_kind(TokenKind::Equal)?;
        let expr = self.parse_expr()?;
        self.expect_kind(TokenKind::Semicolon)?;
        Ok(Stmt::Assign { name, expr })
    }

    fn parse_if(&mut self) -> Result<Stmt, CompileError> {
        self.expect_kind(TokenKind::If)?;
        self.expect_kind(TokenKind::LParen)?;
        let cond = self.parse_expr()?;
        self.expect_kind(TokenKind::RParen)?;
        self.expect_kind(TokenKind::LBrace)?;
        let mut then_body = Vec::new();
        while !self.check_kind(&TokenKind::RBrace) {
            then_body.push(self.parse_stmt()?);
        }
        self.expect_kind(TokenKind::RBrace)?;

        let mut else_body = Vec::new();
        if self.check_kind(&TokenKind::Else) {
            self.advance();
            self.expect_kind(TokenKind::LBrace)?;
            while !self.check_kind(&TokenKind::RBrace) {
                else_body.push(self.parse_stmt()?);
            }
            self.expect_kind(TokenKind::RBrace)?;
        }

        Ok(Stmt::If { cond, then_body, else_body })
    }

    fn parse_while(&mut self) -> Result<Stmt, CompileError> {
        self.expect_kind(TokenKind::While)?;
        self.expect_kind(TokenKind::LParen)?;
        let cond = self.parse_expr()?;
        self.expect_kind(TokenKind::RParen)?;
        self.expect_kind(TokenKind::LBrace)?;
        let mut body = Vec::new();
        while !self.check_kind(&TokenKind::RBrace) {
            body.push(self.parse_stmt()?);
        }
        self.expect_kind(TokenKind::RBrace)?;
        Ok(Stmt::While { cond, body })
    }

    fn parse_do_while(&mut self) -> Result<Stmt, CompileError> {
        self.expect_kind(TokenKind::DoWhile)?;
        self.expect_kind(TokenKind::LBrace)?;
        let mut body = Vec::new();
        while !self.check_kind(&TokenKind::RBrace) {
            body.push(self.parse_stmt()?);
        }
        self.expect_kind(TokenKind::RBrace)?;
        self.expect_kind(TokenKind::While)?;
        self.expect_kind(TokenKind::LParen)?;
        let cond = self.parse_expr()?;
        self.expect_kind(TokenKind::RParen)?;
        self.expect_kind(TokenKind::Semicolon)?;
        Ok(Stmt::DoWhile { body, cond })
    }

    fn parse_for(&mut self) -> Result<Stmt, CompileError> {
        self.expect_kind(TokenKind::For)?;
        self.expect_kind(TokenKind::LParen)?;

        let init = match self.peek_kind() {
            Some(TokenKind::Int)
            | Some(TokenKind::Bool)
            | Some(TokenKind::StringType)
            | Some(TokenKind::Float)
            | Some(TokenKind::Double)
            | Some(TokenKind::CharType)
            | Some(TokenKind::Var) => self.parse_var_decl()?,
            Some(TokenKind::Ident(_)) => self.parse_assign()?,
            _ => {
                let t = self.current();
                return Err(CompileError::new(
                    "invalid for-loop init",
                    t.line,
                    t.column,
                ));
            }
        };

        let cond = self.parse_expr()?;
        self.expect_kind(TokenKind::Semicolon)?;

        let step = if self.lookahead_is_assign() {
            self.parse_assign()?
        } else {
            let expr = self.parse_expr()?;
            self.expect_kind(TokenKind::RParen)?;
            Stmt::ExprStmt(expr)
        };

        if !matches!(step, Stmt::ExprStmt(_) | Stmt::Assign { .. }) {
            let t = self.current();
            return Err(CompileError::new(
                "invalid for-loop step",
                t.line,
                t.column,
            ));
        }

        if !self.check_kind(&TokenKind::LBrace) {
            if self.check_kind(&TokenKind::RParen) {
                self.advance();
            }
        }

        self.expect_kind(TokenKind::LBrace)?;
        let mut body = Vec::new();
        while !self.check_kind(&TokenKind::RBrace) {
            body.push(self.parse_stmt()?);
        }
        self.expect_kind(TokenKind::RBrace)?;

        Ok(Stmt::For {
            init: Box::new(init),
            cond,
            step: Box::new(step),
            body,
        })
    }

    fn parse_switch(&mut self) -> Result<Stmt, CompileError> {
        self.expect_kind(TokenKind::Switch)?;
        self.expect_kind(TokenKind::LParen)?;
        let expr = self.parse_expr()?;
        self.expect_kind(TokenKind::RParen)?;
        self.expect_kind(TokenKind::LBrace)?;

        let mut cases = Vec::new();
        while !self.check_kind(&TokenKind::RBrace) {
            if self.check_kind(&TokenKind::Case) {
                self.advance();
                let val = self.parse_expr()?;
                self.expect_kind(TokenKind::Colon)?;
                let mut stmts = Vec::new();
                while !self.check_kind(&TokenKind::Case)
                    && !self.check_kind(&TokenKind::Default)
                    && !self.check_kind(&TokenKind::RBrace)
                {
                    stmts.push(self.parse_stmt()?);
                }
                cases.push((Some(val), stmts));
            } else if self.check_kind(&TokenKind::Default) {
                self.advance();
                self.expect_kind(TokenKind::Colon)?;
                let mut stmts = Vec::new();
                while !self.check_kind(&TokenKind::Case)
                    && !self.check_kind(&TokenKind::Default)
                    && !self.check_kind(&TokenKind::RBrace)
                {
                    stmts.push(self.parse_stmt()?);
                }
                cases.push((None, stmts));
            } else {
                let t = self.current();
                return Err(CompileError::new(
                    "expected 'case' or 'default' in switch",
                    t.line,
                    t.column,
                ));
            }
        }
        self.expect_kind(TokenKind::RBrace)?;

        Ok(Stmt::Switch { expr, cases })
    }

    fn parse_var_decl(&mut self) -> Result<Stmt, CompileError> {
        let explicit_type = match self.peek_kind() {
            Some(TokenKind::Int) => {
                self.advance();
                Some(TypeName::Int)
            }
            Some(TokenKind::Bool) => {
                self.advance();
                Some(TypeName::Bool)
            }
            Some(TokenKind::StringType) => {
                self.advance();
                Some(TypeName::String)
            }
            Some(TokenKind::Float) => {
                self.advance();
                Some(TypeName::Float)
            }
            Some(TokenKind::Double) => {
                self.advance();
                Some(TypeName::Double)
            }
            Some(TokenKind::CharType) => {
                self.advance();
                Some(TypeName::Char)
            }
            Some(TokenKind::Var) => {
                self.advance();
                None
            }
            _ => {
                let t = self.current();
                return Err(CompileError::new(
                    "expected type or 'var'",
                    t.line,
                    t.column,
                ));
            }
        };

        let t = self.current();
        let name = match &t.kind {
            TokenKind::Ident(s) => {
                let s = s.clone();
                self.advance();
                s
            }
            _ => {
                return Err(CompileError::new(
                    "expected variable name",
                    t.line,
                    t.column,
                ))
            }
        };

        self.expect_kind(TokenKind::Equal)?;
        let init = self.parse_expr()?;
        self.expect_kind(TokenKind::Semicolon)?;

        Ok(Stmt::VarDecl { explicit_type, name, init })
    }

    fn parse_expr(&mut self) -> Result<Expr, CompileError> {
        self.parse_ternary()
    }

    fn parse_ternary(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.parse_or()?;
        if self.check_kind(&TokenKind::Question) {
            self.advance();
            let then_expr = self.parse_expr()?;
            self.expect_kind(TokenKind::Colon)?;
            let else_expr = self.parse_expr()?;
            expr = Expr::Ternary {
                cond: Box::new(expr),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
            };
        }
        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.parse_and()?;
        loop {
            if self.check_kind(&TokenKind::OrOr) {
                self.advance();
                let rhs = self.parse_and()?;
                expr = Expr::Binary {
                    op: BinOp::Or,
                    left: Box::new(expr),
                    right: Box::new(rhs),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.parse_equality()?;
        loop {
            if self.check_kind(&TokenKind::AndAnd) {
                self.advance();
                let rhs = self.parse_equality()?;
                expr = Expr::Binary {
                    op: BinOp::And,
                    left: Box::new(expr),
                    right: Box::new(rhs),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.parse_comparison()?;
        loop {
            match self.peek_kind() {
                Some(TokenKind::EqualEqual) => {
                    self.advance();
                    let rhs = self.parse_comparison()?;
                    expr = Expr::Binary {
                        op: BinOp::Equal,
                        left: Box::new(expr),
                        right: Box::new(rhs),
                    };
                }
                Some(TokenKind::BangEqual) => {
                    self.advance();
                    let rhs = self.parse_comparison()?;
                    expr = Expr::Binary {
                        op: BinOp::NotEqual,
                        left: Box::new(expr),
                        right: Box::new(rhs),
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.parse_add_sub()?;
        loop {
            match self.peek_kind() {
                Some(TokenKind::Less) => {
                    self.advance();
                    let rhs = self.parse_add_sub()?;
                    expr = Expr::Binary {
                        op: BinOp::Less,
                        left: Box::new(expr),
                        right: Box::new(rhs),
                    };
                }
                Some(TokenKind::Greater) => {
                    self.advance();
                    let rhs = self.parse_add_sub()?;
                    expr = Expr::Binary {
                        op: BinOp::Greater,
                        left: Box::new(expr),
                        right: Box::new(rhs),
                    };
                }
                Some(TokenKind::LessEqual) => {
                    self.advance();
                    let rhs = self.parse_add_sub()?;
                    expr = Expr::Binary {
                        op: BinOp::LessEqual,
                        left: Box::new(expr),
                        right: Box::new(rhs),
                    };
                }
                Some(TokenKind::GreaterEqual) => {
                    self.advance();
                    let rhs = self.parse_add_sub()?;
                    expr = Expr::Binary {
                        op: BinOp::GreaterEqual,
                        left: Box::new(expr),
                        right: Box::new(rhs),
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_add_sub(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.parse_mul_div()?;
        loop {
            match self.peek_kind() {
                Some(TokenKind::Plus) => {
                    self.advance();
                    let rhs = self.parse_mul_div()?;
                    expr = Expr::Binary {
                        op: BinOp::Add,
                        left: Box::new(expr),
                        right: Box::new(rhs),
                    };
                }
                Some(TokenKind::Minus) => {
                    self.advance();
                    let rhs = self.parse_mul_div()?;
                    expr = Expr::Binary {
                        op: BinOp::Sub,
                        left: Box::new(expr),
                        right: Box::new(rhs),
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_mul_div(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.parse_unary()?;
        loop {
            match self.peek_kind() {
                Some(TokenKind::Star) => {
                    self.advance();
                    let rhs = self.parse_unary()?;
                    expr = Expr::Binary {
                        op: BinOp::Mul,
                        left: Box::new(expr),
                        right: Box::new(rhs),
                    };
                }
                Some(TokenKind::Slash) => {
                    self.advance();
                    let rhs = self.parse_unary()?;
                    expr = Expr::Binary {
                        op: BinOp::Div,
                        left: Box::new(expr),
                        right: Box::new(rhs),
                    };
                }
                Some(TokenKind::Percent) => {
                    self.advance();
                    let rhs = self.parse_unary()?;
                    expr = Expr::Binary {
                        op: BinOp::Mod,
                        left: Box::new(expr),
                        right: Box::new(rhs),
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, CompileError> {
        match self.peek_kind() {
            Some(TokenKind::Minus) => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                })
            }
            Some(TokenKind::Bang) => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                })
            }
            Some(TokenKind::PlusPlus) => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::PreInc,
                    expr: Box::new(expr),
                })
            }
            Some(TokenKind::MinusMinus) => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::PreDec,
                    expr: Box::new(expr),
                })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek_kind() {
                Some(TokenKind::PlusPlus) => {
                    self.advance();
                    expr = Expr::Unary {
                        op: UnaryOp::PostInc,
                        expr: Box::new(expr),
                    };
                }
                Some(TokenKind::MinusMinus) => {
                    self.advance();
                    expr = Expr::Unary {
                        op: UnaryOp::PostDec,
                        expr: Box::new(expr),
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, CompileError> {
        let t = self.current().clone();
        match t.kind {
            TokenKind::Number(n) => {
                self.advance();
                Ok(Expr::Number(n))
            }
            TokenKind::FloatLit(f) => {
                self.advance();
                Ok(Expr::Float(f))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::BoolLiteral(true))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::BoolLiteral(false))
            }
            TokenKind::StringLit(s) => {
                self.advance();
                Ok(Expr::StringLiteral(s))
            }
            TokenKind::CharLit(c) => {
                self.advance();
                Ok(Expr::CharLiteral(c))
            }
            TokenKind::Ident(name) => {
                self.advance();
                if self.check_kind(&TokenKind::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    if !self.check_kind(&TokenKind::RParen) {
                        loop {
                            let arg = self.parse_expr()?;
                            args.push(arg);
                            if self.check_kind(&TokenKind::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect_kind(TokenKind::RParen)?;
                    Ok(Expr::Call { name, args })
                } else {
                    Ok(Expr::Var(name))
                }
            }
            TokenKind::LParen => {
                self.advance();
                let e = self.parse_expr()?;
                self.expect_kind(TokenKind::RParen)?;
                Ok(Expr::Paren(Box::new(e)))
            }
            _ => Err(CompileError::new(
                "expected expression",
                t.line,
                t.column,
            )),
        }
    }

    fn check_kind(&self, kind: &TokenKind) -> bool {
        self.peek_kind().map_or(false, |k| k == kind)
    }

    fn expect_kind(&mut self, kind: TokenKind) -> Result<(), CompileError> {
        let t = self.current();
        if t.kind == kind {
            self.advance();
            Ok(())
        } else {
            Err(CompileError::new(
                format!("expected {:?}, got {:?}", kind, t.kind),
                t.line,
                t.column,
            ))
        }
    }
}

/* =======================
   SEMANTIC CHECKS
   ======================= */

fn check_program(program: &Program) -> Result<(), String> {
    for func in &program.functions {
        check_function(func)?;
    }
    Ok(())
}

fn check_function(func: &FunctionDecl) -> Result<(), String> {
    fn walk_stmts(
        stmts: &[Stmt],
        ret_ty: TypeName,
        fname: &str,
    ) -> Result<(), String> {
        for s in stmts {
            match s {
                Stmt::Return(opt_expr) => {
                    match (ret_ty, opt_expr) {
                        (TypeName::Void, Some(_)) => {
                            return Err(format!(
                                "function '{fname}' is void but returns a value"
                            ));
                        }
                        (TypeName::Void, None) => {}
                        (TypeName::Int
                        | TypeName::Bool
                        | TypeName::String
                        | TypeName::Float
                        | TypeName::Double
                        | TypeName::Char, None) => {
                            return Err(format!(
                                "function '{fname}' must return a value of its return type"
                            ));
                        }
                        (TypeName::Int
                        | TypeName::Bool
                        | TypeName::String
                        | TypeName::Float
                        | TypeName::Double
                        | TypeName::Char, Some(_)) => {}
                    }
                }
                Stmt::If { then_body, else_body, .. } => {
                    walk_stmts(then_body, ret_ty, fname)?;
                    walk_stmts(else_body, ret_ty, fname)?;
                }
                Stmt::While { body, .. } => {
                    walk_stmts(body, ret_ty, fname)?;
                }
                Stmt::DoWhile { body, .. } => {
                    walk_stmts(body, ret_ty, fname)?;
                }
                Stmt::For { body, .. } => {
                    walk_stmts(body, ret_ty, fname)?;
                }
                Stmt::Switch { cases, .. } => {
                    for (_, stmts) in cases {
                        walk_stmts(stmts, ret_ty, fname)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    if func.name == "Main" && func.return_type != TypeName::Int {
        return Err("Main must return int".to_string());
    }

    walk_stmts(&func.body, func.return_type, &func.name)
}

/* =======================
   C++ CODEGEN (RAYON + PROGRESS)
   ======================= */

fn cpp_type(ty: TypeName) -> &'static str {
    match ty {
        TypeName::Int => "int",
        TypeName::Bool => "bool",
        TypeName::Void => "void",
        TypeName::String => "std::string",
        TypeName::Float => "float",
        TypeName::Double => "double",
        TypeName::Char => "char",
    }
}

fn generate_cpp(program: &Program) -> String {
    let mut out = String::new();
    writeln!(&mut out, "// Snek++ → C++").unwrap();
    writeln!(&mut out, "#include <iostream>").unwrap();
    writeln!(&mut out, "#include <string>").unwrap();
    writeln!(&mut out, "").unwrap();

    let total = program.functions.len();
    let counter = Arc::new(AtomicUsize::new(0));

    if total > 0 {
        println!("Generating C++ code for {} functions...", total);
    }

    let funcs_cpp: Vec<String> = program
        .functions
        .par_iter()
        .map(|f| {
            let mut s = String::new();
            emit_function(&mut s, f).unwrap();
            counter.fetch_add(1, Ordering::Relaxed);
            s
        })
        .collect();

    // Print final progress in main thread
    if total > 0 {
        println!("Finished generating C++ code for {} functions.", total);
        println!("Invoking clang++ to compile the generated code...");

    }

    for f in funcs_cpp {
        writeln!(&mut out, "{f}").unwrap();
    }

    out
}

fn emit_function(out: &mut String, func: &FunctionDecl) -> std::fmt::Result {
    let ret_ty = cpp_type(func.return_type);
    let name = if func.name == "Main" { "main" } else { &func.name };

    write!(out, "{ret_ty} {name}(")?;
    for (i, p) in func.params.iter().enumerate() {
        if i > 0 {
            write!(out, ", ")?;
        }
        let ty = cpp_type(p.ty);
        write!(out, "{ty} {}", p.name)?;
    }
    writeln!(out, ") {{")?;

    for stmt in &func.body {
        emit_stmt(out, stmt, 1)?;
    }

    writeln!(out, "}}")?;
    Ok(())
}

fn indent(out: &mut String, level: usize) -> std::fmt::Result {
    for _ in 0..level {
        write!(out, "    ")?;
    }
    Ok(())
}

fn emit_stmt(out: &mut String, stmt: &Stmt, level: usize) -> std::fmt::Result {
    match stmt {
        Stmt::VarDecl { explicit_type, name, init } => {
            indent(out, level)?;
            if let Some(ty) = explicit_type {
                let ty_str = cpp_type(*ty);
                write!(out, "{ty_str} {name} = ")?;
            } else {
                write!(out, "auto {name} = ")?;
            }
            emit_expr(out, init)?;
            writeln!(out, ";")?;
        }
        Stmt::Assign { name, expr } => {
            indent(out, level)?;
            write!(out, "{name} = ")?;
            emit_expr(out, expr)?;
            writeln!(out, ";")?;
        }
        Stmt::Return(opt_expr) => {
            indent(out, level)?;
            match opt_expr {
                Some(expr) => {
                    write!(out, "return ")?;
                    emit_expr(out, expr)?;
                    writeln!(out, ";")?;
                }
                None => {
                    writeln!(out, "return;")?;
                }
            }
        }
        Stmt::ExprStmt(expr) => {
            if let Expr::Call { name, args } = expr {
                if name == "print" {
                    indent(out, level)?;
                    write!(out, "std::cout")?;
                    if !args.is_empty() {
                        write!(out, " << ")?;
                        emit_expr(out, &args[0])?;
                        for arg in &args[1..] {
                            write!(out, " << \" \" << ")?;
                            emit_expr(out, arg)?;
                        }
                    }
                    writeln!(out, " << std::endl;")?;
                    return Ok(());
                }
            }
            indent(out, level)?;
            emit_expr(out, expr)?;
            writeln!(out, ";")?;
        }
        Stmt::If { cond, then_body, else_body } => {
            indent(out, level)?;
            write!(out, "if (")?;
            emit_expr(out, cond)?;
            writeln!(out, ") {{")?;
            for s in then_body {
                emit_stmt(out, s, level + 1)?;
            }
            indent(out, level)?;
            writeln!(out, "}}")?;
            if !else_body.is_empty() {
                indent(out, level)?;
                writeln!(out, "else {{")?;
                for s in else_body {
                    emit_stmt(out, s, level + 1)?;
                }
                indent(out, level)?;
                writeln!(out, "}}")?;
            }
        }
        Stmt::While { cond, body } => {
            indent(out, level)?;
            write!(out, "while (")?;
            emit_expr(out, cond)?;
            writeln!(out, ") {{")?;
            for s in body {
                emit_stmt(out, s, level + 1)?;
            }
            indent(out, level)?;
            writeln!(out, "}}")?;
        }
        Stmt::DoWhile { body, cond } => {
            indent(out, level)?;
            writeln!(out, "do {{")?;
            for s in body {
                emit_stmt(out, s, level + 1)?;
            }
            indent(out, level)?;
            write!(out, "}} while (")?;
            emit_expr(out, cond)?;
            writeln!(out, ");")?;
        }
        Stmt::For { init, cond, step, body } => {
            indent(out, level)?;
            write!(out, "for (")?;
            match &**init {
                Stmt::VarDecl { explicit_type, name, init } => {
                    if let Some(ty) = explicit_type {
                        let ty_str = cpp_type(*ty);
                        write!(out, "{ty_str} {name} = ")?;
                    } else {
                        write!(out, "auto {name} = ")?;
                    }
                    emit_expr(out, init)?;
                }
                Stmt::Assign { name, expr } => {
                    write!(out, "{name} = ")?;
                    emit_expr(out, expr)?;
                }
                _ => {}
            }
            write!(out, "; ")?;
            emit_expr(out, cond)?;
            write!(out, "; ")?;
            match &**step {
                Stmt::Assign { name, expr } => {
                    write!(out, "{name} = ")?;
                    emit_expr(out, expr)?;
                }
                Stmt::ExprStmt(e) => {
                    emit_expr(out, e)?;
                }
                _ => {}
            }
            writeln!(out, ") {{")?;
            for s in body {
                emit_stmt(out, s, level + 1)?;
            }
            indent(out, level)?;
            writeln!(out, "}}")?;
        }
        Stmt::Switch { expr, cases } => {
            indent(out, level)?;
            write!(out, "switch (")?;
            emit_expr(out, expr)?;
            writeln!(out, ") {{")?;
            for (case_val, stmts) in cases {
                if let Some(val) = case_val {
                    indent(out, level + 1)?;
                    write!(out, "case ")?;
                    emit_expr(out, val)?;
                    writeln!(out, ":")?;
                } else {
                    indent(out, level + 1)?;
                    writeln!(out, "default:")?;
                }
                for s in stmts {
                    emit_stmt(out, s, level + 2)?;
                }
            }
            indent(out, level)?;
            writeln!(out, "}}")?;
        }
        Stmt::Break => {
            indent(out, level)?;
            writeln!(out, "break;")?;
        }
        Stmt::Continue => {
            indent(out, level)?;
            writeln!(out, "continue;")?;
        }
    }
    Ok(())
}

fn emit_expr(out: &mut String, expr: &Expr) -> std::fmt::Result {
    match expr {
        Expr::Number(n) => write!(out, "{n}"),
        Expr::Float(f) => write!(out, "{f}"),
        Expr::BoolLiteral(b) => write!(out, "{}", if *b { "true" } else { "false" }),
        Expr::StringLiteral(s) => {
            write!(out, "\"")?;
            for ch in s.chars() {
                match ch {
                    '\\' => write!(out, "\\\\")?,
                    '"' => write!(out, "\\\"")?,
                    '\n' => write!(out, "\\n")?,
                    '\r' => write!(out, "\\r")?,
                    '\t' => write!(out, "\\t")?,
                    c => write!(out, "{c}")?,
                }
            }
            write!(out, "\"")
        }
        Expr::CharLiteral(c) => {
            write!(out, "'")?;
            match c {
                '\\' => write!(out, "\\\\")?,
                '\'' => write!(out, "\\'")?,
                '\n' => write!(out, "\\n")?,
                '\r' => write!(out, "\\r")?,
                '\t' => write!(out, "\\t")?,
                ch => write!(out, "{ch}")?,
            }
            write!(out, "'")
        }
        Expr::Var(name) => write!(out, "{name}"),
        Expr::Call { name, args } => {
            write!(out, "{name}(")?;
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    write!(out, ", ")?;
                }
                emit_expr(out, a)?;
            }
            write!(out, ")")
        }
        Expr::Paren(inner) => {
            write!(out, "(")?;
            emit_expr(out, inner)?;
            write!(out, ")")
        }
        Expr::Unary { op, expr } => {
            match op {
                UnaryOp::Neg => write!(out, "-")?,
                UnaryOp::Not => write!(out, "!")?,
                UnaryOp::PreInc => write!(out, "++")?,
                UnaryOp::PreDec => write!(out, "--")?,
                UnaryOp::PostInc => {
                    emit_expr(out, expr)?;
                    write!(out, "++")?;
                    return Ok(());
                }
                UnaryOp::PostDec => {
                    emit_expr(out, expr)?;
                    write!(out, "--")?;
                    return Ok(());
                }
            }
            emit_expr(out, expr)
        }
        Expr::Ternary { cond, then_expr, else_expr } => {
            write!(out, "(")?;
            emit_expr(out, cond)?;
            write!(out, " ? ")?;
            emit_expr(out, then_expr)?;
            write!(out, " : ")?;
            emit_expr(out, else_expr)?;
            write!(out, ")")
        }
        Expr::Binary { op, left, right } => {
            emit_expr(out, left)?;
            let op_str = match op {
                BinOp::Add => " + ",
                BinOp::Sub => " - ",
                BinOp::Mul => " * ",
                BinOp::Div => " / ",
                BinOp::Mod => " % ",
                BinOp::Less => " < ",
                BinOp::Greater => " > ",
                BinOp::LessEqual => " <= ",
                BinOp::GreaterEqual => " >= ",
                BinOp::Equal => " == ",
                BinOp::NotEqual => " != ",
                BinOp::And => " && ",
                BinOp::Or => " || ",
            };
            write!(out, "{op_str}")?;
            emit_expr(out, right)
        }
    }
}