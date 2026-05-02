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
        .arg("-png")
        // some versions use -singlefile, but we'll just let it generate fallback names
        .arg(pdf_path)
        .arg(&prefix)
        .status()?;

    if !status.success() {
        return Err(anyhow!("pdftoppm failed with status: {}", status));
    }

    // Try multiple possible output names
    let possible_outputs = [
        temp_dir.path().join("cover.png"),      // if -singlefile
        temp_dir.path().join("cover-1.png"),    // typical without -singlefile
        temp_dir.path().join("cover-01.png"),
        temp_dir.path().join("cover-001.png"),
    ];

    let mut found = false;
    for po in &possible_outputs {
        if po.exists() {
            std::fs::rename(po, output_png_path)?;
            found = true;
            break;
        }
    }

    if !found {
        return Err(anyhow!("pdftoppm did not produce any expected output files"));
    }

    Ok(())
}

pub fn generate_base64_cover(pdf_path: &Path) -> Result<String> {
    let mut temp_png = tempfile::NamedTempFile::new()?;
    let png_path = temp_png.path();
    
    // Use our existing function to generate the file
    generate_cover_from_pdf(pdf_path, png_path)?;
    
    // Read the file and encode to base64
    let bytes = std::fs::read(png_path)?;
    use base64::{Engine as _, engine::general_purpose};
    let b64 = general_purpose::STANDARD.encode(bytes);
    
    Ok(b64)
}
