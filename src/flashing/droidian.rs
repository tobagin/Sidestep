// Droidian installer backend
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

/// Orchestrates Droidian installation
pub struct DroidianInstaller {
    serial: String,
    release_url: String,
    artifact_pattern: String,
    download_dir: PathBuf,
}

impl DroidianInstaller {
    pub fn new(serial: String, release_url: String, artifact_pattern: String) -> Self {
        let download_dir = crate::flashing::download_dir().join("droidian");

        Self {
            serial,
            release_url,
            artifact_pattern,
            download_dir,
        }
    }

    /// Spawn the installer on a background thread, returning immediately.
    /// Progress is reported via the returned mpsc::Receiver.
    pub fn spawn(self, cancel: std::sync::Arc<std::sync::atomic::AtomicBool>) -> std::sync::mpsc::Receiver<InstallProgress> {
        let (sender, receiver) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime");

            rt.block_on(async {
                if let Err(e) = self.run(&sender, cancel).await {
                    log::error!("Droidian installation failed: {:#}", e);
                    let _ = sender.send(InstallProgress::Error(format!("{:#}", e)));
                }
            });
        });

        receiver
    }

    /// Main installation flow (runs on background thread)
    async fn run(&self, sender: &Sender<InstallProgress>, cancel: std::sync::Arc<std::sync::atomic::AtomicBool>) -> Result<()> {
        // Flashing is driven by the flash_all.sh bundled in each Droidian zip
        // (handles A/B slots, vendor_boot, vbmeta, userdata per the device's own
        // device-configuration.conf), so no static flash_partitions are needed.
        let downloader = ImageDownloader::new(self.download_dir.clone()).with_cancel(cancel.clone());
        let adb = Adb::new();

        // Best-effort IMEI/EFS backup while still in adb mode (rooted devices
        // only; everyone else is covered by the mandatory safety warning).
        let backup_dir = crate::flashing::download_dir().join("imei-backup").join(&self.serial);
        let saved = adb.backup_critical_partitions(&self.serial, &backup_dir).await;
        if !saved.is_empty() {
            let _ = sender.send(InstallProgress::StatusChanged(format!(
                "Backed up {} to {}",
                saved.join(", "),
                backup_dir.display()
            )));
        }
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
        // A missing checksum used to silently skip verification entirely (the
        // download and the verify step were both gated on Some). Refuse instead —
        // a renamed/absent SHA256SUMS line must not turn into "flash unverified".
        let expected_hash = self
            .download_and_parse_checksums(&downloader, &checksums_url, &zip_name)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("No checksum found for {zip_name}; refusing to flash unverified image")
            })?;

        // ── Step 3: Download ZIP ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Downloading Droidian image...".into(),
        ));
        let sender_clone = sender.clone();
        let zip_path = downloader
            .download_if_needed(
                &zip_url,
                &zip_name,
                Some(&expected_hash),
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
        let _ = sender.send(InstallProgress::VerifyProgress {
            verified: 0,
            total: 1,
            file_name: zip_name.clone(),
        });
        if !ChecksumVerifier::verify(&zip_path, &expected_hash)? {
            anyhow::bail!("Checksum mismatch for {}", zip_name);
        }
        let _ = sender.send(InstallProgress::VerifyProgress {
            verified: 1,
            total: 1,
            file_name: zip_name.clone(),
        });

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

        // ── Step 8: Run Droidian's own flasher (flash_all.sh) ──
        // It reads the device's device-configuration.conf and flashes the right
        // partitions (A/B or a-only, vendor_boot, vbmeta, userdata) and reboots.
        let _ = sender.send(InstallProgress::StatusChanged(
            "Flashing Droidian...".into(),
        ));
        let _ = sender.send(InstallProgress::FlashProgress {
            current: 0,
            total: 1,
            description: "Flashing Droidian (flash_all.sh)...".into(),
        });
        self.run_flash_all(&extract_dir, sender).await?;
        let _ = sender.send(InstallProgress::FlashProgress {
            current: 1,
            total: 1,
            description: "Flash complete".into(),
        });

        let _ = sender.send(InstallProgress::Complete);
        Ok(())
    }

    /// Execute the `flash_all.sh` bundled in the extracted Droidian zip. The
    /// script flashes exactly the partitions this device needs (per its
    /// device-configuration.conf) and reboots. We run it non-interactively:
    /// `yes` feeds "y" to its device-confirmation prompt, stderr is merged into
    /// stdout, and the bundled fastboot is put on PATH.
    async fn run_flash_all(&self, extract_dir: &PathBuf, sender: &Sender<InstallProgress>) -> Result<()> {
        use tokio::io::{AsyncBufReadExt, BufReader};
        use tokio::process::Command;

        let script = extract_dir.join("flash_all.sh");
        if !script.exists() {
            anyhow::bail!(
                "flash_all.sh not found in the Droidian image — cannot flash safely"
            );
        }

        // Put the bundled fastboot (and adb) directory on PATH so the script's
        // bare `fastboot` calls resolve to the version we ship.
        let mut path = std::env::var("PATH").unwrap_or_default();
        if let Ok(fb) = std::env::var("FASTBOOT_PATH") {
            if let Some(dir) = std::path::Path::new(&fb).parent() {
                path = format!("{}:{}", dir.display(), path);
            }
        }

        let mut child = Command::new("bash")
            .arg("-c")
            .arg("yes y | bash flash_all.sh 2>&1")
            .current_dir(extract_dir)
            .env("PATH", path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context("Failed to launch flash_all.sh (is bash available?)")?;

        // Stream the script's output into the terminal overlay and status line.
        if let Some(out) = child.stdout.take() {
            let mut lines = BufReader::new(out).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                // The script prefixes messages with I:/W:/E:.
                let _ = sender.send(InstallProgress::StatusChanged(trimmed.to_string()));
            }
        }

        let status = child.wait().await.context("flash_all.sh did not complete")?;
        if !status.success() {
            anyhow::bail!("Droidian flash_all.sh failed (exit {:?})", status.code());
        }
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
            // enclosed_name() rejects `..` and absolute paths (zip-slip); a
            // crafted entry name must not write outside extract_dir.
            let Some(rel) = entry.enclosed_name() else {
                log::warn!("Skipping unsafe ZIP entry name: {}", entry.name());
                continue;
            };
            let out_path = extract_dir.join(rel);

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
