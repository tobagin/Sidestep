// postmarketOS installer backend
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::flashing::checksum::ChecksumVerifier;
use crate::flashing::decompressor::Decompressor;
use crate::flashing::downloader::ImageDownloader;
use crate::flashing::progress::InstallProgress;
use crate::hardware::adb::Adb;
use crate::hardware::fastboot::Fastboot;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

/// Orchestrates postmarketOS installation for supported devices.
///
/// Flow:
///   1. Scrape the image server for the latest build directory
///   2. Scrape the build directory for boot + rootfs image URLs and SHA256 hashes
///   3. Download both .img.xz files with progress
///   4. Verify SHA256 checksums
///   5. Decompress both XZ files → .img
///   6. Reboot to bootloader → wait for fastboot
///   7. Flash boot, userdata
///   8. Reboot
pub struct PostmarketosInstaller {
    serial: String,
    base_url: String,
    channel: String,
    interface: String,
    device: String,
    download_dir: PathBuf,
}

impl PostmarketosInstaller {
    pub fn new(
        serial: String,
        base_url: String,
        channel: String,
        interface: String,
        device: String,
    ) -> Self {
        let download_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("sidestep")
            .join("postmarketos");

        Self {
            serial,
            base_url,
            channel,
            interface,
            device,
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
                    log::error!("postmarketOS installation failed: {:#}", e);
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

        // ── Step 1: Discover latest build directory ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Discovering latest postmarketOS build...".into(),
        ));
        let listing_url = format!(
            "{}{}/{}/{}/",
            self.base_url, self.channel, self.device, self.interface
        );
        let build_dir = self.discover_latest_build(&listing_url).await?;
        let build_url = format!("{}{}/", listing_url, build_dir);
        log::info!("Found latest postmarketOS build: {}", build_dir);

        // ── Step 2: Discover image files and checksums ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Fetching image details...".into(),
        ));
        let (boot_name, boot_hash, rootfs_name, rootfs_hash) =
            self.discover_images(&build_url).await?;
        log::info!("Boot image: {} (sha256: {})", boot_name, boot_hash);
        log::info!("Rootfs image: {} (sha256: {})", rootfs_name, rootfs_hash);

        // ── Step 3: Download boot image ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Downloading boot image...".into(),
        ));
        let boot_url = format!("{}{}", build_url, boot_name);
        let sender_clone = sender.clone();
        let boot_path = downloader
            .download_if_needed(
                &boot_url,
                &boot_name,
                Some(&boot_hash),
                Some(Box::new(move |downloaded, total| {
                    let _ = sender_clone.send(InstallProgress::DownloadProgress {
                        downloaded,
                        total,
                        file_name: "Boot image".into(),
                    });
                })),
            )
            .await
            .context("Failed to download boot image")?;

        // ── Step 4: Download rootfs image ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Downloading rootfs image...".into(),
        ));
        let rootfs_url = format!("{}{}", build_url, rootfs_name);
        let sender_clone = sender.clone();
        let rootfs_path = downloader
            .download_if_needed(
                &rootfs_url,
                &rootfs_name,
                Some(&rootfs_hash),
                Some(Box::new(move |downloaded, total| {
                    let _ = sender_clone.send(InstallProgress::DownloadProgress {
                        downloaded,
                        total,
                        file_name: "Rootfs image".into(),
                    });
                })),
            )
            .await
            .context("Failed to download rootfs image")?;

        // ── Step 5: Verify checksums ──
        let _ = sender.send(InstallProgress::VerifyProgress {
            verified: 0,
            total: 2,
            file_name: "Verifying boot image".into(),
        });
        let ok = ChecksumVerifier::verify(&boot_path, &boot_hash)?;
        if !ok {
            anyhow::bail!("Checksum mismatch for boot image {}", boot_name);
        }
        let _ = sender.send(InstallProgress::VerifyProgress {
            verified: 1,
            total: 2,
            file_name: "Verifying rootfs image".into(),
        });
        let ok = ChecksumVerifier::verify(&rootfs_path, &rootfs_hash)?;
        if !ok {
            anyhow::bail!("Checksum mismatch for rootfs image {}", rootfs_name);
        }
        let _ = sender.send(InstallProgress::VerifyProgress {
            verified: 2,
            total: 2,
            file_name: "All checksums verified".into(),
        });

        // ── Step 6: Decompress boot image ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Decompressing boot image...".into(),
        ));
        let boot_img = Decompressor::decompress_xz(&boot_path, None, None)
            .context("Failed to decompress boot image")?;
        log::info!("Decompressed boot image: {}", boot_img.display());

        // ── Step 7: Decompress rootfs image ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Decompressing rootfs image (this may take a while)...".into(),
        ));
        let rootfs_img = Decompressor::decompress_xz(&rootfs_path, None, None)
            .context("Failed to decompress rootfs image")?;
        log::info!("Decompressed rootfs image: {}", rootfs_img.display());

        // ── Step 8: Reboot to bootloader ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Rebooting to bootloader...".into(),
        ));
        if let Err(e) = adb.reboot_bootloader(&self.serial).await {
            log::warn!(
                "ADB reboot-bootloader failed (device may already be in fastboot): {}",
                e
            );
        }

        // ── Step 9: Wait for fastboot device ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Waiting for device in fastboot mode...".into(),
        ));
        self.wait_for_fastboot(&fastboot).await?;

        // ── Step 10: Flash partitions ──
        let total_steps = 2;

        // 10a: Flash boot
        let _ = sender.send(InstallProgress::FlashProgress {
            current: 1,
            total: total_steps,
            description: "Flashing boot...".into(),
        });
        fastboot
            .flash(&self.serial, "boot", &boot_img)
            .await
            .context("Failed to flash boot")?;

        // 10b: Flash userdata (rootfs)
        let _ = sender.send(InstallProgress::FlashProgress {
            current: 2,
            total: total_steps,
            description: "Flashing rootfs (this may take a while)...".into(),
        });
        fastboot
            .flash(&self.serial, "userdata", &rootfs_img)
            .await
            .context("Failed to flash userdata (rootfs)")?;

        // ── Step 11: Reboot ──
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

    /// Scrape the interface listing page for the latest date-stamped build directory.
    /// Directories follow the pattern `YYYYMMDD-HHMM/`.
    async fn discover_latest_build(&self, listing_url: &str) -> Result<String> {
        let client = reqwest::Client::builder()
            .user_agent(format!("Sidestep/{}", crate::config::VERSION))
            .build()?;

        let resp = client
            .get(listing_url)
            .send()
            .await
            .context("Failed to fetch postmarketOS build listing")?;

        if !resp.status().is_success() {
            anyhow::bail!(
                "postmarketOS image server returned status {} for {}",
                resp.status(),
                listing_url
            );
        }

        let html = resp.text().await.context("Failed to read build listing")?;

        // Look for href="YYYYMMDD-HHMM/" patterns (date-stamped directories)
        let mut best_dir = String::new();

        for line in html.lines() {
            if let Some(start) = line.find("href=\"") {
                let rest = &line[start + 6..];
                if let Some(end) = rest.find('"') {
                    let href = &rest[..end];
                    // Match pattern: YYYYMMDD-HHMM/
                    if href.ends_with('/')
                        && href.len() == 14
                        && href[..8].chars().all(|c| c.is_ascii_digit())
                        && href.as_bytes()[8] == b'-'
                        && href[9..13].chars().all(|c| c.is_ascii_digit())
                    {
                        let dir_name = &href[..13]; // without trailing slash
                        if dir_name > best_dir.as_str() {
                            best_dir = dir_name.to_string();
                        }
                    }
                }
            }
        }

        if best_dir.is_empty() {
            anyhow::bail!(
                "No build directories found at {}. \
                 Check that channel={} and interface={} are valid for device={}.",
                listing_url,
                self.channel,
                self.interface,
                self.device
            );
        }

        Ok(best_dir)
    }

    /// Scrape a build directory page for boot + rootfs image filenames and their SHA256 hashes.
    /// Returns (boot_filename, boot_sha256, rootfs_filename, rootfs_sha256).
    async fn discover_images(
        &self,
        build_url: &str,
    ) -> Result<(String, String, String, String)> {
        let client = reqwest::Client::builder()
            .user_agent(format!("Sidestep/{}", crate::config::VERSION))
            .build()?;

        let resp = client
            .get(build_url)
            .send()
            .await
            .context("Failed to fetch postmarketOS build directory")?;

        if !resp.status().is_success() {
            anyhow::bail!(
                "postmarketOS image server returned status {} for {}",
                resp.status(),
                build_url
            );
        }

        let html = resp.text().await.context("Failed to read build directory")?;

        // Collect all .img.xz hrefs from the page
        let mut boot_name: Option<String> = None;
        let mut rootfs_name: Option<String> = None;

        for line in html.lines() {
            if let Some(start) = line.find("href=\"") {
                let rest = &line[start + 6..];
                if let Some(end) = rest.find('"') {
                    let href = &rest[..end];
                    if href.ends_with(".img.xz") && !href.ends_with(".sha256") && !href.ends_with(".sha512") {
                        if href.contains("-boot.img.xz") {
                            boot_name = Some(href.to_string());
                        } else {
                            rootfs_name = Some(href.to_string());
                        }
                    }
                }
            }
        }

        let boot_name = boot_name.ok_or_else(|| {
            anyhow::anyhow!("Could not find boot image in {}", build_url)
        })?;
        let rootfs_name = rootfs_name.ok_or_else(|| {
            anyhow::anyhow!("Could not find rootfs image in {}", build_url)
        })?;

        // Parse SHA256 hashes from the page.
        // Format on the page: sha256 link followed by ": HASH" on the same line or nearby.
        // The HTML contains patterns like:
        //   <a href="filename.sha256">sha256</a>: HASH_VALUE
        let boot_hash = self.extract_sha256(&html, &boot_name)?;
        let rootfs_hash = self.extract_sha256(&html, &rootfs_name)?;

        Ok((boot_name, boot_hash, rootfs_name, rootfs_hash))
    }

    /// Extract the SHA256 hash for a given filename from the build directory HTML.
    /// Looks for patterns like: `href="filename.sha256">sha256</a>: HASH`
    fn extract_sha256(&self, html: &str, filename: &str) -> Result<String> {
        let sha256_file = format!("{}.sha256", filename);

        // The HTML may span multiple lines, e.g.:
        //   <a href="file.sha256"
        //      >sha256</a>: HASH
        // So we check both the current line and the next line for the hash.
        let lines: Vec<&str> = html.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.contains(&sha256_file) {
                // Check current line and next line for the hash pattern
                for check_line in &lines[i..std::cmp::min(i + 3, lines.len())] {
                    if let Some(pos) = check_line.find("sha256</a>:") {
                        let after = &check_line[pos + 11..]; // skip "sha256</a>:"
                        let hash = after.trim().split_whitespace().next().unwrap_or("");
                        // Strip any trailing HTML tags
                        let hash = hash.split('<').next().unwrap_or(hash);
                        if hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
                            return Ok(hash.to_string());
                        }
                    }
                }
            }
        }

        anyhow::bail!(
            "Could not find SHA256 hash for {} in build directory",
            filename
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
