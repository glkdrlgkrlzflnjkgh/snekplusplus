use crate::ast::{Expr, FunctionDecl, Program, Stmt, TypeName};
use rayon::prelude::*;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

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

pub fn generate_cpp(program: &Program) -> String {
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
        Stmt::Empty => { 
            // i refuse to do anything.
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
                '\'' => write!(out, "\\'" )?,
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
                crate::ast::UnaryOp::Neg => write!(out, "-")?,
                crate::ast::UnaryOp::Not => write!(out, "!")?,
                crate::ast::UnaryOp::PreInc => write!(out, "++")?,
                crate::ast::UnaryOp::PreDec => write!(out, "--")?,
                crate::ast::UnaryOp::PostInc => {
                    emit_expr(out, expr)?;
                    write!(out, "++")?;
                    return Ok(());
                }
                crate::ast::UnaryOp::PostDec => {
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
                crate::ast::BinOp::Add => " + ",
                crate::ast::BinOp::Sub => " - ",
                crate::ast::BinOp::Mul => " * ",
                crate::ast::BinOp::Div => " / ",
                crate::ast::BinOp::Mod => " % ",
                crate::ast::BinOp::Less => " < ",
                crate::ast::BinOp::Greater => " > ",
                crate::ast::BinOp::LessEqual => " <= ",
                crate::ast::BinOp::GreaterEqual => " >= ",
                crate::ast::BinOp::Equal => " == ",
                crate::ast::BinOp::NotEqual => " != ",
                crate::ast::BinOp::And => " && ",
                crate::ast::BinOp::Or => " || ",
            };
            write!(out, "{op_str}")?;
            emit_expr(out, right)
        }
    }
}
