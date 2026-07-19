// UBports (Ubuntu Touch) installer backend
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::flashing::checksum::ChecksumVerifier;
use crate::flashing::downloader::ImageDownloader;
use crate::flashing::progress::InstallProgress;
use crate::hardware::adb::Adb;
use crate::hardware::fastboot::Fastboot;
use crate::models::installer::{BootstrapStep, FirmwareImage, UbportsDownload};
use crate::models::system_image::SystemImageIndex;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

/// UBports system-image server base URL
const SYSTEM_IMAGE_SERVER: &str = "https://system-image.ubports.com";

/// GPG keyring files needed for system-image recovery install
const GPG_KEYRINGS: &[&str] = &[
    "gpg/image-master.tar.xz",
    "gpg/image-master.tar.xz.asc",
    "gpg/image-signing.tar.xz",
    "gpg/image-signing.tar.xz.asc",
];

/// Orchestrates Ubuntu Touch installation
pub struct UbportsInstaller {
    serial: String,
    channel_path: String,
    firmware: Vec<FirmwareImage>,
    downloads: Vec<UbportsDownload>,
    bootstrap: Vec<BootstrapStep>,
    download_dir: PathBuf,
}

impl UbportsInstaller {
    pub fn new(
        serial: String,
        channel_path: String,
        firmware: Vec<FirmwareImage>,
        downloads: Vec<UbportsDownload>,
        bootstrap: Vec<BootstrapStep>,
    ) -> Self {
        let download_dir = crate::flashing::download_dir().join("ubports");

        Self {
            serial,
            channel_path,
            firmware,
            downloads,
            bootstrap,
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
                    log::error!("UBports installation failed: {:#}", e);
                    let _ = sender.send(InstallProgress::Error(format!("{:#}", e)));
                }
            });
        });

        receiver
    }

    /// Main installation flow (runs on background thread)
    async fn run(&self, sender: &Sender<InstallProgress>, cancel: std::sync::Arc<std::sync::atomic::AtomicBool>) -> Result<()> {
        // New devices carry a `bootstrap` DSL sequence; legacy ones (sargo) carry
        // a flat `firmware` list. One or the other must be present.
        if self.firmware.is_empty() && self.bootstrap.is_empty() {
            anyhow::bail!("No firmware/bootstrap configured for UBports installation");
        }
        let use_bootstrap = !self.bootstrap.is_empty();

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

        // ── Step 1+2: Download (+ verify) bootstrap files or legacy firmware ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Downloading device firmware...".into(),
        ));
        if use_bootstrap {
            self.download_bootstrap_files(&downloader, sender).await?;
        } else {
            self.download_firmware(&downloader, sender).await?;
            self.verify_firmware(sender)?;
        }

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

        // ── Step 6+7: Get the device into its flashing mode ──
        // Samsung Exynos devices (Heimdall bootstrap) have no fastboot — they use
        // Odin download mode; everyone else uses fastboot.
        let heimdall_boot = use_bootstrap
            && self
                .bootstrap
                .iter()
                .any(|s| matches!(s, BootstrapStep::HeimdallFlash { .. }));
        if heimdall_boot {
            let _ = sender.send(InstallProgress::StatusChanged(
                "Rebooting to download mode...".into(),
            ));
            let _ = adb.reboot_download(&self.serial).await;
            crate::hardware::heimdall::Heimdall::new()
                .wait_for_download_mode(120)
                .await
                .context("Device did not enter download mode")?;
        } else {
            let _ = sender.send(InstallProgress::StatusChanged(
                "Rebooting to bootloader...".into(),
            ));
            adb.reboot_bootloader(&self.serial).await?;
            let _ = sender.send(InstallProgress::StatusChanged(
                "Waiting for device in fastboot mode...".into(),
            ));
            self.wait_for_fastboot(&fastboot).await?;
        }

        // ── Step 8+9: Bootstrap the device ──
        if use_bootstrap {
            // Execute the per-device fastboot bootstrap DSL (translated from
            // ubports/installer-configs), then hand off to the recovery push.
            self.run_bootstrap(&fastboot, &adb, sender).await?;
        } else {
            // Legacy flat-firmware path (sargo): flash firmware + format userdata.
            let _ = sender.send(InstallProgress::StatusChanged("Flashing firmware...".into()));
            self.flash_firmware(&fastboot, sender).await?;
            let _ = sender.send(InstallProgress::FlashProgress {
                current: self.firmware.len() + 1,
                total: self.firmware.len() + 2,
                description: "Formatting userdata...".into(),
            });
            fastboot
                .format(&self.serial, "userdata", "ext4")
                .await
                .context("Failed to format userdata")?;
        }

        // ── Step 10: User enters recovery mode ──
        let _ = sender.send(InstallProgress::WaitingForRecovery);
        adb.wait_for_recovery(&self.serial).await?;
        let _ = sender.send(InstallProgress::RecoveryDetected);

        // ── Step 11: Prepare system image (matches UBports adb:preparesystemimage) ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Preparing system image...".into(),
        ));
        let _ = adb.shell(&self.serial, "mount -a").await;
        let _ = adb.shell(&self.serial, "rm -rf /cache/recovery").await;
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
        let mut cumulative_downloaded: u64 = 0;

        for fw in &self.firmware {
            let file_name = fw.filename.clone();
            let sender_clone = sender.clone();
            let prev_downloaded = cumulative_downloaded;

            let path = downloader
                .download_if_needed(
                    &fw.url,
                    &fw.filename,
                    Some(&fw.sha256),
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

            let file_size = tokio::fs::metadata(&path).await?.len();
            cumulative_downloaded += file_size;
        }

        Ok(())
    }

    fn verify_firmware(&self, sender: &Sender<InstallProgress>) -> Result<()> {
        let total = self.firmware.len();
        for (i, fw) in self.firmware.iter().enumerate() {
            let path = self.download_dir.join(&fw.filename);
            let _ = sender.send(InstallProgress::VerifyProgress {
                verified: i + 1,
                total,
                file_name: fw.filename.clone(),
            });

            let ok = ChecksumVerifier::verify(&path, &fw.sha256)?;
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
                checksum: String::new(),
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

    /// Download (and optionally unpack) the files the bootstrap references.
    async fn download_bootstrap_files(
        &self,
        downloader: &ImageDownloader,
        sender: &Sender<InstallProgress>,
    ) -> Result<()> {
        for dl in &self.downloads {
            let sender_clone = sender.clone();
            let name = dl.filename.clone();
            let path = downloader
                .download_if_needed(
                    &dl.url,
                    &dl.filename,
                    dl.sha256.as_deref(),
                    Some(Box::new(move |downloaded, total| {
                        let _ = sender_clone.send(InstallProgress::DownloadProgress {
                            downloaded,
                            total,
                            file_name: name.clone(),
                        });
                    })),
                )
                .await
                .with_context(|| format!("Failed to download {}", dl.filename))?;
            if let Some(ref sha) = dl.sha256 {
                if !ChecksumVerifier::verify(&path, sha)? {
                    anyhow::bail!("Checksum mismatch for {}", dl.filename);
                }
            }
            if dl.unpack {
                let _ = sender.send(InstallProgress::StatusChanged(format!(
                    "Unpacking {}...",
                    dl.filename
                )));
                self.unpack_archive(&path)?;
            }
        }
        Ok(())
    }

    /// Unpack a zip archive into `download_dir/unpacked/`. Bootstrap `flash`
    /// steps reference these as `unpacked/<path>`.
    fn unpack_archive(&self, archive: &PathBuf) -> Result<()> {
        let out = self.download_dir.join("unpacked");
        std::fs::create_dir_all(&out)?;
        let file = std::fs::File::open(archive).context("Failed to open firmware archive")?;
        let mut zip = zip::ZipArchive::new(file).context("Failed to read firmware archive")?;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i)?;
            let Some(rel) = entry.enclosed_name() else { continue };
            let path = out.join(rel);
            if entry.is_dir() {
                std::fs::create_dir_all(&path)?;
            } else {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut outfile = std::fs::File::create(&path)?;
                std::io::copy(&mut entry, &mut outfile)?;
            }
        }
        Ok(())
    }

    /// Resolve a bootstrap `file` reference (plain name or `unpacked/<path>`).
    fn resolve_file(&self, file: &str) -> PathBuf {
        self.download_dir.join(file)
    }

    /// Execute the ordered bootstrap DSL (fastboot phase). Faithful to the
    /// per-device sequence translated from ubports/installer-configs.
    async fn run_bootstrap(
        &self,
        fastboot: &Fastboot,
        _adb: &Adb,
        sender: &Sender<InstallProgress>,
    ) -> Result<()> {
        use crate::hardware::heimdall::{Heimdall, HeimdallFlash};
        let total = self.bootstrap.len();
        for (i, step) in self.bootstrap.iter().enumerate() {
            let _ = sender.send(InstallProgress::FlashProgress {
                current: i + 1,
                total,
                description: bootstrap_label(step),
            });
            match step {
                BootstrapStep::Flash { partition, file, flags } => {
                    let path = self.resolve_file(file);
                    if flags.is_empty() {
                        fastboot
                            .flash(&self.serial, partition, &path)
                            .await
                            .with_context(|| format!("Failed to flash {partition}"))?;
                    } else {
                        let fr: Vec<&str> = flags.iter().map(|s| s.as_str()).collect();
                        fastboot
                            .flash_with_flags(&self.serial, partition, &path, &fr)
                            .await
                            .with_context(|| format!("Failed to flash {partition}"))?;
                    }
                }
                BootstrapStep::Format { partition, fs } => {
                    fastboot.format(&self.serial, partition, fs).await?;
                }
                BootstrapStep::Erase { partition } => {
                    fastboot.erase(&self.serial, partition).await?;
                }
                BootstrapStep::SetActive { slot } => {
                    fastboot.set_active(&self.serial, slot).await?;
                }
                BootstrapStep::DeleteLogicalPartition { partition } => {
                    fastboot.delete_logical_partition(&self.serial, partition).await?;
                }
                BootstrapStep::ResizeLogicalPartition { partition, size } => {
                    fastboot
                        .resize_logical_partition(&self.serial, partition, &size.to_string())
                        .await?;
                }
                BootstrapStep::WipeSuper => {
                    fastboot.wipe_super(&self.serial).await?;
                }
                BootstrapStep::RebootBootloader => {
                    fastboot.reboot_bootloader(&self.serial).await?;
                    self.wait_for_fastboot(fastboot).await?;
                }
                BootstrapStep::RebootFastboot => {
                    fastboot.reboot_fastboot(&self.serial).await?;
                    self.wait_for_fastboot(fastboot).await?;
                }
                BootstrapStep::RebootRecovery => {
                    fastboot.reboot_recovery(&self.serial).await?;
                }
                BootstrapStep::Reboot => {
                    fastboot.reboot(&self.serial).await?;
                }
                BootstrapStep::Wait => {
                    self.wait_for_fastboot(fastboot).await?;
                }
                BootstrapStep::AssertVar { variable, value } => {
                    let got = fastboot.getvar(&self.serial, variable).await.unwrap_or_default();
                    if got.trim() != value {
                        anyhow::bail!(
                            "Device precondition failed: {variable} is '{}', expected '{value}'",
                            got.trim()
                        );
                    }
                }
                BootstrapStep::UserAction { message } => {
                    let _ = sender.send(InstallProgress::WaitingForUserAction(message.clone()));
                }
                BootstrapStep::HeimdallFlash { partition, file } => {
                    let hd = Heimdall::new();
                    hd.flash(
                        &[HeimdallFlash {
                            partition: partition.clone(),
                            image: self.resolve_file(file),
                        }],
                        false,
                    )
                    .await?;
                }
            }
        }
        Ok(())
    }

    async fn flash_firmware(
        &self,
        fastboot: &Fastboot,
        sender: &Sender<InstallProgress>,
    ) -> Result<()> {
        let total = self.firmware.len() + 2; // +1 for format, +1 for overall

        for (i, fw) in self.firmware.iter().enumerate() {
            let _ = sender.send(InstallProgress::FlashProgress {
                current: i + 1,
                total,
                description: format!("Flashing {}...", fw.partition),
            });

            let image_path = self.download_dir.join(&fw.filename);

            if fw.flags.is_empty() {
                fastboot
                    .flash(&self.serial, &fw.partition, &image_path)
                    .await
                    .with_context(|| format!("Failed to flash {}", fw.partition))?;
            } else {
                let flag_refs: Vec<&str> = fw.flags.iter().map(|s| s.as_str()).collect();
                fastboot
                    .flash_with_flags(&self.serial, &fw.partition, &image_path, &flag_refs)
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
        let mut all_files: Vec<(PathBuf, String)> = system_files
            .iter()
            .map(|f| (f.local_path.clone(), f.remote_name.clone()))
            .collect();

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

/// Human-readable label for a bootstrap step, shown in the progress UI.
fn bootstrap_label(step: &BootstrapStep) -> String {
    match step {
        BootstrapStep::Flash { partition, .. } => format!("Flashing {partition}..."),
        BootstrapStep::Format { partition, .. } => format!("Formatting {partition}..."),
        BootstrapStep::Erase { partition } => format!("Erasing {partition}..."),
        BootstrapStep::SetActive { slot } => format!("Setting active slot {slot}..."),
        BootstrapStep::DeleteLogicalPartition { partition } => {
            format!("Removing logical partition {partition}...")
        }
        BootstrapStep::ResizeLogicalPartition { partition, .. } => {
            format!("Resizing {partition}...")
        }
        BootstrapStep::WipeSuper => "Wiping super partition...".to_string(),
        BootstrapStep::RebootBootloader => "Rebooting to bootloader...".to_string(),
        BootstrapStep::RebootFastboot => "Rebooting to fastbootd...".to_string(),
        BootstrapStep::RebootRecovery => "Rebooting to recovery...".to_string(),
        BootstrapStep::Reboot => "Rebooting...".to_string(),
        BootstrapStep::Wait => "Waiting for device...".to_string(),
        BootstrapStep::AssertVar { variable, .. } => format!("Checking {variable}..."),
        BootstrapStep::UserAction { .. } => "Waiting for user...".to_string(),
        BootstrapStep::HeimdallFlash { partition, .. } => format!("Flashing {partition} (Odin)..."),
    }
}
