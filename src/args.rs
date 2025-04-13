use clap::{Parser, ArgAction};
use atty::Stream;
use anyhow::{Result, anyhow};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

// Global static configuration
pub static GLOBAL_CASE_ENABLED: AtomicBool = AtomicBool::new(true);
pub static GLOBAL_RENAME_FILE_ENABLED: AtomicBool = AtomicBool::new(true);
pub static GLOBAL_RENAME_DIR_ENABLED: AtomicBool = AtomicBool::new(true);

/// Execution mode of the application
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Mode {
    #[default]
    None,         // No specific mode
    StdinStdout,  // Read from stdin, write to stdout
    Files,        // Replace only file contents
    FilesAndNames, // Replace file contents and filenames
    Copy,         // Copy files/directories with replacements
}

/// Copy operation specification
#[derive(Debug, Clone)]
pub struct CopySpec {
    /// Source path
    pub source: PathBuf,

    /// Target path
    pub target: PathBuf,
}

/// Replacement rule
#[derive(Debug, Clone)]
pub struct ReplacementRule {
    /// FROM string to replace
    pub from: String,

    /// TO string to replace with
    pub to: String,
}

/// Command line arguments parser
#[derive(Parser, Debug)]
#[command(author, version, about = "A command-line replacement tool without requiring template files")]
#[command(name = "mane")]
pub struct Args {
    /// Replacement rules to apply
    #[arg(short = 'r', long = "replace", value_names = ["FROM", "TO"], num_args = 2, action = ArgAction::Append)]
    pub replacement_rules: Vec<String>,

    /// Copy files or directories to a single target
    #[arg(short = 'c', long = "copy", value_names = ["SOURCE", "TARGET"], num_args = 2.., action = ArgAction::Append)]
    pub copy_specs_raw: Vec<String>,

    /// Files to process
    pub files: Vec<PathBuf>,

    /// Replace in file/directory names as well
    #[arg(short = 'i', long = "in-place")]
    pub in_place: bool,

    /// Include files that match .gitignore patterns
    #[arg(long = "include-git-ignore")]
    pub include_git_ignore: bool,

    /// Enable verbose output
    #[arg(long = "verbose")]
    pub verbose: bool,

    #[arg(skip)]
    pub mode: Mode,

    /// Compiled list of replacement rules
    #[arg(skip)]
    pub rules: Vec<ReplacementRule>,

    /// Compiled list of copy specifications
    #[arg(skip)]
    pub copy_specs: Vec<CopySpec>,

    /// Case transformation options
    #[arg(skip)]
    pub case_enabled: bool,

    /// File name replacement options
    #[arg(skip)]
    pub rename_file: bool,

    /// Directory name replacement options
    #[arg(skip)]
    pub rename_dir: bool,
}

/// Parse command line arguments and validate them
///
/// # Returns
/// * `Result<Args>` - Parsed and validated arguments
pub fn parse() -> Result<Args> {
    let mut args = Args::parse();

    // Set defaults for options
    args.case_enabled = true;
    args.rename_file = true;
    args.rename_dir = true;
    args.copy_specs = Vec::new();

    // Initialize global static configuration
    GLOBAL_CASE_ENABLED.store(true, Ordering::Relaxed);
    GLOBAL_RENAME_FILE_ENABLED.store(true, Ordering::Relaxed);
    GLOBAL_RENAME_DIR_ENABLED.store(true, Ordering::Relaxed);

    // Process copy specs if any
    if !args.copy_specs_raw.is_empty() {
        // Need at least 2 arguments for --copy (at least one source and one target)
        if args.copy_specs_raw.len() < 2 {
            return Err(anyhow!("The -c/--copy option requires at least one SOURCE and one TARGET argument"));
        }

        // The last argument is always the target
        let target_path = args.copy_specs_raw.last().unwrap();
        let target = PathBuf::from(target_path);

        // All preceding arguments are sources
        for i in 0..args.copy_specs_raw.len() - 1 {
            let source_path = &args.copy_specs_raw[i];
            let source = PathBuf::from(source_path);

            args.copy_specs.push(CopySpec {
                source,
                target: target.clone(),
            });
        }

        // Set mode to Copy if we have copy specs
        args.mode = Mode::Copy;
    } else {
        // Determine the execution mode if no copy specs
        if args.in_place {
            args.mode = Mode::FilesAndNames;
        } else if !args.files.is_empty() {
            args.mode = Mode::Files;
        } else if !atty::is(Stream::Stdin) {
            args.mode = Mode::StdinStdout;
        }
    }

    // Validate arguments
    validate_args(&mut args)?;

    Ok(args)
}

// Add Default implementation for Args
impl Default for Args {
    fn default() -> Self {
        Self {
            replacement_rules: Vec::new(),
            copy_specs_raw: Vec::new(),
            files: Vec::new(),
            in_place: false,
            include_git_ignore: false,
            verbose: false,
            mode: Mode::default(),
            rules: Vec::new(),
            copy_specs: Vec::new(),
            case_enabled: true,
            rename_file: true,
            rename_dir: true,
        }
    }
}

/// Validate command line arguments for consistency
///
/// # Arguments
/// * `args` - Command line arguments to validate
///
/// # Returns
/// * `Result<()>` - Ok if valid, Error otherwise
fn validate_args(args: &mut Args) -> Result<()> {
    // If there are replacement rules specified on the command line
    if !args.replacement_rules.is_empty() {
        if args.replacement_rules.len() % 2 != 0 {
            return Err(anyhow!("Each -r/--replace option requires both FROM and TO arguments"));
        }

        // Check for empty FROM values (which are invalid according to the spec)
        for i in (0..args.replacement_rules.len()).step_by(2) {
            if args.replacement_rules[i].is_empty() {
                return Err(anyhow!("Empty FROM string is not allowed in replacement rules"));
            }
        }

        // Process all replacement rules from command line
        for i in (0..args.replacement_rules.len()).step_by(2) {
            // Check if the rule already exists in loaded rules (override config file rules)
            let from = args.replacement_rules[i].clone();
            let to = args.replacement_rules[i + 1].clone();

            // Remove existing rules with the same FROM string
            args.rules.retain(|rule| rule.from != from);

            // Add new rule
            args.rules.push(ReplacementRule {
                from,
                to,
            });
        }
    }

    // If we have replacement rules but no valid mode is set,
    // it means there's no input source (files or stdin)
    if !args.rules.is_empty() && args.mode == Mode::None {
        return Err(anyhow!("No replacement target specified. Provide files or standard input."));
    }

    // Check if we have replacement rules
    if args.mode != Mode::Copy && args.rules.is_empty() {
        // Error if no replacement rules are specified on command line
        // and we're not in copy mode (copy mode can work without replacement rules)
        return Err(anyhow!("No replacement rules specified. Use -r/--replace FROM TO"));
    }

    // For copy mode, we need copy specs
    if args.mode == Mode::Copy && args.copy_specs.is_empty() {
        return Err(anyhow!("No copy specifications provided. Use -c SOURCE [SOURCE...] TARGET"));
    }

    // When not in copy mode, verify that we have input files (or using stdin)
    if args.mode == Mode::Files && args.files.is_empty() {
        return Err(anyhow!("No input files provided. Specify files to process or use stdin."));
    }

    Ok(())
}
