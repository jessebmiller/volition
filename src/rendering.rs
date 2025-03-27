// src/rendering.rs
use anyhow::Result;
// Remove lazy_static and syntect imports if no longer needed elsewhere
// use lazy_static::lazy_static;
// use syntect::highlighting::ThemeSet;
// use syntect::parsing::SyntaxSet;
use termimad::MadSkin;
// Using crossterm color is common, but adjust if your terminal setup prefers another backend
use termimad::crossterm::style::Color;

// Remove the lazy_static block for syntect
/*
lazy_static! {
    pub static ref SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
    pub static ref THEME_SET: ThemeSet = ThemeSet::load_defaults();
}
*/

// Helper function to create a MadSkin with basic code block styling.
fn create_skin() -> MadSkin {
    let mut skin = MadSkin::default(); // Start with the default termimad skin.

    // --- Code Block Styling ---
    // Set a background color for code blocks.
    // You can customize the color here.
    skin.code_block.set_bg(Color::Rgb { r: 45, g: 45, b: 45 }); // Slightly dark grey background
    // Optionally set a foreground color if the default isn't suitable
    // skin.code_block.set_fg(Color::White);
    // Optionally add indentation or other line styling if needed
    // skin.code_block.indentation = 2; // Example: Indent code blocks by 2 spaces

    // --- Other Element Styling (Examples) ---
    // skin.bold.set_fg(Color::Yellow);
    // skin.italic.set_fg(Color::Magenta);
    // skin.headers[0].set_fg(Color::Red); // Style H1 headers
    // skin.inline_code.set_fg(Color::Cyan); // Style inline code differently

    skin // Return the configured skin
}

/// Prints markdown text formatted to the terminal.
/// Code blocks will have basic styling (e.g., background color) but no syntax highlighting.
///
/// This function uses a basic `MadSkin`.
/// It prints directly to stdout.
pub fn print_formatted(markdown_text: &str) -> Result<()> {
    // Create the skin with basic styling.
    let skin = create_skin();

    // Print the markdown using the configured skin.
    skin.print_text(markdown_text);

    // Assuming success if no panic occurred.
    Ok(())
}
