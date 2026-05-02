use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BookMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
}

pub fn extract_text_from_pdf(pdf_data: &[u8]) -> Result<String> {
    // Create a temporary file or use a cursor if pdf-extract supports it
    // pdf-extract usually takes a path, so we'll use a temp file for now.
    use std::io::Write;
    let mut temp_file = tempfile::NamedTempFile::new()?;
    temp_file.write_all(pdf_data)?;
    
    let text = pdf_extract::extract_text(temp_file.path())?;
    Ok(text)
}

pub fn extract_metadata(pdf_data: &[u8]) -> Result<BookMetadata> {
    use lopdf::Document;
    use std::io::Cursor;

    let doc = Document::load_from(Cursor::new(pdf_data))?;
    let mut metadata = BookMetadata {
        title: None,
        author: None,
    };

    if let Ok(info) = doc.get_dictionary(doc.trailer.get(b"Info")?.as_reference()?) {
        if let Ok(title) = info.get(b"Title") {
            metadata.title = title.as_str().map(|s| String::from_utf8_lossy(s).into_owned()).ok();
        }
        if let Ok(author) = info.get(b"Author") {
            metadata.author = author.as_str().map(|s| String::from_utf8_lossy(s).into_owned()).ok();
        }
    }

    Ok(metadata)
}
