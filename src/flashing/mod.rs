// Flashing engine module
// SPDX-License-Identifier: GPL-3.0-or-later

pub mod downloader;
pub mod decompressor;
pub mod checksum;
pub mod progress;
pub mod ubports;
pub mod droidian;
pub mod mobian;
pub mod postmarketos;
pub mod lineageos;
pub mod eos;
pub mod factory_image;
pub mod grapheneos;
pub mod heimdall;

use std::path::PathBuf;

/// Base directory for downloaded images. Reads the user's `download-path`
/// setting; falls back to the system cache dir. Callers append a per-distro
/// subdir (e.g. `.join("mobian")`).
pub fn download_dir() -> PathBuf {
    use gtk::prelude::SettingsExt;
    let configured = gtk::gio::Settings::new(crate::config::APP_ID).string("download-path");
    if !configured.is_empty() {
        return PathBuf::from(configured.as_str());
    }
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("sidestep")
}

pub use checksum::ChecksumVerifier;
pub use progress::InstallProgress;
pub use ubports::UbportsInstaller;
pub use droidian::DroidianInstaller;
pub use mobian::MobianInstaller;
pub use postmarketos::PostmarketosInstaller;
pub use lineageos::LineageosInstaller;
pub use eos::EosInstaller;
pub use factory_image::FactoryImageInstaller;
pub use grapheneos::GrapheneosInstaller;
pub use heimdall::HeimdallInstaller;
