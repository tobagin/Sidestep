// YAML Parser
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::models::distro_config::DeviceDistroConfig;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

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

    pub fn parse_device_config(&self, manufacturer: &str, codename: &str) -> Result<DeviceDistroConfig> {
        let path = self.devices_dir.join(manufacturer).join(codename).join("distros.yml");
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read distros.yml at {:?}", path))?;

        let config: DeviceDistroConfig = serde_yaml::from_str(&content)
            .with_context(|| "Failed to parse distros.yml")?;

        Ok(config)
    }
}
