// UBports (Ubuntu Touch) installer backend
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::flashing::checksum::ChecksumVerifier;
use crate::flashing::downloader::ImageDownloader;
use crate::flashing::progress::InstallProgress;
use crate::hardware::adb::Adb;
use crate::hardware::fastboot::Fastboot;
use crate::models::system_image::SystemImageIndex;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

/// Firmware image descriptor (hardcoded for sargo MVP)
struct FirmwareImage {
    url: &'static str,
    filename: &'static str,
    partition: &'static str,
    sha256: &'static str,
    /// Extra fastboot flags (e.g., --disable-verity for vbmeta)
    flags: &'static [&'static str],
}

/// Sargo firmware images from cdimage.ubports.com
const SARGO_FIRMWARE: &[FirmwareImage] = &[
    FirmwareImage {
        url: "https://cdimage.ubports.com/devices/sargo/boot.img",
        filename: "boot.img",
        partition: "boot",
        sha256: "3125fa5cdd097cd69b8005af13e4c6a4a4cc61b83c6b13b219799def51fff2fa",
        flags: &[],
    },
    FirmwareImage {
        url: "https://cdimage.ubports.com/devices/sargo/dtbo.img",
        filename: "dtbo.img",
        partition: "dtbo",
        sha256: "51e63686ee4bb15e1ddc296f8809996d645d114347daebacc561cf02d2bfce2d",
        flags: &[],
    },
    FirmwareImage {
        url: "https://cdimage.ubports.com/devices/sargo/vbmeta.img",
        filename: "vbmeta.img",
        partition: "vbmeta",
        sha256: "854a2c2a5e82c2c49a5d9d62c70334002c7dcd9203f904952ff5fc43b2eac420",
        flags: &["--disable-verity", "--disable-verification"],
    },
];

/// UBports system-image server base URL
const SYSTEM_IMAGE_SERVER: &str = "https://system-image.ubports.com";

/// GPG keyring files needed for system-image recovery install
const GPG_KEYRINGS: &[&str] = &[
    "gpg/image-master.tar.xz",
    "gpg/image-master.tar.xz.asc",
    "gpg/image-signing.tar.xz",
    "gpg/image-signing.tar.xz.asc",
];

/// Orchestrates Ubuntu Touch installation on sargo
pub struct UbportsInstaller {
    serial: String,
    channel_path: String,
    download_dir: PathBuf,
}

impl UbportsInstaller {
    pub fn new(serial: String, channel_path: String) -> Self {
        let download_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("sidestep")
            .join("ubports");

        Self {
            serial,
            channel_path,
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
                    log::error!("UBports installation failed: {:#}", e);
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

        // ── Step 1: Download firmware images ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Downloading firmware...".into(),
        ));
        self.download_firmware(&downloader, sender).await?;

        // ── Step 2: Verify firmware checksums ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Verifying firmware checksums...".into(),
        ));
        self.verify_firmware(sender)?;

        // ── Step 3: Fetch system-image index and download files ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Fetching system image index...".into(),
        ));
        let system_files = self
            .download_system_image(&downloader, sender)
            .await?;

        // ── Step 4: Download GPG keyrings ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Downloading GPG keyrings...".into(),
        ));
        self.download_gpg_keyrings(&downloader, sender).await?;

        // ── Step 5: Verify system-image checksums ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Verifying system image checksums...".into(),
        ));
        self.verify_system_image(&system_files, sender)?;

        // ── Step 6: Reboot to bootloader ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Rebooting to bootloader...".into(),
        ));
        adb.reboot_bootloader(&self.serial).await?;

        // ── Step 7: Wait for fastboot device ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Waiting for device in fastboot mode...".into(),
        ));
        self.wait_for_fastboot(&fastboot).await?;

        // ── Step 8: Flash firmware partitions ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Flashing firmware...".into(),
        ));
        self.flash_firmware(&fastboot, sender).await?;

        // ── Step 9: Format userdata as ext4 ──
        // Matches UBports installer: fastboot format:ext4 userdata
        let _ = sender.send(InstallProgress::FlashProgress {
            current: SARGO_FIRMWARE.len() + 1,
            total: SARGO_FIRMWARE.len() + 2,
            description: "Formatting userdata...".into(),
        });
        fastboot
            .format(&self.serial, "userdata", "ext4")
            .await
            .context("Failed to format userdata")?;

        // ── Step 10: User enters recovery mode ──
        // The official UBports installer prompts the user to select
        // "Recovery mode" from the fastboot menu. The halium boot.img
        // must boot in recovery mode — a normal reboot will fail
        // because there is no system partition yet.
        let _ = sender.send(InstallProgress::WaitingForRecovery);
        adb.wait_for_recovery(&self.serial).await?;
        let _ = sender.send(InstallProgress::RecoveryDetected);

        // ── Step 11: Prepare system image (matches UBports adb:preparesystemimage) ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Preparing system image...".into(),
        ));
        // Mount all partitions (errors are non-fatal)
        let _ = adb.shell(&self.serial, "mount -a").await;
        // Wipe stale cache
        let _ = adb.shell(&self.serial, "rm -rf /cache/recovery").await;
        // Create recovery directory
        adb.shell(&self.serial, "mkdir -p /cache/recovery")
            .await
            .context("Failed to create /cache/recovery")?;

        // ── Step 12: Push system-image files to /cache/recovery/ ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Pushing system image files to device...".into(),
        ));
        self.push_system_image_files(&adb, &system_files, sender)
            .await?;

        // ── Step 13: Write ubuntu_command and reboot ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Writing install command...".into(),
        ));
        self.write_ubuntu_command(&adb, &system_files).await?;

        let _ = sender.send(InstallProgress::StatusChanged(
            "Rebooting into recovery to apply update...".into(),
        ));
        adb.shell(&self.serial, "reboot recovery").await?;

        let _ = sender.send(InstallProgress::Complete);
        Ok(())
    }

    // ────────────────────────────────────────────────────────────────
    // Sub-steps
    // ────────────────────────────────────────────────────────────────

    async fn download_firmware(
        &self,
        downloader: &ImageDownloader,
        sender: &Sender<InstallProgress>,
    ) -> Result<()> {
        let total_size: u64 = 0; // We don't know sizes upfront; use per-file progress
        let mut cumulative_downloaded: u64 = 0;

        for fw in SARGO_FIRMWARE {
            let file_name = fw.filename.to_string();
            let sender_clone = sender.clone();
            let prev_downloaded = cumulative_downloaded;

            let path = downloader
                .download_if_needed(
                    fw.url,
                    fw.filename,
                    Some(fw.sha256),
                    Some(Box::new(move |downloaded, total| {
                        let _ = sender_clone.send(InstallProgress::DownloadProgress {
                            downloaded: prev_downloaded + downloaded,
                            total: prev_downloaded + total,
                            file_name: file_name.clone(),
                        });
                    })),
                )
                .await
                .with_context(|| format!("Failed to download {}", fw.filename))?;

            // Update cumulative size from the file we just downloaded
            let file_size = tokio::fs::metadata(&path).await?.len();
            cumulative_downloaded += file_size;
        }

        let _ = cumulative_downloaded;
        let _ = total_size;
        Ok(())
    }

    fn verify_firmware(&self, sender: &Sender<InstallProgress>) -> Result<()> {
        let total = SARGO_FIRMWARE.len();
        for (i, fw) in SARGO_FIRMWARE.iter().enumerate() {
            let path = self.download_dir.join(fw.filename);
            let _ = sender.send(InstallProgress::VerifyProgress {
                verified: i + 1,
                total,
                file_name: fw.filename.to_string(),
            });

            let ok = ChecksumVerifier::verify(&path, fw.sha256)?;
            if !ok {
                anyhow::bail!(
                    "Checksum mismatch for {} — expected {}",
                    fw.filename,
                    fw.sha256
                );
            }
        }
        Ok(())
    }

    /// Fetch the system-image index.json, pick latest full image, download all files.
    /// Returns list of (local_path, remote_filename) pairs for files to push.
    async fn download_system_image(
        &self,
        downloader: &ImageDownloader,
        sender: &Sender<InstallProgress>,
    ) -> Result<Vec<SystemFile>> {
        let index_url = format!(
            "{}/{}/index.json",
            SYSTEM_IMAGE_SERVER, self.channel_path
        );
        log::info!("Fetching system-image index from {}", index_url);

        let client = reqwest::Client::builder()
            .user_agent(format!("Sidestep/{}", crate::config::VERSION))
            .build()?;
        let index_text = client.get(&index_url).send().await?.text().await?;
        let index: SystemImageIndex =
            serde_json::from_str(&index_text).context("Failed to parse system-image index.json")?;

        let entry = index
            .latest_full()
            .context("No full image found in system-image index")?;

        log::info!(
            "Selected system-image version {} ({} files)",
            entry.version,
            entry.files.len()
        );

        // Calculate total download size
        let total_size: u64 = entry.files.iter().map(|f| f.size).sum();

        let mut downloaded_so_far: u64 = 0;
        let mut system_files = Vec::new();

        for file in &entry.files {
            let url = format!("{}{}", SYSTEM_IMAGE_SERVER, file.path);
            let filename = file
                .path
                .rsplit('/')
                .next()
                .unwrap_or(&file.path);

            let sender_clone = sender.clone();
            let file_name_str = filename.to_string();
            let prev_downloaded = downloaded_so_far;
            let total = total_size;

            let local_path = downloader
                .download_if_needed(
                    &url,
                    filename,
                    Some(&file.checksum),
                    Some(Box::new(move |dl, _per_file_total| {
                        let _ = sender_clone.send(InstallProgress::DownloadProgress {
                            downloaded: prev_downloaded + dl,
                            total,
                            file_name: file_name_str.clone(),
                        });
                    })),
                )
                .await
                .with_context(|| format!("Failed to download {}", filename))?;

            system_files.push(SystemFile {
                local_path: local_path.clone(),
                remote_name: filename.to_string(),
                checksum: file.checksum.clone(),
            });

            // Also download the .asc signature
            let sig_url = format!("{}{}", SYSTEM_IMAGE_SERVER, file.signature);
            let sig_filename = file
                .signature
                .rsplit('/')
                .next()
                .unwrap_or(&file.signature);

            let sig_path = downloader
                .download_if_needed(&sig_url, sig_filename, None, None)
                .await
                .with_context(|| format!("Failed to download signature {}", sig_filename))?;

            system_files.push(SystemFile {
                local_path: sig_path,
                remote_name: sig_filename.to_string(),
                checksum: String::new(), // .asc files aren't individually checksummed
            });

            downloaded_so_far += file.size;
        }

        Ok(system_files)
    }

    async fn download_gpg_keyrings(
        &self,
        downloader: &ImageDownloader,
        sender: &Sender<InstallProgress>,
    ) -> Result<()> {
        for keyring_path in GPG_KEYRINGS {
            let url = format!("{}/{}", SYSTEM_IMAGE_SERVER, keyring_path);
            let filename = keyring_path.rsplit('/').next().unwrap_or(keyring_path);

            let _ = sender.send(InstallProgress::StatusChanged(
                format!("Downloading {}...", filename),
            ));

            downloader
                .download_if_needed(&url, filename, None, None)
                .await
                .with_context(|| format!("Failed to download GPG keyring {}", filename))?;
        }
        Ok(())
    }

    fn verify_system_image(
        &self,
        system_files: &[SystemFile],
        sender: &Sender<InstallProgress>,
    ) -> Result<()> {
        let checksum_files: Vec<_> = system_files
            .iter()
            .filter(|f| !f.checksum.is_empty())
            .collect();
        let total = checksum_files.len();

        for (i, file) in checksum_files.iter().enumerate() {
            let _ = sender.send(InstallProgress::VerifyProgress {
                verified: i + 1,
                total,
                file_name: file.remote_name.clone(),
            });

            let ok = ChecksumVerifier::verify(&file.local_path, &file.checksum)?;
            if !ok {
                anyhow::bail!(
                    "Checksum mismatch for {}",
                    file.remote_name
                );
            }
        }
        Ok(())
    }

    async fn wait_for_fastboot(&self, fastboot: &Fastboot) -> Result<()> {
        // Poll fastboot devices until our device appears
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

    async fn flash_firmware(
        &self,
        fastboot: &Fastboot,
        sender: &Sender<InstallProgress>,
    ) -> Result<()> {
        let total = SARGO_FIRMWARE.len() + 2; // +1 for erase, +1 for overall

        for (i, fw) in SARGO_FIRMWARE.iter().enumerate() {
            let _ = sender.send(InstallProgress::FlashProgress {
                current: i + 1,
                total,
                description: format!("Flashing {}...", fw.partition),
            });

            let image_path = self.download_dir.join(fw.filename);

            if fw.flags.is_empty() {
                fastboot
                    .flash(&self.serial, fw.partition, &image_path)
                    .await
                    .with_context(|| format!("Failed to flash {}", fw.partition))?;
            } else {
                fastboot
                    .flash_with_flags(&self.serial, fw.partition, &image_path, fw.flags)
                    .await
                    .with_context(|| format!("Failed to flash {}", fw.partition))?;
            }
        }

        Ok(())
    }

    async fn push_system_image_files(
        &self,
        adb: &Adb,
        system_files: &[SystemFile],
        sender: &Sender<InstallProgress>,
    ) -> Result<()> {
        // Collect all files to push: system-image files + GPG keyrings
        let mut all_files: Vec<(PathBuf, String)> = system_files
            .iter()
            .map(|f| (f.local_path.clone(), f.remote_name.clone()))
            .collect();

        // Add GPG keyring files
        for keyring_path in GPG_KEYRINGS {
            let filename = keyring_path.rsplit('/').next().unwrap_or(keyring_path);
            let local = self.download_dir.join(filename);
            if local.exists() {
                all_files.push((local, filename.to_string()));
            }
        }

        let total = all_files.len();
        for (i, (local_path, remote_name)) in all_files.iter().enumerate() {
            let _ = sender.send(InstallProgress::FlashProgress {
                current: i + 1,
                total,
                description: format!("Pushing {}...", remote_name),
            });

            let remote = format!("/cache/recovery/{}", remote_name);
            adb.push(&self.serial, local_path, &remote)
                .await
                .with_context(|| format!("Failed to push {}", remote_name))?;
        }

        Ok(())
    }

    async fn write_ubuntu_command(
        &self,
        adb: &Adb,
        system_files: &[SystemFile],
    ) -> Result<()> {
        // Build the ubuntu_command content
        // Only include .tar.xz files (not .asc) for update lines
        let tar_files: Vec<_> = system_files
            .iter()
            .filter(|f| f.remote_name.ends_with(".tar.xz"))
            .collect();

        let mut lines = Vec::new();
        lines.push("format system".to_string());
        lines.push("load_keyring image-master.tar.xz image-master.tar.xz.asc".to_string());
        lines.push("load_keyring image-signing.tar.xz image-signing.tar.xz.asc".to_string());
        lines.push("mount system".to_string());
        lines.push("format data".to_string());

        for file in &tar_files {
            let asc_name = format!("{}.asc", file.remote_name);
            lines.push(format!("update {} {}", file.remote_name, asc_name));
        }

        lines.push("unmount system".to_string());

        let command_content = lines.join("\n");
        log::info!("ubuntu_command:\n{}", command_content);

        // Write to /cache/recovery/ubuntu_command via adb shell
        // Escape content for shell
        let escaped = command_content.replace('\'', "'\\''");
        let shell_cmd = format!(
            "echo '{}' > /cache/recovery/ubuntu_command",
            escaped
        );
        adb.shell(&self.serial, &shell_cmd).await?;

        Ok(())
    }
}

/// A downloaded system-image file with metadata
#[derive(Debug, Clone)]
struct SystemFile {
    local_path: PathBuf,
    remote_name: String,
    checksum: String,
}
