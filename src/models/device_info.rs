// Device Info Model
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceInfo {
    pub device: DeviceInfoData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceInfoData {
    pub name: String,
    pub codename: String,
    pub release_date: Option<String>,
    pub specs: DeviceSpecs,
    pub display: DisplaySpecs,
    pub connectivity: ConnectivitySpecs,
    pub cameras: Vec<CameraSpecs>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceSpecs {
    pub soc: String,
    pub ram: String,
    pub cpu: String,
    pub gpu: String,
    pub storage: String,
    pub battery: String,
    pub arch: Option<String>,
    pub dimensions: Option<Dimensions>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Dimensions {
    pub height: String,
    pub width: String,
    pub depth: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DisplaySpecs {
    pub size: String,
    pub resolution: String,
    pub density: String,
    pub panel_type: String,
    pub refresh_rate: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConnectivitySpecs {
    pub network: Vec<String>,
    pub bluetooth: String,
    pub wifi: String,
    pub peripherals: Vec<String>,
    pub sensors: Vec<String>,
    pub location: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CameraSpecs {
    pub label: String,
    pub resolution: String,
    pub features: String,
}
