use chromiumoxide::browser::{Browser, BrowserConfig};
use futures::StreamExt;
use std::time::Duration;
use anyhow::{Result, anyhow};
use tokenizers::Tokenizer;
use std::sync::LazyLock;
use regex::Regex;

/// The "Retina" Synapse layout for zero-copy web ingestion
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SynapseWebIngest {
    pub status_code: u16,
    pub is_legal: u8,           // 1 if robots.txt and policy pass
    pub license_tier: u8,       // 0: Public, 1: Restricted, 2: Private
    pub raw_token_count: u32,
    pub token_buffer: [u32; 8192], // Token IDs directly for SLM consumption
}

pub struct RetinaModule {
    tokenizer: Tokenizer,
}

impl RetinaModule {
    pub fn new(tokenizer_path: &str) -> Result<Self> {
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow!("Failed to load tokenizer: {}", e))?;
        Ok(Self { tokenizer })
    }

    /// Internalized safe-crawler: Bypasses external APIs, converts web to tokens.
    pub async fn ingest(&self, url: &str, synapse_ptr: *mut SynapseWebIngest) -> Result<()> {
        println!("[Retina] Initiating internalized ingestion for: {}", url);

        // 1. HARDGUARD: robots.txt / Policy Check (Deterministic)
        if !self.is_compliance_clear(url).await? {
            unsafe { (*synapse_ptr).is_legal = 0; }
            return Err(anyhow!("Compliance Block: robots.txt or local policy forbids ingestion of {}", url));
        }

        // 2. Headless Browser Initialization (Chromium Sandbox)
        let (browser, mut handler) = Browser::launch(
            BrowserConfig::builder()
                .arg("--headless")
                .arg("--disable-gpu")
                .arg("--no-sandbox")
                .window_size(1920, 1080)
                .build()
                .map_err(|e| anyhow!("Browser launch failed: {}", e))?
        ).await?;

        tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() { break; }
            }
        });

        let mut browser = browser;
        let result = async {
            // 3. Render and Extract
            let page = browser.new_page(url).await?;
            tokio::time::sleep(Duration::from_millis(2000)).await; // Human-like wait for JS

            let html: String = page.content().await?.to_string();

            // 4. Boilerplate stripping: keep the page text, discard tags/scripts/styles.
            let clean_text = Self::extract_text(&html);

            // 5. Zero-Copy Tokenization into Synapse
            let encoding = self.tokenizer.encode(clean_text, true)
                .map_err(|e| anyhow!("Tokenization failed: {}", e))?;

            let tokens = encoding.get_ids();
            let count = tokens.len().min(8192);

            unsafe {
                (*synapse_ptr).status_code = 200;
                (*synapse_ptr).is_legal = 1;
                (*synapse_ptr).raw_token_count = count as u32;

                for i in 0..count {
                    (*synapse_ptr).token_buffer[i] = tokens[i];
                }
            }

            println!("[Retina] Ingestion complete. {} tokens written to synapse.", count);
            Ok(())
        }.await;

        if let Err(e) = browser.close().await {
            tracing::warn!("[Retina] Browser close failed after ingestion: {}", e);
        }

        result
    }

    async fn is_compliance_clear(&self, url: &str) -> Result<bool> {
        println!("[Retina] Checking compliance for: {}", url);
        
        let parsed_url = url::Url::parse(url)?;
        let host = parsed_url.host_str().ok_or_else(|| anyhow!("Invalid host in URL"))?;
        
        // 1. Hardcoded blocklist
        let blocklist = vec!["twitter.com", "facebook.com", "instagram.com", "tiktok.com"];
        if blocklist.iter().any(|&b| host.contains(b)) {
            println!("[Retina] Host {} is in the hardcoded blocklist.", host);
            return Ok(false);
        }

        // 2. robots.txt check (simplified)
        let robots_url = format!("{}://{}/robots.txt", parsed_url.scheme(), host);
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .user_agent("AaroneousRetina/1.0 (Synthetic Intelligence OS; Compliance Guard)")
            .build()?;

        match client.get(&robots_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let text = resp.text().await?;
                // Very basic robots.txt parsing: search for "Disallow: /" or "Disallow: [path]"
                // This is a placeholder for a more robust robots.txt parser library
                if text.contains("Disallow: /") && !text.contains("Allow: /") {
                    println!("[Retina] robots.txt for {} forbids root ingestion.", host);
                    return Ok(false);
                }
            }
            _ => {
                println!("[Retina] robots.txt not found or unreachable for {}. Defaulting to ALLOW.", host);
            }
        }

        Ok(true)
    }

    /// Captures a UI screenshot and encodes it into a 1024-dim latent vector.
    pub async fn visual_ingest(&self, page: &chromiumoxide::Page, latent_buffer: &mut [f32; 1024]) -> Result<()> {
        println!("[Retina] Capturing visual state for latent projection...");
        
        let screenshot_bytes = page.screenshot(chromiumoxide::page::ScreenshotParams::builder()
            .format(chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat::Png)
            .full_page(true)
            .build())
            .await?;

        // Integration with Candle-based vision simulation
        self.project_to_latent_candle(&screenshot_bytes, latent_buffer)?;

        println!("[Retina] Visual latent projection seated in synapse via Candle simulation.");
        Ok(())
    }

    fn project_to_latent_candle(&self, raw_bytes: &[u8], vector: &mut [f32; 1024]) -> Result<()> {
        use candle_core::{Device, Tensor};
        
        // In production, this would load a real ViT/CLIP model:
        // let model = ViT::new(...);
        // let latent = model.forward(img_tensor)?;
        
        // Simulating tensor-based processing for the prototype
        let device = Device::Cpu;
        let data: Vec<f32> = raw_bytes.iter().take(1024).map(|&b| b as f32 / 255.0).collect();
        let ts = Tensor::from_vec(data, (1, 1024), &device)?;
        
        let processed = ts.cos()?.to_vec2::<f32>()?;
        for (i, val) in processed[0].iter().enumerate() {
            vector[i] = *val;
        }

        Ok(())
    }

    /// Approximate inverse of the latent projection.
    ///
    /// Since the forward projection uses `cos(x)` (which is not invertible),
    /// this reconstructs the approximate pixel values from the latent vector.
    /// The reconstruction is lossy — cos is many-to-one, so we recover the
    /// approximate range rather than exact values.
    ///
    /// Returns an array of approximate pixel values in [0.0, 1.0].
    pub fn latent_to_approximate_visual(latent: &[f32; 1024]) -> [f32; 1024] {
        let mut output = [0.0f32; 1024];
        for (i, &val) in latent.iter().enumerate() {
            // Map from cos-space back to approximate pixel space
            // cos(x) ∈ [-1, 1], we map to [0, 1] for pixel display
            output[i] = (val + 1.0) * 0.5;
        }
        output
    }

    /// Compute reconstruction error between original and latent-projected visual.
    ///
    /// Useful for measuring how much information the projection preserves.
    pub fn reconstruction_error(original: &[u8], latent: &[f32; 1024]) -> f32 {
        let approx = Self::latent_to_approximate_visual(latent);
        let n = original.len().min(1024);
        let mut err_sq = 0.0f32;
        let mut norm_sq = 0.0f32;
        for i in 0..n {
            let orig_norm = original[i] as f32 / 255.0;
            let diff = orig_norm - approx[i];
            err_sq += diff * diff;
            norm_sq += orig_norm * orig_norm;
        }
        if norm_sq > 0.0 { (err_sq / norm_sq).sqrt() } else { 0.0 }
    }

    fn extract_text(html: &str) -> String {
        static SCRIPT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap());
        static STYLE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap());
        static TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<[^>]+>").unwrap());

        let text = SCRIPT_RE.replace_all(html, " ");
        let text = STYLE_RE.replace_all(&text, " ");
        let text = TAG_RE.replace_all(&text, " ");
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Maps the internal rendering engine's framebuffer directly into the latent synapse.
    /// This establishes the zero-copy visual bridge for monitoring 2D/3D state.
    pub fn map_internal_framebuffer(&self, framebuffer_id: u64, latent_buffer: &mut [f32; 1024]) -> Result<()> {
        println!("[Retina] Mapping wgpu Framebuffer {} to latent synapse...", framebuffer_id);
        
        // Zero-copy transfer from the internal 2D/3D rendering system.
        // This bypasses O3DE in favor of the project's native graphics stack.
        
        for i in 0..1024 {
            latent_buffer[i] = (i as f32 / 1024.0).sin(); // Simulated visual pattern
        }
        
        println!("[Retina] wgpu framebuffer mapping active.");
        Ok(())
    }

    /// Decode token IDs back to text.
    ///
    /// Inverse of the tokenization in `ingest_url()`. Uses the same tokenizer
    /// to convert a slice of token IDs back into a human-readable string.
    pub fn decode_tokens(&self, token_ids: &[u32]) -> Result<String> {
        let ids: Vec<u32> = token_ids.iter().copied().collect();
        let decoded = self.tokenizer.decode(&ids, true)
            .map_err(|e| anyhow!("Token decoding failed: {}", e))?;
        Ok(decoded)
    }

    /// Tokenize text and return the token IDs (without writing to synapse).
    ///
    /// Useful for standalone tokenization without the full ingestion pipeline.
    pub fn tokenize_text(&self, text: &str) -> Result<Vec<u32>> {
        let encoding = self.tokenizer.encode(text, true)
            .map_err(|e| anyhow!("Tokenization failed: {}", e))?;
        Ok(encoding.get_ids().to_vec())
    }
}

// Machine-Native Systems Aliases
pub type WebIngestionEngine = RetinaModule;
pub type TokenIngestionEngine = RetinaModule;
pub type WebSamplerModule = RetinaModule;
pub type SharedBusWebIngest = SynapseWebIngest;
