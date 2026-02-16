// Shared installation progress types
// SPDX-License-Identifier: GPL-3.0-or-later

/// Progress messages sent from installer threads to the UI
#[derive(Debug, Clone)]
pub enum InstallProgress {
    /// File download progress (aggregated across all downloads)
    DownloadProgress {
        downloaded: u64,
        total: u64,
        file_name: String,
    },
    /// Checksum verification progress
    VerifyProgress {
        verified: usize,
        total: usize,
        file_name: String,
    },
    /// Flash/push step progress
    FlashProgress {
        current: usize,
        total: usize,
        description: String,
    },
    /// Status text update
    StatusChanged(String),
    /// Waiting for user to select Recovery mode on device
    WaitingForRecovery,
    /// Device entered recovery mode
    RecoveryDetected,
    /// Waiting for user to perform an action on the device (shown as info banner)
    WaitingForUserAction(String),
    /// Installation completed successfully
    Complete,
    /// An error occurred
    Error(String),
}
