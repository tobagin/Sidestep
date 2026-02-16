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

    pub fn with_path(path: String) -> Self {
        Self { binary_path: path }
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

    /// Get device manufacturer
    pub async fn get_manufacturer(&self, serial: &str) -> Result<String> {
        self.getprop(serial, "ro.product.manufacturer").await
    }

    /// Reboot into bootloader mode
    pub async fn reboot_bootloader(&self, serial: &str) -> Result<()> {
        log::info!("Rebooting {} to bootloader", serial);
        
        Command::new(&self.binary_path)
            .args(["-s", serial, "reboot", "bootloader"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to reboot to bootloader")?;

        Ok(())
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

    /// Wait for device to be connected
    pub async fn wait_for_device(&self, serial: &str) -> Result<()> {
        Command::new(&self.binary_path)
            .args(["-s", serial, "wait-for-device"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to wait for device")?;

        Ok(())
    }

    /// Wait for device to enter recovery mode
    pub async fn wait_for_recovery(&self, serial: &str) -> Result<()> {
        log::info!("Waiting for {} to enter recovery mode", serial);

        Command::new(&self.binary_path)
            .args(["-s", serial, "wait-for-recovery"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to wait for recovery")?;

        Ok(())
    }

    pub async fn wait_for_sideload(&self, serial: &str) -> Result<()> {
        log::info!("Waiting for {} to enter sideload mode", serial);

        Command::new(&self.binary_path)
            .args(["-s", serial, "wait-for-sideload"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to wait for sideload")?;

        Ok(())
    }

    /// Reboot into recovery mode
    pub async fn reboot_recovery(&self, serial: &str) -> Result<()> {
        log::info!("Rebooting {} to recovery", serial);

        Command::new(&self.binary_path)
            .args(["-s", serial, "reboot", "recovery"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to reboot to recovery")?;

        Ok(())
    }

    /// Sideload a zip file via ADB sideload (used in recovery mode)
    pub async fn sideload(&self, serial: &str, zip_path: &Path) -> Result<()> {
        log::info!("Sideloading {} to {}", zip_path.display(), serial);

        let output = Command::new(&self.binary_path)
            .args([
                "-s", serial,
                "sideload",
                zip_path.to_str().unwrap(),
            ])
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
            .args([
                "-s", serial,
                "push",
                local.to_str().unwrap(),
                remote,
            ])
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
}
