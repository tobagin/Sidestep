// Device database sync
// SPDX-License-Identifier: GPL-3.0-or-later
//
// Fetches the latest device configs from the project's signed release assets so
// users get new device support without waiting for an app update. Downloads the
// `devices.tar.gz` bundle, verifies its minisign signature, then extracts the
// `data/devices` subtree into the user data dir, which the database loader
// prefers over the bundled copy.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

// Minisign public key that signs the device-DB bundle. Paste the *second* line
// of your `minisign.pub` here (the base64 key, no comment line). While empty,
// sync still works but the bundle is NOT cryptographically verified — a warning
// is logged. Set this (and sign releases with packaging/sign-devices.sh) to make
// verification mandatory, which is the real defence against a repo compromise.
const DEVICE_DB_PUBLIC_KEY: &str = "";

// Signed release assets, NOT a branch ref or GitHub's auto-generated archive.
// The maintainer builds `devices.tar.gz` from data/devices, signs it into
// `devices.tar.gz.minisig`, and uploads both to the release matching this build
// (see packaging/sign-devices.sh). If the assets are absent the fetch 404s and
// we fall back to the bundled device DB (see window.rs) — safe by default.
fn assets_base() -> String {
    format!(
        "https://github.com/tobagin/Sidestep/releases/download/v{}",
        env!("CARGO_PKG_VERSION")
    )
}

fn tarball_url() -> String {
    format!("{}/devices.tar.gz", assets_base())
}

fn signature_url() -> String {
    format!("{}/devices.tar.gz.minisig", assets_base())
}

/// Verify the downloaded bundle against the embedded public key. Fail-closed
/// when a key is configured; when it isn't, warn and allow (so sync keeps
/// working before the maintainer has set signing up).
fn verify_signature(bytes: &[u8], signature: Option<&str>) -> Result<()> {
    if DEVICE_DB_PUBLIC_KEY.trim().is_empty() {
        log::warn!(
            "Device-DB signing key not configured — syncing device configs WITHOUT \
             signature verification. Set DEVICE_DB_PUBLIC_KEY to enforce."
        );
        return Ok(());
    }
    let signature = signature.context("Missing .minisig signature for device bundle")?;
    let pk = minisign_verify::PublicKey::from_base64(DEVICE_DB_PUBLIC_KEY.trim())
        .context("Invalid embedded device-DB public key")?;
    let sig = minisign_verify::Signature::decode(signature)
        .context("Malformed device-DB signature")?;
    pk.verify(bytes, &sig, false)
        .map_err(|e| anyhow::anyhow!("Device bundle signature verification failed: {e}"))?;
    log::info!("Device-DB bundle signature verified");
    Ok(())
}

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
    let (bytes, signature) = rt.block_on(async {
        // https_only rejects a redirect that downgrades to http.
        let client = reqwest::Client::builder().https_only(true).build()?;
        let bytes = client
            .get(tarball_url())
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        // The signature is only needed when a key is configured; treat a missing
        // one as absent and let verify_signature decide whether that's fatal.
        let signature = match client.get(signature_url()).send().await {
            Ok(resp) => resp.error_for_status().ok().map(|r| r.text()),
            Err(_) => None,
        };
        let signature = match signature {
            Some(fut) => Some(fut.await?),
            None => None,
        };
        Ok::<_, anyhow::Error>((bytes, signature))
    })?;

    verify_signature(&bytes, signature.as_deref())?;

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
/// inside `data/devices`. Accepts both the maintainer bundle (entries rooted at
/// `data/devices/...`) and a GitHub-style archive (`<top>/data/devices/...`) by
/// trying to strip the prefix directly, then after dropping one top-level dir.
fn device_subpath(path: &Path) -> Option<PathBuf> {
    use std::path::Component;
    let sub: PathBuf = if let Ok(s) = path.strip_prefix("data/devices") {
        s.to_path_buf()
    } else {
        // Fall back to dropping a top-level wrapper dir (GitHub-style archive).
        let mut comps = path.components();
        comps.next();
        comps.as_path().strip_prefix("data/devices").ok()?.to_path_buf()
    };
    if sub.as_os_str().is_empty() {
        return None;
    }
    // Reject traversal: `..` or an absolute/root component would let a crafted
    // tarball escape `target` (tar's entry.unpack does no containment check).
    if sub
        .components()
        .any(|c| matches!(c, Component::ParentDir | Component::RootDir | Component::Prefix(_)))
    {
        log::warn!("Rejecting device-DB entry with unsafe path: {:?}", path);
        return None;
    }
    Some(sub)
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
        // Maintainer bundle: entries rooted directly at data/devices.
        assert_eq!(
            device_subpath(Path::new("data/devices/google/sargo/info.yml")),
            Some(PathBuf::from("google/sargo/info.yml"))
        );
        // GitHub-style archive with a top-level wrapper dir also works.
        assert_eq!(
            device_subpath(Path::new("Sidestep-main/data/devices/google/sargo/info.yml")),
            Some(PathBuf::from("google/sargo/info.yml"))
        );
        // Non-device paths and the bare prefix are ignored.
        assert_eq!(device_subpath(Path::new("Sidestep-main/README.md")), None);
        assert_eq!(device_subpath(Path::new("data/devices")), None);
        assert_eq!(device_subpath(Path::new("Sidestep-main/src/main.rs")), None);
    }

    #[test]
    fn rejects_path_traversal() {
        // A crafted tarball must not escape the target directory.
        assert_eq!(
            device_subpath(Path::new("data/devices/../../../.bashrc")),
            None
        );
        assert_eq!(
            device_subpath(Path::new("Sidestep-0.3.0/data/devices/a/../../../etc/passwd")),
            None
        );
        // A normal nested path still maps through.
        assert_eq!(
            device_subpath(Path::new("data/devices/google/sargo/info.yml")),
            Some(PathBuf::from("google/sargo/info.yml"))
        );
    }
}
