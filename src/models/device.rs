// Device model
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Deserialize;

/// Represents an Android device with its metadata
#[derive(Debug, Clone, Deserialize)]
pub struct Device {
    /// Device codename (e.g., "sargo" for Pixel 3a)
    pub codename: String,
    
    /// Human-readable device name
    pub name: String,
    
    /// Manufacturer name
    pub maker: String,
    
    /// Whether this device has experimental support
    #[serde(default)]
    pub experimental: bool,
    
    /// Minimum battery level required before installation
    #[serde(default = "default_battery_min")]
    pub battery_min: u8,
    
    /// Device-specific warnings to display
    #[serde(default)]
    pub warnings: Vec<String>,
    
    /// Alternative codenames for this device
    #[serde(default)]
    pub aliases: Vec<String>,

    /// User-facing variant names (e.g. miatoll covers POCO M2 Pro, Redmi Note 9S, etc.)
    #[serde(default)]
    pub variants: Vec<String>,

    /// Whether the bootloader is known to be locked
    #[serde(skip)]
    pub is_locked: Option<bool>,

    /// USB serial number (set at runtime by device detection, not from config)
    #[serde(skip)]
    pub serial: Option<String>,

    /// Android version (e.g. "12") — set at runtime via ADB
    #[serde(skip)]
    pub android_version: Option<String>,

    /// Build ID (e.g. "SP2A.220505.008") — set at runtime via ADB
    #[serde(skip)]
    pub build_id: Option<String>,

    /// Battery level (0–100) — set at runtime via ADB
    #[serde(skip)]
    pub battery_level: Option<u8>,
}

fn default_battery_min() -> u8 {
    50
}

impl Device {
    /// Check if a codename matches this device (including aliases)
    pub fn matches_codename(&self, codename: &str) -> bool {
        if self.codename == codename {
            return true;
        }
        self.aliases.iter().any(|alias| alias == codename)
    }
}
