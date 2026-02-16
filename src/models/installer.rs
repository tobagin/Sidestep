use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallerConfig {
    pub name: String,
    pub prerequisites: Vec<Prerequisite>,
    pub steps: HashMap<String, Step>,
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
    // Add more types as needed, e.g., DownloadAndFlash
}
