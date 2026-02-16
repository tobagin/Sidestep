// Factory image installer (stock Android)
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::flashing::checksum::ChecksumVerifier;
use crate::flashing::downloader::ImageDownloader;
use crate::flashing::progress::InstallProgress;
use crate::hardware::fastboot::Fastboot;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

/// Orchestrates flashing a stock Android factory image.
///
/// Follows Google's `flash-all.sh` sequence:
/// 1. Flash bootloader → reboot-bootloader → wait
/// 2. Flash radio → reboot-bootloader → wait
/// 3. `fastboot update -w image-*.zip` (all system partitions + wipe)
pub struct FactoryImageInstaller {
    serial: String,
    url: String,
    sha256: String,
    android_version: String,
    download_dir: PathBuf,
}

impl FactoryImageInstaller {
    pub fn new(serial: String, url: String, sha256: String, android_version: String) -> Self {
        let download_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("sidestep")
            .join("factory-image");

        Self {
            serial,
            url,
            sha256,
            android_version,
            download_dir,
        }
    }

    /// Spawn the installer on a background thread, returning immediately.
    pub fn spawn(self) -> std::sync::mpsc::Receiver<InstallProgress> {
        let (sender, receiver) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime");

            rt.block_on(async {
                if let Err(e) = self.run(&sender).await {
                    log::error!("Factory image installation failed: {:#}", e);
                    let _ = sender.send(InstallProgress::Error(format!("{:#}", e)));
                }
            });
        });

        receiver
    }

    async fn run(&self, sender: &Sender<InstallProgress>) -> Result<()> {
        let downloader = ImageDownloader::new(self.download_dir.clone());
        let fastboot = Fastboot::new();

        // Derive filename from URL
        let zip_name = self
            .url
            .rsplit('/')
            .next()
            .unwrap_or("factory-image.zip")
            .to_string();

        // ── Step 1: Download factory ZIP ──
        let _ = sender.send(InstallProgress::StatusChanged(
            format!("Downloading {}...", self.android_version),
        ));
        let sender_clone = sender.clone();
        let android_ver = self.android_version.clone();
        let zip_path = downloader
            .download_if_needed(
                &self.url,
                &zip_name,
                Some(&self.sha256),
                Some(Box::new(move |downloaded, total| {
                    let _ = sender_clone.send(InstallProgress::DownloadProgress {
                        downloaded,
                        total,
                        file_name: android_ver.clone(),
                    });
                })),
            )
            .await
            .context("Failed to download factory image ZIP")?;

        // ── Step 2: Verify SHA256 ──
        let _ = sender.send(InstallProgress::StatusChanged("Verifying checksum...".into()));
        let _ = sender.send(InstallProgress::VerifyProgress {
            verified: 0,
            total: 1,
            file_name: zip_name.clone(),
        });
        let ok = ChecksumVerifier::verify(&zip_path, &self.sha256)?;
        if !ok {
            anyhow::bail!("Checksum mismatch for {}", zip_name);
        }
        let _ = sender.send(InstallProgress::VerifyProgress {
            verified: 1,
            total: 1,
            file_name: zip_name.clone(),
        });

        // ── Step 3: Extract outer ZIP ──
        let _ = sender.send(InstallProgress::StatusChanged("Extracting factory image...".into()));
        let extract_dir = self.download_dir.join("extracted");
        self.extract_zip(&zip_path, &extract_dir)?;

        // ── Step 4: Locate files inside the extracted directory ──
        // Factory images have a subdirectory like `sargo-pq3b.190801.002/`
        let inner_dir = self.find_inner_dir(&extract_dir)?;
        let bootloader_img = self.find_file(&inner_dir, "bootloader-")?;
        let radio_img = self.find_file(&inner_dir, "radio-")?;
        let image_zip = self.find_file(&inner_dir, "image-")?;

        log::info!("Bootloader: {}", bootloader_img.display());
        log::info!("Radio: {}", radio_img.display());
        log::info!("Image ZIP: {}", image_zip.display());

        // ── Step 5: Flash bootloader ──
        let _ = sender.send(InstallProgress::FlashProgress {
            current: 1,
            total: 3,
            description: "Flashing bootloader...".into(),
        });
        fastboot
            .flash(&self.serial, "bootloader", &bootloader_img)
            .await
            .context("Failed to flash bootloader")?;

        let _ = sender.send(InstallProgress::StatusChanged(
            "Rebooting to bootloader...".into(),
        ));
        fastboot.reboot_bootloader(&self.serial).await?;
        self.wait_for_fastboot(&fastboot).await?;

        // ── Step 6: Flash radio ──
        let _ = sender.send(InstallProgress::FlashProgress {
            current: 2,
            total: 3,
            description: "Flashing radio...".into(),
        });
        fastboot
            .flash(&self.serial, "radio", &radio_img)
            .await
            .context("Failed to flash radio")?;

        let _ = sender.send(InstallProgress::StatusChanged(
            "Rebooting to bootloader...".into(),
        ));
        fastboot.reboot_bootloader(&self.serial).await?;
        self.wait_for_fastboot(&fastboot).await?;

        // ── Step 7: Flash all system partitions + wipe ──
        let _ = sender.send(InstallProgress::FlashProgress {
            current: 3,
            total: 3,
            description: "Flashing system partitions...".into(),
        });
        fastboot
            .update(&self.serial, &image_zip, true)
            .await
            .context("Failed to flash system partitions (fastboot update)")?;

        let _ = sender.send(InstallProgress::Complete);
        Ok(())
    }

    // ────────────────────────────────────────────────────────────────
    // Helpers
    // ────────────────────────────────────────────────────────────────

    fn extract_zip(&self, zip_path: &PathBuf, extract_dir: &PathBuf) -> Result<()> {
        if extract_dir.exists() {
            std::fs::remove_dir_all(extract_dir)
                .context("Failed to clean previous extraction")?;
        }
        std::fs::create_dir_all(extract_dir)
            .context("Failed to create extraction directory")?;

        let file =
            std::fs::File::open(zip_path).context("Failed to open factory image ZIP")?;
        let mut archive =
            zip::ZipArchive::new(file).context("Failed to read factory image ZIP")?;

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

        log::info!("Extracted factory image to {}", extract_dir.display());
        Ok(())
    }

    /// Find the single subdirectory inside the extracted ZIP (e.g. `sargo-pq3b.190801.002/`).
    fn find_inner_dir(&self, extract_dir: &PathBuf) -> Result<PathBuf> {
        for entry in std::fs::read_dir(extract_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                return Ok(entry.path());
            }
        }
        // If no subdirectory, files are directly in extract_dir
        Ok(extract_dir.clone())
    }

    /// Find a file whose name starts with the given prefix.
    fn find_file(&self, dir: &PathBuf, prefix: &str) -> Result<PathBuf> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with(prefix) && !entry.file_type()?.is_dir() {
                return Ok(entry.path());
            }
        }
        anyhow::bail!(
            "Could not find file starting with '{}' in {}",
            prefix,
            dir.display()
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
