// Flash executor
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::hardware::Fastboot;
use crate::models::{Distro, PartitionImage};
use anyhow::{Context, Result};
use std::path::Path;

/// Callback for flash progress
pub type FlashProgressCallback = Box<dyn Fn(&str, usize, usize) + Send + Sync>;

/// Executes the flashing sequence
pub struct FlashExecutor {
    fastboot: Fastboot,
}

impl FlashExecutor {
    pub fn new() -> Self {
        Self {
            fastboot: Fastboot::new(),
        }
    }

    /// Flash all partitions for a distro
    pub async fn flash_distro(
        &self,
        serial: &str,
        distro: &Distro,
        images_dir: &Path,
        on_progress: Option<FlashProgressCallback>,
    ) -> Result<()> {
        let total = distro.partitions.len();

        for (index, partition) in distro.partitions.iter().enumerate() {
            if let Some(ref callback) = on_progress {
                callback(&partition.partition, index + 1, total);
            }

            self.flash_partition(serial, partition, images_dir).await?;
        }

        Ok(())
    }

    /// Flash a single partition
    pub async fn flash_partition(
        &self,
        serial: &str,
        partition: &PartitionImage,
        images_dir: &Path,
    ) -> Result<()> {
        let image_path = images_dir.join(&partition.image);

        // Check if image exists (might need decompression first)
        let actual_path = if image_path.exists() {
            image_path
        } else {
            // Try without compression extension
            let mut no_ext = image_path.clone();
            if let Some(ext) = image_path.extension() {
                if ext == "xz" || ext == "gz" {
                    no_ext.set_extension("");
                    if no_ext.exists() {
                        no_ext
                    } else {
                        return Err(anyhow::anyhow!(
                            "Image not found: {}",
                            image_path.display()
                        ));
                    }
                } else {
                    return Err(anyhow::anyhow!(
                        "Image not found: {}",
                        image_path.display()
                    ));
                }
            } else {
                return Err(anyhow::anyhow!(
                    "Image not found: {}",
                    image_path.display()
                ));
            }
        };

        log::info!(
            "Flashing {} to partition {}",
            actual_path.display(),
            partition.partition
        );

        // Erase partition first if needed
        if partition.erase_first {
            self.fastboot
                .erase(serial, &partition.partition)
                .await
                .context("Failed to erase partition")?;
        }

        // Flash the image
        self.fastboot
            .flash(serial, &partition.partition, &actual_path)
            .await
            .context("Failed to flash partition")?;

        Ok(())
    }

    /// Reboot the device after flashing
    pub async fn reboot(&self, serial: &str) -> Result<()> {
        log::info!("Rebooting device");
        self.fastboot.reboot(serial).await
    }
}

impl Default for FlashExecutor {
    fn default() -> Self {
        Self::new()
    }
}
