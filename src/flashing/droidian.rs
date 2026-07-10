// Droidian installer backend
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::flashing::checksum::ChecksumVerifier;
use crate::flashing::downloader::ImageDownloader;
use crate::flashing::progress::InstallProgress;
use crate::hardware::adb::Adb;
use crate::hardware::fastboot::Fastboot;
use crate::models::installer::FlashPartition;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

/// Orchestrates Droidian installation
pub struct DroidianInstaller {
    serial: String,
    release_url: String,
    artifact_pattern: String,
    flash_partitions: Vec<FlashPartition>,
    download_dir: PathBuf,
}

impl DroidianInstaller {
    pub fn new(
        serial: String,
        release_url: String,
        artifact_pattern: String,
        flash_partitions: Vec<FlashPartition>,
    ) -> Self {
        let download_dir = crate::flashing::download_dir().join("droidian");

        Self {
            serial,
            release_url,
            artifact_pattern,
            flash_partitions,
            download_dir,
        }
    }

    /// Spawn the installer on a background thread, returning immediately.
    /// Progress is reported via the returned mpsc::Receiver.
    pub fn spawn(self) -> std::sync::mpsc::Receiver<InstallProgress> {
        let (sender, receiver) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime");

            rt.block_on(async {
                if let Err(e) = self.run(&sender).await {
                    log::error!("Droidian installation failed: {:#}", e);
                    let _ = sender.send(InstallProgress::Error(format!("{:#}", e)));
                }
            });
        });

        receiver
    }

    /// Main installation flow (runs on background thread)
    async fn run(&self, sender: &Sender<InstallProgress>) -> Result<()> {
        if self.flash_partitions.is_empty() {
            anyhow::bail!("No flash partitions configured for Droidian installation");
        }

        let downloader = ImageDownloader::new(self.download_dir.clone());
        let adb = Adb::new();
        let fastboot = Fastboot::new();

        // ── Step 1: Query GitHub API for latest release ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Fetching release info...".into(),
        ));
        let (zip_url, zip_name, checksums_url) = self.fetch_release_info().await?;
        log::info!("Found release artifact: {}", zip_name);

        // ── Step 2: Download SHA256SUMS ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Downloading checksums...".into(),
        ));
        let expected_hash = self
            .download_and_parse_checksums(&downloader, &checksums_url, &zip_name)
            .await?;

        // ── Step 3: Download ZIP ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Downloading Droidian image...".into(),
        ));
        let sender_clone = sender.clone();
        let zip_path = downloader
            .download_if_needed(
                &zip_url,
                &zip_name,
                expected_hash.as_deref(),
                Some(Box::new(move |downloaded, total| {
                    let _ = sender_clone.send(InstallProgress::DownloadProgress {
                        downloaded,
                        total,
                        file_name: "Droidian image".into(),
                    });
                })),
            )
            .await
            .context("Failed to download Droidian ZIP")?;

        // ── Step 4: Verify ZIP checksum ──
        if let Some(ref hash) = expected_hash {
            let _ = sender.send(InstallProgress::VerifyProgress {
                verified: 0,
                total: 1,
                file_name: zip_name.clone(),
            });
            let ok = ChecksumVerifier::verify(&zip_path, hash)?;
            if !ok {
                anyhow::bail!("Checksum mismatch for {}", zip_name);
            }
            let _ = sender.send(InstallProgress::VerifyProgress {
                verified: 1,
                total: 1,
                file_name: zip_name.clone(),
            });
        }

        // ── Step 5: Extract ZIP ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Extracting...".into(),
        ));
        let extract_dir = self.download_dir.join("extracted");
        self.extract_zip(&zip_path, &extract_dir)?;

        // ── Step 6: Reboot to bootloader ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Rebooting to bootloader...".into(),
        ));
        if let Err(e) = adb.reboot_bootloader(&self.serial).await {
            log::warn!(
                "ADB reboot-bootloader failed (device may already be in fastboot): {}",
                e
            );
        }

        // ── Step 7: Wait for fastboot device ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Waiting for device in fastboot mode...".into(),
        ));
        self.wait_for_fastboot(&fastboot).await?;

        // ── Step 8: Flash partitions ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Flashing partitions...".into(),
        ));
        let total = self.flash_partitions.len();
        for (i, part) in self.flash_partitions.iter().enumerate() {
            let _ = sender.send(InstallProgress::FlashProgress {
                current: i + 1,
                total,
                description: format!("Flashing {}...", part.partition),
            });

            let image_path = extract_dir.join(&part.image_path);
            if !image_path.exists() {
                anyhow::bail!(
                    "Image file not found: {} (expected at {})",
                    part.image_path,
                    image_path.display()
                );
            }

            if part.flags.is_empty() {
                fastboot
                    .flash(&self.serial, &part.partition, &image_path)
                    .await
                    .with_context(|| format!("Failed to flash {}", part.partition))?;
            } else {
                let flag_refs: Vec<&str> = part.flags.iter().map(|s| s.as_str()).collect();
                fastboot
                    .flash_with_flags(&self.serial, &part.partition, &image_path, &flag_refs)
                    .await
                    .with_context(|| format!("Failed to flash {}", part.partition))?;
            }
        }

        // ── Step 9: Reboot ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Rebooting device...".into(),
        ));
        fastboot.reboot(&self.serial).await?;

        let _ = sender.send(InstallProgress::Complete);
        Ok(())
    }

    // ────────────────────────────────────────────────────────────────
    // Sub-steps
    // ────────────────────────────────────────────────────────────────

    async fn fetch_release_info(&self) -> Result<(String, String, String)> {
        let client = reqwest::Client::builder()
            .user_agent(format!("Sidestep/{}", crate::config::VERSION))
            .build()?;

        let resp = client
            .get(&self.release_url)
            .send()
            .await
            .context("Failed to query GitHub releases API")?;

        if !resp.status().is_success() {
            anyhow::bail!(
                "GitHub API returned status {} for {}",
                resp.status(),
                self.release_url
            );
        }

        let text = resp.text().await.context("Failed to read GitHub release response")?;
        let body: serde_json::Value =
            serde_json::from_str(&text).context("Failed to parse GitHub release JSON")?;

        let assets = body["assets"]
            .as_array()
            .context("No 'assets' array in release JSON")?;

        let zip_asset = assets
            .iter()
            .find(|a| {
                a["name"]
                    .as_str()
                    .is_some_and(|n: &str| n.contains(&self.artifact_pattern))
            })
            .context(format!(
                "No asset matching '{}' in release",
                self.artifact_pattern
            ))?;

        let zip_url = zip_asset["browser_download_url"]
            .as_str()
            .context("No browser_download_url for ZIP asset")?
            .to_string();
        let zip_name = zip_asset["name"]
            .as_str()
            .context("No name for ZIP asset")?
            .to_string();

        let checksums_asset = assets
            .iter()
            .find(|a| {
                a["name"]
                    .as_str()
                    .is_some_and(|n: &str| n.contains("SHA256SUMS"))
            })
            .context("No SHA256SUMS asset in release")?;

        let checksums_url = checksums_asset["browser_download_url"]
            .as_str()
            .context("No browser_download_url for SHA256SUMS")?
            .to_string();

        Ok((zip_url, zip_name, checksums_url))
    }

    async fn download_and_parse_checksums(
        &self,
        downloader: &ImageDownloader,
        checksums_url: &str,
        zip_name: &str,
    ) -> Result<Option<String>> {
        let checksums = downloader.download_checksums(checksums_url).await?;
        Ok(checksums.get(zip_name).cloned())
    }

    fn extract_zip(&self, zip_path: &PathBuf, extract_dir: &PathBuf) -> Result<()> {
        if extract_dir.exists() {
            std::fs::remove_dir_all(extract_dir)
                .context("Failed to clean previous extraction")?;
        }
        std::fs::create_dir_all(extract_dir)
            .context("Failed to create extraction directory")?;

        let file = std::fs::File::open(zip_path)
            .context("Failed to open ZIP file")?;
        let mut archive = zip::ZipArchive::new(file)
            .context("Failed to read ZIP archive")?;

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let out_path = extract_dir.join(entry.name());

            if entry.is_dir() {
                std::fs::create_dir_all(&out_path)?;
            } else {
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut outfile = std::fs::File::create(&out_path)?;
                std::io::copy(&mut entry, &mut outfile)?;
            }
        }

        log::info!("Extracted ZIP to {}", extract_dir.display());
        Ok(())
    }

    async fn wait_for_fastboot(&self, fastboot: &Fastboot) -> Result<()> {
        for _ in 0..60 {
            if let Ok(devices) = fastboot.devices().await {
                if devices.iter().any(|d| d.serial == self.serial) {
                    return Ok(());
                }
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        anyhow::bail!("Timed out waiting for device in fastboot mode")
    }
}
