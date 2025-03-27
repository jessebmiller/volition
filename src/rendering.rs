// src/rendering.rs
// Removed MadView, Area, MadSkin imports as they are no longer directly used by print_formatted
// Added print_text import
use termimad::print_text;
 // Still needed? print_text writes to stdout by default. Let's keep it for now, cargo check will tell us.
use anyhow::Result; // Keep Result for function signature consistency, though print_text doesn't return one.

// create_skin is no longer needed by print_formatted, but keep it in case it's used elsewhere
// or for future customization if print_text isn't sufficient.
// fn create_skin() -> MadSkin {
//     MadSkin::default()
// }

/// Prints markdown text formatted to the terminal using termimad::print_text.
/// This function prints directly to stdout without clearing the screen.
/// Note: termimad::print_text does not return a Result, so this function now always returns Ok(()).
/// Error handling (e.g., for I/O errors) is handled internally by termimad or would cause a panic.
pub fn print_formatted(markdown_text: &str) -> Result<()> {
    // Use termimad's simple print function
    print_text(markdown_text);

    // Since print_text doesn't return a Result, we just return Ok.
    // If print_text panics on error, that will stop execution anyway.
    Ok(())
}
