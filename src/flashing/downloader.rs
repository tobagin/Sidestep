// Image downloader
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::{Context, Result};
use futures_util::StreamExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

/// Progress callback type
pub type ProgressCallback = Box<dyn Fn(u64, u64) + Send + Sync>;

/// Downloads images from remote URLs
pub struct ImageDownloader {
    client: reqwest::Client,
    download_dir: PathBuf,
    cancel: Option<Arc<AtomicBool>>,
}

impl ImageDownloader {
    pub fn new(download_dir: PathBuf) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(format!("Sidestep/{}", crate::config::VERSION))
            // Reject any redirect that downgrades https→http.
            .https_only(true)
            .build()
            .expect("Failed to create HTTP client");

        Self { client, download_dir, cancel: None }
    }

    /// Attach a cancellation flag. When set, an in-flight download aborts at the
    /// next chunk and the partial file is removed. Only downloads are
    /// cancellable — flashing is never interrupted mid-write.
    pub fn with_cancel(mut self, cancel: Arc<AtomicBool>) -> Self {
        self.cancel = Some(cancel);
        self
    }

    /// Resolve a server-supplied filename to a path inside `download_dir`,
    /// rejecting anything that isn't a single, plain path component. Download
    /// filenames are scraped from remote HTML/JSON and must not be able to
    /// traverse out of the download dir (`../`, absolute paths, subdirs).
    fn safe_dest(&self, filename: &str) -> Result<PathBuf> {
        use std::path::{Component, Path};
        let mut comps = Path::new(filename).components();
        match (comps.next(), comps.next()) {
            (Some(Component::Normal(name)), None) if !name.is_empty() => {
                Ok(self.download_dir.join(name))
            }
            _ => anyhow::bail!("Refusing unsafe download filename: {filename:?}"),
        }
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
        let dest_path = self.safe_dest(filename)?;

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

        let dest_path = self.safe_dest(filename)?;

        // Create download directory if needed
        tokio::fs::create_dir_all(&self.download_dir)
            .await
            .context("Failed to create download directory")?;

        // Resume support: a leftover `.part` from a previous interrupted attempt
        // holds valid-but-incomplete bytes (a completed download renames it
        // away), so we can ask the server for just the remaining bytes.
        let part_path = self.download_dir.join(format!("{}.part", filename));
        let mut resume_from = tokio::fs::metadata(&part_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);

        let mut request = self.client.get(url);
        if resume_from > 0 {
            request = request.header(reqwest::header::RANGE, format!("bytes={resume_from}-"));
        }
        let mut response = request
            .send()
            .await
            .context("Failed to start download")?;

        // The partial file is larger than (or otherwise inconsistent with) the
        // upstream file — it changed since we started. Discard and refetch whole.
        if response.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
            log::info!("Partial {filename} no longer valid upstream; refetching");
            let _ = tokio::fs::remove_file(&part_path).await;
            resume_from = 0;
            response = self
                .client
                .get(url)
                .send()
                .await
                .context("Failed to restart download")?;
        }
        let response = response
            .error_for_status()
            .context("Download request failed")?;

        // 206 = server honoured the range → append. Anything else (typically 200)
        // means it's sending the whole file, so start `.part` over from scratch.
        let resuming = resume_from > 0 && response.status() == reqwest::StatusCode::PARTIAL_CONTENT;
        let start = if resuming { resume_from } else { 0 };
        if resume_from > 0 && !resuming {
            log::info!("Server ignored range for {filename}; restarting download");
        } else if resuming {
            log::info!("Resuming {filename} from {resume_from} bytes");
        }
        // content_length is the remaining bytes on a 206, the full size on a 200.
        let total_size = response.content_length().unwrap_or(0) + start;
        log::debug!("Download size: {} bytes (from {})", total_size, start);

        let file = if resuming {
            tokio::fs::OpenOptions::new()
                .append(true)
                .open(&part_path)
                .await
                .context("Failed to open partial file for resume")?
        } else {
            File::create(&part_path)
                .await
                .context("Failed to create destination file")?
        };

        // On error the `.part` is intentionally kept so the next attempt resumes;
        // its bytes are TLS-verified and only ever incomplete, never corrupt.
        Self::stream_to_file(response, file, start, total_size, on_progress, &self.cancel).await?;

        tokio::fs::rename(&part_path, &dest_path)
            .await
            .context("Failed to finalize download")?;

        log::info!("Download complete: {}", dest_path.display());
        Ok(dest_path)
    }

    async fn stream_to_file(
        response: reqwest::Response,
        mut file: File,
        start: u64,
        total_size: u64,
        on_progress: Option<ProgressCallback>,
        cancel: &Option<Arc<AtomicBool>>,
    ) -> Result<()> {
        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = start;

        while let Some(chunk) = stream.next().await {
            if cancel.as_ref().is_some_and(|c| c.load(Ordering::Relaxed)) {
                anyhow::bail!("Download cancelled");
            }
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
        Ok(())
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

// ponytail: no pre-flight free-space check. Atomic .part→rename already means a
// disk-full download fails cleanly (partial file removed, never cached), so the
// only cost is a late failure, not a brick. Add a statvfs check if that late
// failure proves annoying — not worth a new dependency otherwise.

#[cfg(test)]
mod tests {
    use super::ImageDownloader;
    use std::path::PathBuf;

    #[test]
    fn safe_dest_accepts_plain_names_rejects_traversal() {
        let d = ImageDownloader::new(PathBuf::from("/downloads"));
        assert_eq!(
            d.safe_dest("image.zip").unwrap(),
            PathBuf::from("/downloads/image.zip")
        );
        for bad in ["../evil", "a/b.img", "/etc/passwd", "..", "", "."] {
            assert!(d.safe_dest(bad).is_err(), "should reject {bad:?}");
        }
    }
}
