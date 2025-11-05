//! omnitype - A hybrid type checker for Python and other dynamic languages.

#![allow(clippy::empty_line_after_doc_comments)]
#![allow(clippy::empty_line_after_outer_attr)]

mod ui;

use clap::Parser;
use log::LevelFilter;
use omnitype::{
    analyzer::{AnalysisResult, Analyzer},
    fixer::Fixer,
    types::TypeEnv,
    utils::find_python_files,
};
use std::{io, path::PathBuf};

/// Command-line interface for omnitype.
#[derive(Parser, Debug)]
#[command(
    name = "omnitype",
    version,
    about = "A hybrid type checker for Python and other dynamic languages",
    long_about = None
)]
#[allow(clippy::empty_line_after_outer_attr)]

struct Cli {
    /// Sets the verbosity level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Run in terminal UI mode
    #[arg(short, long)]
    tui: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Parser, Debug)]
#[allow(clippy::empty_line_after_outer_attr)]

enum Commands {
    /// Check types in the specified project
    Check {
        /// Path to the project directory or file
        path: PathBuf,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Fix type annotations in the specified project
    Fix {
        /// Path to the project directory or file
        path: PathBuf,

        /// Apply changes in-place
        #[arg(short, long)]
        in_place: bool,
    },

    /// Run the runtime type tracer
    Trace {
        /// Path to the test file or module to trace
        path: PathBuf,

        /// Test function to run (default: run all tests)
        #[arg(short, long)]
        test: Option<String>,
    },
}

fn setup_logging(level: &str) -> io::Result<()> {

    let log_level = match level.to_lowercase().as_str() {
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "info" => LevelFilter::Info,
        "warn" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        _ => LevelFilter::Info,
    };

    env_logger::Builder::new()
        .filter_level(log_level)
        .format_timestamp(None)
        .init();

    Ok(())
}

fn main() -> io::Result<()> {

    let cli = Cli::parse();

    // Set up logging
    setup_logging(&cli.log_level)?;

    // If no command is provided or TUI flag is set, run the TUI
    if cli.command.is_none() || cli.tui {

        let mut app = ui::App::new();

        return app.run();
    }

    // Handle command-line commands
    if let Some(command) = cli.command {

        match command {
            Commands::Check { path, format } => {

                let path_exists = std::fs::metadata(&path)
                    .map(|m| m.is_file() || m.is_dir())
                    .unwrap_or(false);

                if !path_exists {

                    eprintln!("Path not found: {:?}", path);

                    return Ok(());
                }

                let mut results: Vec<AnalysisResult> = Vec::new();

                if path.is_file() {

                    if path.extension().and_then(|e| e.to_str()) == Some("py") {

                        match Analyzer::analyze_python_file(&path) {
                            Ok(res) => results.push(res),
                            Err(e) => eprintln!("Failed to analyze {:?}: {}", path, e),
                        }
                    } else {

                        eprintln!("File is not a Python file: {:?}", path);
                    }
                } else {

                    for file in find_python_files(&path) {

                        match Analyzer::analyze_python_file(&file) {
                            Ok(res) => results.push(res),
                            Err(e) => eprintln!("Failed to analyze {:?}: {}", file, e),
                        }
                    }
                }

                let mut total_diagnostics = 0usize;

                match format.as_str() {
                    "json" => match serde_json::to_string_pretty(&results) {
                        Ok(s) => println!("{}", s),
                        Err(e) => eprintln!("Failed to serialize JSON: {}", e),
                    },
                    _ => {
                        if results.is_empty() {

                            println!("No Python files found or all analyses failed.");
                        } else {

                            for r in &results {

                                println!(
                                    "{}: functions={}, classes={}",
                                    r.path, r.function_count, r.class_count
                                );

                                for d in &r.diagnostics {

                                    println!(
                                        "  {}:{}:{}: {} {}",
                                        r.path,
                                        d.line + 1,
                                        d.column + 1,
                                        d.severity,
                                        d.message
                                    );
                                }

                                total_diagnostics += r.diagnostics.len();
                            }
                        }
                    },
                }

                if total_diagnostics > 0 {

                    std::process::exit(1);
                }
            },
            Commands::Fix { path, in_place } => {

                let fixer = Fixer::new(TypeEnv::new(), in_place);

                if let Err(e) = fixer.fix_path(&path) {

                    eprintln!("Fix failed: {}", e);
                } else {

                    println!("Fix completed{}", if in_place { " (in-place)" } else { "" });
                }
            },
            Commands::Trace { path, test } => {

                use omnitype::tracer::RuntimeTracer;

                fn setup_logging(level: &str) -> std::io::Result<()> {

                    // RUST_LOG takes precedence if set; otherwise fall back to CLI value.
                    let env = env_logger::Env::default().filter_or("RUST_LOG", level);

                    env_logger::Builder::from_env(env)
                        .format_timestamp(None)
                        .try_init()
                        .ok(); // Ignore if already initialized
                    Ok(())
                }

                setup_logging("info")?;

                let mut tracer = RuntimeTracer::new(false);

                match tracer.run(&path, test.as_deref()) {
                    Ok(()) => {

                        let trace = tracer.into_traces();

                        println!("Tracing completed successfully.");

                        println!("Variables:");

                        for (name, types) in &trace.variables {

                            println!("  {}: {:?}", name, types);
                        }

                        println!("Functions:");

                        for (name, (args, returns)) in &trace.functions {

                            println!("  {}: args={:?}, returns={:?}", name, args, returns);
                        }
                    },
                    Err(e) => eprintln!("Tracing failed: {}", e),
                }
            },
        }
    }

    Ok(())
}
