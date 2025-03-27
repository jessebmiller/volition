// src/rendering.rs
use anyhow::Result;
use lazy_static::lazy_static;
use std::io::{self, Write};
use pulldown_cmark::{Event, Parser, Tag, CodeBlockKind, Options, TagEnd};
use syntect::{
    easy::HighlightLines,
    highlighting::{Color as SyntectColor, FontStyle, Style, Theme, ThemeSet},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};
use termimad::{
    crossterm::style::{
        Attribute, Color, ResetColor, SetAttribute, SetForegroundColor, /* Removed SetBackgroundColor */
    },
    MadSkin,
};

// --- Syntect Setup ---
lazy_static! {
    static ref SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
    static ref THEME_SET: ThemeSet = ThemeSet::load_defaults();
    // Theme is still needed for foreground color information
    static ref THEME_NAME: String = "base16-ocean.dark".to_string();
    static ref CODE_THEME: &'static Theme = THEME_SET.themes.get(&*THEME_NAME)
        .unwrap_or_else(|| &THEME_SET.themes["base16-ocean.dark"]);
}

// Helper to convert syntect Color to crossterm Color
fn syntect_to_crossterm_color(color: SyntectColor) -> Option<Color> {
    // Ignore fully transparent
    if color.a == 0 { None } else { Some(Color::Rgb { r: color.r, g: color.g, b: color.b }) }
}

// --- Syntect Code Highlighting Function (Simplified Colors) ---
fn highlight_code<W: Write>(
    writer: &mut W,
    code: &str,
    language: Option<&str>,
    syntax_set: &SyntaxSet,
    theme: &Theme,
) -> Result<(), io::Error> {
    let lower_lang = language.map(|l| l.to_lowercase());
    let lang_token_opt = lower_lang.as_deref().map(|lang_str| {
        match lang_str {
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
        }
    });

    let syntax = lang_token_opt
        .and_then(|token| syntax_set.find_syntax_by_token(token))
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

    let mut highlighter = HighlightLines::new(syntax, theme);

    // Initial reset before starting
    write!(writer, "{}", ResetColor)?;

    for line in LinesWithEndings::from(code) {
        // Reset everything at the start of the line
        write!(writer, "{}", ResetColor)?;
        write!(writer, "{}", SetAttribute(Attribute::Reset))?;

        let ranges: Vec<(Style, &str)> = highlighter
            .highlight_line(line, syntax_set)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        for (style, content) in ranges {
            let fg = style.foreground;

            // Apply style foreground if non-transparent
            if fg.a > 0 { // a=0 is transparent
                if let Some(crossterm_fg) = syntect_to_crossterm_color(fg) {
                    write!(writer, "{}", SetForegroundColor(crossterm_fg))?;
                } else {
                    write!(writer, "{}", ResetColor)?;
                }
            } else {
                write!(writer, "{}", ResetColor)?;
            }

            // Apply font styles
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

            // Reset attributes and color after each segment
            if applied_attrs {
                write!(writer, "{}", SetAttribute(Attribute::Reset))?;
            }
            write!(writer, "{}", ResetColor)?;
        }
    }

    // Final reset after the whole block
    write!(writer, "{}", ResetColor)?;
    Ok(())
}

// --- Termimad Skin Creation (Simplified) ---
fn create_skin() -> MadSkin {
    let mut skin = MadSkin::default();

    // Keep inline code slightly distinct with just foreground color
    skin.inline_code.set_fg(Color::Cyan);
    // Use Color::Reset to unset the background color
    skin.inline_code.set_bg(Color::Reset);

    // Use Color::Reset to unset specific code block colors
    skin.code_block.set_fg(Color::Reset);
    skin.code_block.set_bg(Color::Reset);

    // Example: Customize headers or bold if desired
    // skin.headers[0].set_fg(Color::Magenta);
    // skin.bold.set_fg(Color::Yellow);

    skin
}

// --- Main Printing Function (Unchanged) ---
pub fn print_formatted(markdown_text: &str) -> Result<()> {
    let skin = create_skin();
    let mut stdout = io::stdout().lock();

    let parser = Parser::new_ext(markdown_text, Options::empty());

    let mut markdown_buffer = String::new();
    let mut code_buffer = String::new();
    let mut current_language: Option<String> = None;
    let mut in_code_block = false;

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {
                if !markdown_buffer.is_empty() {
                    skin.write_text_on(&mut stdout, &markdown_buffer)?;
                    markdown_buffer.clear();
                }
                in_code_block = true;
                current_language = Some(lang.into_string());
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
                    code_buffer.push_str(&text);
                } else {
                    markdown_buffer.push_str(&text);
                }
            }
            evt => {
                 if !in_code_block {
                     match evt {
                         Event::SoftBreak => markdown_buffer.push('\n'),
                         Event::HardBreak => markdown_buffer.push_str("\n\n"),
                         Event::Start(Tag::Paragraph) => {} 
                         Event::End(TagEnd::Paragraph) => markdown_buffer.push_str("\n\n"),
                         Event::Start(Tag::Heading { level, .. }) => {
                             markdown_buffer.push('\n');
                             for _ in 0..level as usize { markdown_buffer.push('#'); }
                             markdown_buffer.push(' ');
                         }
                         Event::End(TagEnd::Heading(..)) => markdown_buffer.push_str("\n\n"),
                         Event::Start(Tag::List(_)) => {} 
                         Event::End(TagEnd::List(..)) => markdown_buffer.push('\n'),
                         Event::Start(Tag::Item) => markdown_buffer.push_str("* "),
                         Event::End(TagEnd::Item) => markdown_buffer.push('\n'),
                         Event::Start(Tag::Emphasis) => markdown_buffer.push('*'),
                         Event::End(TagEnd::Emphasis) => markdown_buffer.push('*'),
                         Event::Start(Tag::Strong) => markdown_buffer.push_str("**"),
                         Event::End(TagEnd::Strong) => markdown_buffer.push_str("**"),
                         _ => {} 
                     }
                 }
            }
        }
    }

    if !markdown_buffer.is_empty() {
        skin.write_text_on(&mut stdout, &markdown_buffer)?;
    }

    Ok(())
}

