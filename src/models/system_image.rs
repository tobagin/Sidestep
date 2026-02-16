// UBports system-image index model
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Deserialize;

/// Top-level index.json structure from the UBports system-image server
#[derive(Debug, Clone, Deserialize)]
pub struct SystemImageIndex {
    pub global: Option<serde_json::Value>,
    pub images: Vec<SystemImageEntry>,
}

/// A single image entry (full or delta update)
#[derive(Debug, Clone, Deserialize)]
pub struct SystemImageEntry {
    #[serde(rename = "type")]
    pub image_type: String,
    pub version: u32,
    pub files: Vec<SystemImageFile>,
}

/// A file within a system-image entry
#[derive(Debug, Clone, Deserialize)]
pub struct SystemImageFile {
    /// Relative path on the server (e.g., "/pool/ubports-foo.tar.xz")
    pub path: String,
    /// SHA256 checksum
    pub checksum: String,
    /// Installation order
    pub order: u32,
    /// File size in bytes
    pub size: u64,
    /// Relative path to the .asc signature file
    pub signature: String,
}

impl SystemImageIndex {
    /// Find the latest "full" image entry (highest version number)
    pub fn latest_full(&self) -> Option<&SystemImageEntry> {
        self.images
            .iter()
            .filter(|e| e.image_type == "full")
            .max_by_key(|e| e.version)
    }
}
