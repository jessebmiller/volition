// volition-cli/src/history.rs

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{BufReader, BufWriter, Write},
    path::{Path, PathBuf}, // Added Path
};
use uuid::Uuid;
use volition_core::models::chat::ChatMessage;

const HISTORY_SUBDIR: &str = ".volition/history"; // Store history relative to project root

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConversationHistory {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub last_updated_at: DateTime<Utc>,
    pub messages: Vec<ChatMessage>,
    // Optional: Add title later if needed
    // pub title: Option<String>,
}

impl ConversationHistory {
    pub fn new(messages: Vec<ChatMessage>) -> Self {
        let now = Utc::now();
        ConversationHistory {
            id: Uuid::new_v4(),
            created_at: now,
            last_updated_at: now,
            messages,
        }
    }
    // Removed append_message and append_messages as they were unused dead code
}

// --- Helper Functions ---

/// Gets the path to the project-specific history storage directory, creating it if necessary.
fn ensure_history_dir(project_root: &Path) -> Result<PathBuf> {
    let history_path = project_root.join(HISTORY_SUBDIR);
    fs::create_dir_all(&history_path)
        .with_context(|| format!("Failed to create history directory at {:?}", history_path))?;
    Ok(history_path)
}

/// Gets the full path for a history file given its ID and project root.
fn get_history_file_path(project_root: &Path, id: Uuid) -> Result<PathBuf> {
    let history_dir = ensure_history_dir(project_root)?; // Ensure directory exists first
    Ok(history_dir.join(format!("{}.json", id)))
}

/// Saves a conversation history to a JSON file within the project's history directory.
pub fn save_history(project_root: &Path, history: &ConversationHistory) -> Result<()> {
    let file_path = get_history_file_path(project_root, history.id)?;
    let file = File::create(&file_path)
        .with_context(|| format!("Failed to create history file at {:?}", file_path))?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, history)
        .with_context(|| format!("Failed to serialize history to {:?}", file_path))?;
    writer.flush()
        .with_context(|| format!("Failed to flush writer for {:?}", file_path))?;
    Ok(())
}

/// Loads a conversation history from a JSON file by ID from the project's history directory.
pub fn load_history(project_root: &Path, id: Uuid) -> Result<ConversationHistory> {
    let file_path = get_history_file_path(project_root, id)?;
    if !file_path.exists() {
         // Check existence before trying to open to give a clearer error
         return Err(anyhow::anyhow!("History file not found at {:?}", file_path));
    }
    let file = File::open(&file_path)
        .with_context(|| format!("Failed to open history file at {:?}", file_path))?;
    let reader = BufReader::new(file);
    let history: ConversationHistory = serde_json::from_reader(reader)
        .with_context(|| format!("Failed to deserialize history from {:?}", file_path))?;
    Ok(history)
}

/// Deletes a conversation history file by ID from the project's history directory.
pub fn delete_history(project_root: &Path, id: Uuid) -> Result<()> {
    let file_path = get_history_file_path(project_root, id)?;
    if file_path.exists() {
        fs::remove_file(&file_path)
            .with_context(|| format!("Failed to delete history file at {:?}", file_path))?;
        Ok(())
    } else {
        Err(anyhow::anyhow!("History with ID {} not found in project.", id))
    }
}

/// Lists all available conversation histories within the project, sorted by last updated time (desc).
pub fn list_histories(project_root: &Path) -> Result<Vec<ConversationHistory>> {
    let history_dir = ensure_history_dir(project_root)?; // Ensures the dir exists, even if empty
    let mut histories = Vec::new();

    // Check if the directory actually exists before trying to read it
    if !history_dir.is_dir() {
        // This case should theoretically be handled by ensure_history_dir, but double-check
         return Ok(histories); // Return empty list if dir doesn't exist or isn't a dir
    }


    for entry in fs::read_dir(&history_dir)
        .with_context(|| format!("Failed to read history directory at {:?}", history_dir))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if let Ok(id) = Uuid::parse_str(stem) {
                    // Load the full history to sort easily
                    // Pass project_root to the load_history call
                    match load_history(project_root, id) {
                        Ok(history) => histories.push(history),
                        Err(e) => {
                            // Log error or handle corrupted files?
                            eprintln!("Warning: Failed to load history file {:?}: {}", path, e);
                        }
                    }
                }
            }
        }
    }

    // Sort by last updated time, newest first
    histories.sort_by(|a, b| b.last_updated_at.cmp(&a.last_updated_at));

    Ok(histories)
}

/// Gets a short preview string of the first user message. (No changes needed here)
pub fn get_history_preview(history: &ConversationHistory) -> String {
    history.messages
        .iter()
        .find(|m| m.role.to_lowercase() == "user") // Find first user message
        .map(|m| {
            // Handle Option<String> safely by providing a default empty string slice
            let content_str = m.content.as_deref().unwrap_or("");
            let preview: String = content_str.chars().take(70).collect();
            if content_str.chars().count() > 70 {
                format!("{}...", preview)
            } else {
                preview
            }
        })
        .unwrap_or_else(|| "[No user messages]".to_string())
}
