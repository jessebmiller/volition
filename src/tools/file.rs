use std::fs;
use std::path::Path;
use anyhow::{Result, Context};
use crate::models::tools::{ReadFileArgs, WriteFileArgs};
use tracing::{info, debug};

pub async fn read_file(args: ReadFileArgs) -> Result<String> {
    let path = &args.path;
    
    info!("Reading file: {}", path);
    
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path))?;
    
    info!("Read {} bytes from file", content.len());
    
    Ok(content)
}

pub async fn write_file(args: WriteFileArgs) -> Result<String> {
    let path = &args.path;
    let content = &args.content;
    
    info!("Writing to file: {}", path);
    
    // Create parent directories if they don't exist
    if let Some(parent) = Path::new(path).parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }
    }
    
    fs::write(path, content)
        .with_context(|| format!("Failed to write to file: {}", path))?;
    
    info!("Successfully wrote {} bytes to file", content.len());
    
    Ok(format!("Successfully wrote to file: {}", path))
}
