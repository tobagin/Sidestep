// Wizard controller - state machine for installation wizard
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::flashing::{ChecksumVerifier, Decompressor, FlashExecutor, ImageDownloader};
use crate::hardware::{Adb, Fastboot};
use crate::models::{Device, DeviceDatabase, Distro, UnlockingStep};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Current state of the wizard
#[derive(Debug, Clone)]
pub enum WizardState {
    /// Displaying safety warnings
    SafetyWarnings,
    
    /// Performing unlocking steps
    Unlocking {
        current_step: usize,
        total_steps: usize,
    },
    
    /// Selecting distribution
    DistroSelection,
    
    /// Downloading images
    Downloading {
        file: String,
        progress: f64,
    },
    
    /// Decompressing images
    Decompressing {
        file: String,
        progress: f64,
    },
    
    /// Flashing partitions
    Flashing {
        partition: String,
        current: usize,
        total: usize,
    },
    
    /// Verifying checksums
    Verifying,
    
    /// Installation complete
    Success,
    
    /// Error occurred
    Error(String),
}

/// Progress update callback
pub type ProgressCallback = Arc<dyn Fn(WizardState) + Send + Sync>;

/// Controller for the installation wizard flow
pub struct WizardController {
    device: Device,
    database: DeviceDatabase,
    adb: Adb,
    fastboot: Fastboot,
    downloader: ImageDownloader,
    executor: FlashExecutor,
    
    state: Arc<Mutex<WizardState>>,
    unlocking_steps: Vec<UnlockingStep>,
    available_distros: Vec<Distro>,
    selected_distro: Option<Distro>,
    device_serial: String,
    download_dir: PathBuf,
}

impl WizardController {
    pub fn new(device: Device, serial: String) -> Self {
        let database = DeviceDatabase::new();
        let unlocking_steps = database.get_unlocking_steps(&device.codename);
        let available_distros = database.get_distros(&device.codename);

        let download_dir = dirs::download_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("sidestep")
            .join(&device.codename);

        Self {
            device,
            database,
            adb: Adb::new(),
            fastboot: Fastboot::new(),
            downloader: ImageDownloader::new(download_dir.clone()),
            executor: FlashExecutor::new(),
            state: Arc::new(Mutex::new(WizardState::SafetyWarnings)),
            unlocking_steps,
            available_distros,
            selected_distro: None,
            device_serial: serial,
            download_dir,
        }
    }

    /// Get current state
    pub async fn state(&self) -> WizardState {
        self.state.lock().await.clone()
    }

    /// Set state
    async fn set_state(&self, state: WizardState) {
        *self.state.lock().await = state;
    }

    /// Get device info
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Get unlocking steps
    pub fn unlocking_steps(&self) -> &[UnlockingStep] {
        &self.unlocking_steps
    }

    /// Get available distros
    pub fn available_distros(&self) -> &[Distro] {
        &self.available_distros
    }

    /// Move to unlocking phase
    pub async fn start_unlocking(&self) {
        self.set_state(WizardState::Unlocking {
            current_step: 1,
            total_steps: self.unlocking_steps.len(),
        }).await;
    }

    /// Execute an automated unlocking step
    pub async fn execute_step(&self, step_index: usize) -> Result<()> {
        let step = &self.unlocking_steps[step_index];

        if let Some(ref command) = step.command {
            log::info!("Executing: {}", command);

            // Parse and execute command
            if command.starts_with("adb ") {
                let args = command.strip_prefix("adb ").unwrap();
                if args == "reboot bootloader" {
                    self.adb.reboot_bootloader(&self.device_serial).await?;
                } else {
                    // Generic shell command
                    self.adb.shell(&self.device_serial, args).await?;
                }
            } else if command.starts_with("fastboot ") {
                let args = command.strip_prefix("fastboot ").unwrap();
                if args == "oem unlock" {
                    self.fastboot.oem_unlock(&self.device_serial).await?;
                } else if args == "flashing unlock" {
                    self.fastboot.oem_unlock(&self.device_serial).await?;
                }
            }
        }

        // Move to next step
        let next_step = step_index + 1;
        if next_step < self.unlocking_steps.len() {
            self.set_state(WizardState::Unlocking {
                current_step: next_step + 1,
                total_steps: self.unlocking_steps.len(),
            }).await;
        } else {
            self.set_state(WizardState::DistroSelection).await;
        }

        Ok(())
    }

    /// Select a distro
    pub fn select_distro(&mut self, distro: Distro) {
        self.selected_distro = Some(distro);
    }

    /// Start the installation process
    pub async fn start_installation(&self) -> Result<()> {
        let distro = self.selected_distro.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No distro selected"))?;

        // Download images
        for partition in &distro.partitions {
            let url = format!("{}{}", distro.download_base_url, partition.image);
            
            self.set_state(WizardState::Downloading {
                file: partition.image.clone(),
                progress: 0.0,
            }).await;

            self.downloader.download(&url, &partition.image, None).await?;
        }

        // Decompress images
        for partition in &distro.partitions {
            let image_path = self.download_dir.join(&partition.image);

            if partition.image.ends_with(".xz") || partition.image.ends_with(".gz") {
                self.set_state(WizardState::Decompressing {
                    file: partition.image.clone(),
                    progress: 0.0,
                }).await;

                Decompressor::decompress(&image_path, None, None)?;
            }
        }

        // Verify checksums if available
        if let Some(ref checksum_url) = distro.checksum_url {
            self.set_state(WizardState::Verifying).await;
            
            let checksums = self.downloader.download_checksums(checksum_url).await?;
            ChecksumVerifier::verify_all(&self.download_dir, &checksums)?;
        }

        // Flash images
        self.set_state(WizardState::Flashing {
            partition: "starting".to_string(),
            current: 0,
            total: distro.partitions.len(),
        }).await;

        self.executor.flash_distro(
            &self.device_serial,
            distro,
            &self.download_dir,
            None,
        ).await?;

        // Reboot
        self.executor.reboot(&self.device_serial).await?;

        self.set_state(WizardState::Success).await;

        Ok(())
    }
}
