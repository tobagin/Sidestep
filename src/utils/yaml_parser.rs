// YAML Parser
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::models::distro::{Distro, DistroTreeNode};
use crate::models::distro_config::{DeviceDistroConfig, DistroConfig};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub struct YamlParser {
    devices_dir: PathBuf,
}

impl YamlParser {
    pub fn new<P: AsRef<Path>>(devices_dir: P) -> Self {
        Self {
            devices_dir: devices_dir.as_ref().to_path_buf(),
        }
    }

    pub fn parse_device_info(&self, manufacturer: &str, codename: &str) -> Result<crate::models::device_info::DeviceInfo> {
        let path = self.devices_dir.join(manufacturer).join(codename).join("info.yml");
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read info.yml at {:?}", path))?;

        let info: crate::models::device_info::DeviceInfo = serde_yaml::from_str(&content)
            .with_context(|| "Failed to parse info.yml")?;

        Ok(info)
    }

    pub fn parse_device_config(&self, manufacturer: &str, codename: &str) -> Result<DeviceDistroConfig> {
        let path = self.devices_dir.join(manufacturer).join(codename).join("distros.yml");
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read distros.yml at {:?}", path))?;

        let config: DeviceDistroConfig = serde_yaml::from_str(&content)
            .with_context(|| "Failed to parse distros.yml")?;

        Ok(config)
    }

    pub fn parse_distros(&self, manufacturer: &str, codename: &str) -> Result<Vec<DistroTreeNode>> {
        let path = self.devices_dir.join(manufacturer).join(codename).join("distros.yml");
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read distros.yml at {:?}", path))?;

        let config: DeviceDistroConfig = serde_yaml::from_str(&content)
            .with_context(|| "Failed to parse distros.yml")?;

        Ok(self.convert_to_tree(config))
    }

    fn convert_to_tree(&self, config: DeviceDistroConfig) -> Vec<DistroTreeNode> {
        let mut roots = Vec::new();

        for distro_conf in config.available_distros {
            // Level 1: Distro Name (e.g. "postmarketOS")
            
            let mut children = Vec::new();

            if let Some(interfaces) = &distro_conf.interfaces {
                // If interfaces exist, group by interface first
                for interface in interfaces {
                    let mut interface_children = Vec::new();

                    for channel in &distro_conf.channels {
                        let item = self.create_distro_item(
                            &config.device_codename,
                            &distro_conf,
                            Some(interface),
                            channel
                        );
                        interface_children.push(DistroTreeNode::Item(item));
                    }

                    children.push(DistroTreeNode::Group {
                        name: interface.label.clone(),
                        description: format!("{} variant", interface.label),
                        children: interface_children,
                    });
                }
            } else {
                // No interfaces, just add channels directly (e.g. generic distro or one flavor)
                for channel in &distro_conf.channels {
                    let item = self.create_distro_item(
                        &config.device_codename,
                        &distro_conf,
                        None,
                        channel
                    );
                    children.push(DistroTreeNode::Item(item));
                }
            }

            roots.push(DistroTreeNode::Group {
                name: distro_conf.name.clone(),
                description: format!("{}, developed by {}", distro_conf.name, distro_conf.developer),
                children,
            });
        }

        roots
    }

    fn create_distro_item(
        &self,
        device_codename: &str,
        distro_conf: &DistroConfig,
        interface: Option<&crate::models::distro_config::InterfaceConfig>,
        channel: &crate::models::distro_config::ChannelConfig,
    ) -> Distro {
        // Resolve Name
        let name = if let Some(iface) = interface {
            format!("{} ({}) {}", distro_conf.name, iface.label, channel.id)
        } else {
            format!("{} {}", distro_conf.name, channel.label)
        };

        // Resolve Description
        let description = channel.label.clone();

        // Resolve Download URL
        // Basic templating: {device}, {channel}, {interface}
        let url = if let Some(base) = &distro_conf.base_url {
            let mut u = base.replace("{device}", device_codename)
                           .replace("{channel}", &channel.id);
            if let Some(iface) = interface {
                u = u.replace("{interface}", &iface.id);
            }
            u
        } else if let Some(root) = &distro_conf.api_root {
             // For UBports-style, just use the root for now or construct path
             if let Some(path) = &channel.path {
                 format!("{}{}", root, path)
             } else {
                 root.clone()
             }
        } else {
            "https://placeholder.com".to_string()
        };

        Distro {
            name,
            version: channel.id.clone(),
            description,
            download_base_url: url,
            checksum_url: None, // Logic for checkusm url usually derived or separate
            partitions: vec![], // Partitions are dynamic, parser shouldn't guess them yet?
                                // Distro struct expects partitions. For now empty vector.
            homepage: None,
            download_size_bytes: None,
            requires_unlock: true,
            post_install_notes: None,
        }
    }
}
