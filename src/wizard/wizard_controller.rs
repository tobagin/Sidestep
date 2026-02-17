// Wizard controller - state machine for installation wizard
// SPDX-License-Identifier: GPL-3.0-or-later
#![allow(dead_code)]

use crate::hardware::{Adb, Fastboot};
use crate::models::{Device, DeviceDatabase, UnlockingStep};
use crate::models::distro_config::DistroConfig;
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

    state: Arc<Mutex<WizardState>>,
    unlocking_steps: Vec<UnlockingStep>,
    available_distros: Vec<DistroConfig>,
    selected_distro: Option<DistroConfig>,
    device_serial: String,
    download_dir: PathBuf,
}

impl WizardController {
    pub fn new(device: Device, serial: String) -> Self {
        let database = DeviceDatabase::new();
        let unlocking_steps = database.get_unlocking_steps(&device.codename);
        let available_distros = database.get_distro_configs(&device);

        let download_dir = dirs::download_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("sidestep")
            .join(&device.codename);

        Self {
            device,
            database,
            adb: Adb::new(),
            fastboot: Fastboot::new(),
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
    pub fn available_distros(&self) -> &[DistroConfig] {
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
    pub fn select_distro(&mut self, distro: DistroConfig) {
        self.selected_distro = Some(distro);
    }

    /// Start the installation process
    ///
    /// NOTE: This method is not currently used — the active install flow goes
    /// through `DeviceDetailsPage` → `FlashingPage` directly. It will need to
    /// be rewritten to work with `DistroConfig` + installer YAML when enabled.
    pub async fn start_installation(&self) -> Result<()> {
        let _distro = self.selected_distro.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No distro selected"))?;

        todo!("Rewrite to use DistroConfig + installer YAML pipeline")
    }
}
