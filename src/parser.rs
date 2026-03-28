use crate::ast::{BinOp, Expr, FunctionDecl, Param, Program, Stmt, Token, TokenKind, TypeName, Visibility};
use crate::error::CompileError;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn current(&mut self) -> &Token {
        while self.pos < self.tokens.len() && self.tokens[self.pos].kind == TokenKind::Comment {
            self.pos += 1;
        }

        if self.pos >= self.tokens.len() {
            return &self.tokens[self.tokens.len() - 1];
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

    pub fn parse_program(&mut self) -> Result<Program, CompileError> {
        let mut functions = Vec::new();

        loop {
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
                return Err(CompileError::new("expected function name", t.line, t.column));
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
                        return Err(CompileError::new("expected parameter name", t.line, t.column));
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
            let before = self.current().clone();
            let stmt = self.parse_stmt()?;

            if self.current().kind == before.kind {
                panic!("compiler BUG! parse_stmt() did not consume any tokens!!!"); // this should NEVER. HAPPEN. PERIOD. If it does? REPORT. A. BUG.
            }

            if !stmt.is_empty() {
                body.push(stmt);
            }
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
            self.advance();
            return Ok(Visibility::Private);
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
                return Err(CompileError::new("expected type", t.line, t.column));
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
                let t = self.current();
                return Err(CompileError::new(
                    &format!("unexpected token '{:?}' in statement", t.kind),
                    t.line,
                    t.column,
    ));

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
                return Err(CompileError::new("expected identifier in assignment", t.line, t.column));
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
            // Check for "else if" vs "else {"
            if self.check_kind(&TokenKind::If) {
                // Recursively parse the else-if as a nested if statement
                let else_if_stmt = self.parse_if()?;
                else_body.push(else_if_stmt);
            } else {
                // Parse else block
                self.expect_kind(TokenKind::LBrace)?;
                while !self.check_kind(&TokenKind::RBrace) {
                    else_body.push(self.parse_stmt()?);
                }
                self.expect_kind(TokenKind::RBrace)?;
            }
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
                return Err(CompileError::new("invalid for-loop init", t.line, t.column));
            }
        };

        let cond = self.parse_expr()?;
        self.expect_kind(TokenKind::Semicolon)?;

        // Parse step: either assignment or expression (without semicolon before closing paren)
        let step = if self.lookahead_is_assign() {
            // Parse assignment without the semicolon
            let t = self.current();
            let name = match &t.kind {
                TokenKind::Ident(s) => {
                    let s = s.clone();
                    self.advance();
                    s
                }
                _ => {
                    return Err(CompileError::new("expected identifier in assignment", t.line, t.column));
                }
            };
            self.expect_kind(TokenKind::Equal)?;
            let expr = self.parse_expr()?;
            Stmt::Assign { name, expr }
        } else {
            let expr = self.parse_expr()?;
            Stmt::ExprStmt(expr)
        };

        self.expect_kind(TokenKind::RParen)?;
        self.expect_kind(TokenKind::LBrace)?;
        let mut body = Vec::new();
        while !self.check_kind(&TokenKind::RBrace) {
            body.push(self.parse_stmt()?);
        }
        self.expect_kind(TokenKind::RBrace)?;

        Ok(Stmt::For { init: Box::new(init), cond, step: Box::new(step), body })
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
                while !self.check_kind(&TokenKind::Case) && !self.check_kind(&TokenKind::Default) && !self.check_kind(&TokenKind::RBrace) {
                    stmts.push(self.parse_stmt()?);
                }
                cases.push((Some(val), stmts));
            } else if self.check_kind(&TokenKind::Default) {
                self.advance();
                self.expect_kind(TokenKind::Colon)?;
                let mut stmts = Vec::new();
                while !self.check_kind(&TokenKind::Case) && !self.check_kind(&TokenKind::Default) && !self.check_kind(&TokenKind::RBrace) {
                    stmts.push(self.parse_stmt()?);
                }
                cases.push((None, stmts));
            } else {
                let t = self.current();
                return Err(CompileError::new("expected 'case' or 'default' in switch", t.line, t.column));
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
                return Err(CompileError::new("expected type or 'var'", t.line, t.column));
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
                return Err(CompileError::new("expected variable name", t.line, t.column));
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
            expr = Expr::Ternary { cond: Box::new(expr), then_expr: Box::new(then_expr), else_expr: Box::new(else_expr) };
        }
        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.parse_and()?;
        loop {
            if self.check_kind(&TokenKind::OrOr) {
                self.advance();
                let rhs = self.parse_and()?;
                expr = Expr::Binary { op: BinOp::Or, left: Box::new(expr), right: Box::new(rhs) };
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
                expr = Expr::Binary { op: BinOp::And, left: Box::new(expr), right: Box::new(rhs) };
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
                    expr = Expr::Binary { op: BinOp::Equal, left: Box::new(expr), right: Box::new(rhs) };
                }
                Some(TokenKind::BangEqual) => {
                    self.advance();
                    let rhs = self.parse_comparison()?;
                    expr = Expr::Binary { op: BinOp::NotEqual, left: Box::new(expr), right: Box::new(rhs) };
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
                    expr = Expr::Binary { op: BinOp::Less, left: Box::new(expr), right: Box::new(rhs) };
                }
                Some(TokenKind::Greater) => {
                    self.advance();
                    let rhs = self.parse_add_sub()?;
                    expr = Expr::Binary { op: BinOp::Greater, left: Box::new(expr), right: Box::new(rhs) };
                }
                Some(TokenKind::LessEqual) => {
                    self.advance();
                    let rhs = self.parse_add_sub()?;
                    expr = Expr::Binary { op: BinOp::LessEqual, left: Box::new(expr), right: Box::new(rhs) };
                }
                Some(TokenKind::GreaterEqual) => {
                    self.advance();
                    let rhs = self.parse_add_sub()?;
                    expr = Expr::Binary { op: BinOp::GreaterEqual, left: Box::new(expr), right: Box::new(rhs) };
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
                    expr = Expr::Binary { op: BinOp::Add, left: Box::new(expr), right: Box::new(rhs) };
                }
                Some(TokenKind::Minus) => {
                    self.advance();
                    let rhs = self.parse_mul_div()?;
                    expr = Expr::Binary { op: BinOp::Sub, left: Box::new(expr), right: Box::new(rhs) };
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
                    expr = Expr::Binary { op: BinOp::Mul, left: Box::new(expr), right: Box::new(rhs) };
                }
                Some(TokenKind::Slash) => {
                    self.advance();
                    let rhs = self.parse_unary()?;
                    expr = Expr::Binary { op: BinOp::Div, left: Box::new(expr), right: Box::new(rhs) };
                }
                Some(TokenKind::Percent) => {
                    self.advance();
                    let rhs = self.parse_unary()?;
                    expr = Expr::Binary { op: BinOp::Mod, left: Box::new(expr), right: Box::new(rhs) };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, CompileError> {
        use crate::ast::UnaryOp;
        match self.peek_kind() {
            Some(TokenKind::Minus) => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary { op: UnaryOp::Neg, expr: Box::new(expr) })
            }
            Some(TokenKind::Bang) => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary { op: UnaryOp::Not, expr: Box::new(expr) })
            }
            Some(TokenKind::PlusPlus) => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary { op: UnaryOp::PreInc, expr: Box::new(expr) })
            }
            Some(TokenKind::MinusMinus) => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary { op: UnaryOp::PreDec, expr: Box::new(expr) })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, CompileError> {
        use crate::ast::UnaryOp;
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek_kind() {
                Some(TokenKind::PlusPlus) => {
                    self.advance();
                    expr = Expr::Unary { op: UnaryOp::PostInc, expr: Box::new(expr) };
                }
                Some(TokenKind::MinusMinus) => {
                    self.advance();
                    expr = Expr::Unary { op: UnaryOp::PostDec, expr: Box::new(expr) };
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
            _ => Err(CompileError::new("expected expression", t.line, t.column)),
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
            Err(CompileError::new(format!("expected {:?}, got {:?}", kind, t.kind), t.line, t.column))
        }
    }
}
