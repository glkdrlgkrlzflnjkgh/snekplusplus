use std::collections::HashMap;
use crate::ast::{BinOp, Expr, FunctionDecl, Program, Stmt, TypeName};
use crate::error::{ErrorCode, SemanticError};

pub fn check_program(program: &Program) -> Result<(), SemanticError> {
    let mut funcs: HashMap<String, &FunctionDecl> = HashMap::new();
    for func in &program.functions {
        if funcs.insert(func.name.clone(), func).is_some() {
            return Err(SemanticError::new(
                ErrorCode::DuplicateFunction,
                format!("duplicate function '{}'", func.name),
            ));
        }
    }

    for func in &program.functions {
        check_function(func, &funcs)?;
    }

    Ok(())
}

fn check_function(func: &FunctionDecl, funcs: &HashMap<String, &FunctionDecl>) -> Result<(), SemanticError> {
    if func.name == "Main" && func.return_type != TypeName::Int {
        return Err(SemanticError::new(ErrorCode::MainMustReturnInt, "Main must return int"));
    }

    let mut env: HashMap<String, TypeName> = HashMap::new();
    for param in &func.params {
        if env.insert(param.name.clone(), param.ty).is_some() {
            return Err(SemanticError::new(
                ErrorCode::DuplicateParameter,
                format!("duplicate parameter '{}'", param.name),
            ));
        }
    }

    check_stmts(&func.body, func.return_type, funcs, &mut env)?;
    println!("=== CHECKING FUNCTION {} ===", func.name);
    println!("return type: {:?}", func.return_type);
    println!("body: {:#?}", func.body);
    println!("guaranteed_return: {}", body_guaranteed_return(&func.body));
    if func.return_type != TypeName::Void && !body_guaranteed_return(&func.body) {
        return Err(SemanticError::new(ErrorCode::NonVoidMustReturn, "non-void function must return a value on all paths"));
    }

    Ok(())
}

fn body_guaranteed_return(stmts: &[Stmt]) -> bool {
    for stmt in stmts {
        if stmt_guaranteed_return(stmt) {
            return true;
        }
    }
    false
}

fn stmt_guaranteed_return(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return(_) => true,

        Stmt::If { then_body, else_body, .. } => {
            // both branches must guarantee return
            !then_body.is_empty()
                && !else_body.is_empty()
                && body_guaranteed_return(then_body)
                && body_guaranteed_return(else_body)
        }

        Stmt::Switch { cases, .. } => {
            let mut has_default = false;
            for (expr, body) in cases {
                if expr.is_none() {
                    has_default = true;
                }
                if !body_guaranteed_return(body) {
                    return false;
                }
            }
            has_default
        }

        // Loops do NOT guarantee return unless you add infinite-loop detection
        Stmt::While { .. } => false,
        Stmt::DoWhile { .. } => false,
        Stmt::For { .. } => false,

        _ => false,
    }
}

fn check_stmts(stmts: &[Stmt], ret_ty: TypeName, funcs: &HashMap<String, &FunctionDecl>, env: &mut HashMap<String, TypeName>) -> Result<(), SemanticError> {
    for stmt in stmts {
        check_stmt(stmt, ret_ty, funcs, env)?;
    }
    Ok(())
}

fn check_stmt(stmt: &Stmt, ret_ty: TypeName, funcs: &HashMap<String, &FunctionDecl>, env: &mut HashMap<String, TypeName>) -> Result<(), SemanticError> {
    match stmt {
        Stmt::VarDecl { explicit_type, name, init } => {
            let init_ty = check_expr(init, funcs, env)?;
            let decl_ty = if let Some(expected) = explicit_type {
                if *expected != init_ty {
                    return Err(SemanticError::new(
                        ErrorCode::AssignmentTypeMismatch,
                        format!("invalid initialization of '{}' (expected {:?}, found {:?})", name, expected, init_ty),
                    ));
                }
                *expected
            } else {
                init_ty
            };

            if env.contains_key(name) {
                return Err(SemanticError::new(
                    ErrorCode::DuplicateParameter,
                    format!("variable '{}' already declared", name),
                ));
            }
            env.insert(name.clone(), decl_ty);
        }
        Stmt::Assign { name, expr } => {
            let var_ty = env.get(name).ok_or_else(|| SemanticError::new(ErrorCode::UndeclaredVariable, format!("use of undeclared variable '{}'", name)))?;
            let expr_ty = check_expr(expr, funcs, env)?;
            if *var_ty != expr_ty {
                return Err(SemanticError::new(
                    ErrorCode::AssignmentTypeMismatch,
                    format!("type mismatch in assignment to '{}' ({:?} = {:?})", name, var_ty, expr_ty),
                ));
            }
        }
        Stmt::Return(opt_expr) => {
            match (ret_ty, opt_expr) {
                (TypeName::Void, Some(_)) => {
                    return Err(SemanticError::new(ErrorCode::ReturnTypeMismatch, "returning a value from void function"));
                }
                (TypeName::Void, None) => {}
                (_, None) => {
                    return Err(SemanticError::new(ErrorCode::NonVoidMustReturn, "non-void function must return a value"));
                }
                (_, Some(expr)) => {
                    let expr_ty = check_expr(expr, funcs, env)?;
                    if expr_ty != ret_ty {
                        return Err(SemanticError::new(ErrorCode::ReturnTypeMismatch, format!("return type mismatch (expected {:?}, found {:?})", ret_ty, expr_ty)));
                    }
                }
            }
        }
        Stmt::ExprStmt(expr) => {
            let _ = check_expr(expr, funcs, env)?;
        }
        Stmt::If { cond, then_body, else_body } => {
            let cond_ty = check_expr(cond, funcs, env)?;
            if cond_ty != TypeName::Bool {
                return Err(SemanticError::new(ErrorCode::ConditionMustBeBool, "if condition must be bool"));
            }
            let mut then_env = env.clone();
            let mut else_env = env.clone();
            check_stmts(then_body, ret_ty, funcs, &mut then_env)?;
            check_stmts(else_body, ret_ty, funcs, &mut else_env)?;
        }
        Stmt::While { cond, body } => {
            let cond_ty = check_expr(cond, funcs, env)?;
            if cond_ty != TypeName::Bool {
                return Err(SemanticError::new(ErrorCode::ConditionMustBeBool, "while condition must be bool"));
            }
            let mut body_env = env.clone();
            check_stmts(body, ret_ty, funcs, &mut body_env)?;
        }
        Stmt::DoWhile { body, cond } => {
            let cond_ty = check_expr(cond, funcs, env)?;
            if cond_ty != TypeName::Bool {
                return Err(SemanticError::new(ErrorCode::ConditionMustBeBool, "do-while condition must be bool"));
            }
            let mut body_env = env.clone();
            check_stmts(body, ret_ty, funcs, &mut body_env)?;
        }
        Stmt::For { init, cond, step, body } => {
            let mut for_env = env.clone();
            check_stmt(init, ret_ty, funcs, &mut for_env)?;
            let cond_ty = check_expr(cond, funcs, &for_env)?;
            if cond_ty != TypeName::Bool {
                return Err(SemanticError::new(ErrorCode::ConditionMustBeBool, "for-loop condition must be bool"));
            }
            check_stmt(step, ret_ty, funcs, &mut for_env)?;
            check_stmts(body, ret_ty, funcs, &mut for_env)?;
        }
        Stmt::Switch { expr, cases } => {
            let switch_ty = check_expr(expr, funcs, env)?;
            if !matches!(switch_ty, TypeName::Int | TypeName::Char) {
                return Err(SemanticError::new(ErrorCode::SwitchTypeMismatch, "switch expression must be int or char"));
            }
            for (case_expr_opt, case_body) in cases {
                if let Some(case_expr) = case_expr_opt {
                    let case_ty = check_expr(case_expr, funcs, env)?;
                    if case_ty != switch_ty {
                        return Err(SemanticError::new(ErrorCode::SwitchTypeMismatch, "switch case value type mismatch"));
                    }
                }
                let mut case_env = env.clone();
                check_stmts(case_body, ret_ty, funcs, &mut case_env)?;
            }
        }
        Stmt::Break | Stmt::Continue => {}
        Stmt::Empty => {
            // Do absolutely nothing.
        }
    }
    Ok(())
}

fn check_expr(expr: &Expr, funcs: &HashMap<String, &FunctionDecl>, env: &HashMap<String, TypeName>) -> Result<TypeName, SemanticError> {
    match expr {
        Expr::Number(_) => Ok(TypeName::Int),
        Expr::Float(_) => Ok(TypeName::Float),
        Expr::BoolLiteral(_) => Ok(TypeName::Bool),
        Expr::StringLiteral(_) => Ok(TypeName::String),
        Expr::CharLiteral(_) => Ok(TypeName::Char),
        Expr::Var(name) => env
            .get(name)
            .cloned()
            .ok_or_else(|| SemanticError::new(ErrorCode::UndeclaredVariable, format!("use of undeclared variable '{}'", name))),
        Expr::Call { name, args } => {
            if name == "print" {
                for arg in args {
                    let _ = check_expr(arg, funcs, env)?;
                }
                return Ok(TypeName::Void);
            }
            let decl = funcs
                .get(name)
                .ok_or_else(|| SemanticError::new(ErrorCode::UndefinedFunction, format!("undefined function '{}'", name)))?;
            if decl.params.len() != args.len() {
                return Err(SemanticError::new(ErrorCode::FunctionArgCountMismatch, format!("function '{}' expects {} arguments, got {}", name, decl.params.len(), args.len())));
            }
            for (arg, param) in args.iter().zip(decl.params.iter()) {
                let arg_ty = check_expr(arg, funcs, env)?;
                if arg_ty != param.ty {
                    return Err(SemanticError::new(ErrorCode::FunctionArgTypeMismatch, format!("function '{}' argument type mismatch (expected {:?}, found {:?})", name, param.ty, arg_ty)));
                }
            }
            Ok(decl.return_type)
        }
        Expr::Binary { op, left, right } => {
            let left_ty = check_expr(left, funcs, env)?;
            let right_ty = check_expr(right, funcs, env)?;
            match op {
                BinOp::Add => {
                    if left_ty == TypeName::String && right_ty == TypeName::String {
                        Ok(TypeName::String)
                    } else if is_numeric_ty(left_ty) && is_numeric_ty(right_ty) {
                        Ok(arithmetic_result_ty(left_ty, right_ty))
                    } else {
                        Err(SemanticError::new(ErrorCode::InvalidBinaryOperation, "invalid types for '+'"))
                    }
                }
                BinOp::Sub | BinOp::Mul | BinOp::Div => {
                    if is_numeric_ty(left_ty) && is_numeric_ty(right_ty) {
                        Ok(arithmetic_result_ty(left_ty, right_ty))
                    } else {
                        Err(SemanticError::new(ErrorCode::InvalidBinaryOperation, "invalid numeric types for arithmetic"))
                    }
                }
                BinOp::Mod => {
                    if left_ty == TypeName::Int && right_ty == TypeName::Int {
                        Ok(TypeName::Int)
                    } else {
                        Err(SemanticError::new(ErrorCode::InvalidBinaryOperation, "% operator requires int operands"))
                    }
                }
                BinOp::Less | BinOp::Greater | BinOp::LessEqual | BinOp::GreaterEqual => {
                    if (is_numeric_ty(left_ty) && is_numeric_ty(right_ty)) || left_ty == right_ty {
                        Ok(TypeName::Bool)
                    } else {
                        Err(SemanticError::new(ErrorCode::InvalidBinaryOperation, "invalid operands for comparison"))
                    }
                }
                BinOp::Equal | BinOp::NotEqual => {
                    if left_ty == right_ty {
                        Ok(TypeName::Bool)
                    } else {
                        Err(SemanticError::new(ErrorCode::InvalidBinaryOperation, "type mismatch for equality operator"))
                    }
                }
                BinOp::And | BinOp::Or => {
                    if left_ty == TypeName::Bool && right_ty == TypeName::Bool {
                        Ok(TypeName::Bool)
                    } else {
                        Err(SemanticError::new(ErrorCode::InvalidBinaryOperation, "logical operators require bool operands"))
                    }
                }
            }
        }
        Expr::Unary { op, expr } => {
            let expr_ty = check_expr(expr, funcs, env)?;
            match op {
                crate::ast::UnaryOp::Neg => {
                    if is_numeric_ty(expr_ty) {
                        Ok(expr_ty)
                    } else {
                        Err(SemanticError::new(ErrorCode::InvalidUnaryOperation, "unary '-' requires numeric type"))
                    }
                }
                crate::ast::UnaryOp::Not => {
                    if expr_ty == TypeName::Bool {
                        Ok(TypeName::Bool)
                    } else {
                        Err(SemanticError::new(ErrorCode::InvalidUnaryOperation, "unary '!' requires bool"))
                    }
                }
                crate::ast::UnaryOp::PreInc | crate::ast::UnaryOp::PreDec | crate::ast::UnaryOp::PostInc | crate::ast::UnaryOp::PostDec => {
                    if is_numeric_ty(expr_ty) || expr_ty == TypeName::Char {
                        Ok(expr_ty)
                    } else {
                        Err(SemanticError::new(ErrorCode::InvalidUnaryOperation, "increment/decrement requires numeric or char type"))
                    }
                }
            }
        }
        Expr::Ternary { cond, then_expr, else_expr } => {
            let cond_ty = check_expr(cond, funcs, env)?;
            if cond_ty != TypeName::Bool {
                return Err(SemanticError::new(ErrorCode::ConditionMustBeBool, "ternary condition must be bool"));
            }
            let then_ty = check_expr(then_expr, funcs, env)?;
            let else_ty = check_expr(else_expr, funcs, env)?;
            if then_ty == else_ty {
                Ok(then_ty)
            } else {
                Err(SemanticError::new(ErrorCode::TernaryTypeMismatch, "ternary branches must have the same type"))
            }
        }
        Expr::Paren(inner) => check_expr(inner, funcs, env),
    }
}

fn is_numeric_ty(ty: TypeName) -> bool {
    matches!(ty, TypeName::Int | TypeName::Float | TypeName::Double)
}

fn arithmetic_result_ty(left: TypeName, right: TypeName) -> TypeName {
    if left == TypeName::Double || right == TypeName::Double {
        TypeName::Double
    } else if left == TypeName::Float || right == TypeName::Float {
        TypeName::Float
    } else {
        TypeName::Int
    }
}
