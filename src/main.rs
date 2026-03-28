mod ast;
mod error;
mod import;
mod lexer;
mod parser;
mod checker;
mod codegen;
mod color;

use crate::ast::format_type;
use crate::checker::check_program;
use crate::codegen::generate_cpp;
use crate::error::{explain_error, explain_error_help, print_error, CompileError, ErrorCode};
use crate::import::load_snekpp_with_imports;
use crate::lexer::Lexer;
use crate::parser::Parser;

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let mut args = env::args().skip(1);

    let mut import_dir: Option<PathBuf> = None;
    let mut entry: Option<String> = None;
    let mut output: Option<String> = None;
    let mut explain_item: Option<String> = None;
    let mut explain_error_code: Option<String> = None;
    let mut explain_mode = false;
    let mut keep_intermediate = false;
    let mut opt_level: OptLevel = OptLevel::None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "explain" => explain_mode = true,
            "--error-code" | "-e" => {
                if let Some(code) = args.next() {
                    explain_mode = true;
                    explain_error_code = Some(code);
                } else {
                    eprintln!("error: --error-code requires a code");
                    std::process::exit(1);
                }
            }
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
            "-optno" => opt_level = OptLevel::None,
            "-optyes" => opt_level = OptLevel::O2,
            "-optvyes" => opt_level = OptLevel::O3,
            "--keep-intermediate" => keep_intermediate = true,
            "--help" | "-h" => {
                println!("usage: snekplusplus <entry.spp> -o <output.exe> [-importdir DIR] [-optno|-optyes|-optvyes] [--keep-intermediate]");
                println!("       snekplusplus explain <entry.spp> [function]");
                println!("       snekplusplus explain --error-code <CODE>");
                return;
            }
            _ => {
                if explain_mode {
                    if entry.is_none() {
                        entry = Some(arg);
                    } else if explain_item.is_none() {
                        explain_item = Some(arg);
                    } else {
                        eprintln!("usage: snekplusplus explain <entry.spp> [item]");
                        std::process::exit(1);
                    }
                } else if entry.is_none() {
                    entry = Some(arg);
                } else {
                    eprintln!("usage: snekplusplus <entry.spp> -o <output.exe> [-importdir DIR] [-optno|-optyes|-optvyes]");
                    std::process::exit(1);
                }
            }
        }
    }

    if let Some(code_str) = explain_error_code {
        if let Some(code) = ErrorCode::from_str(&code_str) {
            explain_error(code);
            return;
        } else {
            eprintln!("error: unknown error code '{}'; use snekplusplus explain --error-code LIST", code_str);
            explain_error_help();
            std::process::exit(1);
        }
    }

    if explain_mode && entry.is_none() {
        explain_error_help();
        return;
    }

    let entry = if let Some(e) = entry {
        e
    } else {
        eprintln!("usage: snekplusplus <entry.spp> -o <output.exe> [-importdir DIR] [-optno|-optyes|-optvyes]");
        std::process::exit(1);
    };

    if explain_mode {
        if let Some(code) = ErrorCode::from_str(&entry) {
            explain_error(code);
            return;
        }
    }

    let output = if explain_mode {
        None
    } else {
        match output {
            Some(o) => Some(o),
            None => {
                eprintln!("error: missing -o <output.exe>");
                std::process::exit(1);
            }
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
        let err = CompileError::with_code(e.code, e.message, 1, 1);
        print_error(&err, &entry, &src);
        std::process::exit(1);
    }

    if explain_mode {
        explain_program(&program, explain_item.as_deref());
        return;
    }

    let cpp = generate_cpp(&program);
    fs::write("out.cpp", &cpp).expect("failed to write out.cpp");

    let mut cmd = String::from("clang++ out.cpp -std=c++20");
    match opt_level {
        OptLevel::None => {}
        OptLevel::O2 => cmd.push_str(" -O2"),
        OptLevel::O3 => cmd.push_str(" -O3"),
        
    }
    cmd.push_str(" -o ");
    cmd.push_str(output.unwrap().as_str());

    let status = Command::new("cmd")
        .args(["/C", &cmd])
        .status()
        .expect("failed to invoke clang++ via cmd.exe");

    eprintln!("clang++ exit status: {status}");

    if !keep_intermediate {
        if let Err(e) = fs::remove_file("out.cpp") {
            eprintln!("warning: failed to remove out.cpp: {e}");
        }
    }
}

fn explain_program(program: &crate::ast::Program, item: Option<&str>) {
    if program.functions.is_empty() {
        println!("no functions to explain");
        return;
    }

    match item {
        None => {
            println!("functions:");
            for func in &program.functions {
                println!("- {}{}({}) -> {:?}",
                    func.name,
                    if func.name == "Main" { " (entry)" } else { "" },
                    func.params.iter().map(|p| format!("{} {}", format_type(p.ty), p.name)).collect::<Vec<_>>().join(", "),
                    func.return_type
                );
            }
        }
        Some(x) => {
            if let Some(func) = program.functions.iter().find(|f| f.name == x) {
                println!("function '{}':", func.name);
                println!("  return: {:?}", func.return_type);
                println!("  visibility: {:?}", func.visibility);
                if func.params.is_empty() {
                    println!("  params: (none)");
                } else {
                    println!("  params:");
                    for p in &func.params {
                        println!("    - {}: {:?}", p.name, p.ty);
                    }
                }
                println!("  body statements: {}", func.body.len());
            } else {
                println!("symbol '{}' not found; available functions:", x);
                for func in &program.functions {
                    println!("  - {}", func.name);
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
enum OptLevel {
    None,
    O2,
    O3
}
