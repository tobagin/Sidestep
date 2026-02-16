// Device database - loads device data from YAML files
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::models::{Device, Distro, UnlockingStep};
use std::collections::HashMap;
use std::path::PathBuf;
use crate::models::PartitionImage;

/// In-memory database of supported devices
pub struct DeviceDatabase {
    devices: HashMap<String, Device>,
    unlocking_steps: HashMap<String, Vec<UnlockingStep>>,
    distros: HashMap<String, Vec<Distro>>,
    data_dir: PathBuf,
}

impl DeviceDatabase {
    pub fn new() -> Self {
        // Default data directory (bundled with app)
        let data_dir = std::env::var("SIDESTEP_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("devices"));

        let mut db = Self {
            devices: HashMap::new(),
            unlocking_steps: HashMap::new(),
            distros: HashMap::new(),
            data_dir,
        };

        // Load hardcoded devices for MVP
        db.load_hardcoded_devices();

        db
    }

    /// Load hardcoded device data (MVP approach)
    fn load_hardcoded_devices(&mut self) {
        // Pixel 3a (sargo)
        self.devices.insert("sargo".to_string(), Device {
            codename: "sargo".to_string(),
            name: "Google Pixel 3a".to_string(),
            maker: "Google".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec!["Pixel 3a".to_string()],
            is_locked: None,
            serial: None,
        });

        // OnePlus 6 (enchilada)
        self.devices.insert("enchilada".to_string(), Device {
            codename: "enchilada".to_string(),
            name: "OnePlus 6".to_string(),
            maker: "OnePlus".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
                "OxygenOS backup recommended before proceeding".to_string(),
            ],
            aliases: vec!["OnePlus6".to_string()],
            is_locked: None,
            serial: None,
        });

        // Surface Duo (zeta)
        self.devices.insert("zeta".to_string(), Device {
            codename: "zeta".to_string(),
            name: "Microsoft Surface Duo".to_string(),
            maker: "Microsoft".to_string(),
            experimental: true,
            battery_min: 60,
            warnings: vec![
                "Experimental device - proceed with caution".to_string(),
                "Dual screen functionality may be limited".to_string(),
            ],
            aliases: vec!["surfaceduo".to_string()],
            is_locked: None,
            serial: None,
        });

        // Load unlocking steps for each device
        self.load_unlocking_steps();
        
        // Load distros for each device
        self.load_distros();
    }

    fn load_unlocking_steps(&mut self) {
        // Common unlocking steps for Pixel 3a
        self.unlocking_steps.insert("sargo".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run OEM unlock command. Confirm on device screen.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot oem unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // OnePlus 6 steps (similar but with some differences)
        self.unlocking_steps.insert("enchilada".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command. Use volume keys to confirm on device.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot oem unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Surface Duo steps
        self.unlocking_steps.insert("zeta".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: Some("Microsoft may detect this and void warranty".to_string()),
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Hold Power + Volume Down when the device is off".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command via fastboot".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);
    }

    fn load_distros(&mut self) {
        // postmarketOS for Pixel 3a
        self.distros.insert("sargo".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "24.06".to_string(),
                description: "Alpine-based mobile Linux distribution".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/v24.06/google-sargo/".to_string(),
                checksum_url: Some("https://images.postmarketos.org/bpo/v24.06/google-sargo/SHA256SUMS".to_string()),
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                    PartitionImage {
                        partition: "system".to_string(),
                        image: "rootfs-google-sargo.img.xz".to_string(),
                        erase_first: true,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(500_000_000),
                requires_unlock: true,
                post_install_notes: Some("First boot may take several minutes".to_string()),
            },
        ]);

        // postmarketOS for OnePlus 6
        self.distros.insert("enchilada".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "24.06".to_string(),
                description: "Alpine-based mobile Linux distribution".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/v24.06/oneplus-enchilada/".to_string(),
                checksum_url: Some("https://images.postmarketos.org/bpo/v24.06/oneplus-enchilada/SHA256SUMS".to_string()),
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                    PartitionImage {
                        partition: "system".to_string(),
                        image: "rootfs-oneplus-enchilada.img.xz".to_string(),
                        erase_first: true,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(550_000_000),
                requires_unlock: true,
                post_install_notes: Some("First boot may take several minutes".to_string()),
            },
        ]);

        // postmarketOS for Surface Duo
        self.distros.insert("zeta".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "edge".to_string(),
                description: "Alpine-based mobile Linux (experimental)".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/edge/microsoft-zeta/".to_string(),
                checksum_url: None,
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(400_000_000),
                requires_unlock: true,
                post_install_notes: Some("Dual screen support is experimental".to_string()),
            },
        ]);
    }

    /// Find a device by its codename
    pub fn find_by_codename(&self, codename: &str) -> Option<Device> {
        // Direct lookup
        if let Some(device) = self.devices.get(codename) {
            return Some(device.clone());
        }

        // Check aliases
        for device in self.devices.values() {
            if device.matches_codename(codename) {
                return Some(device.clone());
            }
        }

        None
    }

    /// Get unlocking steps for a device
    pub fn get_unlocking_steps(&self, codename: &str) -> Vec<UnlockingStep> {
        self.unlocking_steps
            .get(codename)
            .cloned()
            .unwrap_or_default()
    }

    /// Get available distros for a device
    pub fn get_distros(&self, codename: &str) -> Vec<Distro> {
        self.distros.get(codename).cloned().unwrap_or_default()
    }

    /// Get all supported device codenames
    pub fn all_codenames(&self) -> Vec<String> {
        self.devices.keys().cloned().collect()
    }
}

impl Default for DeviceDatabase {
    fn default() -> Self {
        Self::new()
    }
}
