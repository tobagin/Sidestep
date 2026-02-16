// LineageOS installer backend
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::flashing::checksum::ChecksumVerifier;
use crate::flashing::downloader::ImageDownloader;
use crate::flashing::progress::InstallProgress;
use crate::hardware::adb::Adb;
use crate::hardware::fastboot::Fastboot;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

/// LineageOS build metadata from the API
#[derive(Debug, Deserialize)]
struct LineageBuild {
    #[allow(dead_code)]
    version: String,
    files: Vec<LineageFile>,
}

/// A single file within a LineageOS build
#[derive(Debug, Deserialize)]
struct LineageFile {
    filename: String,
    url: String,
    sha256: String,
    size: u64,
}

/// Orchestrates LineageOS installation for supported devices.
///
/// Fresh install flow:
///   1. Fetch latest build from LineageOS API
///   2. Download boot.img + lineage-*.zip with progress
///   3. Verify SHA256 checksums
///   4. Reboot to bootloader → wait for fastboot
///   5. Flash boot.img (installs LineageOS recovery)
///   6. Reboot to recovery → wait for recovery
///   7. Prompt user: "Factory reset" → "Format data/factory reset"
///   8. Prompt user: "Apply update" → "Apply from ADB"
///   9. adb sideload lineage-*.zip
///  10. Prompt user: "Reboot system now"
///
/// Update flow (update_only = true):
///   1. Fetch latest build from LineageOS API
///   2. Download lineage-*.zip with progress (skip boot.img)
///   3. Verify SHA256 checksum
///   4. Reboot to recovery directly
///   5. Prompt user: "Apply update" → "Apply from ADB"
///   6. adb sideload lineage-*.zip
///   7. Prompt user: "Reboot system now"
pub struct LineageosInstaller {
    serial: String,
    api_url: String,
    update_only: bool,
    download_dir: PathBuf,
}

impl LineageosInstaller {
    pub fn new(
        serial: String,
        api_url: String,
        update_only: bool,
    ) -> Self {
        let download_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("sidestep")
            .join("lineageos");

        Self {
            serial,
            api_url,
            update_only,
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
                    log::error!("LineageOS installation failed: {:#}", e);
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

        // ── Step 1: Fetch latest build from API ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Fetching latest LineageOS build...".into(),
        ));
        let build = self.fetch_latest_build().await?;
        log::info!("Found LineageOS build: {}", build.version);

        // ── Step 2: Find the files we need ──
        let zip_file = build
            .files
            .iter()
            .find(|f| f.filename.ends_with("-signed.zip") || f.filename.starts_with("lineage-"))
            .filter(|f| f.filename.ends_with(".zip") && !f.filename.ends_with(".img"))
            .or_else(|| {
                // Fallback: find largest zip file (the ROM zip)
                build.files.iter()
                    .filter(|f| f.filename.ends_with(".zip"))
                    .max_by_key(|f| f.size)
            })
            .ok_or_else(|| anyhow::anyhow!("No ROM zip found in build files"))?;

        let boot_file = build
            .files
            .iter()
            .find(|f| f.filename == "boot.img");

        log::info!("ROM zip: {} ({} bytes)", zip_file.filename, zip_file.size);
        if let Some(boot) = boot_file {
            log::info!("Boot image: {} ({} bytes)", boot.filename, boot.size);
        }

        // ── Step 3: Download files ──
        if !self.update_only {
            if let Some(boot) = boot_file {
                // Download boot.img
                let _ = sender.send(InstallProgress::StatusChanged(
                    "Downloading boot image...".into(),
                ));
                let sender_clone = sender.clone();
                downloader
                    .download_if_needed(
                        &boot.url,
                        &boot.filename,
                        Some(&boot.sha256),
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
            }
        }

        // Download ROM zip
        let _ = sender.send(InstallProgress::StatusChanged(
            "Downloading LineageOS ROM...".into(),
        ));
        let sender_clone = sender.clone();
        let zip_path = downloader
            .download_if_needed(
                &zip_file.url,
                &zip_file.filename,
                Some(&zip_file.sha256),
                Some(Box::new(move |downloaded, total| {
                    let _ = sender_clone.send(InstallProgress::DownloadProgress {
                        downloaded,
                        total,
                        file_name: "LineageOS ROM".into(),
                    });
                })),
            )
            .await
            .context("Failed to download LineageOS ROM")?;

        // ── Step 4: Verify checksums ──
        let verify_count = if self.update_only { 1 } else if boot_file.is_some() { 2 } else { 1 };
        let mut verified = 0;

        if !self.update_only {
            if let Some(boot) = boot_file {
                let _ = sender.send(InstallProgress::VerifyProgress {
                    verified,
                    total: verify_count,
                    file_name: "Verifying boot image".into(),
                });
                let boot_path = self.download_dir.join(&boot.filename);
                let ok = ChecksumVerifier::verify(&boot_path, &boot.sha256)?;
                if !ok {
                    anyhow::bail!("Checksum mismatch for boot image");
                }
                verified += 1;
            }
        }

        let _ = sender.send(InstallProgress::VerifyProgress {
            verified,
            total: verify_count,
            file_name: "Verifying ROM zip".into(),
        });
        let ok = ChecksumVerifier::verify(&zip_path, &zip_file.sha256)?;
        if !ok {
            anyhow::bail!("Checksum mismatch for LineageOS ROM zip");
        }
        verified += 1;
        let _ = sender.send(InstallProgress::VerifyProgress {
            verified,
            total: verify_count,
            file_name: "All checksums verified".into(),
        });

        if self.update_only {
            // ── Update flow: reboot straight to recovery ──
            let _ = sender.send(InstallProgress::StatusChanged(
                "Rebooting to recovery...".into(),
            ));
            if let Err(e) = adb.reboot_recovery(&self.serial).await {
                log::warn!(
                    "ADB reboot-recovery failed (device may already be in recovery): {}",
                    e
                );
            }
        } else {
            // ── Fresh install: reboot to bootloader → flash boot → reboot to recovery ──
            let _ = sender.send(InstallProgress::StatusChanged(
                "Rebooting to bootloader...".into(),
            ));
            if let Err(e) = adb.reboot_bootloader(&self.serial).await {
                log::warn!(
                    "ADB reboot-bootloader failed (device may already be in fastboot): {}",
                    e
                );
            }

            // Wait for fastboot
            let _ = sender.send(InstallProgress::StatusChanged(
                "Waiting for device in fastboot mode...".into(),
            ));
            self.wait_for_fastboot(&fastboot).await?;

            // Flash boot.img (installs LineageOS recovery)
            if let Some(boot) = boot_file {
                let boot_path = self.download_dir.join(&boot.filename);
                let _ = sender.send(InstallProgress::FlashProgress {
                    current: 1,
                    total: 2,
                    description: "Flashing LineageOS recovery (boot.img)...".into(),
                });
                fastboot
                    .flash(&self.serial, "boot", &boot_path)
                    .await
                    .context("Failed to flash boot image")?;
            }

            // Reboot to recovery
            let _ = sender.send(InstallProgress::StatusChanged(
                "Rebooting to recovery...".into(),
            ));
            fastboot.reboot_recovery(&self.serial).await?;
        }

        // ── Wait for recovery mode ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Waiting for recovery mode...".into(),
        ));
        adb.wait_for_recovery(&self.serial).await?;
        let _ = sender.send(InstallProgress::RecoveryDetected);

        // Give recovery a moment to fully initialize
        tokio::time::sleep(Duration::from_secs(3)).await;

        // ── Prompt: Factory reset (fresh install only) ──
        if !self.update_only {
            let _ = sender.send(InstallProgress::StatusChanged(
                "Waiting for factory reset...".into(),
            ));
            let _ = sender.send(InstallProgress::WaitingForUserAction(
                "On your phone: Select \"Factory reset\" → \"Format data/factory reset\" → confirm, then come back here".into(),
            ));

            // Wait for the user to perform the factory reset in recovery.
            // After a factory reset, recovery restarts — we wait for it to reappear.
            tokio::time::sleep(Duration::from_secs(5)).await;
            adb.wait_for_recovery(&self.serial).await?;
            tokio::time::sleep(Duration::from_secs(3)).await;
        }

        // ── Prompt: Apply from ADB ──
        let _ = sender.send(InstallProgress::WaitingForUserAction(
            "On your phone: Select \"Apply update\" → \"Apply from ADB\"".into(),
        ));

        // Give the user time to navigate the recovery menu
        tokio::time::sleep(Duration::from_secs(5)).await;

        // ── Sideload the ROM zip ──
        let sideload_step = if self.update_only { 1 } else { 2 };
        let sideload_total = if self.update_only { 1 } else { 2 };
        let _ = sender.send(InstallProgress::FlashProgress {
            current: sideload_step,
            total: sideload_total,
            description: "Sideloading LineageOS ROM...".into(),
        });
        adb.sideload(&self.serial, &zip_path)
            .await
            .context("Failed to sideload LineageOS ROM")?;

        // ── Prompt: Reboot ──
        let _ = sender.send(InstallProgress::WaitingForUserAction(
            "Sideload complete! On your phone: Select \"Reboot system now\"".into(),
        ));

        let _ = sender.send(InstallProgress::Complete);
        Ok(())
    }

    /// Fetch the latest build from the LineageOS API v2
    async fn fetch_latest_build(&self) -> Result<LineageBuild> {
        let client = reqwest::Client::builder()
            .user_agent("Sidestep/0.1.0")
            .build()?;

        let resp = client
            .get(&self.api_url)
            .send()
            .await
            .context("Failed to fetch LineageOS API")?;

        if !resp.status().is_success() {
            anyhow::bail!(
                "LineageOS API returned status {} for {}",
                resp.status(),
                self.api_url
            );
        }

        let text = resp.text().await.context("Failed to read API response")?;
        let builds: Vec<LineageBuild> = serde_json::from_str(&text)
            .context("Failed to parse LineageOS API response")?;

        builds
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No builds found in LineageOS API response"))
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
