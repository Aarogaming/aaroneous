use anyhow::{bail, Result};
use std::process::Command;
use std::path::Path;
use std::sync::Arc;
use crate::memory_pipeline::EpisodicInsertionPipeline;
use tracing::{info, error};

/// SEMANTIC-11: Local OCR Vision Parsing
/// Uses Tesseract or local vision models to extract semantic text from 
/// images, PDFs, and screenshots for zero-shot RAG ingestion.
pub struct OcrVisionParser {
    pipeline: Arc<EpisodicInsertionPipeline>,
}

impl OcrVisionParser {
    pub fn new(pipeline: Arc<EpisodicInsertionPipeline>) -> Self {
        Self { pipeline }
    }

    /// Spawns a background process to run local Tesseract OCR on an image file.
    pub fn parse_image_to_memory(&self, image_path: impl AsRef<Path>) -> Result<()> {
        let path = image_path.as_ref().to_path_buf();
        let pipeline = self.pipeline.clone();

        std::thread::spawn(move || {
            info!("Starting Local OCR on {:?}...", path);
            
            // Execute local tesseract CLI. 
            // In a production build without CLI dependencies, we would link leptess (Tesseract C-API)
            let output = Command::new("tesseract")
                .arg(&path)
                .arg("stdout") // Output to stdout
                .output();

            match output {
                Ok(out) if out.status.success() => {
                    let extracted_text = String::from_utf8_lossy(&out.stdout).into_owned();
                    let clean_text = extracted_text.trim();
                    
                    if !clean_text.is_empty() {
                        info!("OCR Success. Extracted {} bytes. Inserting to memory...", clean_text.len());
                        let _ = pipeline.embed_and_insert(clean_text, &format!("#ocr {:?}", path));
                    }
                }
                Ok(out) => {
                    error!("Tesseract failed: {}", String::from_utf8_lossy(&out.stderr));
                }
                Err(e) => {
                    error!("Failed to launch Tesseract (is it in PATH?): {}", e);
                }
            }
        });

        Ok(())
    }
}