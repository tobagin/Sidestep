// Image decompressor
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};

/// Progress callback type
pub type ProgressCallback = Box<dyn Fn(u64, u64) + Send + Sync>;

/// Decompresses .xz and .gz images
pub struct Decompressor;

impl Decompressor {
    /// Decompress a file, auto-detecting format from extension
    pub fn decompress(
        input_path: &Path,
        output_path: Option<&Path>,
        on_progress: Option<ProgressCallback>,
    ) -> Result<PathBuf> {
        let extension = input_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match extension {
            "xz" => Self::decompress_xz(input_path, output_path, on_progress),
            "gz" => Self::decompress_gz(input_path, output_path, on_progress),
            _ => {
                // Not compressed, just return the input path
                log::debug!("File {} is not compressed", input_path.display());
                Ok(input_path.to_path_buf())
            }
        }
    }

    /// Decompress an .xz file
    pub fn decompress_xz(
        input_path: &Path,
        output_path: Option<&Path>,
        on_progress: Option<ProgressCallback>,
    ) -> Result<PathBuf> {
        log::info!("Decompressing XZ: {}", input_path.display());

        let output = output_path
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| {
                let mut out = input_path.to_path_buf();
                out.set_extension("");
                out
            });

        let input_file = File::open(input_path)
            .context("Failed to open input file")?;
        let input_size = input_file.metadata()?.len();
        let reader = BufReader::new(input_file);

        let mut decoder = xz2::read::XzDecoder::new(reader);
        let mut output_file = File::create(&output)
            .context("Failed to create output file")?;

        let mut buffer = [0u8; 64 * 1024]; // 64KB buffer
        let _total_read: u64 = 0;
        let mut total_written: u64 = 0;

        loop {
            let bytes_read = decoder.read(&mut buffer)
                .context("Error reading from XZ stream")?;

            if bytes_read == 0 {
                break;
            }

            output_file.write_all(&buffer[..bytes_read])
                .context("Error writing decompressed data")?;

            total_written += bytes_read as u64;

            // Estimate progress (we don't know exact compressed read position)
            if let Some(ref callback) = on_progress {
                // Approximate based on output size ratio
                let progress = (total_written * 100) / (input_size * 4); // Rough estimate
                callback(progress.min(100), 100);
            }
        }

        log::info!("Decompressed {} bytes to {}", total_written, output.display());
        Ok(output)
    }

    /// Decompress a .gz file
    pub fn decompress_gz(
        input_path: &Path,
        output_path: Option<&Path>,
        on_progress: Option<ProgressCallback>,
    ) -> Result<PathBuf> {
        log::info!("Decompressing GZ: {}", input_path.display());

        let output = output_path
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| {
                let mut out = input_path.to_path_buf();
                out.set_extension("");
                out
            });

        let input_file = File::open(input_path)
            .context("Failed to open input file")?;
        let input_size = input_file.metadata()?.len();
        let reader = BufReader::new(input_file);

        let mut decoder = GzDecoder::new(reader);
        let mut output_file = File::create(&output)
            .context("Failed to create output file")?;

        let mut buffer = [0u8; 64 * 1024];
        let mut total_written: u64 = 0;

        loop {
            let bytes_read = decoder.read(&mut buffer)
                .context("Error reading from GZ stream")?;

            if bytes_read == 0 {
                break;
            }

            output_file.write_all(&buffer[..bytes_read])
                .context("Error writing decompressed data")?;

            total_written += bytes_read as u64;

            if let Some(ref callback) = on_progress {
                let progress = (total_written * 100) / (input_size * 3);
                callback(progress.min(100), 100);
            }
        }

        log::info!("Decompressed {} bytes to {}", total_written, output.display());
        Ok(output)
    }
}
