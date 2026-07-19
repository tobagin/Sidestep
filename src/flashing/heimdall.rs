// Heimdall installer backend (Samsung Exynos download-mode flashing)
// SPDX-License-Identifier: GPL-3.0-or-later
//
// For Samsung Exynos devices that have no fastboot. Downloads and verifies a set
// of images (from the device config), reboots the device into download mode, and
// flashes them over the Odin protocol via `heimdall`.
//
// NOT YET VALIDATED ON HARDWARE. Partition names are device-specific PIT entries;
// only enable this for a device whose config has been tested on the real phone.

use crate::flashing::checksum::ChecksumVerifier;
use crate::flashing::downloader::ImageDownloader;
use crate::flashing::progress::InstallProgress;
use crate::hardware::adb::Adb;
use crate::hardware::heimdall::{Heimdall, HeimdallFlash};
use crate::models::installer::FirmwareImage;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::mpsc::Sender;

/// Orchestrates a Samsung download-mode install.
///
/// Flow:
///   1. Download each configured image with progress
///   2. Verify SHA256 checksums
///   3. `adb reboot download` → wait for download mode
///   4. `heimdall flash --<PARTITION> <image> ...`
pub struct HeimdallInstaller {
    serial: String,
    images: Vec<FirmwareImage>,
    download_dir: PathBuf,
}

impl HeimdallInstaller {
    pub fn new(serial: String, images: Vec<FirmwareImage>) -> Self {
        let download_dir = crate::flashing::download_dir().join("heimdall");
        Self {
            serial,
            images,
            download_dir,
        }
    }

    pub fn spawn(
        self,
        cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> std::sync::mpsc::Receiver<InstallProgress> {
        let (sender, receiver) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime");

            rt.block_on(async {
                if let Err(e) = self.run(&sender, cancel).await {
                    log::error!("Heimdall installation failed: {:#}", e);
                    let _ = sender.send(InstallProgress::Error(format!("{:#}", e)));
                }
            });
        });

        receiver
    }

    async fn run(
        &self,
        sender: &Sender<InstallProgress>,
        cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<()> {
        if self.images.is_empty() {
            anyhow::bail!("No images configured for Heimdall installation");
        }

        let downloader = ImageDownloader::new(self.download_dir.clone()).with_cancel(cancel.clone());
        let adb = Adb::new();
        let heimdall = Heimdall::new();

        // Best-effort IMEI/EFS backup while still in adb mode (rooted devices
        // only; everyone else is covered by the mandatory safety warning).
        let backup_dir = crate::flashing::download_dir()
            .join("imei-backup")
            .join(&self.serial);
        let saved = adb.backup_critical_partitions(&self.serial, &backup_dir).await;
        if !saved.is_empty() {
            let _ = sender.send(InstallProgress::StatusChanged(format!(
                "Backed up {} to {}",
                saved.join(", "),
                backup_dir.display()
            )));
        }

        // ── Step 1 & 2: Download + verify each image ──
        let mut flashes: Vec<HeimdallFlash> = Vec::new();
        for image in &self.images {
            let _ = sender.send(InstallProgress::StatusChanged(format!(
                "Downloading {}...",
                image.filename
            )));
            let sender_clone = sender.clone();
            let name = image.filename.clone();
            let path = downloader
                .download_if_needed(
                    &image.url,
                    &image.filename,
                    Some(&image.sha256),
                    Some(Box::new(move |downloaded, total| {
                        let _ = sender_clone.send(InstallProgress::DownloadProgress {
                            downloaded,
                            total,
                            file_name: name.clone(),
                        });
                    })),
                )
                .await
                .with_context(|| format!("Failed to download {}", image.filename))?;

            let _ = sender.send(InstallProgress::VerifyProgress {
                verified: 0,
                total: 1,
                file_name: image.filename.clone(),
            });
            if !ChecksumVerifier::verify(&path, &image.sha256)? {
                anyhow::bail!("Checksum mismatch for {}", image.filename);
            }
            let _ = sender.send(InstallProgress::VerifyProgress {
                verified: 1,
                total: 1,
                file_name: image.filename.clone(),
            });

            flashes.push(Heimdall::flash_entry(&image.partition, &path));
        }

        // ── Step 3: Reboot into download mode ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Rebooting to download mode...".into(),
        ));
        if let Err(e) = adb.reboot_download(&self.serial).await {
            log::warn!(
                "adb reboot download failed (device may already be in download mode): {}",
                e
            );
        }
        let _ = sender.send(InstallProgress::WaitingForUserAction(
            "If prompted on the device, press Volume Up to continue into download mode.".into(),
        ));
        heimdall
            .wait_for_download_mode(120)
            .await
            .context("Device did not enter download mode")?;

        // ── Step 4: Flash all partitions in one Odin session ──
        let total = flashes.len();
        let _ = sender.send(InstallProgress::FlashProgress {
            current: 0,
            total,
            description: "Flashing via Heimdall...".into(),
        });
        heimdall
            .flash(&flashes, true)
            .await
            .context("Heimdall flash failed")?;
        let _ = sender.send(InstallProgress::FlashProgress {
            current: total,
            total,
            description: "Flash complete".into(),
        });

        let _ = sender.send(InstallProgress::Complete);
        Ok(())
    }
}
