// src/rendering.rs
use termimad::{MadSkin, MadView, Area};
use std::io::stdout;
use anyhow::{Result, Context}; // Use anyhow::Result

/// Creates a default skin for rendering markdown.
fn create_skin() -> MadSkin {
    // Start with the default skin - syntax highlighting is often enabled by default
    // if syntect is available (which it should be as a termimad dependency).
    let skin = MadSkin::default();

    // We will rely on the default theme handling for now.
    // If customization is needed later, we'll consult the termimad docs for
    // the correct API (e.g., skin.code_block.set_syntect_theme(...)).

    skin
}

/// Prints markdown text formatted to the terminal.
/// Handles wrapping and syntax highlighting for code blocks.
/// Returns an anyhow::Result to handle potential errors.
pub fn print_formatted(markdown_text: &str) -> Result<()> { // Changed return type
    let skin = create_skin();
    let area = Area::full_screen(); // Use full terminal width
    // MadView takes ownership, so we clone or pass owned string
    let view = MadView::from(markdown_text.to_string(), area, skin);

    // Use the write_on method and convert potential termimad::Error
    view.write_on(&mut stdout())
        .context("Failed to write formatted markdown to stdout")?; // Convert error to anyhow

    Ok(())
}
