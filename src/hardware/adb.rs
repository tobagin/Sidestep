// ADB wrapper
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

/// Represents an ADB connection to a device
#[derive(Debug, Clone)]
pub struct AdbDevice {
    pub serial: String,
    pub state: String,
}

/// ADB command wrapper
#[derive(Debug, Clone)]
pub struct Adb {
    binary_path: String,
}

impl Default for Adb {
    fn default() -> Self {
        Self::new()
    }
}

impl Adb {
    pub fn new() -> Self {
        // Try to find adb in common locations
        let binary_path = std::env::var("ADB_PATH")
            .unwrap_or_else(|_| "adb".to_string());
        
        Self { binary_path }
    }

    /// List connected ADB devices
    pub async fn devices(&self) -> Result<Vec<AdbDevice>> {
        let output = Command::new(&self.binary_path)
            .arg("devices")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run adb devices")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut devices = Vec::new();

        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                devices.push(AdbDevice {
                    serial: parts[0].to_string(),
                    state: parts[1].to_string(),
                });
            }
        }

        log::debug!("ADB devices: {:?}", devices);
        Ok(devices)
    }

    /// Get device property via getprop
    pub async fn getprop(&self, serial: &str, prop: &str) -> Result<String> {
        let output = Command::new(&self.binary_path)
            .args(["-s", serial, "shell", "getprop", prop])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run adb getprop")?;

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Get device codename (ro.product.device)
    pub async fn get_codename(&self, serial: &str) -> Result<String> {
        self.getprop(serial, "ro.product.device").await
    }

    /// Get device model name
    pub async fn get_model(&self, serial: &str) -> Result<String> {
        self.getprop(serial, "ro.product.model").await
    }

    /// Run an adb subcommand and fail if it exits non-zero. Many callers
    /// previously ignored the exit status, so a device that had vanished or
    /// errored looked like success — dangerous right before a flash step.
    async fn run_checked(&self, args: &[&str], ctx: &'static str) -> Result<()> {
        let output = Command::new(&self.binary_path)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context(ctx)?;
        if !output.status.success() {
            anyhow::bail!("{}: {}", ctx, String::from_utf8_lossy(&output.stderr).trim());
        }
        Ok(())
    }

    /// Run an adb `wait-for-*` command, enforcing our own timeout and killing
    /// the child if it's exceeded — adb's wait commands otherwise block forever
    /// if the device never appears, hanging the whole install.
    async fn wait_checked(&self, args: &[&str], ctx: &'static str) -> Result<()> {
        use std::time::Duration;
        let fut = Command::new(&self.binary_path)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .output();
        let output = tokio::time::timeout(Duration::from_secs(300), fut)
            .await
            .map_err(|_| anyhow::anyhow!("{}: timed out after 5 minutes", ctx))?
            .context(ctx)?;
        if !output.status.success() {
            anyhow::bail!("{}: {}", ctx, String::from_utf8_lossy(&output.stderr).trim());
        }
        Ok(())
    }

    /// Reboot into bootloader mode
    pub async fn reboot_bootloader(&self, serial: &str) -> Result<()> {
        log::info!("Rebooting {} to bootloader", serial);
        self.run_checked(
            &["-s", serial, "reboot", "bootloader"],
            "Failed to reboot to bootloader",
        )
        .await
    }

    /// Run a shell command on the device
    pub async fn shell(&self, serial: &str, cmd: &str) -> Result<String> {
        let output = Command::new(&self.binary_path)
            .args(["-s", serial, "shell", cmd])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run adb shell")?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Get Android version (ro.build.version.release)
    pub async fn get_android_version(&self, serial: &str) -> Result<String> {
        self.getprop(serial, "ro.build.version.release").await
    }

    /// Get build display ID (ro.build.display.id)
    pub async fn get_build_id(&self, serial: &str) -> Result<String> {
        self.getprop(serial, "ro.build.display.id").await
    }

    /// Get battery level (0–100)
    pub async fn get_battery_level(&self, serial: &str) -> Result<u8> {
        // Try sysfs first
        let output = self.shell(serial, "cat /sys/class/power_supply/battery/capacity").await?;
        let trimmed = output.trim();
        if let Ok(level) = trimmed.parse::<u8>() {
            return Ok(level);
        }

        // Fallback: parse "dumpsys battery" for the "level:" line
        let dump = self.shell(serial, "dumpsys battery").await?;
        for line in dump.lines() {
            let line = line.trim();
            if let Some(val) = line.strip_prefix("level:") {
                if let Ok(level) = val.trim().parse::<u8>() {
                    return Ok(level);
                }
            }
        }

        anyhow::bail!("Could not determine battery level")
    }

    /// Check if device is unlocked
    pub async fn is_unlocked(&self, serial: &str) -> Result<bool> {
        // Method 1: Check ro.boot.flash.locked
        let locked = self.getprop(serial, "ro.boot.flash.locked").await?;
        if locked == "0" {
            return Ok(true);
        }
        
        // Method 2: Check sys.oem_unlock_allowed (developer settings)
        // This usually just means it CAN be unlocked, not that it IS unlocked, 
        // but sometimes it's relevant context. 
        // For actual lock status, ro.boot.flash.locked or ro.boot.verifiedbootstate are better.
        
        // Method 3: Check ro.boot.verifiedbootstate
        let boot_state = self.getprop(serial, "ro.boot.verifiedbootstate").await?;
        if boot_state == "orange" { // Orange means unlocked/unverified
            return Ok(true);
        }

        Ok(false)
    }

    /// Wait for device to enter recovery mode
    pub async fn wait_for_recovery(&self, serial: &str) -> Result<()> {
        log::info!("Waiting for {} to enter recovery mode", serial);
        self.wait_checked(
            &["-s", serial, "wait-for-recovery"],
            "Failed to wait for recovery",
        )
        .await
    }

    pub async fn wait_for_sideload(&self, serial: &str) -> Result<()> {
        log::info!("Waiting for {} to enter sideload mode", serial);
        self.wait_checked(
            &["-s", serial, "wait-for-sideload"],
            "Failed to wait for sideload",
        )
        .await
    }

    /// Reboot into recovery mode
    pub async fn reboot_recovery(&self, serial: &str) -> Result<()> {
        log::info!("Rebooting {} to recovery", serial);
        self.run_checked(
            &["-s", serial, "reboot", "recovery"],
            "Failed to reboot to recovery",
        )
        .await
    }

    /// Reboot into download (Odin) mode — Samsung Exynos flashing via Heimdall.
    pub async fn reboot_download(&self, serial: &str) -> Result<()> {
        log::info!("Rebooting {} to download mode", serial);
        self.run_checked(
            &["-s", serial, "reboot", "download"],
            "Failed to reboot to download mode",
        )
        .await
    }

    /// Sideload a zip file via ADB sideload (used in recovery mode)
    pub async fn sideload(&self, serial: &str, zip_path: &Path) -> Result<()> {
        log::info!("Sideloading {} to {}", zip_path.display(), serial);

        let output = Command::new(&self.binary_path)
            .arg("-s")
            .arg(serial)
            .arg("sideload")
            .arg(zip_path) // OsStr — no panic on non-UTF8 paths
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run adb sideload")?;

        // adb sideload returns exit code 0 on success, but may also return
        // exit code 1 with "serving" messages that are actually fine.
        // The real failure indicator is specific error strings.
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        log::debug!("adb sideload stdout: {}", stdout);
        log::debug!("adb sideload stderr: {}", stderr);

        if !output.status.success() {
            // "adb: sideload connection failed" is a real error
            if stderr.contains("sideload connection failed") || stderr.contains("error") {
                anyhow::bail!("adb sideload failed: {}", stderr);
            }
            // Otherwise it likely completed successfully despite non-zero exit
            log::warn!("adb sideload exited with non-zero status but no error detected");
        }

        Ok(())
    }

    /// Push a local file to the device
    pub async fn push(&self, serial: &str, local: &Path, remote: &str) -> Result<()> {
        log::info!("Pushing {} to {}", local.display(), remote);

        let output = Command::new(&self.binary_path)
            .arg("-s")
            .arg(serial)
            .arg("push")
            .arg(local) // OsStr — no panic on non-UTF8 paths
            .arg(remote)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run adb push")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("adb push failed: {}", stderr);
        }

        Ok(())
    }

    /// Pull a file from the device to a local path.
    pub async fn pull(&self, serial: &str, remote: &str, local: &Path) -> Result<()> {
        let output = Command::new(&self.binary_path)
            .arg("-s")
            .arg(serial)
            .arg("pull")
            .arg(remote)
            .arg(local) // OsStr — no panic on non-UTF8 paths
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run adb pull")?;

        if !output.status.success() {
            anyhow::bail!("adb pull failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        Ok(())
    }

    /// Best-effort backup of the partitions that carry cellular identity (IMEI,
    /// modem calibration). These can ONLY be read with root in adb mode — they
    /// are unreadable from fastboot — so on the typical unlocked-but-unrooted
    /// device this backs up nothing and returns an empty list. It never fails
    /// the install; it's a bonus for rooted users on top of the safety warning.
    /// Returns the names of partitions actually saved.
    pub async fn backup_critical_partitions(&self, serial: &str, dest_dir: &Path) -> Vec<String> {
        const PARTITIONS: &[&str] = &["efs", "modemst1", "modemst2", "persist", "fsg", "nvram"];

        // Root is required to read raw block devices.
        let has_root = self
            .shell(serial, "su -c id 2>/dev/null")
            .await
            .map(|o| o.contains("uid=0"))
            .unwrap_or(false);
        if !has_root {
            log::info!("No root on {serial}; skipping IMEI/EFS auto-backup (see safety warning)");
            return Vec::new();
        }

        if let Err(e) = tokio::fs::create_dir_all(dest_dir).await {
            log::warn!("Could not create backup dir {}: {e}", dest_dir.display());
            return Vec::new();
        }

        let mut saved = Vec::new();
        for p in PARTITIONS {
            let remote = format!("/data/local/tmp/sidestep-{p}.img");
            // by-name symlinks are the portable path across recent devices.
            let dd = format!(
                "su -c 'dd if=/dev/block/bootdevice/by-name/{p} of={remote} 2>/dev/null'"
            );
            if self.shell(serial, &dd).await.is_err() {
                continue;
            }
            let local = dest_dir.join(format!("{p}.img"));
            if self.pull(serial, &remote, &local).await.is_ok() {
                // A zero-byte pull means the partition didn't exist — drop it.
                if tokio::fs::metadata(&local).await.map(|m| m.len() > 0).unwrap_or(false) {
                    saved.push(p.to_string());
                } else {
                    let _ = tokio::fs::remove_file(&local).await;
                }
            }
            let _ = self.shell(serial, &format!("su -c 'rm -f {remote}'")).await;
        }

        if saved.is_empty() {
            log::info!("IMEI/EFS auto-backup found no readable partitions on {serial}");
        } else {
            log::info!("Backed up partitions {:?} to {}", saved, dest_dir.display());
        }
        saved
    }
}
