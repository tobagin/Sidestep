// Checksum verifier
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// Verifies file checksums
pub struct ChecksumVerifier;

impl ChecksumVerifier {
    /// Calculate SHA256 hash of a file
    pub fn sha256(path: &Path) -> Result<String> {
        log::debug!("Calculating SHA256 for {}", path.display());

        let file = File::open(path)
            .context("Failed to open file for checksum")?;
        let mut reader = BufReader::new(file);
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 64 * 1024];

        loop {
            let bytes_read = reader.read(&mut buffer)
                .context("Failed to read file")?;

            if bytes_read == 0 {
                break;
            }

            hasher.update(&buffer[..bytes_read]);
        }

        let hash = hasher.finalize();
        let hex_hash = hex::encode(hash);

        log::debug!("SHA256: {}", hex_hash);
        Ok(hex_hash)
    }

    /// Verify a file against an expected hash
    pub fn verify(path: &Path, expected_hash: &str) -> Result<bool> {
        let calculated = Self::sha256(path)?;
        let matches = calculated.to_lowercase() == expected_hash.to_lowercase();

        if matches {
            log::info!("Checksum verified for {}", path.display());
        } else {
            log::error!(
                "Checksum mismatch for {}: expected {}, got {}",
                path.display(),
                expected_hash,
                calculated
            );
        }

        Ok(matches)
    }

    /// Verify files against a checksum map
    pub fn verify_all(
        files_dir: &Path,
        checksums: &std::collections::HashMap<String, String>,
    ) -> Result<Vec<(String, bool)>> {
        let mut results = Vec::new();

        for (filename, expected) in checksums {
            let path = files_dir.join(filename);
            if path.exists() {
                let ok = Self::verify(&path, expected)?;
                results.push((filename.clone(), ok));
            } else {
                log::warn!("File not found for checksum: {}", filename);
                results.push((filename.clone(), false));
            }
        }

        Ok(results)
    }
}
