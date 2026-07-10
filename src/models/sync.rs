// Device database sync
// SPDX-License-Identifier: GPL-3.0-or-later
//
// Fetches the latest device configs from the project repo so users get new
// device support without waiting for an app update. Downloads the main-branch
// tarball and extracts its `data/devices` subtree into the user data dir,
// which the database loader prefers over the bundled copy.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

const TARBALL_URL: &str =
    "https://github.com/tobagin/Sidestep/archive/refs/heads/main.tar.gz";

/// User-writable data dir; synced device configs land under `devices/`.
pub fn user_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("sidestep")
}

/// Unix timestamp (seconds) of the last successful sync, if any.
pub fn last_sync() -> Option<u64> {
    std::fs::read_to_string(user_data_dir().join("last-sync"))
        .ok()?
        .trim()
        .parse()
        .ok()
}

/// True if a sync is overdue given the interval (hours). Used for auto-sync.
pub fn is_due(interval_hours: i32) -> bool {
    let Some(last) = last_sync() else { return true };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    now.saturating_sub(last) >= (interval_hours.max(1) as u64) * 3600
}

/// Blocking sync: downloads and extracts the device configs. Runs its own
/// current-thread tokio runtime, matching the flashing modules. Returns the
/// number of files written.
pub fn run_blocking() -> Result<usize> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let bytes = rt.block_on(async {
        reqwest::get(TARBALL_URL)
            .await?
            .error_for_status()?
            .bytes()
            .await
    })?;

    let target = user_data_dir().join("devices");
    let count = extract_devices(&bytes, &target)?;
    write_last_sync()?;
    log::info!("Synced {} device files into {:?}", count, target);
    Ok(count)
}

/// Extract only `<repo>/data/devices/**` from the gzipped tarball into `target`.
fn extract_devices(bytes: &[u8], target: &Path) -> Result<usize> {
    let gz = flate2::read::GzDecoder::new(bytes);
    let mut archive = tar::Archive::new(gz);
    let mut count = 0;

    for entry in archive.entries().context("reading tarball")? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let Some(sub) = device_subpath(&path) else { continue };
        let out = target.join(sub);
        if entry.header().entry_type().is_dir() {
            std::fs::create_dir_all(&out).ok();
        } else {
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            entry.unpack(&out)?;
            count += 1;
        }
    }
    Ok(count)
}

/// Map a tarball entry path to its path under `devices/`, or None if it's not
/// inside `<top>/data/devices`. Strips the archive's top-level dir first.
fn device_subpath(path: &Path) -> Option<PathBuf> {
    let mut comps = path.components();
    comps.next(); // drop top-level "Sidestep-main/"
    let sub = comps.as_path().strip_prefix("data/devices").ok()?;
    if sub.as_os_str().is_empty() {
        return None;
    }
    Some(sub.to_path_buf())
}

fn write_last_sync() -> Result<()> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    std::fs::create_dir_all(user_data_dir())?;
    std::fs::write(user_data_dir().join("last-sync"), ts.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::device_subpath;
    use std::path::{Path, PathBuf};

    #[test]
    fn maps_device_files_and_ignores_others() {
        let yes = "Sidestep-main/data/devices/google/sargo/info.yml";
        assert_eq!(
            device_subpath(Path::new(yes)),
            Some(PathBuf::from("google/sargo/info.yml"))
        );
        // Non-device paths and the bare prefix are ignored.
        assert_eq!(device_subpath(Path::new("Sidestep-main/README.md")), None);
        assert_eq!(device_subpath(Path::new("Sidestep-main/data/devices")), None);
        assert_eq!(device_subpath(Path::new("Sidestep-main/src/main.rs")), None);
    }
}
