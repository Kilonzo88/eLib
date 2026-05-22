use anyhow::{Result, anyhow};
use std::process::Command;
use std::path::Path;

pub fn generate_cover_from_pdf(pdf_path: &Path, output_png_path: &Path) -> Result<()> {
    // We create a temporary directory just for the output to ensure we don't clobber
    // or mis-guess the filename.
    let temp_dir = tempfile::tempdir()?;
    let prefix = temp_dir.path().join("cover");
    
    let status = Command::new("pdftoppm")
        .arg("-f")
        .arg("1")
        .arg("-l")
        .arg("1")
        .arg("-singlefile")
        .arg("-png")
        .arg(pdf_path)
        .arg(&prefix)
        .status()?;

    if !status.success() {
        return Err(anyhow!("pdftoppm failed with status: {}", status));
    }

    let output_file = temp_dir.path().join("cover.png");
    if output_file.exists() {
        std::fs::rename(output_file, output_png_path)?;
        Ok(())
    } else {
        Err(anyhow!("pdftoppm did not produce the expected cover.png file"))
    }
}

pub fn generate_base64_cover(pdf_path: &Path) -> Result<String> {
    let temp_png = tempfile::NamedTempFile::new()?;
    let png_path = temp_png.path();
    
    // Use our existing function to generate the file
    generate_cover_from_pdf(pdf_path, png_path)?;
    
    // Read the file and encode to base64
    let bytes = std::fs::read(png_path)?;
    use base64::{Engine as _, engine::general_purpose};
    let b64 = general_purpose::STANDARD.encode(bytes);
    
    Ok(b64)
}
