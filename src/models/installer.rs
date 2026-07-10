use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallerConfig {
    pub name: String,
    pub prerequisites: Vec<Prerequisite>,
    pub steps: HashMap<String, Step>,
    #[serde(default)]
    pub firmware: Vec<FirmwareImage>,
    #[serde(default)]
    pub flash_partitions: Vec<FlashPartition>,
    #[serde(default)]
    pub flash_operations: Vec<FlashOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prerequisite {
    pub title: String,
    pub check: String,
    pub message: String,
    pub on_success: String,
    pub on_failure: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Step {
    Instruction {
        message: String,
        link: Option<String>,
        action_label: Option<String>,
    },
    Flash {
        url: String,
    },
    FlashAndroid {
        android_version: String,
        url: String,
        sha256: String,
        message: String,
    },
}

/// Firmware image to download and flash before the main install (UBports).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareImage {
    pub url: String,
    pub filename: String,
    pub partition: String,
    pub sha256: String,
    #[serde(default)]
    pub flags: Vec<String>,
}

/// Image within a ZIP archive to flash to a partition (Droidian).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashPartition {
    pub image_path: String,
    pub partition: String,
    #[serde(default)]
    pub flags: Vec<String>,
}

/// A single fastboot operation in a flash sequence (Mobian, postmarketOS).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FlashOperation {
    Flash {
        partition: String,
        source: String,
        #[serde(default)]
        flags: Vec<String>,
    },
    FlashSparse {
        partition: String,
        source: String,
        chunk_size: String,
    },
    Format {
        partition: String,
        fs_type: String,
    },
    Erase {
        partition: String,
    },
    Oem {
        args: Vec<String>,
    },
}
