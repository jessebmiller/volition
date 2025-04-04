// volition-cli/src/history.rs

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{BufReader, BufWriter, Write},
    // Path removed, PathBuf kept
    path::PathBuf,
};
use uuid::Uuid;
use volition_core::models::chat::ChatMessage; // Kept as it's used below

const HISTORY_DIR_NAME: &str = "volition/history";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConversationHistory {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub last_updated_at: DateTime<Utc>,
    pub messages: Vec<ChatMessage>,
    // Optional: Add title later if needed
    // pub title: Option<String>,
}

// Allow dead code for append methods as they are not used in the CLI directly
// but might be useful for the struct elsewhere.
#[allow(dead_code)]
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

    pub fn append_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
        self.last_updated_at = Utc::now();
    }

     pub fn append_messages(&mut self, messages: Vec<ChatMessage>) {
        self.messages.extend(messages);
        self.last_updated_at = Utc::now();
    }
}

// --- Helper Functions ---

/// Gets the path to the history storage directory, creating it if necessary.
fn get_history_dir() -> Result<PathBuf> {
    let data_dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get local data directory"))?;
    let history_path = data_dir.join(HISTORY_DIR_NAME);
    fs::create_dir_all(&history_path)
        .with_context(|| format!("Failed to create history directory at {:?}", history_path))?;
    Ok(history_path)
}

/// Gets the full path for a history file given its ID.
fn get_history_file_path(id: Uuid) -> Result<PathBuf> {
    Ok(get_history_dir()?.join(format!("{}.json", id)))
}

/// Saves a conversation history to a JSON file.
pub fn save_history(history: &ConversationHistory) -> Result<()> {
    let file_path = get_history_file_path(history.id)?;
    let file = File::create(&file_path)
        .with_context(|| format!("Failed to create history file at {:?}", file_path))?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, history)
        .with_context(|| format!("Failed to serialize history to {:?}", file_path))?;
    writer.flush()
        .with_context(|| format!("Failed to flush writer for {:?}", file_path))?;
    Ok(())
}

/// Loads a conversation history from a JSON file by ID.
pub fn load_history(id: Uuid) -> Result<ConversationHistory> {
    let file_path = get_history_file_path(id)?;
    let file = File::open(&file_path)
        .with_context(|| format!("Failed to open history file at {:?}", file_path))?;
    let reader = BufReader::new(file);
    let history: ConversationHistory = serde_json::from_reader(reader)
        .with_context(|| format!("Failed to deserialize history from {:?}", file_path))?;
    Ok(history)
}

/// Deletes a conversation history file by ID.
pub fn delete_history(id: Uuid) -> Result<()> {
    let file_path = get_history_file_path(id)?;
    if file_path.exists() {
        fs::remove_file(&file_path)
            .with_context(|| format!("Failed to delete history file at {:?}", file_path))?;
        Ok(())
    } else {
        Err(anyhow::anyhow!("History with ID {} not found.", id))
    }
}

/// Lists all available conversation histories, sorted by last updated time (desc).
pub fn list_histories() -> Result<Vec<ConversationHistory>> {
    let history_dir = get_history_dir()?;
    let mut histories = Vec::new();

    for entry in fs::read_dir(&history_dir)
        .with_context(|| format!("Failed to read history directory at {:?}", history_dir))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if let Ok(id) = Uuid::parse_str(stem) {
                    // Load only necessary metadata for listing if performance becomes an issue
                    // For now, load the full history to sort easily
                    match load_history(id) {
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

/// Gets a short preview string of the first user message.
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
