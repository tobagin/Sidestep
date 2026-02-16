// Fastboot wrapper
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

/// Represents a device in fastboot mode
#[derive(Debug, Clone)]
pub struct FastbootDevice {
    pub serial: String,
    pub product: Option<String>,
}

/// Fastboot command wrapper
#[derive(Debug, Clone)]
pub struct Fastboot {
    binary_path: String,
}

impl Default for Fastboot {
    fn default() -> Self {
        Self::new()
    }
}

impl Fastboot {
    pub fn new() -> Self {
        // Try to find fastboot in common locations
        let binary_path = std::env::var("FASTBOOT_PATH")
            .unwrap_or_else(|_| "fastboot".to_string());
        
        Self { binary_path }
    }

    pub fn with_path(path: String) -> Self {
        Self { binary_path: path }
    }

    /// List connected fastboot devices
    pub async fn devices(&self) -> Result<Vec<FastbootDevice>> {
        let output = Command::new(&self.binary_path)
            .arg("devices")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run fastboot devices")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut devices = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if !parts.is_empty() && parts.last() == Some(&"fastboot") {
                devices.push(FastbootDevice {
                    serial: parts[0].to_string(),
                    product: None,
                });
            }
        }

        log::debug!("Fastboot devices: {:?}", devices);
        Ok(devices)
    }

    /// Get a variable from the device
    pub async fn getvar(&self, serial: &str, var: &str) -> Result<String> {
        let output = Command::new(&self.binary_path)
            .args(["-s", serial, "getvar", var])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run fastboot getvar")?;

        // Fastboot outputs to stderr
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Parse "var: value" format
        for line in stderr.lines() {
            if let Some(value) = line.strip_prefix(&format!("{}: ", var)) {
                return Ok(value.to_string());
            }
        }

        Ok(String::new())
    }

    /// Get unlock status
    pub async fn is_unlocked(&self, serial: &str) -> Result<bool> {
        let value = self.getvar(serial, "unlocked").await?;
        Ok(value == "yes")
    }

    /// Unlock the bootloader (OEM unlock)
    pub async fn oem_unlock(&self, serial: &str) -> Result<()> {
        log::info!("Attempting OEM unlock on {}", serial);
        
        let output = Command::new(&self.binary_path)
            .args(["-s", serial, "oem", "unlock"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run fastboot oem unlock")?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        log::debug!("OEM unlock output: {}", stderr);

        if !output.status.success() {
            anyhow::bail!("OEM unlock failed: {}", stderr);
        }

        Ok(())
    }

    /// Flash an image to a partition
    pub async fn flash(&self, serial: &str, partition: &str, image: &Path) -> Result<()> {
        log::info!("Flashing {} to partition {}", image.display(), partition);

        let output = Command::new(&self.binary_path)
            .args([
                "-s", serial,
                "flash", partition,
                image.to_str().unwrap()
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run fastboot flash")?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        log::debug!("Flash output: {}", stderr);

        if !output.status.success() {
            anyhow::bail!("Flash failed: {}", stderr);
        }

        Ok(())
    }

    /// Reboot the device
    pub async fn reboot(&self, serial: &str) -> Result<()> {
        log::info!("Rebooting device {}", serial);
        
        Command::new(&self.binary_path)
            .args(["-s", serial, "reboot"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to reboot device")?;

        Ok(())
    }

    /// Reboot the device directly into recovery mode
    pub async fn reboot_recovery(&self, serial: &str) -> Result<()> {
        log::info!("Rebooting device {} into recovery", serial);

        let output = Command::new(&self.binary_path)
            .args(["-s", serial, "reboot", "recovery"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to reboot into recovery")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Reboot to recovery failed: {}", stderr);
        }

        Ok(())
    }

    /// Erase a partition
    pub async fn erase(&self, serial: &str, partition: &str) -> Result<()> {
        log::info!("Erasing partition {} on {}", partition, serial);
        
        let output = Command::new(&self.binary_path)
            .args(["-s", serial, "erase", partition])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to erase partition")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Erase failed: {}", stderr);
        }

        Ok(())
    }

    /// Set active slot (for A/B devices)
    pub async fn set_active(&self, serial: &str, slot: &str) -> Result<()> {
        log::info!("Setting active slot to {} on {}", slot, serial);

        Command::new(&self.binary_path)
            .args(["-s", serial, "set_active", slot])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to set active slot")?;

        Ok(())
    }

    /// Flash an image with additional flags (e.g., --disable-verity --disable-verification for vbmeta)
    ///
    /// Matches UBports promise-android-tools flag placement:
    ///   fastboot -s SERIAL flash PARTITION [FLAGS...] FILE
    pub async fn flash_with_flags(
        &self,
        serial: &str,
        partition: &str,
        image: &Path,
        flags: &[&str],
    ) -> Result<()> {
        log::info!(
            "Flashing {} to partition {} with flags {:?}",
            image.display(),
            partition,
            flags
        );

        let mut args = vec!["-s", serial, "flash", partition];
        args.extend_from_slice(flags);
        args.push(image.to_str().unwrap());

        let output = Command::new(&self.binary_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run fastboot flash")?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        log::debug!("Flash output: {}", stderr);

        if !output.status.success() {
            anyhow::bail!("Flash with flags failed: {}", stderr);
        }

        Ok(())
    }

    /// Format a partition with a given filesystem type
    pub async fn format(&self, serial: &str, partition: &str, fs_type: &str) -> Result<()> {
        log::info!("Formatting partition {} as {} on {}", partition, fs_type, serial);

        let format_arg = format!("format:{}", fs_type);
        let output = Command::new(&self.binary_path)
            .args(["-s", serial, &format_arg, partition])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to format partition")?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        log::debug!("Format output: {}", stderr);

        if !output.status.success() {
            anyhow::bail!("Format failed: {}", stderr);
        }

        Ok(())
    }
}
