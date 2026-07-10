// postmarketOS installer backend
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::flashing::checksum::ChecksumVerifier;
use crate::flashing::decompressor::Decompressor;
use crate::flashing::downloader::ImageDownloader;
use crate::flashing::progress::InstallProgress;
use crate::hardware::adb::Adb;
use crate::hardware::fastboot::Fastboot;
use crate::models::installer::FlashOperation;
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
///   7. Execute flash_operations from YAML config
///   8. Reboot
pub struct PostmarketosInstaller {
    serial: String,
    base_url: String,
    channel: String,
    interface: String,
    device: String,
    flash_operations: Vec<FlashOperation>,
    download_dir: PathBuf,
}

impl PostmarketosInstaller {
    pub fn new(
        serial: String,
        base_url: String,
        channel: String,
        interface: String,
        device: String,
        flash_operations: Vec<FlashOperation>,
    ) -> Self {
        let download_dir = crate::flashing::download_dir().join("postmarketos");

        Self {
            serial,
            base_url,
            channel,
            interface,
            device,
            flash_operations,
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
        if self.flash_operations.is_empty() {
            anyhow::bail!("No flash operations configured for postmarketOS installation");
        }

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

        // ── Step 10: Execute flash operations ──
        let total_steps = self.flash_operations.len();
        for (i, op) in self.flash_operations.iter().enumerate() {
            self.execute_flash_operation(
                &fastboot,
                sender,
                op,
                i + 1,
                total_steps,
                &boot_img,
                &rootfs_img,
            ).await?;
        }

        // ── Step 11: Reboot ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Rebooting device...".into(),
        ));
        fastboot.reboot(&self.serial).await?;

        let _ = sender.send(InstallProgress::Complete);
        Ok(())
    }

    /// Execute a single flash operation, resolving source names to actual paths.
    async fn execute_flash_operation(
        &self,
        fastboot: &Fastboot,
        sender: &Sender<InstallProgress>,
        op: &FlashOperation,
        current: usize,
        total: usize,
        boot_img: &PathBuf,
        rootfs_img: &PathBuf,
    ) -> Result<()> {
        match op {
            FlashOperation::Flash { partition, source, flags } => {
                let image_path = self.resolve_source(source, boot_img, rootfs_img);
                let _ = sender.send(InstallProgress::FlashProgress {
                    current,
                    total,
                    description: format!("Flashing {}...", partition),
                });
                if flags.is_empty() {
                    fastboot
                        .flash(&self.serial, partition, &image_path)
                        .await
                        .with_context(|| format!("Failed to flash {}", partition))?;
                } else {
                    let flag_refs: Vec<&str> = flags.iter().map(|s| s.as_str()).collect();
                    fastboot
                        .flash_with_flags(&self.serial, partition, &image_path, &flag_refs)
                        .await
                        .with_context(|| format!("Failed to flash {}", partition))?;
                }
            }
            FlashOperation::FlashSparse { partition, source, chunk_size } => {
                let image_path = self.resolve_source(source, boot_img, rootfs_img);
                let _ = sender.send(InstallProgress::FlashProgress {
                    current,
                    total,
                    description: format!("Flashing {} (this may take a while)...", partition),
                });
                fastboot
                    .flash_sparse(&self.serial, partition, &image_path, chunk_size)
                    .await
                    .with_context(|| format!("Failed to flash {} (sparse)", partition))?;
            }
            FlashOperation::Format { partition, fs_type } => {
                let _ = sender.send(InstallProgress::FlashProgress {
                    current,
                    total,
                    description: format!("Formatting {}...", partition),
                });
                fastboot
                    .format(&self.serial, partition, fs_type)
                    .await
                    .with_context(|| format!("Failed to format {} as {}", partition, fs_type))?;
            }
            FlashOperation::Erase { partition } => {
                let _ = sender.send(InstallProgress::FlashProgress {
                    current,
                    total,
                    description: format!("Erasing {}...", partition),
                });
                fastboot
                    .erase(&self.serial, partition)
                    .await
                    .with_context(|| format!("Failed to erase {}", partition))?;
            }
            FlashOperation::Oem { args } => {
                let desc = format!("oem {}", args.join(" "));
                let _ = sender.send(InstallProgress::FlashProgress {
                    current,
                    total,
                    description: format!("Running {}...", desc),
                });
                let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                fastboot
                    .oem(&self.serial, &arg_refs)
                    .await
                    .with_context(|| format!("Failed to run {}", desc))?;
            }
        }
        Ok(())
    }

    /// Resolve a source name like "boot" or "rootfs" to an actual file path.
    fn resolve_source(&self, source: &str, boot_img: &PathBuf, rootfs_img: &PathBuf) -> PathBuf {
        match source {
            "boot" => boot_img.clone(),
            "rootfs" => rootfs_img.clone(),
            other => PathBuf::from(other),
        }
    }

    // ────────────────────────────────────────────────────────────────
    // Sub-steps
    // ────────────────────────────────────────────────────────────────

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

        let mut best_dir = String::new();

        for line in html.lines() {
            if let Some(start) = line.find("href=\"") {
                let rest = &line[start + 6..];
                if let Some(end) = rest.find('"') {
                    let href = &rest[..end];
                    if href.ends_with('/')
                        && href.len() == 14
                        && href[..8].chars().all(|c| c.is_ascii_digit())
                        && href.as_bytes()[8] == b'-'
                        && href[9..13].chars().all(|c| c.is_ascii_digit())
                    {
                        let dir_name = &href[..13];
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

        let boot_hash = self.extract_sha256(&html, &boot_name)?;
        let rootfs_hash = self.extract_sha256(&html, &rootfs_name)?;

        Ok((boot_name, boot_hash, rootfs_name, rootfs_hash))
    }

    fn extract_sha256(&self, html: &str, filename: &str) -> Result<String> {
        let sha256_file = format!("{}.sha256", filename);

        let lines: Vec<&str> = html.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.contains(&sha256_file) {
                for check_line in &lines[i..std::cmp::min(i + 3, lines.len())] {
                    if let Some(pos) = check_line.find("sha256</a>:") {
                        let after = &check_line[pos + 11..];
                        let hash = after.trim().split_whitespace().next().unwrap_or("");
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
