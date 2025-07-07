# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

The Kryon Compiler (`kryc`) is a Rust-based compiler for the KRY declarative UI language that produces optimized KRB binary files for cross-platform execution. The project compiles KRY source files (like HTML/CSS for native apps) into compact binary format for runtime engines.

## Development Commands

### Building
```bash
cargo build                    # Debug build
cargo build --release         # Optimized release build
```

### Testing
```bash
cargo test                     # Run unit tests
cargo test --release          # Run tests in release mode
```

### Benchmarking
```bash
cargo bench                    # Run performance benchmarks using Criterion
```

### Running the Compiler
```bash
# Basic usage
cargo run -- input.kry output.krb

# Using the enhanced CLI
cargo run -- compile input.kry -o output.krb --optimization aggressive --platform desktop
cargo run -- check input.kry --recursive           # Syntax checking
cargo run -- analyze output.krb --format json      # Binary analysis
cargo run -- init myproject --template component   # Project initialization
cargo run -- benchmark input.kry --iterations 100  # Performance testing
```

### Development Tools
```bash
# Linting (if rustfmt/clippy configured)
cargo fmt                      # Format code
cargo clippy                   # Lint code

# Watch mode for development
cargo run -- compile input.kry --watch
```

## Architecture Overview

The compiler follows a multi-phase compilation pipeline:

### Core Compilation Pipeline (`src/lib.rs:29-40`)
1. **Phase 0.1**: Preprocessor - Handle `@include` directives
2. **Phase 0.2**: Variables - Process `@variables` blocks and substitution
3. **Phase 1**: Lexer & Parser - Tokenize and build AST
4. **Phase 1.2**: Style Resolver - Resolve style inheritance
5. **Phase 1.5**: Component Resolver - Expand components and resolve properties
6. **Phase 2**: Size Calculator - Calculate final offsets and sizes
7. **Phase 3**: Code Generator - Write optimized KRB binary

### Key Modules
- **`lexer.rs`**: Tokenizes KRY source code
- **`parser.rs`**: Builds Abstract Syntax Tree (AST)
- **`ast.rs`**: AST node definitions and structures
- **`preprocessor.rs`**: Handles file inclusion and preprocessing
- **`component_resolver.rs`**: Expands component definitions and instances
- **`style_resolver.rs`**: Resolves CSS-like style inheritance
- **`semantic.rs`**: Semantic analysis and validation
- **`codegen.rs`**: Generates KRB binary format
- **`optimizer.rs`**: Code optimization passes
- **`cli.rs`**: Enhanced command-line interface with subcommands

### Data Flow
```
KRY Source → Lexer → Parser → AST → Semantic Analysis → 
Component Resolution → Style Resolution → Size Calculation → Code Generation → KRB Binary
```

## KRY Language Features

The compiler supports a declarative UI language with:
- **Elements**: App, Container, Text, Button, Input, Image
- **Styling**: CSS-like styles with inheritance and pseudo-selectors (`:hover`, `:active`)
- **Layout System**: Flex and absolute positioning with proper layout flag compilation
- **Percentage Support**: CSS-like percentage sizing (width: 100%, height: 50%)
- **Components**: Reusable UI components with properties and templates
- **Variables**: `@variables` blocks with expression evaluation and substitution
- **Scripts**: Multi-language scripting (Lua, JavaScript, Python, Wren)
- **Includes**: Modular development with `@include` directives

## Target Platforms

The compiler optimizes for different platforms:
- **Desktop**: High DPI, keyboard navigation
- **Mobile**: Touch optimization, battery efficiency
- **Web**: WebAssembly integration
- **Embedded**: Memory footprint minimization
- **Universal**: Cross-platform compatibility (default)

## Configuration

The compiler supports configuration via JSON/TOML files:
```json
{
  "optimization_level": 1,
  "target_platform": "universal",
  "embed_scripts": false,
  "compress_output": false,
  "include_directories": ["./components", "./shared"],
  "custom_variables": {
    "theme": "dark",
    "version": "1.0"
  }
}
```

## Development Status

According to `ROADMAP.md`, the project is in active development:
- ✅ **Completed**: CLI, optimization levels, target platforms, configuration
- ✅ **Completed**: KRY parsing with percentage support, layout flag compilation
- ✅ **Completed**: KRB generation with proper style resolution and property conversion
- ✅ **Completed**: Variable substitution and expression evaluation system
- 🟡 **Partially Implemented**: Component definitions and template expansion
- 🔴 **Missing**: Advanced optimization passes, resource bundling, animation support

The current focus is on Phase 1 (MVP Compiler) to achieve basic KRY→KRB compilation that works with the kryon-renderer runtime.

## Testing Strategy

The project uses multiple testing approaches:
- **Unit tests**: In individual modules (`cargo test`)
- **Integration tests**: In `tests/` directory
- **Benchmarks**: Performance testing in `benches/` using Criterion
- **CLI testing**: Enhanced CLI has comprehensive subcommand testing

## Binary Format (KRB)

The output KRB format is a compact binary representation:
- **Header**: Magic number, version, flags, section offsets
- **Sections**: Elements, styles, components, scripts, strings, resources
- **Optimization**: 65-75% size reduction vs source, string deduplication, property sharing

## Key Dependencies

- **clap**: CLI argument parsing with derive macros
- **serde/serde_json**: Serialization for configuration and debugging
- **notify**: File watching for auto-recompilation
- **criterion**: Performance benchmarking
- **byteorder**: Binary data writing
- **regex/meval**: Text processing and expression evaluation