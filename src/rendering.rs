// src/rendering.rs
use anyhow::Result;
use lazy_static::lazy_static;
use std::io::{self, Write};
// Updated pulldown_cmark imports
use pulldown_cmark::{Event, Parser, Tag, CodeBlockKind, Options, TagEnd};
use syntect::{
    easy::HighlightLines,
    highlighting::{Color as SyntectColor, FontStyle, Style, Theme, ThemeSet},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};
use termimad::{
    crossterm::style::{
        Attribute, Color, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
    },
    MadSkin,
};

// --- Syntect Setup ---
lazy_static! {
    static ref SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
    static ref THEME_SET: ThemeSet = ThemeSet::load_defaults();
    static ref THEME_NAME: String = "base16-ocean.dark".to_string();
    static ref CODE_THEME: &'static Theme = THEME_SET.themes.get(&*THEME_NAME)
        .unwrap_or_else(|| &THEME_SET.themes["base16-ocean.dark"]);
}

// Helper to convert syntect Color to crossterm Color
fn syntect_to_crossterm_color(color: SyntectColor) -> Option<Color> {
    // Ignore fully transparent
    if color.a == 0 { None } else { Some(Color::Rgb { r: color.r, g: color.g, b: color.b }) }
}

// --- Syntect Code Highlighting Function ---
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
    let theme_bg = theme.settings.background.and_then(syntect_to_crossterm_color);
    let default_fg = theme.settings.foreground.and_then(syntect_to_crossterm_color);

    // Set overall background for the block
    if let Some(bg) = theme_bg {
        write!(writer, "{}", SetBackgroundColor(bg))?;
    } else {
        write!(writer, "{}", ResetColor)?;
    }
    // Set default foreground for the block
    if let Some(fg) = default_fg {
         write!(writer, "{}", SetForegroundColor(fg))?;
    }

    for line in LinesWithEndings::from(code) {
        // Reset attributes and apply default colors for the start of the line
        write!(writer, "{}", SetAttribute(Attribute::Reset))?;
        if let Some(bg) = theme_bg {
            write!(writer, "{}", SetBackgroundColor(bg))?;
        } else {
             write!(writer, "{}", ResetColor)?;
        }
        if let Some(fg) = default_fg {
             write!(writer, "{}", SetForegroundColor(fg))?;
        }

        let ranges: Vec<(Style, &str)> = highlighter
            .highlight_line(line, syntax_set)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        for (style, content) in ranges {
            let fg = style.foreground;
            let bg = style.background;

            // Apply style background if non-transparent, otherwise theme default is kept
            if bg.a > 0 { // a=0 is transparent
                if let Some(crossterm_bg) = syntect_to_crossterm_color(bg) {
                    write!(writer, "{}", SetBackgroundColor(crossterm_bg))?;
                } else { 
                    // If conversion fails, fall back to theme default bg
                    if let Some(theme_bg_color) = theme_bg {
                        write!(writer, "{}", SetBackgroundColor(theme_bg_color))?;
                    } else {
                        write!(writer, "{}", ResetColor)?; // Reset if no theme bg
                    }
                }
            }

            // Apply style foreground if non-transparent, otherwise theme default
            if fg.a > 0 { // a=0 is transparent
                if let Some(crossterm_fg) = syntect_to_crossterm_color(fg) {
                    write!(writer, "{}", SetForegroundColor(crossterm_fg))?;
                } else { 
                    // If conversion fails, fall back to theme default fg
                    if let Some(default_fg_color) = default_fg {
                        write!(writer, "{}", SetForegroundColor(default_fg_color))?;
                    } else {
                        write!(writer, "{}", ResetColor)?; // Reset fg
                        if let Some(bg) = theme_bg { write!(writer, "{}", SetBackgroundColor(bg))?; } // Reapply bg
                    }
                }
            } else {
                // Explicitly re-apply default foreground if style fg is transparent
                if let Some(default_fg_color) = default_fg {
                    write!(writer, "{}", SetForegroundColor(default_fg_color))?;
                } else {
                    write!(writer, "{}", ResetColor)?; // Reset fg
                    if let Some(bg) = theme_bg { write!(writer, "{}", SetBackgroundColor(bg))?; } // Reapply bg
                }
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

            // Reset attributes if any were applied for this segment
            // Reapply default colors after resetting attributes
            if applied_attrs {
                write!(writer, "{}", SetAttribute(Attribute::Reset))?;
                 if let Some(bg) = theme_bg { write!(writer, "{}", SetBackgroundColor(bg))?; } else { write!(writer, "{}", ResetColor)?; }
                 if let Some(fg) = default_fg { write!(writer, "{}", SetForegroundColor(fg))?; }
            }
        }
        // Reset colors at the end of each line to prevent color bleeding
        write!(writer, "{}", ResetColor)?;
    }

    // Reset colors completely after the code block
    write!(writer, "{}", ResetColor)?;
    Ok(())
}

// --- Termimad Skin Creation ---
fn create_skin() -> MadSkin {
    let mut skin = MadSkin::default_dark();
    skin.inline_code.set_fg(Color::Cyan);
    if let Some(theme_bg_color) = CODE_THEME.settings.background.and_then(syntect_to_crossterm_color) {
        if let Color::Rgb { r, g, b } = theme_bg_color {
             let inline_bg = Color::Rgb {
                r: r.saturating_add(10).min(255),
                g: g.saturating_add(10).min(255),
                b: b.saturating_add(10).min(255),
             };
             skin.inline_code.set_bg(inline_bg);
        }
    } else {
        skin.inline_code.set_bg(Color::Rgb { r: 30, g: 30, b: 30 });
    }
    skin.code_block.set_fg(Color::DarkGrey);
    skin.code_block.set_bg(Color::Rgb{ r: 20, g: 20, b: 20 });
    skin
}

// --- Main Printing Function ---
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

    writeln!(stdout)?;

    Ok(())
}

