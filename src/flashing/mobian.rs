// Mobian installer backend
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::flashing::checksum::ChecksumVerifier;
use crate::flashing::downloader::ImageDownloader;
use crate::flashing::progress::InstallProgress;
use crate::hardware::adb::Adb;
use crate::hardware::fastboot::Fastboot;
use crate::models::installer::FlashOperation;
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
///   6. Execute flash_operations from YAML config
///   7. Reboot
pub struct MobianInstaller {
    serial: String,
    base_url: String,
    interface: String,
    device_model: String,
    chipset: String,
    flash_operations: Vec<FlashOperation>,
    download_dir: PathBuf,
}

impl MobianInstaller {
    pub fn new(
        serial: String,
        base_url: String,
        interface: String,
        chipset: String,
        device_model: String,
        flash_operations: Vec<FlashOperation>,
    ) -> Self {
        let download_dir = crate::flashing::download_dir().join("mobian");

        Self {
            serial,
            base_url,
            interface,
            device_model,
            chipset,
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
                    log::error!("Mobian installation failed: {:#}", e);
                    let _ = sender.send(InstallProgress::Error(format!("{:#}", e)));
                }
            });
        });

        receiver
    }

    /// Main installation flow (runs on background thread)
    async fn run(&self, sender: &Sender<InstallProgress>) -> Result<()> {
        if self.flash_operations.is_empty() {
            anyhow::bail!("No flash operations configured for Mobian installation");
        }

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

        // ── Step 8: Execute flash operations ──
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

        // ── Step 9: Reboot ──
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

    async fn discover_latest_image(&self) -> Result<(String, String)> {
        let client = reqwest::Client::builder()
            .user_agent(format!("Sidestep/{}", crate::config::VERSION))
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

        let mut best_date = String::new();
        let mut best_name = String::new();

        for line in html.lines() {
            if let Some(start) = line.find("href=\"") {
                let rest = &line[start + 6..];
                if let Some(end) = rest.find('"') {
                    let href = &rest[..end];
                    let expected_prefix = format!("mobian-{}-{}-", self.chipset, self.interface);
                    if href.starts_with(&expected_prefix) && href.ends_with(".tar.xz") {
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
                "No Mobian image found for chipset={} interface={} at {}",
                self.chipset,
                self.interface,
                self.base_url,
            );
        }

        let full_url = format!("{}{}", self.base_url, best_name);
        Ok((best_name, full_url))
    }

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

    fn extract_tar_xz(&self, tar_path: &PathBuf, extract_dir: &PathBuf) -> Result<()> {
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

    fn find_extracted_file(&self, extract_dir: &PathBuf, pattern: &str) -> Result<PathBuf> {
        let suffix = pattern.trim_start_matches('*');

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
