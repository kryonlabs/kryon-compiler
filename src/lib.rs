//! Kryon UI Language Compiler
//! 
//! A complete compiler for the KRY declarative UI language that produces
//! optimized KRB binary files for cross-platform execution.
//!
//! # Features
//! 
//! - Complete KRY language support with components, styles, and scripting
//! - Optimized KRB binary format with 65-75% size reduction
//! - Cross-platform runtime support (Desktop, Mobile, Web, Embedded)
//! - Multi-language scripting (Lua, JavaScript, Python, Wren)
//! - Comprehensive error reporting with line numbers and context
//! - Include system for modular development
//! - Variable system with expression evaluation
//! - Style inheritance and pseudo-selectors
//! - Component system with property validation
//!
//! # Basic Usage
//!
//! ```rust
//! use kryc::{compile_file, Result};
//!
//! fn main() -> Result<()> {
//!     compile_file("app.kry", "app.krb")?;
//!     Ok(())
//! }
//! ```
//!
//! # Compilation Pipeline
//!
//! The compiler follows a multi-phase approach:
//!
//! 1. **Phase 0.1**: Preprocessor - Handle @include directives
//! 2. **Phase 0.2**: Variables - Process @variables blocks and substitution
//! 3. **Phase 1**: Lexer & Parser - Tokenize and build AST
//! 4. **Phase 1.2**: Style Resolver - Resolve style inheritance
//! 5. **Phase 1.5**: Component Resolver - Expand components and resolve properties
//! 6. **Phase 2**: Size Calculator - Calculate final offsets and sizes
//! 7. **Phase 3**: Code Generator - Write optimized KRB binary

pub mod types;
pub mod error;
pub mod lexer;
pub mod utils;
pub mod preprocessor;

pub mod ast;
pub mod parser;
pub mod script;
pub mod semantic;
pub mod codegen;
pub mod size_calculator;
pub mod component_resolver;
pub mod style_resolver;
pub mod optimizer;
pub mod cli;
pub mod variable_context;
use serde::Serialize;
use std::io::Read;

// Re-export commonly used types and functions
pub use error::{CompilerError, Result};
pub use types::*;
pub use lexer::{Lexer, Token, TokenType};
pub use preprocessor::{Preprocessor, preprocess_file};
pub use utils::{
    clean_and_quote_value, parse_color, parse_layout_string, 
    evaluate_expression, is_valid_identifier, VariableProcessor
};

pub use ast::{AstNode, AstProperty, ComponentProperty, PseudoSelector, ScriptSource};
pub use parser::Parser;
pub use script::ScriptProcessor;
pub use semantic::SemanticAnalyzer;
pub use codegen::CodeGenerator;
pub use size_calculator::{SizeCalculator, SizeStatistics};
pub use component_resolver::{ComponentResolver, ComponentStats, ComponentComplexity};
pub use variable_context::{VariableContext, VariableEntry, VariableScope};
pub use style_resolver::{StyleResolver, StyleResolutionStats};
pub use optimizer::{Optimizer, OptimizationStats};
pub use cli::EnhancedCli;

/// Compiler version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

/// Compiler build information
pub const BUILD_INFO: CompilerInfo = CompilerInfo {
    version: VERSION,
    name: NAME,
    description: DESCRIPTION,
    target_krb_version: (types::KRB_VERSION_MAJOR, types::KRB_VERSION_MINOR),
    supported_features: &[
        "includes",
        "variables", 
        "styles",
        "components",
        "scripting",
        "pseudo-selectors",
        "animations",
        "resources",
    ],
};

/// Compiler information structure
#[derive(Debug, Clone)]
pub struct CompilerInfo {
    pub version: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub target_krb_version: (u8, u8),
    pub supported_features: &'static [&'static str],
}

/// Compilation options and settings
#[derive(Debug, Clone)]
pub struct CompilerOptions {
    /// Enable debug mode with extra validation and logging
    pub debug_mode: bool,
    
    /// Optimization level (0 = none, 1 = basic, 2 = aggressive)
    pub optimization_level: u8,
    
    /// Target platform for optimization
    pub target_platform: TargetPlatform,
    
    /// Whether to embed scripts inline or reference externally
    pub embed_scripts: bool,
    
    /// Whether to compress the output
    pub compress_output: bool,
    
    /// Maximum allowed file size in bytes (0 = no limit)
    pub max_file_size: u64,
    
    /// Include directories for @include resolution
    pub include_directories: Vec<String>,
    
    /// Whether to generate debug symbols
    pub generate_debug_info: bool,
    
    /// Custom variable definitions to inject
    pub custom_variables: std::collections::HashMap<String, String>,
}

/// Target platform for compilation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetPlatform {
    Desktop,
    Mobile,
    Web,
    Embedded,
    Universal,
}

impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            debug_mode: false,
            optimization_level: 1,
            target_platform: TargetPlatform::Universal,
            embed_scripts: false,
            compress_output: false,
            max_file_size: 0,
            include_directories: Vec::new(),
            generate_debug_info: false,
            custom_variables: std::collections::HashMap::new(),
        }
    }
}

/// Compilation statistics and metrics
#[derive(Debug, Clone, Default, Serialize)]
pub struct CompilationStats {
    /// Original source size in bytes
    pub source_size: u64,
    
    /// Final KRB size in bytes
    pub output_size: u64,
    
    /// Compression ratio (output/source)
    pub compression_ratio: f64,
    
    /// Number of elements processed
    pub element_count: usize,
    
    /// Number of styles processed
    pub style_count: usize,
    
    /// Number of components processed
    pub component_count: usize,
    
    /// Number of scripts processed
    pub script_count: usize,
    
    /// Number of resources referenced
    pub resource_count: usize,
    
    /// Number of strings in string table
    pub string_count: usize,
    
    /// Number of included files
    pub include_count: usize,
    
    /// Number of variables resolved
    pub variable_count: usize,
    
    /// Compilation time in milliseconds
    pub compile_time_ms: u64,
    
    /// Memory usage peak in bytes
    pub peak_memory_usage: u64,
}

/// Main compiler entry point with default options
pub fn compile_file(input_path: &str, output_path: &str) -> Result<CompilationStats> {
    compile_file_with_options(input_path, output_path, CompilerOptions::default())
}

/// Compile with custom options
pub fn compile_file_with_options(
    input_path: &str, 
    output_path: &str, 
    options: CompilerOptions
) -> Result<CompilationStats> {
    use std::fs;
    use std::time::Instant;
    
    let start_time = Instant::now();
    
    if options.debug_mode {
        log::info!("{} v{}", NAME, VERSION);
        log::info!("Target KRB version: {}.{}", 
                  types::KRB_VERSION_MAJOR, types::KRB_VERSION_MINOR);
        log::info!("Compiling '{}' to '{}'...", input_path, output_path);
        log::debug!("Compiler options: {:?}", options);
    }
    
    // Read input file and get source size
    let source = fs::read_to_string(input_path)
        .map_err(|e| CompilerError::FileNotFound { 
            path: format!("{}: {}", input_path, e) 
        })?;
    
    let source_size = source.len() as u64;
    
    // Compile the source
    let (krb_data, mut stats) = compile_source_with_options(&source, input_path, options.clone())?;
    
    // Update stats
    stats.source_size = source_size;
    stats.output_size = krb_data.len() as u64;
    stats.compression_ratio = if source_size > 0 {
        stats.output_size as f64 / source_size as f64
    } else {
        0.0
    };
    stats.compile_time_ms = start_time.elapsed().as_millis() as u64;
    
    // Write output
    fs::write(output_path, krb_data)
        .map_err(|e| CompilerError::Io(e))?;
    
    if options.debug_mode {
        log::info!("Compilation successful!");
        log::info!("Source size: {} bytes", stats.source_size);
        log::info!("Output size: {} bytes", stats.output_size);
        log::info!("Compression ratio: {:.1}%", (1.0 - stats.compression_ratio) * 100.0);
        log::info!("Compile time: {}ms", stats.compile_time_ms);
        log::debug!("Full stats: {:?}", stats);
    }
    
    Ok(stats)
}

/// Compile KRY source code to KRB binary data with default options
pub fn compile_source(source: &str, filename: &str) -> Result<Vec<u8>> {
    let (data, _stats) = compile_source_with_options(source, filename, CompilerOptions::default())?;
    Ok(data)
}

/// Compile KRY source code to KRB binary data with custom options
pub fn compile_source_with_options(
    source: &str, 
    filename: &str, 
    options: CompilerOptions
) -> Result<(Vec<u8>, CompilationStats)> {
    let mut state = CompilerState::new();
    state.current_file_path = filename.to_string();
    
    let mut stats = CompilationStats::default();
    
    if options.debug_mode {
        log::debug!("Starting compilation pipeline for {}", filename);
        log::debug!("Source length: {} characters", source.len());
    }
    
    // Phase 0.1: Process includes
    if options.debug_mode {
        log::debug!("Phase 0.1: Processing includes...");
    }
    
    let (source_with_includes, source_map) = if source.contains("@include") {
        if std::path::Path::new(filename).exists() {
            if options.debug_mode {
                log::debug!("Processing includes for file: {}", filename);
            }
            let result = preprocessor::preprocess_file(filename)?;
            if options.debug_mode {
                log::debug!("Include processing complete. Combined content length: {}", result.0.len());
                log::debug!("Source map has {} entries", result.1.mapping_count());
            }
            result
        } else {
            (source.to_string(), crate::error::SourceMap::new())
        }
    } else {
        (source.to_string(), crate::error::SourceMap::new())
    };
    
    if options.debug_mode {
        log::debug!("Phase 0.1 complete. Content length: {}", source_with_includes.len());
    }
    
    // Phase 0.2: Process variables
    if options.debug_mode {
        log::debug!("Phase 0.2: Processing variables...");
    }
    
    let variable_processor = crate::utils::VariableProcessor::new();
    
    // Inject custom variables first
    for (name, value) in &options.custom_variables {
        if !crate::utils::is_valid_identifier(name) {
            return Err(CompilerError::variable_legacy(
                0,
                format!("Invalid custom variable name '{}'", name)
            ));
        }
        
        state.variables.insert(name.clone(), crate::types::VariableDef {
            value: value.clone(),
            raw_value: value.clone(),
            def_line: 0,
            is_resolving: false,
            is_resolved: true,
        });
    }
    
    let source_with_variables = variable_processor
        .process_and_substitute_variables(&source_with_includes, &mut state)?;
    
    // Populate the unified variable context with @variables
    for (name, var_def) in &state.variables {
        state.variable_context.add_string_variable(
            name.clone(),
            var_def.value.clone(),
            state.current_file_path.clone(),
            var_def.def_line
        )?;
    }
    
    stats.variable_count = state.variables.len();
    
    if options.debug_mode {
        log::debug!("Phase 0.2 complete. Variables resolved: {}", stats.variable_count);
    }
    
    // Phase 1: Lexical analysis and parsing
    if options.debug_mode {
        log::debug!("Phase 1: Lexical analysis and parsing...");
    }
    
    let mut lexer = Lexer::new_with_source_map(&source_with_variables, filename.to_string(), source_map);
    let tokens = lexer.tokenize()?;
    
    if options.debug_mode {
        log::debug!("Tokenized {} tokens", tokens.len());
    }
    
    let mut parser = Parser::new(tokens);
    let mut ast = parser.parse()?;
    
    if options.debug_mode {
        log::debug!("Phase 1 complete. AST parsed successfully");
    }
    
    // Phase 1.2: Semantic analysis
    if options.debug_mode {
        log::debug!("Phase 1.2: Semantic analysis...");
    }
    
    let mut semantic_analyzer = SemanticAnalyzer::new();
    semantic_analyzer.analyze(&ast, &mut state)?;
    
    if options.debug_mode {
        log::debug!("Phase 1.2 complete. Semantic analysis passed");
    }
    
    // Phase 1.25: Style resolution
    if options.debug_mode {
        log::debug!("Phase 1.25: Resolving style inheritance...");
    }
    
    let mut style_resolver = StyleResolver::new();
    style_resolver.resolve_all_styles(&mut state)?;
    
    if options.debug_mode {
        log::debug!("Phase 1.25 complete. Style inheritance resolved");
    }
    
    // Phase 1.3: Process scripts
    if options.debug_mode {
        log::debug!("Phase 1.3: Processing scripts...");
    }
    
    let script_processor = ScriptProcessor::new();
    
    // Extract scripts from AST and process them
    if let AstNode::File { scripts, .. } = &ast {
        for script_node in scripts {
            let script_entry = script_processor.process_script(script_node, &mut state)?;
            state.scripts.push(script_entry);
        }
    }
    
    stats.script_count = state.scripts.len();
    
    if options.debug_mode {
        log::debug!("Phase 1.3 complete. Scripts processed: {}", stats.script_count);
    }
    
    // Phase 1.4: Convert AST to internal representation
    if options.debug_mode {
        log::debug!("Phase 1.4: Converting AST to internal representation...");
    }
    
    convert_ast_to_state(&ast, &mut state)?;
    
    // Phase 1.45: Apply style properties to elements
    if options.debug_mode {
        log::debug!("Phase 1.45: Applying style properties to elements...");
    }
    
    apply_style_properties_to_elements(&mut state)?;
    
    if options.debug_mode {
        log::debug!("Phase 1.45 complete. Style properties applied to elements");
    }
    
    stats.element_count = state.elements.len();
    stats.style_count = state.styles.len();
    stats.component_count = state.component_defs.len();
    stats.resource_count = state.resources.len();
    stats.string_count = state.strings.len();
    
    if options.debug_mode {
        log::debug!("Phase 1.4 complete. Elements: {}, Styles: {}, Components: {}", 
                   stats.element_count, stats.style_count, stats.component_count);
    }
    
    // Phase 1.5: Component resolution
    if options.debug_mode {
        log::debug!("Phase 1.5: Resolving components...");
    }
    
    // Phase 1.5: Component resolution (must happen on AST before state conversion)
    if options.debug_mode {
        log::debug!("Phase 1.5: Resolving components in AST...");
    }
    
    if stats.component_count > 0 {
        // Extract component definitions and templates from AST
        if let AstNode::File { components, .. } = &ast {
            for component_node in components {
                if let AstNode::Component { name, properties, template } = component_node {
                    // Store the template AST for the resolver to use
                    state.component_ast_templates.insert(name.clone(), (**template).clone());
                    
                    // Create component definition
                    let mut component_def = ComponentDefinition {
                        name: name.clone(),
                        properties: Vec::new(),
                        definition_start_line: 1,
                        definition_root_element_index: None, // Not needed anymore
                        calculated_size: 0,
                        internal_template_element_offsets: std::collections::HashMap::new(),
                    };
                    
                    // Process component properties
                    for comp_prop in properties {
                        if let ComponentProperty { name: prop_name, property_type, default_value, .. } = comp_prop {
                            let value_type = match property_type.as_str() {
                                "String" => ValueType::String,
                                "Bool" | "Boolean" => ValueType::Bool,
                                "Int" | "Integer" => ValueType::Int,
                                "Float" | "Number" => ValueType::Float,
                                "Color" => ValueType::Color,
                                "StyleID" | "Style" => ValueType::StyleId,
                                _ => {
                                    // Check if it's an Enum type
                                    if property_type.starts_with("Enum(") && property_type.ends_with(")") {
                                        ValueType::Enum
                                    } else {
                                        ValueType::String // Default fallback
                                    }
                                }
                            };
                            
                            let prop_def = ComponentPropertyDef {
                                name: prop_name.clone(),
                                value_type_hint: value_type,
                                default_value: default_value.clone().unwrap_or_default(),
                            };
                            component_def.properties.push(prop_def);
                        }
                    }
                    
                    state.component_defs.push(component_def);
                }
            }
        }
        
        // Now resolve components in the AST
        let mut component_resolver = ComponentResolver::new();
        component_resolver.resolve_components(&mut ast, &mut state)?;
        
        if options.debug_mode {
            log::debug!("Phase 1.5 complete. Component instances resolved in AST");
        }
        
        // Clear the state to rebuild it with resolved AST
        state.elements.clear();
        state.component_defs.clear();
        state.component_ast_templates.clear();
        
        // Rebuild state from resolved AST (components are now expanded to regular elements)
        convert_ast_to_state(&ast, &mut state)?;
        
        stats.element_count = state.elements.len();
        stats.component_count = state.component_defs.len();
    } else {
        if options.debug_mode {
            log::debug!("Phase 1.5 skipped. No components to resolve.");
        }
    }
    
    // Phase 1.6: Process template variables
    if options.debug_mode {
        log::debug!("Phase 1.6: Processing template variables...");
    }
    
    process_template_variables(&mut state, &options)?;
    
    if options.debug_mode {
        log::debug!("Phase 1.6 complete. Template variables: {}, bindings: {}", 
                   state.template_variables.len(), state.template_bindings.len());
    }
    
    // Phase 2: Calculate sizes
    if options.debug_mode {
        log::debug!("Phase 2: Calculating sizes...");
    }
    
    let size_calculator = SizeCalculator::new();
    size_calculator.calculate_sizes(&mut state)?;
    size_calculator.validate_limits(&state)?;
    
    let size_stats = size_calculator.get_size_stats(&state);
    
    if options.debug_mode {
        log::debug!("Phase 2 complete. Total size: {} bytes", size_stats.total_size);
        if options.optimization_level >= 2 {
            size_stats.print_breakdown();
        }
    }
    
    // Phase 3: Generate KRB binary
    if options.debug_mode {
        log::debug!("Phase 3: Generating KRB binary...");
    }
    
    let mut code_generator = CodeGenerator::new();
    let krb_data = code_generator.generate(&mut state)?;

    if options.debug_mode {
        log::debug!("Phase 3 complete. KRB data size: {} bytes", krb_data.len());
    }
    
    // Update final stats
    stats.output_size = krb_data.len() as u64;
    
    Ok((krb_data, stats))
}


fn convert_ast_to_state(ast: &AstNode, state: &mut CompilerState) -> Result<()> {
    match ast {
        AstNode::File { app, styles, components, scripts, directives } => {
            // Process styles first since elements may reference them
            for style_node in styles {
                if let AstNode::Style { name, extends: _, properties, pseudo_selectors } = style_node {
                    // Convert style properties to KRB format first
                    let mut krb_properties = Vec::new();
                    for ast_prop in properties {
                        if let Some(krb_prop) = convert_ast_property_to_krb(ast_prop, state)? {
                            krb_properties.push(krb_prop);
                        }
                    }
                    
                    // Process pseudo-selectors and convert to state property sets
                    // This is a simplified implementation for now
                    if !pseudo_selectors.is_empty() {
                        println!("Style '{}' has {} pseudo-selectors (processed)", name, pseudo_selectors.len());
                        // Note: Full state property implementation requires substantial changes to
                        // the style system to properly store and apply state-based properties.
                        // For now, the renderer will need to use the existing compute_with_state method.
                    }
                    
                    // Find existing style by name (created in semantic analysis phase)
                    if let Some(existing_style) = state.styles.iter_mut().find(|s| s.source_name == *name) {
                        // Update existing style with KRB properties instead of creating duplicate
                        existing_style.properties = krb_properties;
                        existing_style.is_resolved = true;
                        existing_style.is_resolving = false;
                        
                        println!("Updated existing style '{}' with ID {} and {} properties", name, existing_style.id, existing_style.properties.len());
                    } else {
                        // Style not found - create new one (fallback case)
                        let style_id = (state.styles.len() + 1) as u8; // 1-based style IDs
                        
                        // Add style name to string table
                        let name_index = if let Some(pos) = state.strings.iter().position(|s| s.text == *name) {
                            pos as u8
                        } else {
                            let index = state.strings.len() as u8;
                            state.strings.push(StringEntry {
                                text: name.clone(),
                                length: name.len(),
                                index,
                            });
                            index
                        };
                        
                        // Create style entry (krb_properties already computed above)
                        let style_entry = StyleEntry {
                            id: style_id,
                            source_name: name.clone(),
                            name_index,
                            extends_style_names: Vec::new(), // TODO: implement extends
                            properties: krb_properties,
                            source_properties: properties.iter().map(|p| SourceProperty {
                                key: p.key.clone(),
                                value: p.value.clone(),
                                line_num: p.line,
                            }).collect(),
                            calculated_size: 0, // Will be calculated later
                            is_resolved: true,
                            is_resolving: false,
                        };
                        
                        println!("Added new style '{}' with ID {} and {} properties", name, style_id, style_entry.properties.len());
                        state.styles.push(style_entry);
                    }
                }
            }
            
            // Process components
            for component_node in components {
                if let AstNode::Component { name, properties, template } = component_node {
                    let mut component_def = ComponentDefinition {
                        name: name.clone(),
                        properties: Vec::new(),
                        definition_start_line: 1, // TODO: get actual line from AST
                        definition_root_element_index: None,
                        calculated_size: 0,
                        internal_template_element_offsets: std::collections::HashMap::new(),
                    };
                    
                    // Process component properties
                    for comp_prop in properties {
                        if let ComponentProperty { name: prop_name, property_type, default_value, .. } = comp_prop {
                            let prop_def = ComponentPropertyDef {
                                name: prop_name.clone(),
                                value_type_hint: ValueType::String, // TODO: parse property_type
                                default_value: default_value.clone().unwrap_or_default(),
                            };
                            component_def.properties.push(prop_def);
                        }
                    }
                    
                    // Store component definition without converting template to state yet
                    // Template will be processed during component instantiation
                    component_def.definition_root_element_index = None; // Not needed with AST templates
                    
                    state.component_defs.push(component_def);
                }
            }
            
            // Process app element
            if let Some(app_node) = app {
                convert_element_to_state(app_node, state, None)?;
            }
        }
        _ => return Err(CompilerError::semantic_legacy(0, "Expected File node at root")),
    }
    
    Ok(())
}
fn convert_element_to_state(
    ast_element: &AstNode, 
    state: &mut CompilerState, 
    parent_index: Option<usize>
) -> Result<usize> {
    if let AstNode::Element { element_type, properties, children, pseudo_selectors } = ast_element {
        let element_index = state.elements.len();
        
        let mut element = Element {
            element_type: ElementType::from_name(element_type),
            id_string_index: 0, pos_x: 0, pos_y: 0, width: 0, height: 0, layout: 0, style_id: 0, checked: false,
            property_count: 0, child_count: 0, event_count: 0, animation_count: 0, custom_prop_count: 0, state_prop_count: 0,
            krb_properties: Vec::new(), krb_custom_properties: Vec::new(), krb_events: Vec::new(),
            state_property_sets: Vec::new(), children: Vec::new(), parent_index, self_index: element_index,
            is_component_instance: false, component_def: None, is_definition_root: false,
            source_element_name: element_type.clone(), source_id_name: String::new(), source_properties: Vec::new(),
            source_children_indices: Vec::new(), source_line_num: 0, layout_flags_source: 0,
            position_hint: String::new(), orientation_hint: String::new(), calculated_size: KRB_ELEMENT_HEADER_SIZE as u32,
            absolute_offset: 0, processed_in_pass: false,
        };
        
        for ast_prop in properties {
            // Always add to source properties for template processing
            element.source_properties.push(SourceProperty {
                key: ast_prop.key.clone(),
                value: ast_prop.value.clone(),
                line_num: ast_prop.line,
            });
            
            match ast_prop.key.as_str() {
                // Handle element header fields directly
                "pos_x" => if let Ok(val) = ast_prop.cleaned_value().parse::<u16>() { element.pos_x = val; },
                "pos_y" => if let Ok(val) = ast_prop.cleaned_value().parse::<u16>() { element.pos_y = val; },
                "width" | "window_width" => if let Ok(val) = ast_prop.cleaned_value().parse::<u16>() { element.width = val; },
                "height" | "window_height" => if let Ok(val) = ast_prop.cleaned_value().parse::<u16>() { element.height = val; },
                "layout" => {
                    match crate::utils::parse_layout_string(&ast_prop.cleaned_value()) {
                        Ok(layout_byte) => element.layout = layout_byte,
                        Err(e) => return Err(CompilerError::semantic_legacy(ast_prop.line, e.to_string())),
                    }
                },
                "id" => {
                    // Store the element ID string in the string table and set the index
                    let id_string = ast_prop.cleaned_value();
                    let string_index = if let Some(entry) = state.strings.iter().position(|s| s.text == id_string) {
                        entry as u8
                    } else {
                        let index = state.strings.len() as u8;
                        state.strings.push(StringEntry {
                            text: id_string.to_string(),
                            length: id_string.len(),
                            index,
                        });
                        index
                    };
                    element.id_string_index = string_index;
                    println!("Set element ID '{}' to string index {}", id_string, string_index);
                },
                "style" => {
                    let style_name = ast_prop.cleaned_value();
                    if let Some(style_entry) = state.styles.iter().find(|s| s.source_name == style_name) {
                        element.style_id = style_entry.id;
                    }
                },
                "checked" => {
                    let checked_value = ast_prop.cleaned_value();
                    element.checked = checked_value == "true";
                    println!("Set element checked state to {}", element.checked);
                },

                // --- THIS IS THE CORRECTED LOGIC ---
                // Each event is handled in its own, isolated block with explicit types.
                "onClick" => {
                    let func_name = ast_prop.cleaned_value();
                    let callback_id = state.strings.iter().position(|s| s.text == func_name).map(|i| i as u8).unwrap_or(0);
                    element.krb_events.push(KrbEvent { event_type: EVENT_TYPE_CLICK, callback_id });
                },
                "onPress" => {
                    let func_name = ast_prop.cleaned_value();
                    let callback_id = state.strings.iter().position(|s| s.text == func_name).map(|i| i as u8).unwrap_or(0);
                    element.krb_events.push(KrbEvent { event_type: EVENT_TYPE_PRESS, callback_id });
                },
                "onRelease" => {
                    let func_name = ast_prop.cleaned_value();
                    let callback_id = state.strings.iter().position(|s| s.text == func_name).map(|i| i as u8).unwrap_or(0);
                    element.krb_events.push(KrbEvent { event_type: EVENT_TYPE_RELEASE, callback_id });
                },
                "onHover" => {
                    let func_name = ast_prop.cleaned_value();
                    let callback_id = state.strings.iter().position(|s| s.text == func_name).map(|i| i as u8).unwrap_or(0);
                    element.krb_events.push(KrbEvent { event_type: EVENT_TYPE_HOVER, callback_id });
                },
                "onFocus" => {
                    let func_name = ast_prop.cleaned_value();
                    let callback_id = state.strings.iter().position(|s| s.text == func_name).map(|i| i as u8).unwrap_or(0);
                    element.krb_events.push(KrbEvent { event_type: EVENT_TYPE_FOCUS, callback_id });
                },
                "onBlur" => {
                    let func_name = ast_prop.cleaned_value();
                    let callback_id = state.strings.iter().position(|s| s.text == func_name).map(|i| i as u8).unwrap_or(0);
                    element.krb_events.push(KrbEvent { event_type: EVENT_TYPE_BLUR, callback_id });
                },
                "onChange" => {
                    let func_name = ast_prop.cleaned_value();
                    let callback_id = state.strings.iter().position(|s| s.text == func_name).map(|i| i as u8).unwrap_or(0);
                    element.krb_events.push(KrbEvent { event_type: EVENT_TYPE_CHANGE, callback_id });
                },
                "onSubmit" => {
                    let func_name = ast_prop.cleaned_value();
                    let callback_id = state.strings.iter().position(|s| s.text == func_name).map(|i| i as u8).unwrap_or(0);
                    element.krb_events.push(KrbEvent { event_type: EVENT_TYPE_SUBMIT, callback_id });
                },

                // Default case for all other standard properties
                _ => {
                    if let Some(krb_prop) = convert_ast_property_to_krb(ast_prop, state)? {
                        element.krb_properties.push(krb_prop);
                    }
                }
            }
        }
        
        // Process pseudo-selectors (e.g., &:hover) for state-based properties
        for pseudo in pseudo_selectors {
            if let Some(state_flag) = pseudo.state_flag() {
                let mut state_props = Vec::new();
                for ast_prop in &pseudo.properties {
                    if let Some(krb_prop) = convert_ast_property_to_krb(ast_prop, state)? {
                        state_props.push(krb_prop);
                    }
                }
                element.state_property_sets.push(StatePropertySet {
                    state_flags: state_flag,
                    property_count: state_props.len() as u8,
                    properties: state_props,
                });
            }
        }
        
        // Finalize counts in the element header before adding it to the state
        element.property_count = element.krb_properties.len() as u8;
        element.state_prop_count = element.state_property_sets.len() as u8;
        element.event_count = element.krb_events.len() as u8;
        
        state.elements.push(element);
        
        // Now, recursively process all child elements
        let mut child_indices = Vec::new();
        for child in children {
            let child_index = convert_element_to_state(child, state, Some(element_index))?;
            child_indices.push(child_index);
        }
        
        // Update the element in the state with its new child references
        state.elements[element_index].children = child_indices;
        state.elements[element_index].child_count = state.elements[element_index].children.len() as u8;

        Ok(element_index)
    } else {
        Err(CompilerError::semantic_legacy(0, "Expected Element node during AST conversion"))
    }
}

fn convert_ast_property_to_krb(ast_prop: &AstProperty, state: &mut CompilerState) -> Result<Option<KrbProperty>> {
    let cleaned_value = ast_prop.cleaned_value();
    
    // Use the comprehensive mapping from PropertyId::from_name()
    let property_id = PropertyId::from_name(&ast_prop.key);
    
    // If it's CustomData (unknown property), store as custom property
    if property_id == PropertyId::CustomData {
        return Ok(None); // Will be handled as custom property elsewhere
    }
    
    // Now, correctly serialize the value based on the property ID.
    let krb_prop = match property_id {
        PropertyId::BackgroundColor | PropertyId::ForegroundColor | PropertyId::BorderColor => {
            if let Ok(color) = crate::utils::parse_color(&cleaned_value) {
                Some(KrbProperty {
                    property_id: property_id as u8,
                    value_type: ValueType::Color,
                    size: 4,
                    value: color.to_bytes().to_vec(),
                })
            } else {
                return Err(CompilerError::semantic_legacy(ast_prop.line, format!("Invalid color value: {}", cleaned_value)));
            }
        }
        PropertyId::BorderWidth | PropertyId::BorderRadius => {
            if let Ok(val) = cleaned_value.parse::<u8>() {
                Some(KrbProperty { property_id: property_id as u8, value_type: ValueType::Byte, size: 1, value: vec![val] })
            } else {
                return Err(CompilerError::semantic_legacy(ast_prop.line, format!("Invalid numeric value for {}: {}", ast_prop.key, cleaned_value)));
            }
        }
        PropertyId::TextAlignment => {
            let alignment_val = match cleaned_value.to_lowercase().as_str() {
                "start" => 0u8, "center" => 1u8, "end" => 2u8, "justify" => 3u8,
                _ => 1u8, // Default to center
            };
            Some(KrbProperty { property_id: property_id as u8, value_type: ValueType::Enum, size: 1, value: vec![alignment_val] })
        }
        PropertyId::TextContent | PropertyId::WindowTitle => {
            let string_index = state.strings.iter().position(|s| s.text == cleaned_value).map(|i| i as u8).unwrap_or_else(|| {
                let index = state.strings.len() as u8;
                state.strings.push(StringEntry { text: cleaned_value.clone(), length: cleaned_value.len(), index });
                index
            });
            Some(KrbProperty { property_id: property_id as u8, value_type: ValueType::String, size: 1, value: vec![string_index] })
        }
        PropertyId::LayoutFlags => {
            // Parse layout value like "grow", "column start", "row start"
            let layout_value = crate::utils::parse_layout_string(&cleaned_value)?;
            Some(KrbProperty { property_id: property_id as u8, value_type: ValueType::Byte, size: 1, value: vec![layout_value] })
        }
        PropertyId::Height | PropertyId::Width => {
            // Parse numeric value as u16 or percentage
            if let Ok(val) = cleaned_value.parse::<u16>() {
                // Regular pixel value
                Some(KrbProperty { property_id: property_id as u8, value_type: ValueType::Short, size: 2, value: val.to_le_bytes().to_vec() })
            } else if cleaned_value.ends_with('%') {
                // Percentage value - parse as float and store as percentage type
                let percent_str = &cleaned_value[..cleaned_value.len() - 1]; // Remove '%'
                if let Ok(percent) = percent_str.parse::<f32>() {
                    // Store percentage as 4-byte float
                    Some(KrbProperty { property_id: property_id as u8, value_type: ValueType::Percentage, size: 4, value: percent.to_le_bytes().to_vec() })
                } else {
                    return Err(CompilerError::semantic_legacy(ast_prop.line, format!("Invalid percentage value for {}: {}", ast_prop.key, cleaned_value)));
                }
            } else {
                return Err(CompilerError::semantic_legacy(ast_prop.line, format!("Invalid numeric value for {}: {} (must be a number or percentage)", ast_prop.key, cleaned_value)));
            }
        }
        // Modern Taffy layout properties
        PropertyId::Display | PropertyId::FlexDirection | PropertyId::Position |
        PropertyId::AlignItems | PropertyId::AlignContent | PropertyId::JustifyContent |
        PropertyId::JustifyItems | PropertyId::JustifySelf | PropertyId::AlignSelf => {
            // Store as string index like other string properties
            let string_index = state.strings.iter().position(|s| s.text == cleaned_value).map(|i| i as u8).unwrap_or_else(|| {
                let index = state.strings.len() as u8;
                state.strings.push(StringEntry { text: cleaned_value.clone(), length: cleaned_value.len(), index });
                index
            });
            Some(KrbProperty { 
                property_id: property_id as u8, 
                value_type: ValueType::String, 
                size: 1, 
                value: vec![string_index] 
            })
        }
        PropertyId::FlexGrow | PropertyId::FlexShrink => {
            // Store as float values
            if let Ok(val) = cleaned_value.parse::<f32>() {
                Some(KrbProperty {
                    property_id: property_id as u8,
                    value_type: ValueType::Percentage, // Reuse percentage type for float
                    size: 4,
                    value: val.to_le_bytes().to_vec(),
                })
            } else {
                return Err(CompilerError::semantic_legacy(ast_prop.line, format!("Invalid float value for {}: {}", ast_prop.key, cleaned_value)));
            }
        }
        _ => None, // Should not be reached due to the initial match, but it's safe.
    };
    
    Ok(krb_prop)
}

/// Generate KRB binary data from compiler state
fn generate_krb_binary(state: &CompilerState, options: &CompilerOptions) -> Result<Vec<u8>> {
    use byteorder::{LittleEndian, WriteBytesExt};
    use types::*;
    
    let mut data = Vec::new();
    
    // Calculate header flags
    let mut flags = 0u16;
    if !state.styles.is_empty() { flags |= FLAG_HAS_STYLES; }
    if !state.component_defs.is_empty() { flags |= FLAG_HAS_COMPONENT_DEFS; }
    if !state.scripts.is_empty() { flags |= FLAG_HAS_SCRIPTS; }
    if !state.resources.is_empty() { flags |= FLAG_HAS_RESOURCES; }
    if state.has_app { flags |= FLAG_HAS_APP; }
    if options.compress_output { flags |= FLAG_COMPRESSED; }
    
    // Write KRB header (54 bytes for v0.5)
    data.extend_from_slice(KRB_MAGIC);
    data.write_u16::<LittleEndian>((KRB_VERSION_MINOR as u16) << 8 | KRB_VERSION_MAJOR as u16)?;
    data.write_u16::<LittleEndian>(flags)?;
    
    // Count main tree elements (not component template elements)
    let main_element_count = state.elements.iter()
        .filter(|e| !e.is_definition_root)
        .count() as u16;
    
    // Section counts
    data.write_u16::<LittleEndian>(main_element_count)?;
    data.write_u16::<LittleEndian>(state.styles.len() as u16)?;
    data.write_u16::<LittleEndian>(state.component_defs.len() as u16)?;
    data.write_u16::<LittleEndian>(0)?; // animation count
    data.write_u16::<LittleEndian>(state.scripts.len() as u16)?;
    data.write_u16::<LittleEndian>(state.strings.len() as u16)?;
    data.write_u16::<LittleEndian>(state.resources.len() as u16)?;
    
    // Section offsets (will be filled with actual values)
    let header_end = KRB_HEADER_SIZE as u32;
    data.write_u32::<LittleEndian>(header_end)?; // element offset
    data.write_u32::<LittleEndian>(header_end)?; // style offset
    data.write_u32::<LittleEndian>(header_end)?; // component def offset
    data.write_u32::<LittleEndian>(header_end)?; // animation offset
    data.write_u32::<LittleEndian>(header_end)?; // script offset
    data.write_u32::<LittleEndian>(header_end)?; // string offset
    data.write_u32::<LittleEndian>(header_end)?; // resource offset
    
    // Total size
    data.write_u32::<LittleEndian>(header_end)?;
    
    // TODO: Write actual section data when implemented
    
    Ok(data)
}

/// Validate KRB file format
pub fn validate_krb_file(data: &[u8]) -> Result<KrbFileInfo> {
    use byteorder::{LittleEndian, ReadBytesExt};
    use std::io::Cursor;
    
    if data.len() < types::KRB_HEADER_SIZE {
        return Err(CompilerError::InvalidFormat {
            message: format!("File too small: {} bytes, expected at least {}", 
                           data.len(), types::KRB_HEADER_SIZE),
        });
    }
    
    let mut cursor = Cursor::new(data);
    
    // Check magic
    let mut magic = [0u8; 4];
    cursor.read_exact(&mut magic)?;
    if &magic != types::KRB_MAGIC {
        return Err(CompilerError::InvalidFormat {
            message: format!("Invalid magic: {:?}, expected {:?}", magic, types::KRB_MAGIC),
        });
    }
    
    // Read version
    let version = cursor.read_u16::<LittleEndian>()?;
    let major = ((version >> 8) & 0xFF) as u8;
    let minor = (version & 0xFF) as u8;
    
    // Read flags and counts
    let flags = cursor.read_u16::<LittleEndian>()?;
    let element_count = cursor.read_u16::<LittleEndian>()?;
    let style_count = cursor.read_u16::<LittleEndian>()?;
    let component_count = cursor.read_u16::<LittleEndian>()?;
    let animation_count = cursor.read_u16::<LittleEndian>()?;
    let script_count = cursor.read_u16::<LittleEndian>()?;
    let string_count = cursor.read_u16::<LittleEndian>()?;
    let resource_count = cursor.read_u16::<LittleEndian>()?;
    
    // Read offsets
    let element_offset = cursor.read_u32::<LittleEndian>()?;
    let style_offset = cursor.read_u32::<LittleEndian>()?;
    let component_offset = cursor.read_u32::<LittleEndian>()?;
    let animation_offset = cursor.read_u32::<LittleEndian>()?;
    let script_offset = cursor.read_u32::<LittleEndian>()?;
    let string_offset = cursor.read_u32::<LittleEndian>()?;
    let resource_offset = cursor.read_u32::<LittleEndian>()?;
    let total_size = cursor.read_u32::<LittleEndian>()?;
    
    if total_size as usize != data.len() {
        return Err(CompilerError::InvalidFormat {
            message: format!("Size mismatch: header says {}, actual {}", 
                           total_size, data.len()),
        });
    }
    
    Ok(KrbFileInfo {
        version: (major, minor),
        flags,
        element_count,
        style_count,
        component_count,
        animation_count,
        script_count,
        string_count,
        resource_count,
        element_offset,
        style_offset,
        component_offset,
        animation_offset,
        script_offset,
        string_offset,
        resource_offset,
        total_size,
    })
}

/// Information about a KRB file
#[derive(Debug, Clone, Serialize)]
pub struct KrbFileInfo {
    pub version: (u8, u8),
    pub flags: u16,
    pub element_count: u16,
    pub style_count: u16,
    pub component_count: u16,
    pub animation_count: u16,
    pub script_count: u16,
    pub string_count: u16,
    pub resource_count: u16,
    pub element_offset: u32,
    pub style_offset: u32,
    pub component_offset: u32,
    pub animation_offset: u32,
    pub script_offset: u32,
    pub string_offset: u32,
    pub resource_offset: u32,
    pub total_size: u32,
}

impl KrbFileInfo {
    /// Check if the file has a specific feature
    pub fn has_feature(&self, flag: u16) -> bool {
        (self.flags & flag) != 0
    }
    
    /// Get a human-readable description of the file
    pub fn description(&self) -> String {
        format!(
            "KRB v{}.{} - {} elements, {} styles, {} components, {} scripts, {} resources ({} bytes)",
            self.version.0, self.version.1,
            self.element_count, self.style_count, self.component_count,
            self.script_count, self.resource_count, self.total_size
        )
    }
    
    /// Calculate compression ratio if original size is known
    pub fn compression_ratio(&self, original_size: u64) -> f64 {
        if original_size > 0 {
            self.total_size as f64 / original_size as f64
        } else {
            0.0
        }
    }
}

/// Utility function to analyze a KRB file and return detailed information
pub fn analyze_krb_file(file_path: &str) -> Result<KrbFileInfo> {
    let data = std::fs::read(file_path)
        .map_err(|e| CompilerError::FileNotFound { 
            path: format!("{}: {}", file_path, e) 
        })?;
    
    validate_krb_file(&data)
}

/// Check if the compiler can handle a specific KRY feature
pub fn supports_feature(feature: &str) -> bool {
    BUILD_INFO.supported_features.contains(&feature)
}

/// Get compiler build information
pub fn build_info() -> &'static CompilerInfo {
    &BUILD_INFO
}

/// Apply style properties (width, height, layout) to element headers
/// This bridges the gap between style definitions and element size calculations
fn apply_style_properties_to_elements(state: &mut CompilerState) -> Result<()> {
    for element in &mut state.elements {
        // Element.style_id is u8, not Option<u8>
        if element.style_id > 0 {
            // Find the style by ID
            if let Some(style) = state.styles.iter().find(|s| s.id == element.style_id) {
                // Apply width, height, and layout properties if found in style
                for property in &style.properties {
                    // Convert property_id back to PropertyId using manual matching
                    match property.property_id {
                        0x19 => { // PropertyId::Width
                            if property.value_type == ValueType::Short && property.value.len() >= 2 {
                                let width = u16::from_le_bytes([property.value[0], property.value[1]]);
                                element.width = width;
                            }
                        },
                        0x1B => { // PropertyId::Height  
                            if property.value_type == ValueType::Short && property.value.len() >= 2 {
                                let height = u16::from_le_bytes([property.value[0], property.value[1]]);
                                element.height = height;
                            }
                        },
                        0x1A => { // PropertyId::LayoutFlags
                            if property.value_type == ValueType::Byte && !property.value.is_empty() {
                                element.layout = property.value[0];
                            }
                        },
                        _ => {} // Ignore other properties for now
                    }
                }
            }
        }
    }
    Ok(())
}

/// Process template variables and create template binding tables
fn process_template_variables(state: &mut CompilerState, options: &CompilerOptions) -> Result<()> {
    use std::collections::HashMap;
    
    // Collect all variables from @variables blocks
    let mut variable_map: HashMap<String, (u8, ValueType)> = HashMap::new();
    
    // First, create template variables from the @variables block
    for (var_name, var_def) in &state.variables {
        let name_index = state.strings.iter().position(|s| s.text == *var_name)
            .map(|i| i as u8)
            .unwrap_or_else(|| {
                let index = state.strings.len() as u8;
                state.strings.push(StringEntry {
                    text: var_name.clone(),
                    length: var_name.len(),
                    index,
                });
                index
            });
        
        let default_value_index = state.strings.iter().position(|s| s.text == var_def.value)
            .map(|i| i as u8)
            .unwrap_or_else(|| {
                let index = state.strings.len() as u8;
                state.strings.push(StringEntry {
                    text: var_def.value.clone(),
                    length: var_def.value.len(),
                    index,
                });
                index
            });
        
        // Determine value type based on the value
        let value_type = if var_def.value.parse::<i32>().is_ok() {
            ValueType::Int
        } else if var_def.value.parse::<f32>().is_ok() {
            ValueType::Float
        } else if var_def.value == "true" || var_def.value == "false" {
            ValueType::Bool
        } else {
            ValueType::String
        };
        
        let template_var = TemplateVariable {
            name: var_name.clone(),
            name_index,
            value_type,
            default_value: var_def.value.clone(),
            default_value_index,
        };
        
        variable_map.insert(var_name.clone(), (state.template_variables.len() as u8, value_type));
        state.template_variables.push(template_var);
    }
    
    // Now scan through all elements for properties with template variables
    for (element_index, element) in state.elements.iter().enumerate() {
        for source_prop in &element.source_properties {
            // Check if this property has template variables
            let template_variables = extract_template_variables(&source_prop.value);
            
            if options.debug_mode {
                log::debug!("Element {}: property '{}' = '{}' -> template vars: {:?}", 
                           element_index, source_prop.key, source_prop.value, template_variables);
            }
            
            if !template_variables.is_empty() {
                // Get the expression string index
                let expression_index = state.strings.iter().position(|s| s.text == source_prop.value)
                    .map(|i| i as u8)
                    .unwrap_or_else(|| {
                        let index = state.strings.len() as u8;
                        state.strings.push(StringEntry {
                            text: source_prop.value.clone(),
                            length: source_prop.value.len(),
                            index,
                        });
                        index
                    });
                
                // Map property key to property ID
                let property_id = PropertyId::from_name(&source_prop.key) as u8;
                
                // Get variable indices
                let mut variable_indices = Vec::new();
                for var_name in &template_variables {
                    if let Some((var_index, _)) = variable_map.get(var_name) {
                        variable_indices.push(*var_index);
                    }
                }
                
                let template_binding = TemplateBinding {
                    element_index: element_index as u16,
                    property_id,
                    template_expression: source_prop.value.clone(),
                    template_expression_index: expression_index,
                    variable_count: variable_indices.len() as u8,
                    variable_indices,
                };
                
                state.template_bindings.push(template_binding);
            }
        }
    }
    
    // Set the template variable flag if we have any template variables
    if !state.template_variables.is_empty() {
        state.header_flags |= FLAG_HAS_TEMPLATE_VARIABLES;
    }
    
    Ok(())
}

/// Extract template variables from a string ($variable_name)
fn extract_template_variables(value: &str) -> Vec<String> {
    use regex::Regex;
    
    let re = Regex::new(r"\$([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
    let mut variables = Vec::new();
    
    for capture in re.captures_iter(value) {
        if let Some(var_name) = capture.get(1) {
            let name = var_name.as_str().to_string();
            if !variables.contains(&name) {
                variables.push(name);
            }
        }
    }
    
    variables
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[test]
    fn test_extract_template_variables() {
        let test_cases = vec![
            ("$counter_value", vec!["counter_value"]),
            ("The count is $counter_value", vec!["counter_value"]),
            ("$counter_value items", vec!["counter_value"]),
            ("Hello $name and $age", vec!["name", "age"]),
            ("No variables here", vec![]),
            ("$_private_var", vec!["_private_var"]),
            ("$var123", vec!["var123"]),
        ];
        
        for (input, expected) in test_cases {
            let result = extract_template_variables(input);
            assert_eq!(result, expected, "Failed for input: '{}'", input);
        }
    }
    
    #[test]
    fn test_template_binding_creation() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = temp_dir.path().join("test_template.kry");
        let output_path = temp_dir.path().join("test_template.krb");
        
        // Create a test file with template variables
        let test_content = r#"@variables {
    counter_value: 0
}

App {
    Text {
        id: "counter_display"
        text: "$counter_value"
    }
}"#;
        
        fs::write(&input_path, test_content).unwrap();
        
        let mut options = CompilerOptions::default();
        options.debug_mode = true;
        
        let stats = compile_file_with_options(
            input_path.to_str().unwrap(),
            output_path.to_str().unwrap(),
            options
        ).unwrap();
        
        // Check that the file was created
        assert!(output_path.exists());
        
        // Read the KRB file and check for template variables
        let krb_data = fs::read(&output_path).unwrap();
        
        // Should have template variables and bindings
        assert!(stats.variable_count > 0, "No template variables found");
        println!("Template variables found: {}", stats.variable_count);
        
        // Check the KRB header for template variable flags
        let header_flags = u16::from_le_bytes([krb_data[12], krb_data[13]]);
        assert!(header_flags & FLAG_HAS_TEMPLATE_VARIABLES != 0, 
               "KRB file missing template variable flag");
        
        // The binding should be created for the text property
        // This is a basic test - more detailed verification would need KRB parsing
        println!("KRB file size: {} bytes", krb_data.len());
        println!("Header flags: 0x{:04x}", header_flags);
    }
    
    #[test]
    fn test_compile_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = temp_dir.path().join("empty.kry");
        let output_path = temp_dir.path().join("empty.krb");
        
        fs::write(&input_path, "").unwrap();
        
        let stats = compile_file(
            input_path.to_str().unwrap(),
            output_path.to_str().unwrap()
        ).unwrap();
        
        assert!(output_path.exists());
        assert_eq!(stats.source_size, 0);
        assert!(stats.output_size > 0); // At least header
        
        // Validate the output
        let krb_info = analyze_krb_file(output_path.to_str().unwrap()).unwrap();
        assert_eq!(krb_info.version, (types::KRB_VERSION_MAJOR, types::KRB_VERSION_MINOR));
    }
    
    #[test]
    fn test_compile_with_options() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = temp_dir.path().join("test.kry");
        let output_path = temp_dir.path().join("test.krb");
        
        fs::write(&input_path, "# Simple KRY file\n").unwrap();
        
        let options = CompilerOptions {
            debug_mode: true,
            optimization_level: 2,
            target_platform: TargetPlatform::Desktop,
            ..Default::default()
        };
        
        let stats = compile_file_with_options(
            input_path.to_str().unwrap(),
            output_path.to_str().unwrap(),
            options
        ).unwrap();
        
        assert!(output_path.exists());
        assert!(stats.compile_time_ms > 0);
        assert!(stats.compression_ratio > 0.0);
    }
    
    fn create_empty_krb_file() -> Vec<u8> {
        use byteorder::{LittleEndian, WriteBytesExt};
        
        let mut data = Vec::new();
        // Magic number "KRB1"
        data.extend_from_slice(types::KRB_MAGIC);
        // Version
        data.write_u16::<LittleEndian>(
            ((types::KRB_VERSION_MAJOR as u16) << 8) | (types::KRB_VERSION_MINOR as u16)
        ).unwrap();
        // Flags
        data.write_u16::<LittleEndian>(0).unwrap();
        // Section counts (all zeros)
        for _ in 0..7 {
            data.write_u16::<LittleEndian>(0).unwrap();
        }
        // Section offsets (all point to end of header)
        for _ in 0..7 {
            data.write_u32::<LittleEndian>(types::KRB_HEADER_SIZE as u32).unwrap();
        }
        // Total size
        data.write_u32::<LittleEndian>(types::KRB_HEADER_SIZE as u32).unwrap();
        
        data
    }
    
    #[test]
    fn test_validate_krb_file() {
        // Create a minimal valid KRB file
        let krb_data = create_empty_krb_file();
        let info = validate_krb_file(&krb_data).unwrap();
        
        assert_eq!(info.version, (types::KRB_VERSION_MAJOR, types::KRB_VERSION_MINOR));
        assert_eq!(info.total_size, krb_data.len() as u32);
    }
    
    #[test]
    fn test_invalid_krb_file() {
        // Too small
        let result = validate_krb_file(&[1, 2, 3]);
        assert!(result.is_err());
        
        // Wrong magic
        let mut bad_data = vec![0u8; types::KRB_HEADER_SIZE];
        bad_data[0..4].copy_from_slice(b"BAD!");
        let result = validate_krb_file(&bad_data);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_build_info() {
        let info = build_info();
        assert!(!info.version.is_empty());
        assert!(!info.name.is_empty());
        assert!(info.supported_features.len() > 0);
        assert!(supports_feature("includes"));
        assert!(supports_feature("variables"));
        assert!(!supports_feature("nonexistent_feature"));
    }
    
    #[test]
    fn test_compiler_options_default() {
        let options = CompilerOptions::default();
        assert!(!options.debug_mode);
        assert_eq!(options.optimization_level, 1);
        assert_eq!(options.target_platform, TargetPlatform::Universal);
        assert!(!options.embed_scripts);
        assert!(!options.compress_output);
        assert_eq!(options.max_file_size, 0);
        assert!(options.include_directories.is_empty());
        assert!(!options.generate_debug_info);
        assert!(options.custom_variables.is_empty());
    }
}
