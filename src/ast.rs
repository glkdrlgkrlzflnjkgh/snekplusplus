#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeName {
    Int,
    Bool,
    Void,
    String,
    Float,
    Double,
    Char,
}

#[derive(Debug, Clone, Copy)]
pub enum Visibility {
    Public,
    Private,
    Protected,
}

#[derive(Debug)]
pub struct FunctionDecl {
    pub name: String,
    pub visibility: Visibility,
    pub return_type: TypeName,
    pub params: Vec<Param>,
    pub body: Vec<Stmt>,
}

#[derive(Debug)]
pub struct Param {
    pub name: String,
    pub ty: TypeName,
}

#[derive(Debug)]
pub enum Stmt {
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
    Empty
}

impl Stmt {
    pub fn is_empty(&self) -> bool {
        matches!(self, Stmt::Empty)
    }
}

#[derive(Debug, Clone)]
pub enum Expr {
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
pub enum BinOp {
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
pub enum UnaryOp {
    Neg,
    Not,
    PostInc,
    PostDec,
    PreInc,
    PreDec,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
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
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug)]
pub struct Program {
    pub functions: Vec<FunctionDecl>,
}

pub fn format_type(ty: TypeName) -> &'static str {
    match ty {
        TypeName::Int => "int",
        TypeName::Bool => "bool",
        TypeName::Void => "void",
        TypeName::String => "string",
        TypeName::Float => "float",
        TypeName::Double => "double",
        TypeName::Char => "char",
    }
}
