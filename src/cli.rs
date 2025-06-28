//! Enhanced command-line interface for the Kryon compiler

use crate::error::{CompilerError, Result};
use crate::types::*;
use crate::{compile_file_with_options, CompilerOptions, TargetPlatform};
use clap::{Arg, ArgAction, Command, ValueEnum};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::{Instant, Duration};
use notify::{RecommendedWatcher, Watcher, RecursiveMode, Event};
use std::io::Write;

#[derive(Debug, Clone, ValueEnum)]
enum OptimizationLevel {
    None,
    Basic,
    Aggressive,
}

#[derive(Debug, Clone, ValueEnum)]
enum Platform {
    Desktop,
    Mobile,
    Web,
    Embedded,
    Universal,
}

#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    Krb,
    Json,     // For debugging - output AST as JSON
    Debug,    // Human-readable debug format
}

#[derive(Debug, Serialize, Deserialize)]
struct ConfigFile {
    optimization_level: Option<u8>,
    target_platform: Option<String>,
    embed_scripts: Option<bool>,
    compress_output: Option<bool>,
    include_directories: Option<Vec<String>>,
    custom_variables: Option<HashMap<String, String>>,
    max_file_size: Option<u64>,
    output_directory: Option<String>,
}

pub struct EnhancedCli {
    config: ConfigFile,
    start_time: Instant,
}

impl EnhancedCli {
    pub fn new() -> Self {
        Self {
            config: ConfigFile {
                optimization_level: None,
                target_platform: None,
                embed_scripts: None,
                compress_output: None,
                include_directories: None,
                custom_variables: None,
                max_file_size: None,
                output_directory: None,
            },
            start_time: Instant::now(),
        }
    }
    
    pub fn run(&mut self) -> Result<()> {
        self.start_time = Instant::now();
        
        let matches = self.build_cli().get_matches();
        
        // Load config file if specified
        if let Some(config_path) = matches.get_one::<String>("config") {
            self.load_config_file(config_path)?;
        }
        
        // Set up logging
        let verbose = matches.get_count("verbose");
        self.setup_logging(verbose)?;
        
        match matches.subcommand() {
            Some(("compile", sub_matches)) => {
                self.handle_compile_command(sub_matches)
            }
            Some(("check", sub_matches)) => {
                self.handle_check_command(sub_matches)
            }
            Some(("analyze", sub_matches)) => {
                self.handle_analyze_command(sub_matches)
            }
            Some(("init", sub_matches)) => {
                self.handle_init_command(sub_matches)
            }
            Some(("benchmark", sub_matches)) => {
                self.handle_benchmark_command(sub_matches)
            }
            _ => {
                println!("No subcommand specified. Use --help for usage information.");
                Ok(())
            }
        }
    }
    
    fn build_cli(&self) -> Command {
        Command::new(crate::NAME)
            .version(crate::VERSION)
            .about(crate::DESCRIPTION)
            .author("Kryon Development Team")
            .arg(
                Arg::new("config")
                    .short('c')
                    .long("config")
                    .value_name("FILE")
                    .help("Configuration file path")
                    .action(ArgAction::Set)
            )
            .arg(
                Arg::new("verbose")
                    .short('v')
                    .long("verbose")
                    .help("Increase verbosity (can be used multiple times)")
                    .action(ArgAction::Count)
            )
            .subcommand(
                Command::new("compile")
                    .about("Compile KRY files to KRB format")
                    .arg(
                        Arg::new("input")
                            .help("Input KRY file")
                            .required(true)
                            .index(1)
                    )
                    .arg(
                        Arg::new("output")
                            .short('o')
                            .long("output")
                            .value_name("FILE")
                            .help("Output KRB file")
                    )
                    .arg(
                        Arg::new("optimization")
                            .short('O')
                            .long("optimization")
                            .value_parser(clap::value_parser!(OptimizationLevel))
                            .default_value("basic")
                            .help("Optimization level")
                    )
                    .arg(
                        Arg::new("platform")
                            .short('p')
                            .long("platform")
                            .value_parser(clap::value_parser!(Platform))
                            .default_value("universal")
                            .help("Target platform")
                    )
                    .arg(
                        Arg::new("format")
                            .short('f')
                            .long("format")
                            .value_parser(clap::value_parser!(OutputFormat))
                            .default_value("krb")
                            .help("Output format")
                    )
                    .arg(
                        Arg::new("embed-scripts")
                            .long("embed-scripts")
                            .help("Embed scripts inline instead of external references")
                            .action(ArgAction::SetTrue)
                    )
                    .arg(
                        Arg::new("compress")
                            .long("compress")
                            .help("Enable output compression")
                            .action(ArgAction::SetTrue)
                    )
                    .arg(
                        Arg::new("debug")
                            .short('d')
                            .long("debug")
                            .help("Enable debug mode with extra validation")
                            .action(ArgAction::SetTrue)
                    )
                    .arg(
                        Arg::new("include")
                            .short('I')
                            .long("include")
                            .value_name("DIR")
                            .help("Add include directory")
                            .action(ArgAction::Append)
                    )
                    .arg(
                        Arg::new("define")
                            .short('D')
                            .long("define")
                            .value_name("VAR=VALUE")
                            .help("Define custom variable")
                            .action(ArgAction::Append)
                    )
                    .arg(
                        Arg::new("stats")
                            .long("stats")
                            .help("Show detailed compilation statistics")
                            .action(ArgAction::SetTrue)
                    )
                    .arg(
                        Arg::new("watch")
                            .short('w')
                            .long("watch")
                            .help("Watch for file changes and recompile")
                            .action(ArgAction::SetTrue)
                    )
            )
            .subcommand(
                Command::new("check")
                    .about("Check KRY files for syntax and semantic errors")
                    .arg(
                        Arg::new("input")
                            .help("Input KRY file or directory")
                            .required(true)
                            .index(1)
                    )
                    .arg(
                        Arg::new("recursive")
                            .short('r')
                            .long("recursive")
                            .help("Check all KRY files in directory recursively")
                            .action(ArgAction::SetTrue)
                    )
            )
            .subcommand(
                Command::new("analyze")
                    .about("Analyze KRY/KRB files and show detailed information")
                    .arg(
                        Arg::new("input")
                            .help("Input KRY or KRB file")
                            .required(true)
                            .index(1)
                    )
                    .arg(
                        Arg::new("output")
                            .short('o')
                            .long("output")
                            .value_name("FILE")
                            .help("Output analysis to file")
                    )
                    .arg(
                        Arg::new("format")
                            .short('f')
                            .long("format")
                            .value_parser(clap::value_parser!(OutputFormat))
                            .default_value("debug")
                            .help("Analysis output format")
                    )
            )
            .subcommand(
                Command::new("init")
                    .about("Initialize a new Kryon project")
                    .arg(
                        Arg::new("name")
                            .help("Project name")
                            .required(true)
                            .index(1)
                    )
                    .arg(
                        Arg::new("template")
                            .short('t')
                            .long("template")
                            .value_name("TEMPLATE")
                            .help("Project template (simple, component, game)")
                            .default_value("simple")
                    )
            )
            .subcommand(
                Command::new("benchmark")
                    .about("Run compilation benchmarks")
                    .arg(
                        Arg::new("input")
                            .help("Input KRY file or directory")
                            .required(true)
                            .index(1)
                    )
                    .arg(
                        Arg::new("iterations")
                            .short('n')
                            .long("iterations")
                            .value_name("N")
                            .help("Number of benchmark iterations")
                            .default_value("10")
                    )
                    .arg(
                        Arg::new("warmup")
                            .long("warmup")
                            .value_name("N")
                            .help("Number of warmup iterations")
                            .default_value("3")
                    )
            )
    }
    
    fn setup_logging(&self, verbose_count: u8) -> Result<()> {
        let log_level = match verbose_count {
            0 => log::LevelFilter::Warn,
            1 => log::LevelFilter::Info,
            2 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        };
        
        env_logger::Builder::from_default_env()
            .filter_level(log_level)
            .format_timestamp_secs()
            .init();
        
        Ok(())
    }
    
    fn load_config_file(&mut self, config_path: &str) -> Result<()> {
        let config_content = fs::read_to_string(config_path)
            .map_err(|e| CompilerError::FileNotFound {
                path: format!("Config file {}: {}", config_path, e),
            })?;
        
        self.config = if config_path.ends_with(".json") {
            serde_json::from_str(&config_content)
                .map_err(|e| CompilerError::InvalidFormat {
                    message: format!("Invalid JSON config: {}", e),
                })?
        } else if config_path.ends_with(".toml") {
            toml::from_str(&config_content)
                .map_err(|e| CompilerError::InvalidFormat {
                    message: format!("Invalid TOML config: {}", e),
                })?
        } else {
            return Err(CompilerError::InvalidFormat {
                message: "Config file must be .json or .toml format".to_string(),
            });
        };
        
        log::info!("Loaded configuration from {}", config_path);
        Ok(())
    }
    
    fn handle_compile_command(&self, matches: &clap::ArgMatches) -> Result<()> {
        let input_path = matches.get_one::<String>("input").unwrap();
        
        let output_path = matches.get_one::<String>("output")
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                Path::new(input_path)
                    .with_extension("krb")
                    .to_string_lossy()
                    .into_owned()
            });

        let options = self.build_compiler_options(matches)?;

        if matches.get_flag("watch") {
            self.watch_and_compile(input_path, &output_path, options)
        } else {
            self.compile_single_file(input_path, &output_path, options, matches)
        }
    }
    
    fn compile_single_file(
        &self, 
        input_path: &str, 
        output_path: &str, 
        options: CompilerOptions,
        matches: &clap::ArgMatches
    ) -> Result<()> {
        println!("🔨 Compiling {} -> {}", input_path, output_path);
        
        let compile_start = Instant::now();
        let stats = compile_file_with_options(input_path, output_path, options)?;
        let compile_time = compile_start.elapsed();
        
        // Success message
        println!("✅ Compilation successful!");
        println!("   Output: {} bytes", stats.output_size);
        println!("   Time: {:.2}ms", compile_time.as_millis());
        
        if stats.source_size > 0 {
            let compression_ratio = (1.0 - stats.compression_ratio) * 100.0;
            println!("   Compression: {:.1}%", compression_ratio);
        }
        
        // Show detailed statistics if requested
        if matches.get_flag("stats") {
            self.print_detailed_stats(&stats)?;
        }
        
        // Handle different output formats
        let format = matches.get_one::<OutputFormat>("format").unwrap();
        match format {
            OutputFormat::Krb => {
                // Already compiled to KRB
            }
            OutputFormat::Json => {
                self.output_debug_json(input_path, output_path)?;
            }
            OutputFormat::Debug => {
                self.output_debug_info(input_path, output_path)?;
            }
        }
        
        Ok(())
    }
    
    fn build_compiler_options(&self, matches: &clap::ArgMatches) -> Result<CompilerOptions> {
        let mut options = CompilerOptions::default();
        
        // Optimization level
        let opt_level = matches.get_one::<OptimizationLevel>("optimization").unwrap();
        options.optimization_level = match opt_level {
            OptimizationLevel::None => 0,
            OptimizationLevel::Basic => 1,
            OptimizationLevel::Aggressive => 2,
        };
        
        // Target platform
        let platform = matches.get_one::<Platform>("platform").unwrap();
        options.target_platform = match platform {
            Platform::Desktop => TargetPlatform::Desktop,
            Platform::Mobile => TargetPlatform::Mobile,
            Platform::Web => TargetPlatform::Web,
            Platform::Embedded => TargetPlatform::Embedded,
            Platform::Universal => TargetPlatform::Universal,
        };
        
        // Debug mode
        options.debug_mode = matches.get_flag("debug");
        
        // Script embedding
        options.embed_scripts = matches.get_flag("embed-scripts") || 
                               self.config.embed_scripts.unwrap_or(false);
        
        // Compression
        options.compress_output = matches.get_flag("compress") ||
                                 self.config.compress_output.unwrap_or(false);
        
        // Include directories
        if let Some(include_dirs) = matches.get_many::<String>("include") {
            options.include_directories.extend(include_dirs.cloned());
        }
        if let Some(config_includes) = &self.config.include_directories {
            options.include_directories.extend(config_includes.clone());
        }
        
        // Custom variables
        if let Some(defines) = matches.get_many::<String>("define") {
            for define in defines {
                if let Some((key, value)) = define.split_once('=') {
                    options.custom_variables.insert(key.to_string(), value.to_string());
                } else {
                    return Err(CompilerError::InvalidFormat {
                        message: format!("Invalid variable definition: {}. Use VAR=VALUE format.", define),
                    });
                }
            }
        }
        if let Some(config_vars) = &self.config.custom_variables {
            for (key, value) in config_vars {
                options.custom_variables.entry(key.clone()).or_insert_with(|| value.clone());
            }
        }
        
        // Max file size
        if let Some(max_size) = self.config.max_file_size {
            options.max_file_size = max_size;
        }
        
        Ok(options)
    }
        
    fn watch_and_compile(
        &self, 
        input_path: &str, 
        output_path: &str, 
        options: CompilerOptions
    ) -> Result<()> {
        println!("👀 Watching {} for changes...", input_path);
        
        let (tx, rx) = channel();
        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                match res {
                    Ok(event) => {
                        if let Err(e) = tx.send(event) {
                            eprintln!("Watch error: {}", e);
                        }
                    }
                    Err(e) => eprintln!("Watch error: {}", e),
                }
            },
            notify::Config::default()
        ).map_err(|e| CompilerError::Io(std::io::Error::new(
            std::io::ErrorKind::Other, 
            format!("Failed to create file watcher: {}", e)
        )))?;
        
        watcher.watch(std::path::Path::new(input_path), RecursiveMode::NonRecursive)
            .map_err(|e| CompilerError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to watch file: {}", e)
            )))?;
        
        // Initial compilation
        if let Err(e) = compile_file_with_options(input_path, output_path, options.clone()) {
            eprintln!("❌ Initial compilation failed: {}", e);
        } else {
            println!("✅ Initial compilation successful");
        }
        
        loop {
            match rx.recv() {
                Ok(_event) => {
                    println!("🔄 File changed, recompiling...");
                    
                    match compile_file_with_options(input_path, output_path, options.clone()) {
                        Ok(stats) => {
                            println!("✅ Recompiled successfully ({} bytes, {:.1}ms)",
                                stats.output_size, stats.compile_time_ms);
                        }
                        Err(e) => {
                            eprintln!("❌ Compilation failed: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Watch error: {}", e);
                    break;
                }
            }
        }
        
        Ok(())
    }   
    fn handle_check_command(&self, matches: &clap::ArgMatches) -> Result<()> {
        let input_path = matches.get_one::<String>("input").unwrap();
        let recursive = matches.get_flag("recursive");
        
        if recursive && Path::new(input_path).is_dir() {
            self.check_directory_recursive(input_path)
        } else {
            self.check_single_file(input_path)
        }
    }
    
    fn check_single_file(&self, input_path: &str) -> Result<()> {
        println!("🔍 Checking {}", input_path);
        
        let options = CompilerOptions {
            debug_mode: true,
            ..Default::default()
        };
        
        match compile_file_with_options(input_path, "/dev/null", options) {
            Ok(_) => {
                println!("✅ {} - No issues found", input_path);
                Ok(())
            }
            Err(e) => {
                println!("❌ {} - {}", input_path, e);
                Err(e)
            }
        }
    }
    
    fn check_directory_recursive(&self, dir_path: &str) -> Result<()> {
        let mut total_files = 0;
        let mut error_files = 0;
        
        for entry in walkdir::WalkDir::new(dir_path) {
            let entry = entry.map_err(|e| CompilerError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Directory traversal error: {}", e)
            )))?;
            
            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "kry" {
                        total_files += 1;
                        
                        if let Err(_) = self.check_single_file(entry.path().to_str().unwrap()) {
                            error_files += 1;
                        }
                    }
                }
            }
        }
        
        println!("\n📊 Check Summary:");
        println!("   Total files: {}", total_files);
        println!("   Files with errors: {}", error_files);
        println!("   Success rate: {:.1}%", 
                (total_files - error_files) as f64 / total_files as f64 * 100.0);
        
        if error_files > 0 {
            Err(CompilerError::semantic(0, format!("{} files have errors", error_files)))
        } else {
            Ok(())
        }
    }
    
    fn handle_analyze_command(&self, matches: &clap::ArgMatches) -> Result<()> {
        let input_path = matches.get_one::<String>("input").unwrap();
        let output_path = matches.get_one::<String>("output");
        let format = matches.get_one::<OutputFormat>("format").unwrap();
        
        println!("🔬 Analyzing {}", input_path);
        
        if input_path.ends_with(".krb") {
            self.analyze_krb_file(input_path, output_path, format)
        } else {
            self.analyze_kry_file(input_path, output_path, format)
        }
    }
    
    fn analyze_krb_file(&self, input_path: &str, output_path: Option<&String>, format: &OutputFormat) -> Result<()> {
        let krb_info = crate::analyze_krb_file(input_path)?;
        
        let analysis = match format {
            OutputFormat::Json => {
                serde_json::to_string_pretty(&krb_info)
                    .map_err(|e| CompilerError::CodeGen {
                        message: format!("JSON serialization error: {}", e),
                    })?
            }
            OutputFormat::Debug | OutputFormat::Krb => {
                format!("KRB File Analysis: {}\n\n{:#?}", input_path, krb_info)
            }
        };
        
        if let Some(output_file) = output_path {
            fs::write(output_file, analysis)?;
            println!("✅ Analysis saved to {}", output_file);
        } else {
            println!("{}", analysis);
        }
        
        Ok(())
    }
    
    fn analyze_kry_file(&self, input_path: &str, output_path: Option<&String>, format: &OutputFormat) -> Result<()> {
        // Compile to get detailed analysis
        let options = CompilerOptions {
            debug_mode: true,
            optimization_level: 0, // No optimization for analysis
            ..Default::default()
        };
        
        let stats = compile_file_with_options(input_path, "/dev/null", options)?;
        
        let analysis = match format {
            OutputFormat::Json => {
                serde_json::to_string_pretty(&stats)
                    .map_err(|e| CompilerError::CodeGen {
                        message: format!("JSON serialization error: {}", e),
                    })?
            }
            OutputFormat::Debug | OutputFormat::Krb => {
                format!("KRY File Analysis: {}\n\n{:#?}", input_path, stats)
            }
        };
        
        if let Some(output_file) = output_path {
            fs::write(output_file, analysis)?;
            println!("✅ Analysis saved to {}", output_file);
        } else {
            println!("{}", analysis);
        }
        
        Ok(())
    }
    
    fn handle_init_command(&self, matches: &clap::ArgMatches) -> Result<()> {
        let project_name = matches.get_one::<String>("name").unwrap();
        let template = matches.get_one::<String>("template").unwrap();
        
        println!("🚀 Initializing new Kryon project: {}", project_name);
        
        let project_dir = PathBuf::from(project_name);
        if project_dir.exists() {
            return Err(CompilerError::InvalidFormat {
                message: format!("Directory '{}' already exists", project_name),
            });
        }
        
        fs::create_dir_all(&project_dir)?;
        
        match template.as_str() {
            "simple" => self.create_simple_template(&project_dir, project_name)?,
            "component" => self.create_component_template(&project_dir, project_name)?,
            "game" => self.create_game_template(&project_dir, project_name)?,
            _ => return Err(CompilerError::InvalidFormat {
                message: format!("Unknown template: {}", template),
            }),
        }
        
        println!("✅ Project created successfully!");
        println!("   Directory: {}", project_dir.display());
        println!("   Template: {}", template);
        println!("\nNext steps:");
        println!("   cd {}", project_name);
        println!("   kryc compile app.kry");
        
        Ok(())
    }

    fn create_simple_template(&self, project_dir: &Path, project_name: &str) -> Result<()> {
        let app_content = format!(r##"# Simple Kryon Application
    @variables {{{{
        app_title: "{}"
        primary_color: "#007BFF"
        window_size: 800
    }}}}

    App {{{{
        window_title: $app_title
        window_width: $window_size
        window_height: 600
        background_color: "#F8F9FA"
        
        Container {{{{
            layout: "column center"
            padding: 32
            
            Text {{{{
                text: "Welcome to {}"
                font_size: 24
                font_weight: "bold"
                text_color: $primary_color
                margin: "0 0 16 0"
            }}}}
            
            Text {{{{
                text: "Your Kryon application is ready!"
                font_size: 16
                text_color: "#6C757D"
                margin: "0 0 32 0"
            }}}}
            
            Button {{{{
                text: "Get Started"
                background_color: $primary_color
                text_color: "#FFFFFF"
                padding: "12 24"
                border_radius: 6
                
                &:hover {{{{
                    background_color: "#0056B3"
                }}}}
            }}}}
        }}}}
    }}}}
    "##, project_name, project_name);
        
        fs::write(project_dir.join("app.kry"), app_content)?;
        
        let config_content = r#"{
    "optimization_level": 1,
    "target_platform": "universal",
    "embed_scripts": false,
    "compress_output": false
    }
    "#;
        
        fs::write(project_dir.join("kryon.json"), config_content)?;
        
        Ok(())
    }
    
    fn create_component_template(&self, project_dir: &Path, project_name: &str) -> Result<()> {
        // Create components directory
        let components_dir = project_dir.join("components");
        fs::create_dir_all(&components_dir)?;
        
        // Create Card component
        let card_component = r##"Define Card {
    Properties {
        title: String = "Card Title"
        content: String = "Card content goes here"
        width: String = "300px"
    }
    
    Container {
        width: $width
        background_color: "#FFFFFF"
        border_color: "#DEE2E6"
        border_width: 1
        border_radius: 8
        padding: 16
        
        Text {
            text: $title
            font_size: 18
            font_weight: "bold"
            margin: "0 0 12 0"
        }
        
        Text {
            text: $content
            font_size: 14
            text_color: "#6C757D"
        }
    }
}
"##;
        
        fs::write(components_dir.join("card.kry"), card_component)?;
        
        // Create Button component
        let button_component = r##"Define ActionButton {
    Properties {
        text: String = "Button"
        variant: Enum(primary, secondary, success, danger) = primary
        size: Enum(small, medium, large) = medium
    }
    
    Button {
        text: $text
        padding: $size == "small" ? "8 16" : ($size == "large" ? "16 32" : "12 24")
        font_size: $size == "small" ? 14 : ($size == "large" ? 18 : 16)
        border_radius: 6
        
        background_color: $variant == "primary" ? "#007BFF" : 
                         ($variant == "success" ? "#28A745" :
                         ($variant == "danger" ? "#DC3545" : "#6C757D"))
        
        text_color: "#FFFFFF"
        
        &:hover {
            opacity: 0.9
        }
        
        &:active {
            opacity: 0.8
        }
    }
}
"##;
        
        fs::write(components_dir.join("button.kry"), button_component)?;
        

        let app_content = format!(r##"@include "components/card.kry"
        @include "components/button.kry"

        @variables {{{{
            app_title: "{}"
            background_color: "#F8F9FA"
        }}}}

        App {{{{
            window_title: $app_title
            window_width: 1000
            window_height: 700
            background_color: $background_color
            
            Container {{{{
                layout: "column center"
                padding: 32
                gap: 24
                
                Text {{{{
                    text: "Component Demo"
                    font_size: 28
                    font_weight: "bold"
                    margin: "0 0 32 0"
                }}}}
                
                Container {{{{
                    layout: "row center"
                    gap: 24
                    
                    Card {{{{
                        title: "Welcome"
                        content: "This is a reusable Card component with customizable properties."
                    }}}}
                    
                    Card {{{{
                        title: "Features"
                        content: "Components make it easy to build consistent, maintainable UIs."
                        width: "350px"
                    }}}}
                    
                    Card {{{{
                        title: "Flexibility"
                        content: "Each component instance can override default property values."
                        width: "320px"
                    }}}}
                }}}}
                
                Container {{{{
                    layout: "row center"
                    gap: 16
                    
                    ActionButton {{{{
                        text: "Primary Action"
                        variant: primary
                        size: medium
                    }}}}
                    
                    ActionButton {{{{
                        text: "Success"
                        variant: success
                        size: small
                    }}}}
                    
                    ActionButton {{{{
                        text: "Danger"
                        variant: danger
                        size: large
                    }}}}
                }}}}
            }}}}
        }}}}
        "##, project_name);

        
        fs::write(project_dir.join("app.kry"), app_content)?;
        
        Ok(())
    }
    fn create_game_template(&self, project_dir: &Path, project_name: &str) -> Result<()> {
        // Create scripts directory
        let scripts_dir = project_dir.join("scripts");
        fs::create_dir_all(&scripts_dir)?;
        
        // Create game logic script
        let game_script = r##"-- Game state
    local game = {
        score = 0,
        lives = 3,
        level = 1,
        paused = false
    }
    
    -- Initialize game
    function initGame()
        game.score = 0
        game.lives = 3
        game.level = 1
        game.paused = false
        
        updateUI()
    end
    
    -- Update UI elements
    function updateUI()
        local scoreText = kryon.getElementById("score")
        if scoreText then
            scoreText.text = "Score: " .. game.score
        end
        
        local livesText = kryon.getElementById("lives")
        if livesText then
            livesText.text = "Lives: " .. game.lives
        end
        
        local levelText = kryon.getElementById("level")
        if levelText then
            levelText.text = "Level: " .. game.level
        end
    end
    
    -- Handle button clicks
    function startGame()
        if game.paused then
            game.paused = false
            kryon.getElementById("startBtn").text = "Pause"
        else
            game.paused = true
            kryon.getElementById("startBtn").text = "Start"
        end
    end
    
    function resetGame()
        initGame()
        local startBtn = kryon.getElementById("startBtn")
        if startBtn then
            startBtn.text = "Start"
        end
    end
    
    function addScore(points)
        game.score = game.score + points
        updateUI()
        
        -- Level up every 1000 points
        if game.score > 0 and game.score % 1000 == 0 then
            game.level = game.level + 1
            updateUI()
        end
    end
    
    -- Initialize when script loads
    initGame()
    "##;
        
        fs::write(scripts_dir.join("game.lua"), game_script)?;
        

        let app_content = format!(r##"@variables {{{{
            app_title: "{} Game"
            primary_color: "#007BFF"
            success_color: "#28A745"
            danger_color: "#DC3545"
            dark_color: "#343A40"
        }}}}

        @script "lua" from "scripts/game.lua"

        style "game_button" {{{{
            padding: "12 24"
            border_radius: 6
            text_color: "#FFFFFF"
            font_weight: "bold"
            cursor: "pointer"
            
            &:hover {{{{
                opacity: 0.9
            }}}}
            
            &:active {{{{
                opacity: 0.8
            }}}}
        }}}}

        App {{{{
            window_title: $app_title
            window_width: 800
            window_height: 600
            background_color: "#F8F9FA"
            
            Container {{{{
                layout: "column"
                padding: 24
                
                Text {{{{
                    text: $app_title
                    font_size: 32
                    font_weight: "bold"
                    text_color: $primary_color
                }}}}
                
                Button {{{{
                    id: "startBtn"
                    text: "Start"
                    style: "game_button"
                    background_color: $success_color
                    onClick: "startGame"
                }}}}
            }}}}
        }}}}
        "##, project_name);
        
        fs::write(project_dir.join("app.kry"), app_content)?;
        
        Ok(())
    }
    
    fn handle_benchmark_command(&self, matches: &clap::ArgMatches) -> Result<()> {
        let input_path = matches.get_one::<String>("input").unwrap();
        let iterations: usize = matches.get_one::<String>("iterations").unwrap().parse()
            .map_err(|_| CompilerError::InvalidFormat {
                message: "Invalid iterations number".to_string(),
            })?;
        let warmup: usize = matches.get_one::<String>("warmup").unwrap().parse()
            .map_err(|_| CompilerError::InvalidFormat {
                message: "Invalid warmup number".to_string(),
            })?;
        
        println!("🏁 Running compilation benchmarks");
        println!("   Input: {}", input_path);
        println!("   Warmup iterations: {}", warmup);
        println!("   Benchmark iterations: {}", iterations);
        
        let options = CompilerOptions::default();
        
        // Warmup
        print!("   Warming up");
        for _ in 0..warmup {
            print!(".");
            std::io::stdout().flush().unwrap();
            let _ = compile_file_with_options(input_path, "/dev/null", options.clone());
        }
        println!(" done");
        
        // Benchmark
        let mut times = Vec::new();
        print!("   Benchmarking");
        
        for _ in 0..iterations {
            print!(".");
            std::io::stdout().flush().unwrap();
            
            let start = Instant::now();
            let result = compile_file_with_options(input_path, "/dev/null", options.clone());
            let elapsed = start.elapsed();
            
            if result.is_ok() {
                times.push(elapsed.as_nanos() as f64 / 1_000_000.0); // Convert to milliseconds
            }
        }
        
        println!(" done");
        
        if times.is_empty() {
            return Err(CompilerError::CodeGen {
                message: "All benchmark iterations failed".to_string(),
            });
        }
        
        // Calculate statistics
        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let min = times[0];
        let max = times[times.len() - 1];
        let median = times[times.len() / 2];
        let mean = times.iter().sum::<f64>() / times.len() as f64;
        let std_dev = {
            let variance = times.iter()
                .map(|&x| (x - mean).powi(2))
                .sum::<f64>() / times.len() as f64;
            variance.sqrt()
        };
        
        println!("\n📊 Benchmark Results:");
        println!("   Successful iterations: {}/{}", times.len(), iterations);
        println!("   Min time: {:.2}ms", min);
        println!("   Max time: {:.2}ms", max);
        println!("   Median time: {:.2}ms", median);
        println!("   Mean time: {:.2}ms ± {:.2}ms", mean, std_dev);
        
        if std_dev / mean > 0.1 {
            println!("   ⚠️  High variance detected ({:.1}%)", (std_dev / mean) * 100.0);
        }
        
        Ok(())
    }
    
    fn print_detailed_stats(&self, stats: &crate::CompilationStats) -> Result<()> {
        println!("\n📊 Detailed Compilation Statistics:");
        println!("   Source size: {} bytes", stats.source_size);
        println!("   Output size: {} bytes", stats.output_size);
        println!("   Compression ratio: {:.1}%", (1.0 - stats.compression_ratio) * 100.0);
        println!("   Compile time: {}ms", stats.compile_time_ms);
        println!("   Peak memory: {} bytes", stats.peak_memory_usage);
        
        println!("\n   Element breakdown:");
        println!("     Elements: {}", stats.element_count);
        println!("     Styles: {}", stats.style_count);
        println!("     Components: {}", stats.component_count);
        println!("     Scripts: {}", stats.script_count);
        println!("     Resources: {}", stats.resource_count);
        println!("     Strings: {}", stats.string_count);
        println!("     Variables: {}", stats.variable_count);
        
        if stats.include_count > 0 {
            println!("     Includes: {}", stats.include_count);
        }
        
        Ok(())
    }
    
    fn output_debug_json(&self, input_path: &str, output_path: &str) -> Result<()> {
        // This would output the AST as JSON for debugging
        let json_path = Path::new(output_path).with_extension("json");
        println!("   Debug JSON: {}", json_path.display());
        // Implementation would serialize the AST to JSON
        Ok(())
    }
    
    fn output_debug_info(&self, input_path: &str, output_path: &str) -> Result<()> {
        // This would output human-readable debug information
        let debug_path = Path::new(output_path).with_extension("debug");
        println!("   Debug info: {}", debug_path.display());
        // Implementation would output detailed compilation info
        Ok(())
    }
}

// Add required dependencies to Cargo.toml:
// clap = { version = "4.0", features = ["derive"] }
// serde = { version = "1.0", features = ["derive"] }
// serde_json = "1.0"
// toml = "0.8"
// notify = "5.0"
// walkdir = "2.0"
// env_logger = "0.10"
// log = "0.4"
// hex = "0.4"

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_build_compiler_options() {
        let cli = EnhancedCli::new();
        let app = cli.build_cli();
        let matches = app.try_get_matches_from(vec![
            "kryc", "compile", "test.kry", 
            "--optimization", "aggressive",
            "--platform", "web",
            "--debug",
            "--embed-scripts"
        ]).unwrap();
        
        if let Some(("compile", sub_matches)) = matches.subcommand() {
            let options = cli.build_compiler_options(sub_matches).unwrap();
            
            assert_eq!(options.optimization_level, 2);
            assert_eq!(options.target_platform, TargetPlatform::Web);
            assert!(options.debug_mode);
            assert!(options.embed_scripts);
        }
    }
    
    #[test]
    fn test_config_file_loading() {
        let mut cli = EnhancedCli::new();
        
        let config_json = r#"{
            "optimization_level": 2,
            "target_platform": "mobile",
            "embed_scripts": true,
            "custom_variables": {
                "theme": "dark",
                "version": "1.0"
            }
        }"#;
        
        // In a real test, you'd write this to a temp file and load it
        let config: ConfigFile = serde_json::from_str(config_json).unwrap();
        
        assert_eq!(config.optimization_level, Some(2));
        assert_eq!(config.target_platform, Some("mobile".to_string()));
        assert_eq!(config.embed_scripts, Some(true));
        assert!(config.custom_variables.is_some());
    }
}
