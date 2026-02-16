// Unlocking step model
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Deserialize;

/// Type of unlocking step
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StepType {
    /// Manual step that user performs and checks off
    Manual,
    /// Automated step that runs a command
    Automated,
}

/// A step in the bootloader unlocking process
#[derive(Debug, Clone, Deserialize)]
pub struct UnlockingStep {
    /// Step order (1-based)
    pub order: u8,
    
    /// Step title
    pub title: String,
    
    /// Detailed description of the step
    pub description: String,
    
    /// Whether this is a manual or automated step
    #[serde(rename = "type")]
    pub step_type: StepType,
    
    /// Command to run for automated steps (e.g., "adb reboot bootloader")
    #[serde(default)]
    pub command: Option<String>,
    
    /// Expected duration in seconds (for progress indication)
    #[serde(default)]
    pub duration_secs: Option<u32>,
    
    /// Whether this step is optional
    #[serde(default)]
    pub optional: bool,
    
    /// Warning message to display before this step
    #[serde(default)]
    pub warning: Option<String>,
}

impl UnlockingStep {
    pub fn is_manual(&self) -> bool {
        self.step_type == StepType::Manual
    }

    pub fn is_automated(&self) -> bool {
        self.step_type == StepType::Automated
    }
}
