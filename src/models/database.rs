// Device database - loads device data from YAML files
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::config;
use crate::models::Device;
use crate::models::distro_config::DistroConfig;
use crate::models::installer::InstallerConfig;
use crate::models::unlocking_step::UnlockingStep;
use crate::utils::yaml_parser::YamlParser;
use std::collections::HashMap;
use std::path::PathBuf;

/// In-memory database of supported devices
pub struct DeviceDatabase {
    devices: HashMap<String, Device>,
}

impl DeviceDatabase {
    pub fn new() -> Self {
        let mut db = Self {
            devices: HashMap::new(),
        };

        db.load_from_data_dir();

        db
    }

    /// Load devices, bundled data first then synced updates overlaid on top.
    /// The bundled copy is authoritative: a synced device only overrides a
    /// bundled one when it parses, and synced-only devices are added — so an
    /// incompatible or partial sync can never drop below the bundled set.
    fn load_from_data_dir(&mut self) {
        self.load_dir(&Self::synced_devices_dir());
        self.load_dir(&Self::bundled_devices_dir());
        log::info!("Loaded {} devices", self.devices.len());
    }

    /// Scan {dir}/{manufacturer}/{codename}/info.yml and insert each device.
    /// Later calls override earlier ones by codename.
    fn load_dir(&mut self, devices_dir: &std::path::Path) {
        let parser = YamlParser::new(devices_dir);

        let manufacturers = match std::fs::read_dir(devices_dir) {
            Ok(entries) => entries,
            Err(_) => return, // dir may not exist (e.g. never synced) — fine
        };

        for mfr_entry in manufacturers.flatten() {
            if !mfr_entry.file_type().is_ok_and(|ft| ft.is_dir()) {
                continue;
            }
            let manufacturer = mfr_entry.file_name().to_string_lossy().to_string();

            let codenames = match std::fs::read_dir(mfr_entry.path()) {
                Ok(entries) => entries,
                Err(_) => continue,
            };

            for dev_entry in codenames.flatten() {
                if !dev_entry.file_type().is_ok_and(|ft| ft.is_dir()) {
                    continue;
                }
                let codename = dev_entry.file_name().to_string_lossy().to_string();

                match parser.parse_device(&manufacturer, &codename) {
                    Ok(device) => {
                        log::debug!("Loaded device: {} ({})", device.name, device.codename);
                        self.devices.insert(device.codename.clone(), device);
                    }
                    Err(e) => {
                        // Non-fatal: a bundled device stays if its synced
                        // overlay fails to parse (e.g. schema drift).
                        log::debug!(
                            "Skipped device {}/{}: {:#}",
                            manufacturer,
                            codename,
                            e
                        );
                    }
                }
            }
        }
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

    /// The bundled (authoritative) devices directory, checking install paths.
    pub fn bundled_devices_dir() -> PathBuf {
        let candidates = vec![
            PathBuf::from(config::PKGDATADIR).join("devices"),
            PathBuf::from("/app/share/sidestep/devices"),
            PathBuf::from("data/devices"),
            PathBuf::from("devices"),
        ];
        candidates
            .into_iter()
            .find(|p| p.exists())
            .unwrap_or_else(|| PathBuf::from("devices"))
    }

    /// The user-synced devices directory (may not exist yet).
    pub fn synced_devices_dir() -> PathBuf {
        crate::models::sync::user_data_dir().join("devices")
    }

    /// Sanitize a manufacturer name for use as a filesystem directory.
    /// Strips characters that aren't alphanumeric, hyphen, or underscore,
    /// then lowercases. e.g. "F(x)tec" → "fxtec".
    pub fn maker_to_dir(maker: &str) -> String {
        maker
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect::<String>()
            .to_lowercase()
    }

    /// Load all distro configs for a device from distros.yml. Bundled data is
    /// tried first; the synced dir is a fallback for synced-only devices.
    pub fn get_distro_configs(&self, device: &Device) -> Vec<DistroConfig> {
        let manufacturer = Self::maker_to_dir(&device.maker);
        for dir in [Self::bundled_devices_dir(), Self::synced_devices_dir()] {
            let parser = YamlParser::new(dir);
            if let Ok(config) = parser.parse_device_config(&manufacturer, &device.codename) {
                if !config.available_distros.is_empty() {
                    return config.available_distros;
                }
            }
        }
        log::debug!("No distros.yml for {}", device.codename);
        Vec::new()
    }

    /// Load the curated bootloader-unlocking steps for a device from
    /// `unlocking.yml`. Returns empty when the device has no curated steps —
    /// callers fall back to the manufacturer-aware guide in `unlock_guide`.
    pub fn get_unlocking_steps(&self, codename: &str) -> Vec<UnlockingStep> {
        #[derive(serde::Deserialize)]
        struct UnlockingFile {
            unlocking_steps: Vec<UnlockingStep>,
        }

        let Some(device) = self.find_by_codename(codename) else {
            return Vec::new();
        };
        let maker = Self::maker_to_dir(&device.maker);

        for base in [Self::bundled_devices_dir(), Self::synced_devices_dir()] {
            // Device dirs are named for the codename; try as-is then lowercased.
            for name in [device.codename.clone(), device.codename.to_lowercase()] {
                let path = base.join(&maker).join(&name).join("unlocking.yml");
                let Ok(content) = std::fs::read_to_string(&path) else {
                    continue;
                };
                match serde_yaml::from_str::<UnlockingFile>(&content) {
                    Ok(f) => return f.unlocking_steps,
                    Err(e) => log::warn!("Failed to parse {}: {}", path.display(), e),
                }
            }
        }

        log::debug!("No curated unlocking steps for {}", codename);
        Vec::new()
    }

    /// Load a single distro config by ID from distros.yml.
    pub fn get_distro_config(&self, device: &Device, distro_id: &str) -> Option<DistroConfig> {
        self.get_distro_configs(device)
            .into_iter()
            .find(|d| d.id == distro_id)
    }

    /// Load an installer config YAML for a specific device + distro.
    pub fn load_installer_config(&self, device: &Device, distro_id: &str) -> Option<InstallerConfig> {
        let possible_dirs = vec![
            PathBuf::from(config::SIDESTEP_DATA_DIR),
            PathBuf::from(config::PKGDATADIR),
            PathBuf::from("/app/share/sidestep"),
            PathBuf::from("data"),
            // Synced last: fallback for synced-only devices bundled lacks.
            crate::models::sync::user_data_dir(),
        ];

        let maker = Self::maker_to_dir(&device.maker);
        for dir in possible_dirs {
            // Device dirs are named for the codename; try as-is then lowercased.
            // Some (e.g. fairphone/FP4, FP5) keep upstream capitalisation, so a
            // blind to_lowercase() would never find them.
            for name in [device.codename.clone(), device.codename.to_lowercase()] {
                let config_path = dir
                    .join("devices")
                    .join(&maker)
                    .join(&name)
                    .join("installers")
                    .join(format!("{}.yml", distro_id));

                if let Ok(content) = std::fs::read_to_string(&config_path) {
                    match serde_yaml::from_str::<InstallerConfig>(&content) {
                        Ok(config) => return Some(config),
                        Err(e) => {
                            log::error!("Failed to parse {}: {}", config_path.display(), e);
                            return None;
                        }
                    }
                }
            }
        }

        log::debug!("No installer config found for {}/{}", device.codename, distro_id);
        None
    }

    /// Get all supported devices
    pub fn all_devices(&self) -> Vec<Device> {
        self.devices.values().cloned().collect()
    }
}

impl Default for DeviceDatabase {
    fn default() -> Self {
        Self::new()
    }
}
