use anyhow::{Result, anyhow};
use std::io::Write;
use epub::doc::EpubDoc;
use crate::services::pdf_service::BookMetadata;

/// Extracts all text from an EPUB file, concatenating chapters in spine order.
pub fn extract_text_from_epub(epub_data: &[u8]) -> Result<String> {
    // The epub crate requires a file path, so we write to a temp file.
    let mut temp_file = tempfile::NamedTempFile::new()?;
    temp_file.write_all(epub_data)?;

    let mut doc = EpubDoc::new(temp_file.path())
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
        doc.go_next();
    }

    if full_text.trim().is_empty() {
        return Err(anyhow!("No text content found in EPUB"));
    }

    Ok(full_text)
}

/// Extracts title and author from EPUB Dublin Core metadata.
pub fn extract_metadata_from_epub(epub_data: &[u8]) -> Result<BookMetadata> {
    let mut temp_file = tempfile::NamedTempFile::new()?;
    temp_file.write_all(epub_data)?;

    let doc = EpubDoc::new(temp_file.path())
        .map_err(|e| anyhow!("Failed to open EPUB: {:?}", e))?;

    let title = doc.mdata("title").map(|m| m.value.clone()).filter(|s| !s.trim().is_empty());
    let author = doc.mdata("creator").map(|m| m.value.clone()).filter(|s| !s.trim().is_empty());

    Ok(BookMetadata { title, author })
}

/// Extracts the cover image bytes and MIME type from an EPUB file.
/// Returns None if no cover is found rather than failing.
pub fn extract_cover_from_epub(epub_data: &[u8]) -> Option<(Vec<u8>, String)> {
    let mut temp_file = tempfile::NamedTempFile::new().ok()?;
    temp_file.write_all(epub_data).ok()?;

    let mut doc = EpubDoc::new(temp_file.path()).ok()?;

    // Most EPUBs declare a cover via a metadata item with name="cover"
    // The `epub` crate has a get_cover() helper for this.
    let (cover_bytes, mime) = doc.get_cover()?;
    Some((cover_bytes, mime))
}
