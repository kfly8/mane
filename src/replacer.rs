use crate::args::Args;
use crate::case;
use anyhow::{Result, Context, anyhow};
use std::fs;
use std::io::{self, Read, Write};

/// Replace content from stdin and write to stdout
/// 
/// # Arguments
/// * `args` - Command line arguments
/// 
/// # Returns
/// * `Result<()>` - Result of the operation
pub fn replace_stdin_stdout(args: &Args) -> Result<()> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    
    // Check if there is any content to replace
    if input.is_empty() {
        return Err(anyhow!("No input provided for replacement"));
    }
    
    let replaced = replace_content(&input, args)?;
    io::stdout().write_all(replaced.as_bytes())?;
    
    // Check if any replacements were made
    if replaced == input && !args.rules.is_empty() {
        eprintln!("Warning: No replacements were made. Check if the pattern exists in the input.");
    }
    
    Ok(())
}

/// Replace content in specified files
/// 
/// # Arguments
/// * `args` - Command line arguments containing files to process
/// 
/// # Returns
/// * `Result<()>` - Result of the operation
pub fn replace_files(args: &Args) -> Result<()> {
    // Make sure we have files to process
    if args.files.is_empty() {
        return Err(anyhow!("No input files provided for replacement"));
    }
    
    let mut any_replacements_made = false;
    
    for file_path in &args.files {
        if !file_path.exists() {
            eprintln!("Warning: File not found: {:?}", file_path);
            continue;
        }
        
        if file_path.is_dir() {
            eprintln!("Warning: Skipping directory: {:?}", file_path);
            continue;
        }
        
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {:?}", file_path))?;
        
        let replaced = replace_content(&content, args)?;
        
        // Track if any replacements were made
        if content != replaced {
            any_replacements_made = true;
            
            if args.in_place {
                // If in-place mode, modify the file
                fs::write(file_path, &replaced)
                    .with_context(|| format!("Failed to write file: {:?}", file_path))?;
                if args.verbose {
                    println!("Modified: {:?}", file_path);
                }
            } else {
                // If not in-place mode, output to stdout
                io::stdout().write_all(replaced.as_bytes())?;
            }
        } else if args.verbose {
            eprintln!("No replacements made in file: {:?}", file_path);
        }
    }
    
    // If no replacements were made across all files, show a warning
    if !any_replacements_made && !args.rules.is_empty() {
        eprintln!("Warning: No replacements were made in any files. Check if the pattern exists in the files.");
    }
    
    Ok(())
}

/// Replace content according to the specified arguments
/// 
/// # Arguments
/// * `content` - The content to replace in
/// * `args` - Command line arguments containing replacement options
/// 
/// # Returns
/// * `Result<String>` - The replaced content
pub fn replace_content(content: &str, args: &Args) -> Result<String> {
    let mut result = content.to_string();
    
    // Apply all replacement rules sequentially
    for rule in &args.rules {
        // Use apply_replacement which handles all the case conversion
        result = apply_replacement(&result, &rule.from, &rule.to, args.case_enabled);
    }
    
    Ok(result)
}


/// Apply a single replacement with case handling
/// 
/// # Arguments
/// * `content` - The content to replace in
/// * `from` - The string to replace
/// * `to` - The replacement string
/// * `case_enabled` - Whether to enable case handling
/// 
/// # Returns
/// * `String` - The replaced content
pub fn apply_replacement(content: &str, from: &str, to: &str, case_enabled: bool) -> String {
    use std::sync::atomic::Ordering;
    use crate::args::GLOBAL_CASE_ENABLED;
    
    // Store the case enabled flag in the global atomic
    GLOBAL_CASE_ENABLED.store(case_enabled, Ordering::Relaxed);
    
    // Use the case-aware replacement function
    match case::replace_with_case_variants(content, from, to) {
        Ok(result) => result,
        Err(_) => {
            // Fallback to simple replacement if case-aware replacement fails
            content.replace(from, to)
        }
    }
}
