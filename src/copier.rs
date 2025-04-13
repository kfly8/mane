use crate::args::{Args, ReplacementRule};
use crate::replacer;
use anyhow::{Result, Context, anyhow};
use std::path::{Path, PathBuf};
use std::fs;
use ignore::WalkBuilder;

/// Copy files and directories with replacements
///
/// # Arguments
/// * `args` - Command line arguments
///
/// # Returns
/// * `Result<()>` - Ok if successful, Error otherwise
pub fn copy_with_replacements(args: &Args) -> Result<()> {
    for copy_spec in &args.copy_specs {
        let source = &copy_spec.source;
        let target = &copy_spec.target;

        // Check if source exists
        if !source.exists() {
            return Err(anyhow!("Source path does not exist: {}", source.display()));
        }

        // Check for invalid combinations - source directory to target file
        if source.is_dir() && target.exists() && target.is_file() {
            eprintln!("Error: Cannot copy directory {} to file {}", source.display(), target.display());
            continue; // Skip this copy spec but continue with others
        }

        if source.is_file() {
            // Copy single file
            copy_file(source, target, args)?;
        } else if source.is_dir() {
            // Copy directory
            copy_directory(source, target, args)?;
        } else {
            return Err(anyhow!("Unsupported source type: {}", source.display()));
        }
    }

    Ok(())
}

/// Copy a single file with replacements
///
/// # Arguments
/// * `source` - Source file path
/// * `target` - Target file path
/// * `args` - Command line arguments
///
/// # Returns
/// * `Result<()>` - Ok if successful, Error otherwise
fn copy_file(source: &Path, target: &Path, args: &Args) -> Result<()> {
    // Handle target path
    let actual_target = if target.is_dir() {
        // If target is a directory, the file will be copied into that directory
        // with the same name as the source file
        let file_name = source.file_name().ok_or_else(||
            anyhow!("Failed to get source file name: {}", source.display()))?;
        target.join(file_name)
    } else {
        target.to_path_buf()
    };

    // Create target directory if it doesn't exist
    if let Some(parent) = actual_target.parent() {
        fs::create_dir_all(parent).context("Failed to create target directory")?;
    }

    // Always override existing files (cp -r standard behavior)
    // We won't show a special message for overriding - it will be shown in the standard output format

    // Check if the source is readable as text
    match fs::read_to_string(source) {
        Ok(content) => {
            // Apply replacements to content
            let replaced_content = apply_all_replacements(&content, &args.rules);

            // Write to target file
            fs::write(&actual_target, replaced_content)
                .context(format!("Failed to write target file: {}", actual_target.display()))?;
        },
        Err(_) => {
            // If reading as text fails, copy the file as binary
            let content = fs::read(source)
                .context(format!("Failed to read source file: {}", source.display()))?;

            // Write to target file
            fs::write(&actual_target, content)
                .context(format!("Failed to write target file: {}", actual_target.display()))?;
        }
    }

    // Print copy information only if verbose mode is enabled
    if args.verbose {
        println!("{} -> {}", source.display(), actual_target.display());
    }
    Ok(())
}

/// Copy a directory recursively with replacements
///
/// # Arguments
/// * `source_dir` - Source directory path
/// * `target_dir` - Target directory path
/// * `args` - Command line arguments
///
/// # Returns
/// * `Result<()>` - Ok if successful, Error otherwise
fn copy_directory(source_dir: &Path, target_dir: &Path, args: &Args) -> Result<()> {
    // Determine the actual target directory
    let actual_target_dir = if target_dir.exists() && target_dir.is_dir() {
        // Get the source directory name
        let source_dir_name = source_dir.file_name().ok_or_else(||
            anyhow!("Failed to get source directory name: {}", source_dir.display()))?;

        // If target is a directory, create a subdirectory with the source dir name
        let target_with_source_name = target_dir.join(source_dir_name);

        // Apply replacements to the directory name if required
        if args.rename_dir {
            let dir_name_str = source_dir_name.to_string_lossy().to_string();
            let mut transformed_name = dir_name_str.clone();

            for rule in &args.rules {
                transformed_name = replacer::apply_replacement(&transformed_name, &rule.from, &rule.to, true);
            }

            target_dir.join(transformed_name)
        } else {
            target_with_source_name
        }
    } else {
        target_dir.to_path_buf()
    };

    // Create target directory if it doesn't exist
    fs::create_dir_all(&actual_target_dir).context("Failed to create target directory")?;

    // Print verbose info for the root directory
    if args.verbose {
        println!("{} -> {}", source_dir.display(), actual_target_dir.display());
    }

    // Build a Walk iterator that respects .gitignore unless specified otherwise
    let walker = if args.include_git_ignore {
        // Include all files, even those that match .gitignore patterns
        WalkBuilder::new(source_dir)
            .git_ignore(false)  // Ignore .gitignore rules
            .build()
    } else {
        // Respect .gitignore rules
        WalkBuilder::new(source_dir)
            .git_ignore(true)   // Respect .gitignore rules
            .build()
    };

    for result in walker {
        let entry = match result {
            Ok(entry) => entry,
            Err(err) => {
                if args.verbose {
                    eprintln!("Warning: {}", err);
                }
                continue;
            }
        };

        let source_path = entry.path();

        // Skip the source directory itself
        if source_path == source_dir {
            continue;
        }

        // Calculate relative path from source directory
        let relative_path = source_path.strip_prefix(source_dir)
            .context(format!("Failed to strip prefix from {}", source_path.display()))?;

        // Apply replacements to each path component if required
        let replaced_relative_path = if args.rename_file || args.rename_dir {
            transform_path(relative_path, &args.rules, args.rename_file, args.rename_dir)?
        } else {
            relative_path.to_path_buf()
        };

        // Combine with target directory
        let target_path = actual_target_dir.join(&replaced_relative_path);

        if source_path.is_file() {
            copy_file(source_path, &target_path, args)?;
        } else if source_path.is_dir() {
            // Always create the directory (or ensure it exists)
            fs::create_dir_all(&target_path)
                .context(format!("Failed to create directory: {}", target_path.display()))?;

            if args.verbose {
                println!("{} -> {}", source_path.display(), target_path.display());
            }
        }
    }

    Ok(())
}

/// Apply all replacement rules to a string
///
/// # Arguments
/// * `content` - String to apply replacements to
/// * `rules` - Replacement rules to apply
///
/// # Returns
/// * `String` - String with replacements applied
fn apply_all_replacements(content: &str, rules: &[ReplacementRule]) -> String {
    let mut result = content.to_string();

    for rule in rules {
        // Use replacer::apply_replacement instead of replace_content
        result = replacer::apply_replacement(&result, &rule.from, &rule.to, true);
    }

    result
}

/// Transform a path by applying replacements to each component
///
/// # Arguments
/// * `path` - Path to transform
/// * `rules` - Replacement rules to apply
/// * `rename_file` - Whether to rename files
/// * `rename_dir` - Whether to rename directories
///
/// # Returns
/// * `Result<PathBuf>` - Transformed path
fn transform_path(
    path: &Path,
    rules: &[ReplacementRule],
    rename_file: bool,
    rename_dir: bool
) -> Result<PathBuf> {
    let mut result = PathBuf::new();

    for component in path.components() {
        let component_str = component.as_os_str().to_string_lossy();
        let is_file = !path.join(&*component_str).is_dir();

        // Apply transformations based on component type
        let transformed_component = if (is_file && rename_file) || (!is_file && rename_dir) {
            // Apply all replacement rules
            let mut transformed = component_str.to_string();
            for rule in rules {
                // Use replacer::apply_replacement which handles all case transformations
                transformed = replacer::apply_replacement(&transformed, &rule.from, &rule.to, true);
            }
            transformed
        } else {
            component_str.to_string()
        };

        result = result.join(transformed_component);
    }

    Ok(result)
}
