// /e/OS installer backend
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

/// Orchestrates /e/OS installation via recovery sideload.
///
/// Flow:
///   1. Scrape image index to find latest recovery + ROM for the selected channel
///   2. Download recovery image + ROM zip with progress
///   3. Verify SHA256 checksums
///   4. Reboot to bootloader → flash recovery to boot partition
///   5. Reboot to recovery → wait for recovery
///   6. Prompt user: Factory reset → Format data
///   7. Prompt user: Apply update → Apply from ADB
///   8. adb sideload ROM zip
///   9. Prompt user: Reboot system now
pub struct EosInstaller {
    serial: String,
    base_url: String,
    codename: String,
    channel: String,
    download_dir: PathBuf,
}

impl EosInstaller {
    pub fn new(
        serial: String,
        base_url: String,
        codename: String,
        channel: String,
    ) -> Self {
        let download_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("sidestep")
            .join("eos");

        Self {
            serial,
            base_url,
            codename,
            channel,
            download_dir,
        }
    }

    pub fn spawn(self) -> std::sync::mpsc::Receiver<InstallProgress> {
        let (sender, receiver) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime");

            rt.block_on(async {
                if let Err(e) = self.run(&sender).await {
                    log::error!("/e/OS installation failed: {:#}", e);
                    let _ = sender.send(InstallProgress::Error(format!("{:#}", e)));
                }
            });
        });

        receiver
    }

    async fn run(&self, sender: &Sender<InstallProgress>) -> Result<()> {
        let downloader = ImageDownloader::new(self.download_dir.clone());
        let adb = Adb::new();
        let fastboot = Fastboot::new();

        // ── Step 1: Scrape image index ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Fetching latest /e/OS build...".into(),
        ));
        let (recovery_url, recovery_name, rom_url, rom_name, sha256_url) =
            self.find_latest_build().await?;

        log::info!("/e/OS recovery: {}", recovery_name);
        log::info!("/e/OS ROM: {}", rom_name);

        // ── Step 2: Download SHA256 checksum ──
        let rom_sha256 = self.fetch_sha256(&sha256_url).await?;
        log::info!("ROM SHA256: {}", rom_sha256);

        // ── Step 3: Download recovery image ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Downloading /e/OS recovery...".into(),
        ));
        let sender_clone = sender.clone();
        let recovery_path = downloader
            .download_if_needed(
                &recovery_url,
                &recovery_name,
                None,
                Some(Box::new(move |downloaded, total| {
                    let _ = sender_clone.send(InstallProgress::DownloadProgress {
                        downloaded,
                        total,
                        file_name: "Recovery image".into(),
                    });
                })),
            )
            .await
            .context("Failed to download /e/OS recovery")?;

        // ── Step 4: Download ROM zip ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Downloading /e/OS ROM...".into(),
        ));
        let sender_clone = sender.clone();
        let rom_path = downloader
            .download_if_needed(
                &rom_url,
                &rom_name,
                Some(&rom_sha256),
                Some(Box::new(move |downloaded, total| {
                    let _ = sender_clone.send(InstallProgress::DownloadProgress {
                        downloaded,
                        total,
                        file_name: "/e/OS ROM".into(),
                    });
                })),
            )
            .await
            .context("Failed to download /e/OS ROM")?;

        // ── Step 5: Verify ROM checksum ──
        let _ = sender.send(InstallProgress::VerifyProgress {
            verified: 0,
            total: 1,
            file_name: "Verifying ROM checksum".into(),
        });
        let ok = ChecksumVerifier::verify(&rom_path, &rom_sha256)?;
        if !ok {
            anyhow::bail!("Checksum mismatch for /e/OS ROM zip");
        }
        let _ = sender.send(InstallProgress::VerifyProgress {
            verified: 1,
            total: 1,
            file_name: "Checksum verified".into(),
        });

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

        // Wait for fastboot
        let _ = sender.send(InstallProgress::StatusChanged(
            "Waiting for device in fastboot mode...".into(),
        ));
        self.wait_for_fastboot(&fastboot).await?;

        // ── Step 7: Flash recovery to boot partition ──
        let _ = sender.send(InstallProgress::FlashProgress {
            current: 1,
            total: 2,
            description: "Flashing /e/OS recovery (boot)...".into(),
        });
        fastboot
            .flash(&self.serial, "boot", &recovery_path)
            .await
            .context("Failed to flash /e/OS recovery to boot partition")?;

        // ── Step 8: User manually boots into recovery ──
        // Same pattern as UBports: prompt user to select recovery from
        // the fastboot menu, then wait for ADB recovery to appear.
        let _ = sender.send(InstallProgress::WaitingForRecovery);
        adb.wait_for_recovery(&self.serial).await?;
        let _ = sender.send(InstallProgress::RecoveryDetected);
        tokio::time::sleep(Duration::from_secs(3)).await;

        // ── Step 10: Factory reset + Apply from ADB ──
        // Tell the user everything they need to do, then wait for sideload
        // mode — that way they have as long as they need.
        let _ = sender.send(InstallProgress::WaitingForUserAction(
            "On your phone:\n1. Select \"Factory reset\" → \"Format data/factory reset\" → confirm\n2. Go back, select \"Apply update\" → \"Apply from ADB\"".into(),
        ));
        adb.wait_for_sideload(&self.serial).await?;
        tokio::time::sleep(Duration::from_secs(2)).await;

        // ── Step 11: Sideload ROM ──
        let _ = sender.send(InstallProgress::FlashProgress {
            current: 2,
            total: 2,
            description: "Sideloading /e/OS ROM...".into(),
        });
        adb.sideload(&self.serial, &rom_path)
            .await
            .context("Failed to sideload /e/OS ROM")?;

        // ── Done ──
        let _ = sender.send(InstallProgress::WaitingForUserAction(
            "Sideload complete! On your phone: Select \"Reboot system now\"".into(),
        ));
        let _ = sender.send(InstallProgress::Complete);
        Ok(())
    }

    /// Scrape the /e/OS image index page to find the latest build files
    /// for the selected channel (e.g. "a15", "a14", "t").
    async fn find_latest_build(&self) -> Result<(String, String, String, String, String)> {
        let client = reqwest::Client::builder()
            .user_agent(format!("Sidestep/{}", crate::config::VERSION))
            .build()?;

        let resp = client
            .get(&self.base_url)
            .send()
            .await
            .context("Failed to fetch /e/OS image index")?;

        let html = resp.text().await.context("Failed to read image index")?;

        // Parse href links from the HTML index.
        // The /e/OS server emits unquoted hrefs (e.g. <a href=filename>)
        // and the HTML is minified (many hrefs per line), so we scan for
        // every "href=" occurrence and extract the value delimited by
        // either a quote or >.
        let mut rom_candidates: Vec<String> = Vec::new();
        let mut recovery_candidates: Vec<String> = Vec::new();

        let channel_pattern = format!("-{}-", self.channel);
        let codename_pattern = format!("-community-{}.", self.codename);

        let mut remaining = html.as_str();
        while let Some(pos) = remaining.find("href=") {
            remaining = &remaining[pos + 5..];
            // Determine delimiter: quoted or unquoted
            let (filename, advance) = if remaining.starts_with('"') {
                // href="filename"
                let rest = &remaining[1..];
                if let Some(end) = rest.find('"') {
                    (&rest[..end], 1 + end + 1)
                } else {
                    continue;
                }
            } else if remaining.starts_with('\'') {
                // href='filename'
                let rest = &remaining[1..];
                if let Some(end) = rest.find('\'') {
                    (&rest[..end], 1 + end + 1)
                } else {
                    continue;
                }
            } else {
                // href=filename> (unquoted)
                if let Some(end) = remaining.find('>') {
                    (&remaining[..end], end + 1)
                } else {
                    continue;
                }
            };
            remaining = &remaining[advance..];

            if !filename.contains(&channel_pattern)
                || !filename.contains(&codename_pattern)
            {
                continue;
            }

            // Categorize
            if filename.starts_with("recovery-") && !filename.ends_with(".sha256sum") && !filename.ends_with(".md5sum") {
                recovery_candidates.push(filename.to_string());
            } else if filename.starts_with("e-") && filename.ends_with(".zip") && !filename.contains("recovery") {
                rom_candidates.push(filename.to_string());
            }
        }

        // Sort to get the latest (filenames contain dates, so lexicographic sort works)
        rom_candidates.sort();
        recovery_candidates.sort();

        let rom_name = rom_candidates
            .last()
            .ok_or_else(|| anyhow::anyhow!(
                "No /e/OS ROM found for channel '{}' and codename '{}'",
                self.channel,
                self.codename
            ))?
            .clone();

        let recovery_name = recovery_candidates
            .last()
            .ok_or_else(|| anyhow::anyhow!(
                "No /e/OS recovery found for channel '{}' and codename '{}'",
                self.channel,
                self.codename
            ))?
            .clone();

        let base = self.base_url.trim_end_matches('/');
        let rom_url = format!("{}/{}", base, rom_name);
        let recovery_url = format!("{}/{}", base, recovery_name);
        let sha256_url = format!("{}.sha256sum", rom_url);

        Ok((recovery_url, recovery_name, rom_url, rom_name, sha256_url))
    }

    /// Fetch and parse a .sha256sum file (format: "hash  filename" or just "hash")
    async fn fetch_sha256(&self, url: &str) -> Result<String> {
        let client = reqwest::Client::builder()
            .user_agent(format!("Sidestep/{}", crate::config::VERSION))
            .build()?;

        let resp = client
            .get(url)
            .send()
            .await
            .context("Failed to download SHA256 checksum")?;

        let text = resp.text().await?;
        let hash = text
            .split_whitespace()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Empty SHA256 checksum file"))?
            .to_string();

        Ok(hash)
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
