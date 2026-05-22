use anyhow::{Result, anyhow};
use epub::doc::EpubDoc;
use crate::services::pdf_service::BookMetadata;

/// Extracts all text from an EPUB file, concatenating chapters in spine order.
pub fn extract_text_from_epub(epub_path: &std::path::Path) -> Result<String> {
    let mut doc = EpubDoc::new(epub_path)
        .map_err(|e| anyhow!("Failed to open EPUB: {:?}", e))?;

    let mut full_text = String::new();

    // Iterate through every item in the spine (ordered reading sequence)
    let num_chapters = doc.get_num_chapters();
    for _ in 0..num_chapters {
        if let Some((content_bytes, _mime)) = doc.get_current() {
            // Convert XHTML chapter bytes → plain text via html2text
            let text = html2text::from_read(content_bytes.as_slice(), usize::MAX).unwrap_or_default();
            full_text.push_str(&text);
            full_text.push('\n');
        }
        let _ = doc.go_next();
    }

    if full_text.trim().is_empty() {
        return Err(anyhow!("No text content found in EPUB"));
    }

    Ok(full_text)
}

/// Structural chapter data extracted directly from EPUB
pub struct ExtractedChapter {
    pub title: Option<String>,
    pub html: String,
}

/// Extract chapter title from HTML by looking for h1, h2, or title tags.
fn extract_title_from_html(html: &str) -> Option<String> {
    // Try h1 first
    for tag in ["h1", "h2", "h3", "title"] {
        let open = format!("<{}>", tag);
        let close = format!("</{}>", tag);
        if let Some(start) = html.find(&open) {
            let content_start = start + open.len();
            if let Some(end) = html[content_start..].find(&close) {
                let title = html[content_start..content_start + end].trim();
                if !title.is_empty() {
                    // Strip any inner HTML tags for clean title
                    let clean = title.replace(|c: char| c == '<' || c == '>', " ");
                    return Some(clean.trim().to_string());
                }
            }
        }
        // Also try with attributes: <h1 class="...">
        if let Some(start) = html.to_lowercase().find(&format!("<{}", tag)) {
            if let Some(tag_end) = html[start..].find('>') {
                let content_start = start + tag_end + 1;
                if let Some(end) = html[content_start..].find(&close) {
                    let title = html[content_start..content_start + end].trim();
                    if !title.is_empty() {
                        let clean = title.replace(|c: char| c == '<' || c == '>', " ");
                        return Some(clean.trim().to_string());
                    }
                }
            }
        }
    }
    None
}

/// Extracts HTML chapters from an EPUB without stripping tags, for native rendering.
pub fn extract_chapters_from_epub(epub_path: &std::path::Path) -> Result<Vec<ExtractedChapter>> {
    let mut doc = EpubDoc::new(epub_path)
        .map_err(|e| anyhow!("Failed to open EPUB: {:?}", e))?;

    let mut chapters = Vec::new();

    let num_chapters = doc.get_num_chapters();
    for chapter_idx in 0..num_chapters {
        if let Some((content_bytes, _mime)) = doc.get_current() {
            let html = String::from_utf8_lossy(&content_bytes).to_string();

            // Extract title before we modify HTML
            let mut title = extract_title_from_html(&html);

            // Fallback: try to get title from EPUB TOC
            if title.is_none() {
                title = doc.toc.get(chapter_idx).and_then(|t| {
                    if t.label.trim().is_empty() { None } else { Some(t.label.clone()) }
                });
            }

            // Basic sanitization: extract just the <body> content if present,
            // otherwise use raw. The frontend will render it.
            let mut final_html = html.clone();
            if let Some(body_start) = html.to_lowercase().find("<body") {
                if let Some(body_end_tag) = html.to_lowercase().find("</body>") {
                    let start_idx = html[body_start..].find('>').unwrap_or(0) + body_start + 1;
                    final_html = html[start_idx..body_end_tag].to_string();
                }
            }

            chapters.push(ExtractedChapter {
                title,
                html: final_html,
            });
        }
        let _ = doc.go_next();
    }

    Ok(chapters)
}

/// Extracts title and author from EPUB Dublin Core metadata.
pub fn extract_metadata_from_epub(epub_path: &std::path::Path) -> Result<BookMetadata> {
    let doc = EpubDoc::new(epub_path)
        .map_err(|e| anyhow!("Failed to open EPUB: {:?}", e))?;

    let title = doc.mdata("title").map(|m| m.value.clone()).filter(|s| !s.trim().is_empty());
    let author = doc.mdata("creator").map(|m| m.value.clone()).filter(|s| !s.trim().is_empty());

    Ok(BookMetadata { title, author })
}

/// Extracts the cover image bytes and MIME type from an EPUB file.
/// Returns None if no cover is found rather than failing.
pub fn extract_cover_from_epub(epub_path: &std::path::Path) -> Option<(Vec<u8>, String)> {
    let mut doc = EpubDoc::new(epub_path).ok()?;

    // Most EPUBs declare a cover via a metadata item with name="cover"
    // The `epub` crate has a get_cover() helper for this.
    let (cover_bytes, mime) = doc.get_cover()?;
    Some((cover_bytes, mime))
}
