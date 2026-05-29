use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BookMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
}

pub fn extract_text_from_pdf(pdf_path: &std::path::Path) -> Result<String> {
    let text = pdf_extract::extract_text(pdf_path)?;
    Ok(text)
}

/// Convert PDF plain text into structured HTML chapters.
/// Uses heuristics to detect headings, paragraphs, and structural elements.
pub fn text_to_html_chapters(text: &str) -> Vec<PdfChapter> {
    let lines: Vec<&str> = text.lines().collect();
    let mut chapters = Vec::new();
    let mut current_paragraphs = Vec::new();
    let mut current_title: Option<String> = None;
    let mut in_paragraph = String::new();

    let is_heading = |line: &str| -> bool {
        let trimmed = line.trim(); // Removes leading and trailing whitespace
        if trimmed.is_empty() {
            return false;
        }
        // Short lines (under 80 chars) that look like titles
        if trimmed.len() < 80 {
            //Checks if the line is in all caps and has more than 3 characters
            if trimmed.to_uppercase() == trimmed && trimmed.len() > 3 {
                return true;
            }
            //Checks if the line is a Roman numeral or numbered chapter
            let upper = trimmed.to_uppercase();
            if upper.starts_with("CHAPTER ")
                || upper.starts_with("PART ")
                || upper.starts_with("BOOK ")
                || upper.starts_with("VOLUME ")
                || upper.starts_with("SECTION ")
                || upper.starts_with("UNIT ")
                || upper.starts_with("STEP ")
                || upper.starts_with("SCENE ")
            {
                return true;
            }
            // Single roman numeral line
            if trimmed.chars().all(|c| "IVXLCDM. ".contains(c)) && trimmed.len() < 15 {
                return true;
            }
        }
        false
    };

    let flush_paragraph = |buffer: &mut String, paragraphs: &mut Vec<String>| {
        let trimmed = buffer.trim();
        if !trimmed.is_empty() {
            paragraphs.push(trimmed.to_string());
        }
        buffer.clear();
    };

    for (_i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Detect chapter/section headings
        if is_heading(trimmed) {
            // Flush any existing content as a chapter
            flush_paragraph(&mut in_paragraph, &mut current_paragraphs);
            if !current_paragraphs.is_empty() || current_title.is_some() {
                let html = paragraphs_to_html(&current_paragraphs);
                chapters.push(PdfChapter {
                    title: current_title.clone(),
                    html,
                });
                current_paragraphs.clear();
            }
            current_title = Some(trimmed.to_string());
            continue;
        }

        // Empty line = paragraph break
        if trimmed.is_empty() {
            flush_paragraph(&mut in_paragraph, &mut current_paragraphs);
            continue;
        }

        // Check for paragraph continuation (indented or normal flow)
        // If the previous line ended with a sentence terminator, this might be a new paragraph
        let prev_ended = in_paragraph.trim_end().ends_with('.')
            || in_paragraph.trim_end().ends_with('!')
            || in_paragraph.trim_end().ends_with('?')
            || in_paragraph.trim_end().ends_with('"')
            || in_paragraph.trim_end().ends_with('\'');

        // If line starts with indentation (3+ spaces) and previous ended, it's a new paragraph
        let is_indented = line.starts_with("   ") || line.starts_with("\t");

        if !in_paragraph.is_empty() && prev_ended && (is_indented || trimmed.len() < 50) {
            flush_paragraph(&mut in_paragraph, &mut current_paragraphs);
        }

        if !in_paragraph.is_empty() {
            in_paragraph.push(' ');
        }
        in_paragraph.push_str(trimmed);
    }

    // Flush final paragraph and chapter
    flush_paragraph(&mut in_paragraph, &mut current_paragraphs);
    if !current_paragraphs.is_empty() || current_title.is_some() {
        let html = paragraphs_to_html(&current_paragraphs);
        chapters.push(PdfChapter {
            title: current_title,
            html,
        });
    }

    // If no chapters were created (no headings detected), treat everything as one chapter
    if chapters.is_empty() && !text.trim().is_empty() {
        chapters.push(PdfChapter {
            title: None,
            html: paragraphs_to_html(&[text.trim().to_string()]),
        });
    }

    chapters
}

#[derive(Debug, Clone)]
pub struct PdfChapter {
    pub title: Option<String>,
    pub html: String,
}

/// Convert plain text paragraphs into HTML with structural markup.
fn paragraphs_to_html(paragraphs: &[String]) -> String {
    let mut html = String::new();

    for (i, para) in paragraphs.iter().enumerate() {
        let trimmed = para.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Detect blockquotes (lines starting with ")
        let is_blockquote = trimmed.starts_with('"') && trimmed.ends_with('"');

        // Detect list items
        let is_list_item = trimmed.starts_with("- ") || trimmed.starts_with("* ");

        if is_blockquote {
            html.push_str("<blockquote>");
            html.push_str(&html_escape(trimmed));
            html.push_str("</blockquote>\n");
        } else if is_list_item {
            html.push_str("<ul>\n");
            for line in trimmed.split('\n') {
                let line_trimmed = line.trim();
                if line_trimmed.starts_with("- ") || line_trimmed.starts_with("* ") {
                    html.push_str("<li>");
                    html.push_str(&html_escape(&line_trimmed[2..]));
                    html.push_str("</li>\n");
                }
            }
            html.push_str("</ul>\n");
        } else {
            // Detect inline bold/italic markers and convert to HTML
            let mut formatted = html_escape(trimmed);

            // Convert **bold** to <strong>bold</strong>
            formatted = replace_inline_markers(&formatted, "**", "<strong>", "</strong>");
            // Convert *italic* to <em>italic</em> (but not already converted **)
            formatted = replace_inline_markers(&formatted, "*", "<em>", "</em>");
            // Convert _italic_ to <em>italic</em>
            formatted = replace_inline_markers(&formatted, "_", "<em>", "</em>");

            html.push_str("<p>");
            html.push_str(&formatted);
            html.push_str("</p>\n");
        }

        // Add section break between major paragraphs
        if i < paragraphs.len() - 1 {
            let next = &paragraphs[i + 1];
            if trimmed.len() > 200 && next.len() > 200 {
                // Long paragraphs — add extra spacing via CSS instead of <br/>
            }
        }
    }

    html
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Replace inline markdown-style markers with HTML tags.
/// Handles paired markers like **text** or *text*.
fn replace_inline_markers(text: &str, marker: &str, open_tag: &str, close_tag: &str) -> String {
    let mut result = String::new();
    let mut last_end = 0;
    let mut in_marker = false;

    while let Some(start) = text[last_end..].find(marker) {
        let absolute_start = last_end + start;
        if in_marker {
            result.push_str(&text[last_end..absolute_start]);
            result.push_str(close_tag);
            in_marker = false;
        } else {
            result.push_str(&text[last_end..absolute_start]);
            result.push_str(open_tag);
            in_marker = true;
        }
        last_end = absolute_start + marker.len();
    }
    result.push_str(&text[last_end..]);

    // Close unclosed marker
    if in_marker {
        result.push_str(close_tag);
    }

    result
}

pub fn extract_metadata(pdf_path: &std::path::Path) -> Result<BookMetadata> {
    use lopdf::Document;

    let doc = Document::load(pdf_path)?;
    let mut metadata = BookMetadata {
        title: None,
        author: None,
    };

    if let Ok(info) = doc.get_dictionary(doc.trailer.get(b"Info")?.as_reference()?) {
        if let Ok(title) = info.get(b"Title") {
            metadata.title = title
                .as_str()
                .map(|s| String::from_utf8_lossy(s).into_owned())
                .ok();
        }
        if let Ok(author) = info.get(b"Author") {
            metadata.author = author
                .as_str()
                .map(|s| String::from_utf8_lossy(s).into_owned())
                .ok();
        }
    }

    Ok(metadata)
}
