// Distro model
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Deserialize;

/// A partition image to flash
#[derive(Debug, Clone, Deserialize)]
pub struct PartitionImage {
    /// Partition name (e.g., "boot", "system", "userdata")
    pub partition: String,
    
    /// Relative path or URL to the image file
    pub image: String,
    
    /// Whether this partition should be erased before flashing
    #[serde(default)]
    pub erase_first: bool,
}

/// A mobile Linux distribution available for a device
#[derive(Debug, Clone, Deserialize)]
pub struct Distro {
    /// Distribution name (e.g., "postmarketOS")
    pub name: String,
    
    /// Distribution version
    pub version: String,
    
    /// Description of this distribution
    #[serde(default)]
    pub description: String,
    
    /// Base URL for downloading images
    pub download_base_url: String,
    
    /// URL to the checksum file (SHA256SUMS)
    #[serde(default)]
    pub checksum_url: Option<String>,
    
    /// List of partition images to flash
    pub partitions: Vec<PartitionImage>,
    
    /// Homepage URL
    #[serde(default)]
    pub homepage: Option<String>,
    
    /// Estimated download size in bytes
    #[serde(default)]
    pub download_size_bytes: Option<u64>,
    
    /// Whether this requires an unlocked bootloader
    #[serde(default = "default_requires_unlock")]
    pub requires_unlock: bool,
    
    /// Post-installation instructions
    #[serde(default)]
    pub post_install_notes: Option<String>,
}

fn default_requires_unlock() -> bool {
    true
}

impl Distro {
    /// Get formatted download size string
    pub fn download_size_string(&self) -> String {
        match self.download_size_bytes {
            Some(bytes) if bytes >= 1_000_000_000 => {
                format!("{:.1} GB", bytes as f64 / 1_000_000_000.0)
            }
            Some(bytes) if bytes >= 1_000_000 => {
                format!("{:.0} MB", bytes as f64 / 1_000_000.0)
            }
            Some(bytes) => format!("{} KB", bytes / 1000),
            None => "Unknown".to_string(),
        }
    }
}

/// A node in the distribution selection tree
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum DistroTreeNode {
    Group {
        name: String,
        description: String,
        children: Vec<DistroTreeNode>,
    },
    Item(Distro),
}

impl DistroTreeNode {
    pub fn name(&self) -> &str {
        match self {
            DistroTreeNode::Group { name, .. } => name,
            DistroTreeNode::Item(d) => &d.name,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            DistroTreeNode::Group { description, .. } => description,
            DistroTreeNode::Item(d) => &d.description,
        }
    }
}
