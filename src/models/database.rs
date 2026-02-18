// Device database - loads device data from YAML files
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::config;
use crate::models::Device;
use crate::models::distro_config::DistroConfig;
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

    /// Scan data/devices/{manufacturer}/{codename}/info.yml and load all devices.
    fn load_from_data_dir(&mut self) {
        let devices_dir = Self::devices_data_dir();
        let parser = YamlParser::new(&devices_dir);

        let manufacturers = match std::fs::read_dir(&devices_dir) {
            Ok(entries) => entries,
            Err(e) => {
                log::error!("Cannot read devices directory {:?}: {}", devices_dir, e);
                return;
            }
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
                        log::warn!(
                            "Failed to load device {}/{}: {:#}",
                            manufacturer,
                            codename,
                            e
                        );
                    }
                }
            }
        }

        log::info!("Loaded {} devices from {:?}", self.devices.len(), devices_dir);
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

    /// Resolve the devices data directory, checking common install paths.
    pub fn devices_data_dir() -> PathBuf {
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

    /// Load all distro configs for a device from distros.yml.
    pub fn get_distro_configs(&self, device: &Device) -> Vec<DistroConfig> {
        let devices_dir = Self::devices_data_dir();
        let manufacturer = Self::maker_to_dir(&device.maker);
        let parser = YamlParser::new(devices_dir);
        match parser.parse_device_config(&manufacturer, &device.codename) {
            Ok(config) => config.available_distros,
            Err(e) => {
                log::debug!("No distros.yml for {}: {:#}", device.codename, e);
                Vec::new()
            }
        }
    }

    /// Load a single distro config by ID from distros.yml.
    pub fn get_distro_config(&self, device: &Device, distro_id: &str) -> Option<DistroConfig> {
        self.get_distro_configs(device)
            .into_iter()
            .find(|d| d.id == distro_id)
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
