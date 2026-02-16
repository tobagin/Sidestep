// Mobian installer backend
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::flashing::checksum::ChecksumVerifier;
use crate::flashing::downloader::ImageDownloader;
use crate::flashing::progress::InstallProgress;
use crate::hardware::adb::Adb;
use crate::hardware::fastboot::Fastboot;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

/// Orchestrates Mobian installation for Qualcomm-based devices.
///
/// Flow:
///   1. Scrape the weekly image listing for the latest tar.xz
///   2. Download checksums + tar.xz with progress
///   3. Verify checksum
///   4. Extract tar.xz (boot image + rootfs image)
///   5. Reboot to bootloader → wait for fastboot
///   6. Flash boot, userdata (sparse), erase dtbo, oem uart enable
///   7. Reboot
pub struct MobianInstaller {
    serial: String,
    base_url: String,
    interface: String,
    device_model: String,
    chipset: String,
    download_dir: PathBuf,
}

impl MobianInstaller {
    pub fn new(
        serial: String,
        base_url: String,
        interface: String,
        chipset: String,
        device_model: String,
    ) -> Self {
        let download_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("sidestep")
            .join("mobian");

        Self {
            serial,
            base_url,
            interface,
            device_model,
            chipset,
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
                    log::error!("Mobian installation failed: {:#}", e);
                    let _ = sender.send(InstallProgress::Error(format!("{:#}", e)));
                }
            });
        });

        receiver
    }

    /// Main installation flow (runs on background thread)
    async fn run(&self, sender: &Sender<InstallProgress>) -> Result<()> {
        let downloader = ImageDownloader::new(self.download_dir.clone());
        let adb = Adb::new();
        let fastboot = Fastboot::new();

        // ── Step 1: Discover latest image ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Discovering latest Mobian image...".into(),
        ));
        let (tar_name, tar_url) = self.discover_latest_image().await?;
        log::info!("Found latest Mobian image: {}", tar_name);

        // ── Step 2: Download checksums ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Downloading checksums...".into(),
        ));
        let checksums_url = format!("{}{}.sha256sums", self.base_url, tar_name);
        let expected_hash = self
            .download_and_parse_checksums(&downloader, &checksums_url, &tar_name)
            .await?;

        // ── Step 3: Download tar.xz ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Downloading Mobian image...".into(),
        ));
        let sender_clone = sender.clone();
        let tar_path = downloader
            .download_if_needed(
                &tar_url,
                &tar_name,
                expected_hash.as_deref(),
                Some(Box::new(move |downloaded, total| {
                    let _ = sender_clone.send(InstallProgress::DownloadProgress {
                        downloaded,
                        total,
                        file_name: "Mobian image".into(),
                    });
                })),
            )
            .await
            .context("Failed to download Mobian tar.xz")?;

        // ── Step 4: Verify checksum ──
        if let Some(ref hash) = expected_hash {
            let _ = sender.send(InstallProgress::VerifyProgress {
                verified: 0,
                total: 1,
                file_name: tar_name.clone(),
            });
            let ok = ChecksumVerifier::verify(&tar_path, hash)?;
            if !ok {
                anyhow::bail!("Checksum mismatch for {}", tar_name);
            }
            let _ = sender.send(InstallProgress::VerifyProgress {
                verified: 1,
                total: 1,
                file_name: tar_name.clone(),
            });
        }

        // ── Step 5: Extract tar.xz ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Extracting image...".into(),
        ));
        let extract_dir = self.download_dir.join("extracted");
        self.extract_tar_xz(&tar_path, &extract_dir)?;

        // Locate the extracted images by glob pattern
        let boot_img = self.find_extracted_file(&extract_dir, &format!("*.boot-{}.img", self.device_model))?;
        let rootfs_img = self.find_extracted_file(&extract_dir, "*.rootfs.img")?;
        log::info!("Boot image: {}", boot_img.display());
        log::info!("Rootfs image: {}", rootfs_img.display());

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
        let total_steps = 5;

        // 8a: Flash boot
        let _ = sender.send(InstallProgress::FlashProgress {
            current: 1,
            total: total_steps,
            description: "Flashing boot...".into(),
        });
        fastboot
            .flash(&self.serial, "boot", &boot_img)
            .await
            .context("Failed to flash boot")?;

        // 8b: Format userdata (wipe old data before flashing new rootfs)
        let _ = sender.send(InstallProgress::FlashProgress {
            current: 2,
            total: total_steps,
            description: "Formatting userdata...".into(),
        });
        fastboot
            .format(&self.serial, "userdata", "ext4")
            .await
            .context("Failed to format userdata as ext4")?;

        // 8c: Flash userdata (sparse rootfs)
        let _ = sender.send(InstallProgress::FlashProgress {
            current: 3,
            total: total_steps,
            description: "Flashing rootfs (this may take a while)...".into(),
        });
        fastboot
            .flash_sparse(&self.serial, "userdata", &rootfs_img, "100M")
            .await
            .context("Failed to flash userdata (rootfs)")?;

        // 8d: Erase dtbo
        let _ = sender.send(InstallProgress::FlashProgress {
            current: 4,
            total: total_steps,
            description: "Erasing dtbo...".into(),
        });
        fastboot
            .erase(&self.serial, "dtbo")
            .await
            .context("Failed to erase dtbo")?;

        // 8e: OEM uart enable
        let _ = sender.send(InstallProgress::FlashProgress {
            current: 5,
            total: total_steps,
            description: "Enabling UART...".into(),
        });
        fastboot
            .oem(&self.serial, &["uart", "enable"])
            .await
            .context("Failed to run oem uart enable")?;

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

    /// Scrape the Mobian weekly image listing for the latest tar.xz matching
    /// this chipset + interface combination. Returns (filename, full_url).
    async fn discover_latest_image(&self) -> Result<(String, String)> {
        let client = reqwest::Client::builder()
            .user_agent("Sidestep/0.1.0")
            .build()?;

        let resp = client
            .get(&self.base_url)
            .send()
            .await
            .context("Failed to fetch Mobian image listing")?;

        if !resp.status().is_success() {
            anyhow::bail!(
                "Mobian image server returned status {} for {}",
                resp.status(),
                self.base_url
            );
        }

        let html = resp.text().await.context("Failed to read image listing")?;

        // Match filenames like: mobian-sdm670-phosh-20260215.tar.xz
        let pattern = format!(
            r"mobian-{}-{}-(\d{{8}})\.tar\.xz",
            regex_escape(&self.chipset),
            regex_escape(&self.interface),
        );

        let mut best_date = String::new();
        let mut best_name = String::new();

        for line in html.lines() {
            // Look for href="filename" patterns in the HTML directory listing
            if let Some(start) = line.find("href=\"") {
                let rest = &line[start + 6..];
                if let Some(end) = rest.find('"') {
                    let href = &rest[..end];
                    // Check if this href matches our pattern
                    let expected_prefix = format!("mobian-{}-{}-", self.chipset, self.interface);
                    if href.starts_with(&expected_prefix) && href.ends_with(".tar.xz") {
                        // Extract date portion
                        let after_prefix = &href[expected_prefix.len()..];
                        if let Some(date_str) = after_prefix.strip_suffix(".tar.xz") {
                            if date_str.len() == 8 && date_str.chars().all(|c| c.is_ascii_digit()) {
                                if date_str > best_date.as_str() {
                                    best_date = date_str.to_string();
                                    best_name = href.to_string();
                                }
                            }
                        }
                    }
                }
            }
        }

        if best_name.is_empty() {
            anyhow::bail!(
                "No Mobian image found for chipset={} interface={} at {}.\n\
                 Pattern: {}",
                self.chipset,
                self.interface,
                self.base_url,
                pattern
            );
        }

        let full_url = format!("{}{}", self.base_url, best_name);
        Ok((best_name, full_url))
    }

    /// Download SHA256SUMS and extract the hash for the given tar filename.
    async fn download_and_parse_checksums(
        &self,
        downloader: &ImageDownloader,
        checksums_url: &str,
        tar_name: &str,
    ) -> Result<Option<String>> {
        match downloader.download_checksums(checksums_url).await {
            Ok(checksums) => Ok(checksums.get(tar_name).cloned()),
            Err(e) => {
                log::warn!("Failed to download checksums (continuing without): {}", e);
                Ok(None)
            }
        }
    }

    /// Extract a tar.xz archive to the given directory.
    fn extract_tar_xz(&self, tar_path: &PathBuf, extract_dir: &PathBuf) -> Result<()> {
        // Clean up previous extraction
        if extract_dir.exists() {
            std::fs::remove_dir_all(extract_dir)
                .context("Failed to clean previous extraction")?;
        }
        std::fs::create_dir_all(extract_dir)
            .context("Failed to create extraction directory")?;

        let file = std::fs::File::open(tar_path)
            .context("Failed to open tar.xz file")?;
        let xz_reader = xz2::read::XzDecoder::new(file);
        let mut archive = tar::Archive::new(xz_reader);

        archive
            .unpack(extract_dir)
            .context("Failed to extract tar.xz archive")?;

        log::info!("Extracted tar.xz to {}", extract_dir.display());
        Ok(())
    }

    /// Find a file matching a glob-like pattern in the extract directory.
    /// Only supports `*` prefix patterns like `*.boot-sargo.img`.
    fn find_extracted_file(&self, extract_dir: &PathBuf, pattern: &str) -> Result<PathBuf> {
        let suffix = pattern.trim_start_matches('*');

        // Search recursively in extract_dir
        for entry in walkdir(extract_dir)? {
            let file_name = entry.file_name();
            let name = file_name.to_str().unwrap_or_default();
            if name.ends_with(suffix) {
                return Ok(entry.path().to_path_buf());
            }
        }

        anyhow::bail!(
            "Could not find file matching '{}' in {}",
            pattern,
            extract_dir.display()
        )
    }

    /// Poll fastboot devices until our device appears.
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

/// Simple recursive directory walk, returning all files (not directories).
fn walkdir(dir: &PathBuf) -> Result<Vec<std::fs::DirEntry>> {
    let mut results = Vec::new();
    walkdir_inner(dir, &mut results)?;
    Ok(results)
}

fn walkdir_inner(dir: &std::path::Path, results: &mut Vec<std::fs::DirEntry>) -> Result<()> {
    for entry in std::fs::read_dir(dir).context("Failed to read directory")? {
        let entry = entry?;
        let ft = entry.file_type()?;
        if ft.is_dir() {
            walkdir_inner(&entry.path(), results)?;
        } else {
            results.push(entry);
        }
    }
    Ok(())
}

/// Escape special regex characters in a string (minimal set for our patterns).
fn regex_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '.' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '\\' | '|' | '^'
            | '$' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}
