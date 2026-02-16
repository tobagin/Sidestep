// Image downloader
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::{Context, Result};
use futures_util::StreamExt;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

/// Progress callback type
pub type ProgressCallback = Box<dyn Fn(u64, u64) + Send + Sync>;

/// Downloads images from remote URLs
pub struct ImageDownloader {
    client: reqwest::Client,
    download_dir: PathBuf,
}

impl ImageDownloader {
    pub fn new(download_dir: PathBuf) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Sidestep/0.1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self { client, download_dir }
    }

    /// Download a file only if it doesn't already exist with the correct checksum.
    /// If `expected_sha256` is provided and a local file matches, the download is skipped.
    /// Returns the local path either way.
    pub async fn download_if_needed(
        &self,
        url: &str,
        filename: &str,
        expected_sha256: Option<&str>,
        on_progress: Option<ProgressCallback>,
    ) -> Result<PathBuf> {
        let dest_path = self.download_dir.join(filename);

        if dest_path.exists() {
            match expected_sha256 {
                Some(expected) => {
                    // Have a checksum — validate the cached file
                    if let Ok(true) = crate::flashing::ChecksumVerifier::verify(&dest_path, expected) {
                        log::info!("Skipping download of {} — cached copy matches checksum", filename);
                        if let Some(ref cb) = on_progress {
                            let size = tokio::fs::metadata(&dest_path).await?.len();
                            cb(size, size);
                        }
                        return Ok(dest_path);
                    }
                    log::info!("Cached {} has wrong checksum, re-downloading", filename);
                }
                None => {
                    // No checksum — trust the cached file if it exists and is non-empty
                    if let Ok(meta) = tokio::fs::metadata(&dest_path).await {
                        if meta.len() > 0 {
                            log::info!("Skipping download of {} — cached copy exists", filename);
                            if let Some(ref cb) = on_progress {
                                cb(meta.len(), meta.len());
                            }
                            return Ok(dest_path);
                        }
                    }
                }
            }
        }

        self.download(url, filename, on_progress).await
    }

    /// Download a file with progress reporting
    pub async fn download(
        &self,
        url: &str,
        filename: &str,
        on_progress: Option<ProgressCallback>,
    ) -> Result<PathBuf> {
        log::info!("Downloading {} from {}", filename, url);

        // Create download directory if needed
        tokio::fs::create_dir_all(&self.download_dir)
            .await
            .context("Failed to create download directory")?;

        let dest_path = self.download_dir.join(filename);

        // Start the download
        let response = self.client
            .get(url)
            .send()
            .await
            .context("Failed to start download")?;

        let total_size = response.content_length().unwrap_or(0);
        log::debug!("Download size: {} bytes", total_size);

        // Open destination file
        let mut file = File::create(&dest_path)
            .await
            .context("Failed to create destination file")?;

        // Stream the download
        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = 0;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Error reading download chunk")?;
            file.write_all(&chunk)
                .await
                .context("Error writing to file")?;

            downloaded += chunk.len() as u64;

            if let Some(ref callback) = on_progress {
                callback(downloaded, total_size);
            }
        }

        file.flush().await?;
        log::info!("Download complete: {}", dest_path.display());

        Ok(dest_path)
    }

    /// Download checksum file and parse it
    pub async fn download_checksums(&self, url: &str) -> Result<std::collections::HashMap<String, String>> {
        log::debug!("Downloading checksums from {}", url);

        let response = self.client
            .get(url)
            .send()
            .await
            .context("Failed to download checksums")?;

        let text = response.text().await?;
        let mut checksums = std::collections::HashMap::new();

        // Parse SHA256SUMS format: "hash  filename"
        for line in text.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let hash = parts[0].to_string();
                let filename = parts[1].trim_start_matches('*').to_string();
                checksums.insert(filename, hash);
            }
        }

        Ok(checksums)
    }
}

impl Default for ImageDownloader {
    fn default() -> Self {
        let download_dir = dirs::download_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("sidestep");
        Self::new(download_dir)
    }
}
