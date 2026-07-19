// GrapheneOS installer backend (Pixel factory-image install)
// SPDX-License-Identifier: GPL-3.0-or-later
//
// GrapheneOS ships a signed install archive per device+build containing a
// `flash-all.sh` (the same shape as Google's Pixel factory images). We discover
// the latest stable build, download the archive, and run its bundled flash-all
// script — the same approach we use for Droidian. Bootloader re-locking (for
// verified boot) is left as a guided manual step, as it wipes and needs an
// on-device confirmation.

use crate::flashing::downloader::ImageDownloader;
use crate::flashing::progress::InstallProgress;
use crate::hardware::adb::Adb;
use crate::hardware::fastboot::Fastboot;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::mpsc::Sender;

const RELEASES: &str = "https://releases.grapheneos.org";

/// GrapheneOS factory-image signing key (from releases.grapheneos.org/allowed_signers),
/// embedded so a compromised server can't swap it. SSHSIG namespace below.
const GRAPHENEOS_PUBKEY: &str =
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIIUg/m5CoP83b0rfSCzYSVA4cw4ir49io5GPoxbgxdJE contact@grapheneos.org";
const GRAPHENEOS_SIG_NAMESPACE: &str = "factory images";

/// Orchestrates a GrapheneOS install on a supported Pixel.
pub struct GrapheneosInstaller {
    serial: String,
    /// GrapheneOS device codename (e.g. "shiba", "oriole").
    device: String,
    download_dir: PathBuf,
}

impl GrapheneosInstaller {
    pub fn new(serial: String, device: String) -> Self {
        let download_dir = crate::flashing::download_dir().join("grapheneos");
        Self {
            serial,
            device,
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
                    log::error!("GrapheneOS installation failed: {:#}", e);
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
        let downloader = ImageDownloader::new(self.download_dir.clone()).with_cancel(cancel.clone());
        let adb = Adb::new();
        let fastboot = Fastboot::new();

        // Best-effort IMEI/EFS backup while still in adb mode (rooted only).
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

        // ── Step 1: Discover the latest stable build ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Checking latest GrapheneOS release...".into(),
        ));
        let build = self.latest_build().await?;
        log::info!("GrapheneOS latest build for {}: {}", self.device, build);

        // ── Step 2: Download the install archive ──
        // Verified below against GrapheneOS's embedded Ed25519 key (their
        // ssh-keygen -Y verify scheme), reproduced in-app with the ssh-key crate.
        let zip_name = format!("{}-install-{}.zip", self.device, build);
        let zip_url = format!("{}/{}", RELEASES, zip_name);
        let _ = sender.send(InstallProgress::StatusChanged(
            "Downloading GrapheneOS...".into(),
        ));
        let sender_clone = sender.clone();
        let zip_path = downloader
            .download_if_needed(
                &zip_url,
                &zip_name,
                None,
                Some(Box::new(move |downloaded, total| {
                    let _ = sender_clone.send(InstallProgress::DownloadProgress {
                        downloaded,
                        total,
                        file_name: "GrapheneOS".into(),
                    });
                })),
            )
            .await
            .context("Failed to download GrapheneOS install archive")?;

        // ── Step 2b: Verify the SSH (Ed25519) signature against the embedded key ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Verifying GrapheneOS signature...".into(),
        ));
        let sig_url = format!("{zip_url}.sig");
        let sig_pem = reqwest::Client::builder()
            .user_agent(format!("Sidestep/{}", crate::config::VERSION))
            .https_only(true)
            .build()?
            .get(&sig_url)
            .send()
            .await
            .context("Failed to download GrapheneOS signature")?
            .error_for_status()
            .context("GrapheneOS signature missing")?
            .text()
            .await?;
        Self::verify_signature(&zip_path, &sig_pem)
            .context("GrapheneOS signature verification failed — refusing to flash")?;

        // ── Step 3: Extract ──
        let _ = sender.send(InstallProgress::StatusChanged("Extracting...".into()));
        let extract_dir = self.download_dir.join("extracted");
        self.extract_zip(&zip_path, &extract_dir)?;

        // ── Step 4: Reboot to bootloader ──
        let _ = sender.send(InstallProgress::StatusChanged(
            "Rebooting to bootloader...".into(),
        ));
        if let Err(e) = adb.reboot_bootloader(&self.serial).await {
            log::warn!("adb reboot-bootloader failed (may already be in fastboot): {e}");
        }
        self.wait_for_fastboot(&fastboot).await?;

        // ── Step 5: Run GrapheneOS's bundled flash-all.sh ──
        let _ = sender.send(InstallProgress::FlashProgress {
            current: 0,
            total: 1,
            description: "Flashing GrapheneOS (flash-all.sh)...".into(),
        });
        self.run_flash_all(&extract_dir, sender).await?;
        let _ = sender.send(InstallProgress::FlashProgress {
            current: 1,
            total: 1,
            description: "Flash complete".into(),
        });

        // ── Step 6: Guide the user to re-lock the bootloader ──
        // Re-locking restores verified boot but wipes data and needs on-device
        // confirmation, so it's a manual step rather than automated.
        let _ = sender.send(InstallProgress::WaitingForUserAction(
            "GrapheneOS flashed. For full security, re-lock the bootloader: boot to \
             fastboot and run `fastboot flashing lock` (this wipes data and needs \
             confirmation on the device). Only do this after GrapheneOS boots successfully."
                .into(),
        ));
        let _ = sender.send(InstallProgress::Complete);
        Ok(())
    }

    /// Fetch the `{device}-stable` metadata and return the build id (first field).
    async fn latest_build(&self) -> Result<String> {
        let client = reqwest::Client::builder()
            .user_agent(format!("Sidestep/{}", crate::config::VERSION))
            .https_only(true)
            .build()?;
        let text = client
            .get(format!("{RELEASES}/{}-stable", self.device))
            .send()
            .await
            .context("Failed to query GrapheneOS releases")?
            .error_for_status()
            .context("GrapheneOS has no stable release for this device")?
            .text()
            .await?;
        text.split_whitespace()
            .next()
            .map(|s| s.to_string())
            .context("Empty GrapheneOS stable metadata")
    }

    /// Verify the install archive's SSHSIG against the embedded GrapheneOS key.
    /// The file is memory-mapped so hashing the multi-GB archive doesn't require
    /// loading it onto the heap. Fail-closed: any mismatch aborts the flash.
    fn verify_signature(zip_path: &std::path::Path, sig_pem: &str) -> Result<()> {
        use ssh_key::{public::PublicKey, SshSig};
        let pk = PublicKey::from_openssh(GRAPHENEOS_PUBKEY)
            .context("Invalid embedded GrapheneOS public key")?;
        let sig = SshSig::from_pem(sig_pem.as_bytes())
            .context("Malformed GrapheneOS signature")?;
        let file = std::fs::File::open(zip_path).context("Failed to open archive for verify")?;
        // SAFETY: the file is not mutated while mapped (we just downloaded it).
        let mmap = unsafe { memmap2::Mmap::map(&file) }.context("Failed to map archive")?;
        pk.verify(GRAPHENEOS_SIG_NAMESPACE, &mmap[..], &sig)
            .map_err(|e| anyhow::anyhow!("signature does not match GrapheneOS key: {e}"))?;
        log::info!("GrapheneOS signature verified");
        Ok(())
    }

    fn extract_zip(&self, zip_path: &PathBuf, extract_dir: &PathBuf) -> Result<()> {
        if extract_dir.exists() {
            std::fs::remove_dir_all(extract_dir).context("Failed to clean previous extraction")?;
        }
        std::fs::create_dir_all(extract_dir).context("Failed to create extraction directory")?;
        let file = std::fs::File::open(zip_path).context("Failed to open GrapheneOS archive")?;
        let mut archive = zip::ZipArchive::new(file).context("Failed to read GrapheneOS archive")?;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let Some(rel) = entry.enclosed_name() else {
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
        Ok(())
    }

    /// Run the archive's `flash-all.sh`. GrapheneOS nests it inside a per-build
    /// subdirectory, so we locate it. NOTE: uses the bundled fastboot on PATH;
    /// the newest Pixels may need a newer platform-tools than the pinned 33.0.3.
    async fn run_flash_all(&self, extract_dir: &PathBuf, sender: &Sender<InstallProgress>) -> Result<()> {
        use tokio::io::{AsyncBufReadExt, BufReader};
        use tokio::process::Command;

        let script_dir = self.find_flash_all_dir(extract_dir)?;

        let mut path = std::env::var("PATH").unwrap_or_default();
        if let Ok(fb) = std::env::var("FASTBOOT_PATH") {
            if let Some(dir) = std::path::Path::new(&fb).parent() {
                path = format!("{}:{}", dir.display(), path);
            }
        }
        // Prefer the newer bundled fastboot (36.x) for GrapheneOS — recent Pixels
        // need platform-tools 35+, and this path never flashes vbmeta with the
        // --disable-verity rewrite that the pinned 33.0.3 exists to protect.
        let newer = "/app/opt/platform-tools-36";
        if std::path::Path::new(newer).join("fastboot").exists() {
            path = format!("{newer}:{path}");
        }

        let mut child = Command::new("bash")
            .arg("-c")
            .arg("bash flash-all.sh 2>&1")
            .current_dir(&script_dir)
            .env("PATH", path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context("Failed to launch flash-all.sh (is bash available?)")?;

        if let Some(out) = child.stdout.take() {
            let mut lines = BufReader::new(out).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    let _ = sender.send(InstallProgress::StatusChanged(trimmed.to_string()));
                }
            }
        }

        let status = child.wait().await.context("flash-all.sh did not complete")?;
        if !status.success() {
            anyhow::bail!("GrapheneOS flash-all.sh failed (exit {:?})", status.code());
        }
        Ok(())
    }

    /// Locate the directory containing `flash-all.sh` (archive root or a nested
    /// per-build subdirectory).
    fn find_flash_all_dir(&self, extract_dir: &PathBuf) -> Result<PathBuf> {
        if extract_dir.join("flash-all.sh").exists() {
            return Ok(extract_dir.clone());
        }
        for entry in std::fs::read_dir(extract_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() && entry.path().join("flash-all.sh").exists() {
                return Ok(entry.path());
            }
        }
        anyhow::bail!("flash-all.sh not found in the GrapheneOS archive")
    }

    async fn wait_for_fastboot(&self, fastboot: &Fastboot) -> Result<()> {
        use std::time::Duration;
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

#[cfg(test)]
mod tests {
    use super::*;
    use ssh_key::{public::PublicKey, SshSig};

    // A real GrapheneOS install-archive signature (shiba). We can't verify it
    // against the multi-GB archive in a unit test, but parsing the embedded key
    // and this signature — and checking the namespace — catches format/API bugs.
    const REAL_SIG: &str = "-----BEGIN SSH SIGNATURE-----\n\
U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAghSD+bkKg/zdvSt9ILNhJUDhzDi\n\
Kvj2KjkY+jFuDF0kQAAAAOZmFjdG9yeSBpbWFnZXMAAAAAAAAABnNoYTUxMgAAAFMAAAAL\n\
c3NoLWVkMjU1MTkAAABA+p1JMjrHzUNVtgK/D3f99vL+O4tFAFrMSFZwl5iBKIrqJ5ny80\n\
crQv15uevW8drVPHiX0WQxRrxg574wWsm5AQ==\n\
-----END SSH SIGNATURE-----\n";

    #[test]
    fn embedded_key_and_signature_parse() {
        let pk = PublicKey::from_openssh(GRAPHENEOS_PUBKEY).expect("embedded key parses");
        assert!(pk.key_data().ed25519().is_some(), "key is ed25519");
        let sig = SshSig::from_pem(REAL_SIG.as_bytes()).expect("real signature parses");
        assert_eq!(sig.namespace(), GRAPHENEOS_SIG_NAMESPACE);
    }
}
