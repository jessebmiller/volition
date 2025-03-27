//TODO This file needs work
/*
For Markdown rendering:

termimad - A specialized crate for rendering Markdown in the terminal
pulldown-cmark - A fast Markdown parser that you can use to identify different elements

For syntax highlighting in code blocks:

syntect - The most comprehensive syntax highlighting library for Rust

For integrating them:

Use pulldown-cmark to parse the Markdown and identify code blocks
Then use syntect to highlight the code inside those blocks
Finally use termimad for the rest of the Markdown formatting

A complete solution might look like:

Parse Markdown with pulldown-cmark
When you encounter a code block with a specified language, use syntect to highlight it
Render the rest of the Markdown using termimad
*/


// src/rendering.rs
use termimad::print_text; // Changed from print_inline
use anyhow::Result;

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
    // Use termimad's function for printing full markdown text
    print_text(markdown_text); // Changed from print_inline

    // Since print_text doesn't return a Result, we just return Ok.
    // If print_text panics on error, that will stop execution anyway.
    Ok(())
}
