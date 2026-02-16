// Device detector - polls for connected devices
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::hardware::{Adb, Fastboot};
use crate::models::{Device, DeviceDatabase};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

/// Device event for communication between detector thread and main loop
#[derive(Debug, Clone)]
pub enum DeviceEvent {
    Connected(Device),
    Disconnected,
}

/// Device detector that polls for USB connected Android devices
pub struct DeviceDetector {
    running: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    sender: Sender<DeviceEvent>,
}

impl std::fmt::Debug for DeviceDetector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceDetector")
            .field("running", &self.running)
            .field("paused", &self.paused)
            .finish_non_exhaustive()
    }
}

impl DeviceDetector {
    /// Create a new detector, returning the detector and event receiver
    pub fn new() -> (Self, Receiver<DeviceEvent>) {
        let (sender, receiver) = mpsc::channel();

        let detector = Self {
            running: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
            sender,
        };

        (detector, receiver)
    }

    pub fn start(&self) {
        self.running.store(true, Ordering::SeqCst);

        let running = self.running.clone();
        let paused = self.paused.clone();
        let sender = self.sender.clone();

        // Start polling in a background thread
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime");

            rt.block_on(async {
                Self::poll_loop(running, paused, sender).await;
            });
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Pause detection — the poll loop will skip all device checks and send no events.
    pub fn pause(&self) {
        log::info!("Device detection paused");
        self.paused.store(true, Ordering::SeqCst);
    }

    /// Resume detection — resets internal state so reconnection is detected fresh.
    pub fn resume(&self) {
        log::info!("Device detection resumed");
        self.paused.store(false, Ordering::SeqCst);
    }

    async fn poll_loop(running: Arc<AtomicBool>, paused: Arc<AtomicBool>, sender: Sender<DeviceEvent>) {
        let adb = Adb::new();
        let fastboot = Fastboot::new();
        let db = DeviceDatabase::new();

        let mut last_device: Option<String> = None;

        while running.load(Ordering::SeqCst) {
            // When paused, skip all device checks and reset state so
            // reconnection is detected fresh when we resume.
            if paused.load(Ordering::SeqCst) {
                last_device = None;
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }

            let mut found_device = false;

            // Check ADB devices first
            if let Ok(adb_devices) = adb.devices().await {
                for dev in adb_devices {
                    if dev.state == "device" {
                        found_device = true;
                        
                        // Check if this is a new device
                        if last_device.as_ref() != Some(&dev.serial) {
                            // New device detected
                            // Try to get codename
                            let codename_result = adb.get_codename(&dev.serial).await;
                            
                            let (codename, display_name) = match codename_result {
                                Ok(name) => (name.clone(), name),
                                Err(e) => {
                                    log::warn!("Failed to get codename for {}: {}", dev.serial, e);
                                    // Try model as fallback
                                    match adb.get_model(&dev.serial).await {
                                        Ok(model) => (model.clone(), model),
                                        Err(_) => ("unknown".to_string(), dev.serial.clone()),
                                    }
                                }
                            };

                            log::info!("Detected device via ADB: {} ({})", dev.serial, codename);
                            
                            // Query additional device info via ADB
                            let android_version = match adb.get_android_version(&dev.serial).await {
                                Ok(v) if !v.is_empty() => Some(v),
                                _ => None,
                            };
                            let build_id = match adb.get_build_id(&dev.serial).await {
                                Ok(v) if !v.is_empty() => Some(v),
                                _ => None,
                            };
                            let battery_level = adb.get_battery_level(&dev.serial).await.ok();

                            // Look up device in database
                            if let Some(mut device) = db.find_by_codename(&codename) {
                                log::info!("Found device in database: {}", device.name);

                                device.serial = Some(dev.serial.clone());
                                device.android_version = android_version;
                                device.build_id = build_id;
                                device.battery_level = battery_level;

                                // Check lock status
                                match adb.is_unlocked(&dev.serial).await {
                                    Ok(unlocked) => device.is_locked = Some(!unlocked),
                                    Err(e) => log::warn!("Failed to check lock status: {}", e),
                                }

                                let _ = sender.send(DeviceEvent::Connected(device));
                            } else {
                                log::warn!("Device {} not in database", codename);
                                // Create an unknown device entry
                                let unknown_device = Device {
                                    codename: codename.clone(),
                                    name: format!("Unknown ({})", display_name),
                                    maker: "Unknown".to_string(),
                                    experimental: true,
                                    battery_min: 50,
                                    warnings: vec!["This device is not in the database.".to_string()],
                                    aliases: vec![],
                                    variants: vec![],
                                    is_locked: None,
                                    serial: Some(dev.serial.clone()),
                                    android_version,
                                    build_id,
                                    battery_level,
                                };
                                let _ = sender.send(DeviceEvent::Connected(unknown_device));
                            }
                            
                            last_device = Some(dev.serial);
                        }
                        break; // Only handle one device at a time
                    }
                }
            }

            // Also check fastboot devices
            if !found_device {
                if let Ok(fb_devices) = fastboot.devices().await {
                    for dev in fb_devices {
                        found_device = true;
                        
                        if last_device.as_ref() != Some(&dev.serial) {
                            log::info!("Detected device in fastboot mode: {}", dev.serial);
                            
                            // For fastboot, we might need to get product name
                            if let Ok(product) = fastboot.getvar(&dev.serial, "product").await {
                                log::info!("Fastboot product: {}", product);
                                
                                if let Some(mut device) = db.find_by_codename(&product) {
                                    device.serial = Some(dev.serial.clone());
                                    // Check lock status via fastboot
                                    match fastboot.is_unlocked(&dev.serial).await {
                                        Ok(unlocked) => device.is_locked = Some(!unlocked),
                                        Err(e) => log::warn!("Failed to check fastboot lock status: {}", e),
                                    }
                                    let _ = sender.send(DeviceEvent::Connected(device));
                                } else {
                                    // Handle unknown fastboot device
                                    log::warn!("Device {} (fastboot) not in database", product);
                                    let unknown_device = Device {
                                        codename: product.clone(),
                                        name: format!("Unknown Fastboot ({})", product),
                                        maker: "Unknown".to_string(),
                                        experimental: true,
                                        battery_min: 0, // Cannot read battery in fastboot usually
                                        warnings: vec![
                                            "This device is not in the database.".to_string(),
                                            "Device is in Fastboot mode.".to_string(),
                                        ],
                                        aliases: vec![],
                                        variants: vec![],
                                        is_locked: None,
                                        serial: Some(dev.serial.clone()),
                                        android_version: None,
                                        build_id: None,
                                        battery_level: None,
                                    };
                                    let _ = sender.send(DeviceEvent::Connected(unknown_device));
                                }
                            } else {
                                // Failed to get product, but device is present
                                log::warn!("Fastboot device detected but failed to get product: {}", dev.serial);
                                let unknown_device = Device {
                                    codename: "unknown".to_string(),
                                    name: format!("Unknown Device ({})", dev.serial),
                                    maker: "Unknown".to_string(),
                                    experimental: true,
                                    battery_min: 0,
                                    warnings: vec![
                                        "Could not identify device details.".to_string(),
                                        "Device is in Fastboot mode.".to_string(),
                                    ],
                                    aliases: vec![],
                                    variants: vec![],
                                    is_locked: None,
                                    serial: Some(dev.serial.clone()),
                                    android_version: None,
                                    build_id: None,
                                    battery_level: None,
                                };
                                let _ = sender.send(DeviceEvent::Connected(unknown_device));
                            }
                            
                            last_device = Some(dev.serial);
                        }
                        break;
                    }
                }
            }

            // Check if device was disconnected
            if !found_device && last_device.is_some() {
                log::info!("Device disconnected");
                last_device = None;
                let _ = sender.send(DeviceEvent::Disconnected);
            }

            // Poll every 2 seconds
            tokio::time::sleep(Duration::from_secs(2)).await;
        }

        log::debug!("Device detection stopped");
    }
}

impl Default for DeviceDetector {
    fn default() -> Self {
        Self::new().0
    }
}
