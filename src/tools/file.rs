use std::fs;
use std::path::Path;
use anyhow::{Result, Context};
use crate::utils::DebugLevel;
use crate::utils::debug_log;
use crate::models::tools::{ReadFileArgs, WriteFileArgs};

pub async fn read_file(args: ReadFileArgs, debug_level: DebugLevel) -> Result<String> {
    let path = &args.path;
    
    if debug_level >= DebugLevel::Minimal {
        debug_log(debug_level, DebugLevel::Minimal, &format!("Reading file: {}", path));
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path))?;

    if debug_level >= DebugLevel::Minimal {
        debug_log(
            debug_level,
            DebugLevel::Minimal,
            &format!("Read {} bytes from file", content.len())
        );
    }

    Ok(content)
}

pub async fn write_file(args: WriteFileArgs, debug_level: DebugLevel) -> Result<String> {
    let path = &args.path;
    let content = &args.content;
    let backup = args.backup.unwrap_or(true);
    
    if debug_level >= DebugLevel::Minimal {
        debug_log(
            debug_level,
            DebugLevel::Minimal,
            &format!("Writing to file: {} (backup: {})", path, backup)
        );
    }

    // Create a backup if requested and the file exists
    if backup && Path::new(path).exists() {
        let backup_path = format!("{}.bak", path);
        fs::copy(path, &backup_path)
            .with_context(|| format!("Failed to create backup of file: {}", path))?;

        if debug_level >= DebugLevel::Minimal {
            debug_log(
                debug_level,
                DebugLevel::Minimal,
                &format!("Created backup at: {}", backup_path)
            );
        }
    }

    // Create parent directories if they don't exist
    if let Some(parent) = Path::new(path).parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }
    }

    // Write the content to the file
    fs::write(path, content)
        .with_context(|| format!("Failed to write to file: {}", path))?;

    if debug_level >= DebugLevel::Minimal {
        debug_log(
            debug_level,
            DebugLevel::Minimal,
            &format!("Successfully wrote {} bytes to file", content.len())
        );
    }

    Ok(format!("Successfully wrote to file: {}", path))
}