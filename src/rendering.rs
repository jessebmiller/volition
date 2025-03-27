// src/rendering.rs
use anyhow::Result;
use lazy_static::lazy_static;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use std::io::{self, Write};
use syntect::{
    easy::HighlightLines,
    highlighting::{Color as SyntectColor, FontStyle, Style, Theme, ThemeSet},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};
use termimad::{
    crossterm::style::{Attribute, Color, ResetColor, SetAttribute, SetForegroundColor},
    Error as TermimadError, MadSkin,
};

use pulldown_cmark_to_cmark::{cmark, Error as CmarkError};

// --- Syntect Setup ---
lazy_static! {
    static ref SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
    static ref THEME_SET: ThemeSet = ThemeSet::load_defaults();
    static ref THEME_NAME: String = "base16-ocean.dark".to_string();
    static ref CODE_THEME: &'static Theme = THEME_SET
        .themes
        .get(&*THEME_NAME)
        .unwrap_or_else(|| &THEME_SET.themes["base16-ocean.dark"]);
}

// Helper to convert syntect Color to crossterm Color
fn syntect_to_crossterm_color(color: SyntectColor) -> Option<Color> {
    if color.a == 0 {
        None
    } else {
        Some(Color::Rgb {
            r: color.r,
            g: color.g,
            b: color.b,
        })
    }
}

// --- Syntect Code Highlighting Function (Simplified Colors) ---
// (No changes needed in this function)
fn highlight_code<W: Write>(
    writer: &mut W,
    code: &str,
    language: Option<&str>,
    syntax_set: &SyntaxSet,
    theme: &Theme,
) -> Result<(), io::Error> {
    // ... (rest of function is unchanged) ...
    let lower_lang = language.map(|l| l.to_lowercase());
    let lang_token_opt = lower_lang.as_deref().map(|lang_str| match lang_str {
        "shell" | "bash" | "sh" => "bash",
        "javascript" | "js" => "javascript",
        "typescript" | "ts" => "typescript",
        "python" | "py" => "python",
        "yaml" | "yml" => "yaml",
        "html" | "htm" => "html",
        "css" => "css",
        "csharp" | "cs" => "c#",
        "rust" | "rs" => "rust",
        "markdown" | "md" => "markdown",
        "json" => "json",
        "toml" => "toml",
        other => other,
    });

    let syntax = lang_token_opt
        .and_then(|token| syntax_set.find_syntax_by_token(token))
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

    let mut highlighter = HighlightLines::new(syntax, theme);

    write!(writer, "{}", ResetColor)?;

    for line in LinesWithEndings::from(code) {
        write!(writer, "{}", ResetColor)?;
        write!(writer, "{}", SetAttribute(Attribute::Reset))?;

        let ranges: Vec<(Style, &str)> = highlighter
            .highlight_line(line, syntax_set)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        for (style, content) in ranges {
            let fg = style.foreground;

            if fg.a > 0 {
                if let Some(crossterm_fg) = syntect_to_crossterm_color(fg) {
                    write!(writer, "{}", SetForegroundColor(crossterm_fg))?;
                } else {
                    write!(writer, "{}", ResetColor)?;
                }
            } else {
                write!(writer, "{}", ResetColor)?;
            }

            let mut applied_attrs = false;
            if style.font_style.contains(FontStyle::BOLD) {
                write!(writer, "{}", SetAttribute(Attribute::Bold))?;
                applied_attrs = true;
            }
            if style.font_style.contains(FontStyle::ITALIC) {
                write!(writer, "{}", SetAttribute(Attribute::Italic))?;
                applied_attrs = true;
            }
            if style.font_style.contains(FontStyle::UNDERLINE) {
                write!(writer, "{}", SetAttribute(Attribute::Underlined))?;
                applied_attrs = true;
            }

            write!(writer, "{}", content)?;

            if applied_attrs {
                write!(writer, "{}", SetAttribute(Attribute::Reset))?;
            }
            write!(writer, "{}", ResetColor)?;
        }
    }
    write!(writer, "{}", ResetColor)?;
    Ok(())
}

// --- Termimad Skin Creation (Simplified) ---
// (No changes needed in this function)
fn create_skin() -> MadSkin {
    let mut skin = MadSkin::default();
    skin.inline_code.set_fg(Color::Cyan);
    skin.inline_code.set_bg(Color::Reset);
    skin.code_block.set_fg(Color::Reset);
    skin.code_block.set_bg(Color::Reset);
    skin
}

// Helper function to flush buffered Markdown events using termimad
// Takes a mutable reference to the events vector.
fn flush_markdown_buffer<W: Write>(
    events: &mut Vec<Event<'_>>, // Use mutable reference again
    skin: &MadSkin,
    writer: &mut W,
) -> Result<(), io::Error> {
    // Return io::Error
    if events.is_empty() {
        return Ok(());
    }

    let mut md_string = String::new();

    // Pass an iterator over references to the events directly.
    // This avoids cloning/owning and should work now that versions match.
    cmark(events.iter(), &mut md_string)
        // Map CmarkError to io::Error
        .map_err(|e: CmarkError| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Markdown generation error: {}", e),
            )
        })?;

    // Write the reconstructed Markdown string using termimad's skin
    // Map TermimadError to io::Error
    skin.write_text_on(writer, &md_string)
        .map_err(|e: TermimadError| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Termimad rendering error: {}", e),
            )
        })?;

    // Clear the original buffer now that it's been processed
    events.clear();
    Ok(())
}

// --- Main Printing Function (Refactored) ---
pub fn print_formatted(markdown_text: &str) -> Result<()> {
    let skin = create_skin();
    let mut stdout = io::stdout().lock();

    // The parser's events borrow from markdown_text
    let parser = Parser::new_ext(markdown_text, Options::empty());

    // Event buffer holds cloned events. Lifetimes might be 'markdown_text or 'static.
    let mut event_buffer: Vec<Event<'_>> = Vec::new();
    let mut code_buffer = String::new();
    let mut current_language: Option<String> = None;
    let mut in_code_block = false;

    for event in parser {
        match &event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {
                // Pass mutable reference to the buffer
                flush_markdown_buffer(&mut event_buffer, &skin, &mut stdout)?;
                in_code_block = true;
                current_language = Some(lang.to_string());
                code_buffer.clear();
                writeln!(stdout)?;
            }
            Event::End(TagEnd::CodeBlock) => {
                if in_code_block {
                    highlight_code(
                        &mut stdout,
                        &code_buffer,
                        current_language.as_deref(),
                        &SYNTAX_SET,
                        &CODE_THEME,
                    )?;
                    in_code_block = false;
                    code_buffer.clear();
                    current_language = None;
                    writeln!(stdout)?;
                }
            }
            Event::Text(text) => {
                if in_code_block {
                    // text borrows from the original event from the parser
                    code_buffer.push_str(text);
                } else {
                    // Clone the event. If text was borrowed, the clone still borrows.
                    event_buffer.push(event.clone());
                }
            }
            _ => {
                if !in_code_block {
                    // Clone the event.
                    event_buffer.push(event.clone());
                }
            }
        }
    }

    // Flush any remaining events in the buffer
    flush_markdown_buffer(&mut event_buffer, &skin, &mut stdout)?;

    Ok(())
}
