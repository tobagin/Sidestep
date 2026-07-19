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
    /// UBports: files to download (and optionally unpack) before the bootstrap.
    #[serde(default)]
    pub ubports_downloads: Vec<UbportsDownload>,
    /// UBports: ordered fastboot bootstrap sequence run before the system-image
    /// is pushed via recovery. Translated from ubports/installer-configs.
    #[serde(default)]
    pub bootstrap: Vec<BootstrapStep>,
    /// Flashing transport. "fastboot" (default) uses the fastboot backends;
    /// "heimdall" (a.k.a. Odin/download mode) routes Samsung Exynos devices to
    /// the Heimdall backend, flashing `firmware` images by partition name.
    #[serde(default = "default_flash_method")]
    pub flash_method: String,
}

/// A file the UBports bootstrap downloads (and optionally unpacks). Checksums
/// are optional — some devices publish none.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UbportsDownload {
    pub url: String,
    pub filename: String,
    #[serde(default)]
    pub sha256: Option<String>,
    /// When true, the downloaded archive is unpacked into `unpacked/` and later
    /// bootstrap `flash` steps reference files as `unpacked/<path>`.
    #[serde(default)]
    pub unpack: bool,
}

/// One ordered step of a UBports device bootstrap (the fastboot phase before the
/// system image is pushed via recovery). Faithful to ubports/installer-configs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum BootstrapStep {
    /// Flash a downloaded file (by name, or `unpacked/<path>`) to a partition.
    Flash {
        partition: String,
        file: String,
        #[serde(default)]
        flags: Vec<String>,
    },
    Format {
        partition: String,
        #[serde(default = "default_fs")]
        fs: String,
    },
    Erase {
        partition: String,
    },
    SetActive {
        slot: String,
    },
    DeleteLogicalPartition {
        partition: String,
    },
    ResizeLogicalPartition {
        partition: String,
        size: u64,
    },
    WipeSuper,
    RebootBootloader,
    RebootFastboot,
    RebootRecovery,
    Reboot,
    /// Wait for the device to reappear in fastboot.
    Wait,
    /// Assert a fastboot variable equals a value (e.g. unlocked=yes) before
    /// continuing — refuses to flash a locked device.
    AssertVar {
        variable: String,
        value: String,
    },
    /// Prompt the user to perform a manual action (hold buttons, etc.).
    UserAction {
        message: String,
    },
    /// Samsung download-mode flash of a partition via Heimdall.
    HeimdallFlash {
        partition: String,
        file: String,
    },
}

fn default_fs() -> String {
    "ext4".to_string()
}

fn default_flash_method() -> String {
    "fastboot".to_string()
}

impl InstallerConfig {
    /// True when this device flashes over the Odin protocol (Samsung Exynos)
    /// rather than fastboot.
    pub fn uses_heimdall(&self) -> bool {
        matches!(
            self.flash_method.as_str(),
            "heimdall" | "odin" | "download_mode"
        )
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn load(path: &str) -> InstallerConfig {
        let s = std::fs::read_to_string(path).unwrap_or_else(|_| panic!("read {path}"));
        serde_yaml::from_str(&s).unwrap_or_else(|e| panic!("parse {path}: {e}"))
    }

    #[test]
    fn generated_ubports_configs_deserialize() {
        // Simple (format+flash), complex (logical partitions), unpack, heimdall.
        let en = load("data/devices/oneplus/enchilada/installers/ubuntutouch.yml");
        assert_eq!(en.ubports_downloads.len(), 2);
        assert!(matches!(en.bootstrap.last(), Some(BootstrapStep::Flash { partition, .. }) if partition == "vbmeta"));

        let fp4 = load("data/devices/fairphone/FP4/installers/ubuntutouch.yml");
        assert!(fp4.bootstrap.iter().any(|s| matches!(s, BootstrapStep::DeleteLogicalPartition { .. })));
        assert!(fp4.bootstrap.iter().any(|s| matches!(s, BootstrapStep::ResizeLogicalPartition { size, .. } if *size > 0)));

        let ber = load("data/devices/xiaomi/beryllium/installers/ubuntutouch.yml");
        assert!(ber.ubports_downloads.iter().any(|d| d.unpack));
        assert!(ber.bootstrap.iter().any(|s| matches!(s, BootstrapStep::Flash { file, .. } if file.starts_with("unpacked/"))));

        let hero = load("data/devices/samsung/herolte/installers/ubuntutouch.yml");
        assert!(hero.bootstrap.iter().any(|s| matches!(s, BootstrapStep::HeimdallFlash { .. })));
    }
}
