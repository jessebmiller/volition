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
    
    if debug_level >= DebugLevel::Minimal {
        debug_log(
            debug_level,
            DebugLevel::Minimal,
            &format!("Writing to file: {}", path)
        );
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
