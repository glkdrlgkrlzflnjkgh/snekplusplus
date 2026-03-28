use crate::color::apply_colors;
#[derive(Debug)]
pub struct SourcePos {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug)]
pub enum ErrorCode {
    MainMustReturnInt,
    NonVoidMustReturn,
    DuplicateFunction,
    DuplicateParameter,
    UndeclaredVariable,
    AssignmentTypeMismatch,
    ReturnTypeMismatch,
    ConditionMustBeBool,
    SwitchTypeMismatch,
    UndefinedFunction,
    FunctionArgCountMismatch,
    FunctionArgTypeMismatch,
    TernaryTypeMismatch,
    InvalidBinaryOperation,
    InvalidUnaryOperation,
    UnsafeNotAllowed
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::MainMustReturnInt => "MAIN_MUST_RETURN_INT",
            ErrorCode::NonVoidMustReturn => "NONVOID_MUST_RETURN",
            ErrorCode::DuplicateFunction => "DUPLICATE_FUNCTION",
            ErrorCode::DuplicateParameter => "DUPLICATE_PARAMETER",
            ErrorCode::UndeclaredVariable => "UNDECLARED_VARIABLE",
            ErrorCode::AssignmentTypeMismatch => "ASSIGNMENT_TYPE_MISMATCH",
            ErrorCode::ReturnTypeMismatch => "RETURN_TYPE_MISMATCH",
            ErrorCode::ConditionMustBeBool => "CONDITION_MUST_BE_BOOL",
            ErrorCode::SwitchTypeMismatch => "SWITCH_TYPE_MISMATCH",
            ErrorCode::UndefinedFunction => "UNDEFINED_FUNCTION",
            ErrorCode::FunctionArgCountMismatch => "FUNCTION_ARG_COUNT_MISMATCH",
            ErrorCode::FunctionArgTypeMismatch => "FUNCTION_ARG_TYPE_MISMATCH",
            ErrorCode::TernaryTypeMismatch => "TERNARY_TYPE_MISMATCH",
            ErrorCode::InvalidBinaryOperation => "INVALID_BINARY_OPERATION",
            ErrorCode::InvalidUnaryOperation => "INVALID_UNARY_OPERATION",
            ErrorCode::UnsafeNotAllowed => "UNSAFE_NOT_ALLOWED"
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value.to_uppercase().as_str() {
            "MAIN_MUST_RETURN_INT" => Some(ErrorCode::MainMustReturnInt),
            "NONVOID_MUST_RETURN" => Some(ErrorCode::NonVoidMustReturn),
            "DUPLICATE_FUNCTION" => Some(ErrorCode::DuplicateFunction),
            "DUPLICATE_PARAMETER" => Some(ErrorCode::DuplicateParameter),
            "UNDECLARED_VARIABLE" => Some(ErrorCode::UndeclaredVariable),
            "ASSIGNMENT_TYPE_MISMATCH" => Some(ErrorCode::AssignmentTypeMismatch),
            "RETURN_TYPE_MISMATCH" => Some(ErrorCode::ReturnTypeMismatch),
            "CONDITION_MUST_BE_BOOL" => Some(ErrorCode::ConditionMustBeBool),
            "SWITCH_TYPE_MISMATCH" => Some(ErrorCode::SwitchTypeMismatch),
            "UNDEFINED_FUNCTION" => Some(ErrorCode::UndefinedFunction),
            "FUNCTION_ARG_COUNT_MISMATCH" => Some(ErrorCode::FunctionArgCountMismatch),
            "FUNCTION_ARG_TYPE_MISMATCH" => Some(ErrorCode::FunctionArgTypeMismatch),
            "TERNARY_TYPE_MISMATCH" => Some(ErrorCode::TernaryTypeMismatch),
            "INVALID_BINARY_OPERATION" => Some(ErrorCode::InvalidBinaryOperation),
            "INVALID_UNARY_OPERATION" => Some(ErrorCode::InvalidUnaryOperation),
            "UNSAFE_NOT_ALLOWED" => Some(ErrorCode::UnsafeNotAllowed),
            _ => None,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ErrorCode::MainMustReturnInt => "Main function must have an int return type.",
            ErrorCode::NonVoidMustReturn => "Non-void function must return a value on all control paths.",
            ErrorCode::DuplicateFunction => "Function name already declared.",
            ErrorCode::DuplicateParameter => "Parameter name duplicated in function signature.",
            ErrorCode::UndeclaredVariable => "Variable used before declaration.",
            ErrorCode::AssignmentTypeMismatch => "Type mismatch in variable assignment or initialization.",
            ErrorCode::ReturnTypeMismatch => "Return expression type does not match function return type.",
            ErrorCode::ConditionMustBeBool => "Conditions in if/while/for must be boolean.",
            ErrorCode::SwitchTypeMismatch => "Switch expression and case values must share compatible type.",
            ErrorCode::UndefinedFunction => "Function call target is not defined.",
            ErrorCode::FunctionArgCountMismatch => "Wrong number of arguments in function call.",
            ErrorCode::FunctionArgTypeMismatch => "Argument type does not match parameter type.",
            ErrorCode::TernaryTypeMismatch => "Ternary branches must evaluate to the same type.",
            ErrorCode::InvalidBinaryOperation => "Invalid operand types for binary operation.",
            ErrorCode::InvalidUnaryOperation => "Invalid operand type for unary operation.",
            ErrorCode::UnsafeNotAllowed => "---IMPORTANT---\n\nthis error is {MAGENTA}{UNDERLINE}NOT{RESET} emitted by the compiler yet as cpp blocks aren't implemented yet!\n\n---END IMPORTANT--- \n\nCpp blocks are not allowed without passing --unsafe."
        }
    }

    pub fn help(&self) -> &'static str {
        match self {
            ErrorCode::MainMustReturnInt => "Add `return` value or change function to void; entry point must return int.",
            ErrorCode::NonVoidMustReturn => "Ensure every control path returns a value in non-void functions.",
            ErrorCode::DuplicateFunction => "Rename one of the functions or remove duplicates.",
            ErrorCode::DuplicateParameter => "Use unique parameter names in function signature.",
            ErrorCode::UndeclaredVariable => "Declare the variable before usage.",
            ErrorCode::AssignmentTypeMismatch => "Match variable type and expression type exactly or use `var` for inference.",
            ErrorCode::ReturnTypeMismatch => "Fix the expression in return to the declared function return type.",
            ErrorCode::ConditionMustBeBool => "Use bool expressions (`true/false`, comparisons) in conditions.",
            ErrorCode::SwitchTypeMismatch => "Ensure switch values all have the same type as switch expression.",
            ErrorCode::UndefinedFunction => "Declare or import the called function before invocation.",
            ErrorCode::FunctionArgCountMismatch => "Pass correct number of arguments to function calls.",
            ErrorCode::FunctionArgTypeMismatch => "Pass arguments matching parameter types.",
            ErrorCode::TernaryTypeMismatch => "Make both branches produce the same type.",
            ErrorCode::InvalidBinaryOperation => "Use compatible operands for the specified binary operator.",
            ErrorCode::InvalidUnaryOperation => "Use operator with matching operand type.",
            ErrorCode::UnsafeNotAllowed => "Pass --unsafe to the compiler on compilation."
        }
    }

    pub fn example(&self) -> &'static str {
        match self {
            ErrorCode::MainMustReturnInt => "public funct int Main() {\n    return 0;\n}",
            ErrorCode::NonVoidMustReturn => "public funct int f() {\n    if (true) { return 1; }\n    // missing return on else path\n}",
            ErrorCode::DuplicateFunction => "public funct int foo() {}\npublic funct int foo() {}",
            ErrorCode::DuplicateParameter => "public funct int f(int a, int a) {}",
            ErrorCode::UndeclaredVariable => "public funct int f() { return x; }",
            ErrorCode::AssignmentTypeMismatch => "int x = true;",
            ErrorCode::ReturnTypeMismatch => "public funct int f() { return true; }",
            ErrorCode::ConditionMustBeBool => "if (42) {}",
            ErrorCode::SwitchTypeMismatch => "switch (1) { case 'a': break; }",
            ErrorCode::UndefinedFunction => "x = foo(); // no foo defined",
            ErrorCode::FunctionArgCountMismatch => "public funct int f(int x) {}\nf();",
            ErrorCode::FunctionArgTypeMismatch => "public funct int f(int x) {}\nf(true);",
            ErrorCode::TernaryTypeMismatch => "true ? 1 : 'a'",
            ErrorCode::InvalidBinaryOperation => "1 + true",
            ErrorCode::InvalidUnaryOperation => "!1",
            ErrorCode::UnsafeNotAllowed => "cpp {std::cout << \"hello!\" << std::endl;}"
        }
    }
}

#[derive(Debug)]
pub struct SemanticError {
    pub code: ErrorCode,
    pub message: String,
}

impl SemanticError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug)]
pub struct CompileError {
    pub message: String,
    pub pos: SourcePos,
    pub code: Option<ErrorCode>,
}

impl CompileError {
    pub fn new(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            message: message.into(),
            pos: SourcePos { line, column },
            code: None,
        }
    }

    pub fn with_code(code: ErrorCode, message: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            message: message.into(),
            pos: SourcePos { line, column },
            code: Some(code),
        }
    }
}


pub fn print_error(err: &CompileError, filename: &str, src: &str) {
    let line = err.pos.line;
    let col = err.pos.column;

    let code_label = if let Some(code) = &err.code {
        format!("[{}] ", code.as_str())
    } else {
        String::new()
    };

    // Escape braces so format! doesn't treat them as placeholders
    let header = format!(
        "{filename}:{line}:{col}: {{RED}}{{BOLD}}error:{{RESET}} {code_label}{}",
        err.message
    );
    eprintln!("{}", apply_colors(&header));

    if let Some(line_str) = src.lines().nth(line - 1) {
        let line_display = format!("{{DIM}}  {line} |{{RESET}} {line_str}");
        eprintln!("{}", apply_colors(&line_display));

        let mut caret = String::new();
        caret.push_str(&format!("{{DIM}}    |{{RESET}} "));
        for _ in 1..col {
            caret.push(' ');
        }
        caret.push_str("{{RED}}↑{{RESET}}");

        eprintln!("{}", apply_colors(&caret));
    }
}

pub fn explain_error(code: ErrorCode) {
    println!("error code: {}", code.as_str());
    println!("description:\n {}", apply_colors(code.description()));
    println!("help: {}", code.help());
    println!("example:\n{}", code.example());
}

pub fn explain_error_help() {
    println!("usage: snekplusplus explain <ERROR_CODE> | explain <file.spp> [function]");
    println!("Available error codes:");
    for code in [
        ErrorCode::MainMustReturnInt,
        ErrorCode::NonVoidMustReturn,
        ErrorCode::DuplicateFunction,
        ErrorCode::DuplicateParameter,
        ErrorCode::UndeclaredVariable,
        ErrorCode::AssignmentTypeMismatch,
        ErrorCode::ReturnTypeMismatch,
        ErrorCode::ConditionMustBeBool,
        ErrorCode::SwitchTypeMismatch,
        ErrorCode::UndefinedFunction,
        ErrorCode::FunctionArgCountMismatch,
        ErrorCode::FunctionArgTypeMismatch,
        ErrorCode::TernaryTypeMismatch,
        ErrorCode::InvalidBinaryOperation,
        ErrorCode::InvalidUnaryOperation,
        ErrorCode::UnsafeNotAllowed
    ] {
        println!("- {}: {}", code.as_str(), code.description());
    }
}
