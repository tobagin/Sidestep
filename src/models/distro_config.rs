// Distro Config Models
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceDistroConfig {
    pub device_codename: String,
    pub last_updated: String,
    pub available_distros: Vec<DistroConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DistroConfig {
    pub id: String,
    pub name: String,
    pub developer: String,
    pub discovery_strategy: String,
    pub base_url: Option<String>,
    pub api_root: Option<String>,
    #[serde(default)]
    pub channels: Vec<ChannelConfig>,
    pub interfaces: Option<Vec<InterfaceConfig>>,
    pub image_pattern: Option<String>,
    pub flash_method: String,
    pub firmware_requirement: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChannelConfig {
    pub id: String,
    pub label: String,
    pub path: Option<String>,
    pub artifact_match: Option<String>,
    pub release_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InterfaceConfig {
    pub id: String,
    pub label: String,
}
