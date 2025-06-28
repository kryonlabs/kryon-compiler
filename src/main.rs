//! Kryon Compiler Binary

use kryc::{compile_file, CompilerError, NAME, VERSION};
use std::env;
use std::process;

fn main() {
    env_logger::init();
    
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 3 {
        eprintln!("Usage: {} <input.kry> <output.krb>", args[0]);
        eprintln!("  {NAME} v{VERSION} - Kryon UI Language Compiler");
        eprintln!("  Compiles KRY source files to optimized KRB binary format");
        process::exit(1);
    }
    
    let input_file = &args[1];
    let output_file = &args[2];
    
    println!("{NAME} v{VERSION}");
    println!("Compiling '{}' to '{}'...", input_file, output_file);
    
    match compile_file(input_file, output_file) {
        Ok(stats) => {
            println!("Compilation successful!");
            println!("Output size: {} bytes", stats.output_size);
        }
        Err(CompilerError::Io(e)) => {
            eprintln!("IO Error: {}", e);
            process::exit(1);
        }
        Err(e) => {
            eprintln!("Compilation failed: {}", e);
            process::exit(1);
        }
    }
}