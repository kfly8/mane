mod args;
mod replacer;
mod scanner;
mod case;
mod copier;

use anyhow::{Result, Context};
use std::process;

/// Main entry point of the application
/// Handles argument parsing and executes the program with error handling
fn main() -> Result<()> {
    // Parse command line arguments
    let args = args::parse().context("Failed to parse arguments")?;

    // Execute the program
    if let Err(e) = run(args) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }

    Ok(())
}

/// Runs the main functionality based on the provided arguments
///
/// # Arguments
/// * `args` - Parsed command line arguments
fn run(args: args::Args) -> Result<()> {
    // Execute appropriate action based on the command mode
    match args.mode {
        args::Mode::StdinStdout => {
            // Read from stdin, write to stdout
            replacer::replace_stdin_stdout(&args)?;
        },
        args::Mode::Files => {
            // Replace content in files
            replacer::replace_files(&args)?;
        },
        args::Mode::FilesAndNames => {
            // Replace content in files and rename files/directories
            scanner::scan_and_replace(&args)?;
        },
        args::Mode::Copy => {
            // Copy files/directories with replacements
            copier::copy_with_replacements(&args)?;
        },
        args::Mode::None => {
            // do nothing
            return Err(anyhow::anyhow!("No action specified. Use --help for more information."));
        }
    }

    Ok(())
}
