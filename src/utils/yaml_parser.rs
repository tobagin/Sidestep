// YAML Parser
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::models::Device;
use crate::models::distro_config::DeviceDistroConfig;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Wrapper for deserializing the `device:` key from info.yml into a Device.
#[derive(Deserialize)]
struct DeviceYaml {
    device: Device,
}

pub struct YamlParser {
    devices_dir: PathBuf,
}

impl YamlParser {
    pub fn new<P: AsRef<Path>>(devices_dir: P) -> Self {
        Self {
            devices_dir: devices_dir.as_ref().to_path_buf(),
        }
    }

    pub fn parse_device_info(&self, manufacturer: &str, codename: &str) -> Result<crate::models::device_info::DeviceInfo> {
        let path = self.devices_dir.join(manufacturer).join(codename).join("info.yml");
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read info.yml at {:?}", path))?;

        let info: crate::models::device_info::DeviceInfo = serde_yaml::from_str(&content)
            .with_context(|| "Failed to parse info.yml")?;

        Ok(info)
    }

    /// Parse a Device from info.yml.
    pub fn parse_device(&self, manufacturer: &str, codename: &str) -> Result<Device> {
        let path = self.devices_dir.join(manufacturer).join(codename).join("info.yml");
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read info.yml at {:?}", path))?;

        let wrapper: DeviceYaml = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse info.yml at {:?}", path))?;

        Ok(wrapper.device)
    }

    pub fn parse_device_config(&self, manufacturer: &str, codename: &str) -> Result<DeviceDistroConfig> {
        let path = self.devices_dir.join(manufacturer).join(codename).join("distros.yml");
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read distros.yml at {:?}", path))?;

        let config: DeviceDistroConfig = serde_yaml::from_str(&content)
            .with_context(|| "Failed to parse distros.yml")?;

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::YamlParser;

    #[test]
    fn parses_generated_pixel_devices() {
        let p = YamlParser::new("data/devices");
        // A representative new Pixel: info, and distros with grapheneos + lineageos.
        let dev = p.parse_device("google", "shiba").expect("parse shiba");
        assert_eq!(dev.name, "Pixel 8");
        let cfg = p.parse_device_config("google", "shiba").expect("parse shiba distros");
        let ids: Vec<_> = cfg.available_distros.iter().map(|d| d.id.as_str()).collect();
        assert!(ids.contains(&"grapheneos"), "grapheneos present: {ids:?}");
    }

    #[test]
    fn parses_generated_community_devices() {
        let p = YamlParser::new("data/devices");
        // A newly-added community device (LG V20) with rich specs from the wiki.
        let dev = p.parse_device("lg", "us996d").expect("parse us996d");
        assert!(!dev.name.is_empty());
        let cfg = p.parse_device_config("lg", "us996d").expect("parse us996d distros");
        let ids: Vec<_> = cfg.available_distros.iter().map(|d| d.id.as_str()).collect();
        assert!(ids.contains(&"lineageos"), "lineageos present: {ids:?}");
        // Motorola with per-vendor unlock steps.
        let m = p.parse_device("motorola", "addison").expect("parse addison");
        assert_eq!(m.name, "moto z play");
    }
}
