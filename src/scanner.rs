use crate::args::Args;
use crate::replacer;
use anyhow::{Result, Context};
use ignore::Walk;
use std::fs;
use std::path::{Path, PathBuf};
use std::io::Write;

/// Scan directories and replace content in files and file names
/// 
/// # Arguments
/// * `args` - Command line arguments
/// 
/// # Returns
/// * `Result<()>` - Result of the operation
pub fn scan_and_replace(args: &Args) -> Result<()> {
    // Determine root paths
    let root_paths = if args.files.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        args.files.clone()
    };
    
    for root_path in root_paths {
        walk_and_process_path(&root_path, args)?;
    }
    
    Ok(())
}

/// Walk through directory structure and process files
/// 
/// # Arguments
/// * `root_path` - Root path to start scanning from
/// * `args` - Command line arguments
/// 
/// # Returns
/// * `Result<()>` - Result of the operation
fn walk_and_process_path(root_path: &Path, args: &Args) -> Result<()> {
    let walker = if args.include_git_ignore {
        Walk::new(root_path)
    } else {
        ignore::WalkBuilder::new(root_path)
            .hidden(false)   // Process hidden files too
            .git_ignore(true)
            .build()
    };
    
    // Collect all files and directories
    let mut all_paths = Vec::new();
    
    for result in walker {
        match result {
            Ok(entry) => {
                let path = entry.path().to_path_buf();
                all_paths.push(path);
            },
            Err(err) => {
                eprintln!("Error walking directory: {}", err);
            }
        }
    }
    
    // Process files and directories
    if args.in_place {
        // First, process file contents
        for path in &all_paths {
            if path.is_file() {
                process_file_content(path, args)?;
            }
        }
        
        // Then, rename files and directories (starting with the deepest paths first)
        let mut sorted_paths = all_paths.clone();
        sorted_paths.sort_by(|a, b| {
            let a_str = a.to_string_lossy();
            let b_str = b.to_string_lossy();
            // Sort by path length (descending) to handle nested paths correctly
            b_str.len().cmp(&a_str.len())
        });
        
        for path in &sorted_paths {
            rename_path(path, args)?;
        }
    } else {
        // For non-in-place mode, just process and output file contents
        for path in &all_paths {
            if path.is_file() {
                output_file_content(path, args)?;
            }
        }
    }
    
    Ok(())
}

/// Process and replace content in a file for in-place mode
/// 
/// # Arguments
/// * `file_path` - Path to the file to process
/// * `args` - Command line arguments
/// 
/// # Returns
/// * `Result<()>` - Result of the operation
fn process_file_content(file_path: &Path, args: &Args) -> Result<()> {
    if !file_path.is_file() {
        return Ok(());
    }
    
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {:?}", file_path))?;
    
    let replaced = replacer::replace_content(&content, args)?;
    
    if content != replaced {
        fs::write(file_path, replaced)
            .with_context(|| format!("Failed to write file: {:?}", file_path))?;
        println!("Modified content: {:?}", file_path);
    }
    
    Ok(())
}

/// Process and output file content for non-in-place mode
/// 
/// # Arguments
/// * `file_path` - Path to the file to process
/// * `args` - Command line arguments
/// 
/// # Returns
/// * `Result<()>` - Result of the operation
fn output_file_content(file_path: &Path, args: &Args) -> Result<()> {
    if !file_path.is_file() {
        return Ok(());
    }
    
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {:?}", file_path))?;
    
    let replaced = replacer::replace_content(&content, args)?;
    
    // Output to stdout
    std::io::stdout().write_all(replaced.as_bytes())?;
    
    Ok(())
}

/// Rename file or directory path
/// 
/// # Arguments
/// * `path` - Path to rename
/// * `args` - Command line arguments
/// 
/// # Returns
/// * `Result<()>` - Result of the operation
fn rename_path(path: &Path, args: &Args) -> Result<()> {
    // Skip based on configuration
    if path.is_file() && !crate::args::GLOBAL_RENAME_FILE_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        return Ok(());
    }
    
    if path.is_dir() && !crate::args::GLOBAL_RENAME_DIR_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        return Ok(());
    }
    
    if let Some(file_name) = path.file_name() {
        let old_name = file_name.to_string_lossy();
        let new_name = replacer::replace_content(&old_name, args)?;
        
        if old_name != new_name {
            let parent = path.parent().unwrap_or(Path::new(""));
            let new_path = parent.join(&new_name);
            
            // Skip if the new path already exists
            if new_path.exists() && new_path != path {
                eprintln!("Warning: Cannot rename {:?} to {:?}: target already exists", path, new_path);
                return Ok(());
            }
            
            fs::rename(path, &new_path)
                .with_context(|| format!("Failed to rename {:?} to {:?}", path, new_path))?;
            println!("Renamed: {:?} -> {:?}", path, new_path);
        }
    }
    
    Ok(())
}
