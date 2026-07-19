// Heimdall wrapper (Samsung download-mode / Odin protocol flashing)
// SPDX-License-Identifier: GPL-3.0-or-later
//
// Samsung Exynos devices have no fastboot — they flash over the Odin protocol in
// "download mode". `heimdall` is the open-source client for that protocol. This
// wraps the CLI the same way `fastboot.rs` wraps fastboot.
//
// NOTE: this backend is not yet validated on real hardware. Partition names are
// PIT entries (e.g. RECOVERY, BOOT, SYSTEM) and are device-specific; wrong names
// or images can brick a device. Ship only with device configs that have been
// tested on the actual phone.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;

/// A partition to flash: the PIT entry name and the image to write to it.
#[derive(Debug, Clone)]
pub struct HeimdallFlash {
    pub partition: String,
    pub image: PathBuf,
}

/// Heimdall command wrapper.
#[derive(Debug, Clone)]
pub struct Heimdall {
    binary_path: String,
}

impl Default for Heimdall {
    fn default() -> Self {
        Self::new()
    }
}

impl Heimdall {
    pub fn new() -> Self {
        let binary_path = std::env::var("HEIMDALL_PATH").unwrap_or_else(|_| "heimdall".to_string());
        Self { binary_path }
    }

    /// True if a device is currently in download mode and reachable.
    pub async fn detect(&self) -> bool {
        Command::new(&self.binary_path)
            .arg("detect")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Build the argument list for a `heimdall flash` invocation. Pure so it can
    /// be unit-tested without a device: `flash --<PART> <image> ... [--no-reboot]`.
    fn flash_args(flashes: &[HeimdallFlash], reboot: bool) -> Vec<std::ffi::OsString> {
        use std::ffi::OsString;
        let mut args: Vec<OsString> = vec!["flash".into()];
        for f in flashes {
            args.push(OsString::from(format!("--{}", f.partition)));
            args.push(f.image.clone().into_os_string());
        }
        if !reboot {
            args.push("--no-reboot".into());
        }
        args
    }

    /// Flash a set of partitions in a single Odin session. `reboot` controls
    /// whether the device reboots afterwards (Heimdall's default is to reboot).
    pub async fn flash(&self, flashes: &[HeimdallFlash], reboot: bool) -> Result<()> {
        if flashes.is_empty() {
            anyhow::bail!("No partitions to flash");
        }
        log::info!(
            "heimdall flash {:?} (reboot={})",
            flashes.iter().map(|f| &f.partition).collect::<Vec<_>>(),
            reboot
        );

        let output = Command::new(&self.binary_path)
            .args(Self::flash_args(flashes, reboot))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run heimdall flash")?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        log::debug!("heimdall flash output: {}", stderr);
        if !output.status.success() {
            anyhow::bail!("heimdall flash failed: {}", stderr);
        }
        Ok(())
    }

    /// Wait (polling) for a device to appear in download mode, up to `secs`.
    pub async fn wait_for_download_mode(&self, secs: u64) -> Result<()> {
        use std::time::Duration;
        for _ in 0..secs {
            if self.detect().await {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        anyhow::bail!("Timed out waiting for device to enter download mode")
    }

    /// Convenience: resolve a partition name + image path into a HeimdallFlash.
    pub fn flash_entry(partition: &str, image: &Path) -> HeimdallFlash {
        HeimdallFlash {
            partition: partition.to_string(),
            image: image.to_path_buf(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_flash_args_in_order_with_reboot_control() {
        let flashes = vec![
            Heimdall::flash_entry("RECOVERY", Path::new("/img/recovery.img")),
            Heimdall::flash_entry("BOOT", Path::new("/img/boot.img")),
        ];
        // Default: reboot → no --no-reboot flag.
        let args = Heimdall::flash_args(&flashes, true);
        let as_str: Vec<_> = args.iter().map(|a| a.to_string_lossy().to_string()).collect();
        assert_eq!(
            as_str,
            vec![
                "flash",
                "--RECOVERY",
                "/img/recovery.img",
                "--BOOT",
                "/img/boot.img",
            ]
        );
        // reboot=false appends --no-reboot.
        let args = Heimdall::flash_args(&flashes, false);
        assert_eq!(args.last().unwrap().to_string_lossy(), "--no-reboot");
    }
}
