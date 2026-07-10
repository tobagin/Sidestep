// Install wizard flow — extracted from DeviceDetailsPage
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::models::Device;
use crate::models::DeviceDatabase;
use crate::models::distro_config::{ChannelConfig, InterfaceConfig};
use crate::models::installer::{InstallerConfig, Step};
use crate::pages::flashing::FlashingPage;
use crate::pages::safety::SafetyPage;
use crate::pages::wizard_choice_page::WizardChoicePage;
use gtk::{gio, glib, prelude::*};
use libadwaita as adw;
use std::rc::Rc;

/// Manages the wizard flow for installing a distro on a device.
///
/// Created on-demand when the user clicks Install on a distro.
/// All navigation pages are pushed onto the provided `NavigationView`.
pub struct InstallFlow {
    nav_view: adw::NavigationView,
    menu_model: Option<gio::MenuModel>,
    device: Device,
    db: DeviceDatabase,
    on_unlock: Rc<dyn Fn()>,
}

impl InstallFlow {
    pub fn new(
        nav_view: adw::NavigationView,
        menu_model: Option<gio::MenuModel>,
        device: Device,
        db: DeviceDatabase,
        on_unlock: impl Fn() + 'static,
    ) -> Rc<Self> {
        Rc::new(Self {
            nav_view,
            menu_model,
            device,
            db,
            on_unlock: Rc::new(on_unlock),
        })
    }

    // ────────────────────────────────────────────────────────────────
    // Entry point
    // ────────────────────────────────────────────────────────────────

    pub fn start(self: &Rc<Self>, distro_id: &str) {
        if self.device.is_locked == Some(false) {
            self.show_safety_page(distro_id);
        } else {
            self.show_unlock_check_page(distro_id);
        }
    }

    // ────────────────────────────────────────────────────────────────
    // Bootloader check & safety pages
    // ────────────────────────────────────────────────────────────────

    fn show_unlock_check_page(self: &Rc<Self>, distro_id: &str) {
        let page = WizardChoicePage::new(
            "Bootloader Check",
            "Bootloader Unlocked?",
            "Your device's bootloader must be unlocked before installing a distribution. Is your bootloader unlocked?",
            "system-lock-screen-symbolic",
        );
        let yes_btn = page.add_button("Yes, it's unlocked", &["suggested-action", "pill"]);
        let no_btn = page.add_button("No / I don't know", &["pill"]);

        let flow = self.clone();
        let distro_id_owned = distro_id.to_string();
        yes_btn.connect_clicked(move |_| {
            flow.show_safety_page(&distro_id_owned);
        });

        let on_unlock = self.on_unlock.clone();
        no_btn.connect_clicked(move |_| {
            on_unlock();
        });

        self.nav_view.push(&page);
    }

    fn show_safety_page(self: &Rc<Self>, distro_id: &str) {
        let safety_page = SafetyPage::new();
        safety_page.set_device(&self.device);

        let flow = self.clone();
        let distro_id_owned = distro_id.to_string();
        safety_page.connect_confirmed(move |_| {
            flow.proceed_to_installer(&distro_id_owned);
        });

        self.nav_view.push(&safety_page);
    }

    // ────────────────────────────────────────────────────────────────
    // Prerequisites
    // ────────────────────────────────────────────────────────────────

    fn proceed_to_installer(self: &Rc<Self>, distro_id: &str) {
        if let Some(config) = self.db.load_installer_config(&self.device, distro_id) {
            if let Some(prereq) = config.prerequisites.first() {
                self.show_prerequisite_page(distro_id, &config, prereq);
                return;
            }
        }

        self.proceed_after_prerequisites(distro_id);
    }

    fn show_prerequisite_page(
        self: &Rc<Self>,
        distro_id: &str,
        config: &InstallerConfig,
        prereq: &crate::models::installer::Prerequisite,
    ) {
        let page = WizardChoicePage::new(
            &prereq.title,
            &prereq.title,
            &prereq.message,
            "system-help-symbolic",
        );
        let yes_btn = page.add_button("Yes, I have it", &["suggested-action", "pill"]);
        let no_btn = page.add_button("No, I don't", &["pill"]);

        // "Yes" → proceed to channel selection / install
        let flow = self.clone();
        let distro_id_owned = distro_id.to_string();
        let on_success = prereq.on_success.clone();
        let config_clone = config.clone();
        yes_btn.connect_clicked(move |_| {
            if let Some(step) = config_clone.steps.get(&on_success) {
                match step {
                    Step::Flash { .. } => {
                        flow.proceed_after_prerequisites(&distro_id_owned);
                    }
                    Step::Instruction { message, link, action_label } => {
                        flow.show_instruction_page(message, link.as_deref(), action_label.as_deref());
                    }
                    Step::FlashAndroid { android_version, url, sha256, message } => {
                        flow.show_flash_android_confirmation(
                            android_version, url, sha256, message,
                        );
                    }
                }
            } else {
                flow.proceed_after_prerequisites(&distro_id_owned);
            }
        });

        // "No" → show instruction page
        let flow2 = self.clone();
        let on_failure = prereq.on_failure.clone();
        let config_clone2 = config.clone();
        no_btn.connect_clicked(move |_| {
            if let Some(step) = config_clone2.steps.get(&on_failure) {
                match step {
                    Step::Instruction { message, link, action_label } => {
                        flow2.show_instruction_page(message, link.as_deref(), action_label.as_deref());
                    }
                    Step::Flash { .. } => {
                        log::warn!("on_failure pointed to a flash step, ignoring");
                    }
                    Step::FlashAndroid { android_version, url, sha256, message } => {
                        flow2.show_flash_android_confirmation(
                            android_version, url, sha256, message,
                        );
                    }
                }
            }
        });

        self.nav_view.push(&page);
    }

    fn show_instruction_page(
        &self,
        message: &str,
        link: Option<&str>,
        action_label: Option<&str>,
    ) {
        let page = WizardChoicePage::new(
            "Instructions",
            "Additional Steps Required",
            message,
            "dialog-information-symbolic",
        );

        if let (Some(url), Some(label)) = (link, action_label) {
            let link_btn = page.add_button(label, &["suggested-action", "pill"]);
            let url_string = url.to_string();
            link_btn.connect_clicked(move |_| {
                let launcher = gtk::UriLauncher::new(&url_string);
                glib::spawn_future_local(async move {
                    if let Err(e) = launcher.launch_future(None::<&gtk::Window>).await {
                        log::warn!("Failed to launch URI: {}", e);
                    }
                });
            });
        }

        self.nav_view.push(&page);
    }

    // ────────────────────────────────────────────────────────────────
    // Channel & interface selection
    // ────────────────────────────────────────────────────────────────

    fn proceed_after_prerequisites(self: &Rc<Self>, distro_id: &str) {
        let channels = self.load_channels(distro_id);

        if channels.is_empty() {
            let interfaces = self.load_interfaces(distro_id);
            if !interfaces.is_empty() {
                self.show_interface_selection_page(distro_id, &interfaces);
                return;
            }
            log::error!("No channels or interfaces found for distro '{}'", distro_id);
            return;
        }

        if channels.len() == 1 {
            self.launch_install(distro_id, &channels[0]);
        } else {
            self.show_channel_selection_page(distro_id, &channels);
        }
    }

    fn load_channels(&self, distro_id: &str) -> Vec<ChannelConfig> {
        match self.load_distro_config(distro_id) {
            Some(c) => c.channels,
            None => Vec::new(),
        }
    }

    fn load_interfaces(&self, distro_id: &str) -> Vec<InterfaceConfig> {
        match self.load_distro_config(distro_id) {
            Some(distro) => distro.interfaces.unwrap_or_default(),
            None => Vec::new(),
        }
    }

    fn load_distro_config(&self, distro_id: &str) -> Option<crate::models::distro_config::DistroConfig> {
        self.db.get_distro_config(&self.device, distro_id)
    }

    fn show_channel_selection_page(
        self: &Rc<Self>,
        distro_id: &str,
        channels: &[ChannelConfig],
    ) {
        let page = WizardChoicePage::new(
            "Select Channel",
            "Select Channel",
            "Choose which channel to install:",
            "emblem-system-symbolic",
        );

        for channel in channels {
            let btn = page.add_button(&channel.label, &["suggested-action", "pill"]);
            let flow = self.clone();
            let distro_id_owned = distro_id.to_string();
            let channel_clone = channel.clone();
            btn.connect_clicked(move |_| {
                flow.launch_install(&distro_id_owned, &channel_clone);
            });
        }

        self.nav_view.push(&page);
    }

    /// Unified interface selection page for Mobian, postmarketOS, etc.
    fn show_interface_selection_page(
        self: &Rc<Self>,
        distro_id: &str,
        interfaces: &[InterfaceConfig],
    ) {
        let page = WizardChoicePage::new(
            "Select Interface",
            "Select Interface",
            "Choose which desktop interface to install:",
            "emblem-system-symbolic",
        );

        for iface in interfaces {
            let btn = page.add_button(&iface.label, &["suggested-action", "pill"]);
            let flow = self.clone();
            let distro_id_owned = distro_id.to_string();
            let iface_id = iface.id.clone();
            btn.connect_clicked(move |_| {
                flow.launch_interface_install(&distro_id_owned, &iface_id);
            });
        }

        self.nav_view.push(&page);
    }

    // ────────────────────────────────────────────────────────────────
    // Flashing page helper
    // ────────────────────────────────────────────────────────────────

    fn push_flashing_page(&self, progress_page: &FlashingPage) {
        let nav_view_weak = self.nav_view.downgrade();
        let menu_model = self.menu_model.clone();
        progress_page.connect_installation_complete(move |page| {
            if let Some(nav) = nav_view_weak.upgrade() {
                if let Some(ref model) = menu_model {
                    let name = page.distro_name();
                    crate::window::SidestepWindow::show_success(&nav, model, &name);
                }
            }
        });

        progress_page.connect_installation_failed(move |page| {
            if let Some(window) = page.root()
                .and_then(|w| w.downcast::<crate::window::SidestepWindow>().ok())
            {
                window.reset_to_waiting();
            }
        });

        self.nav_view.push(progress_page);
    }

    /// Pause device detection via the SidestepWindow.
    fn pause_detection(&self) {
        if let Some(window) = self.nav_view.root()
            .and_then(|w| w.downcast::<crate::window::SidestepWindow>().ok())
        {
            window.pause_detection();
        }
    }

    // ────────────────────────────────────────────────────────────────
    // Install launchers
    // ────────────────────────────────────────────────────────────────

    fn launch_install(
        self: &Rc<Self>,
        distro_id: &str,
        channel: &ChannelConfig,
    ) {
        self.pause_detection();

        match distro_id {
            "ubuntutouch" | "ubports" => self.launch_ubports_install(channel),
            "droidian" => self.launch_droidian_install(channel),
            "postmarketos" => self.launch_postmarketos_interface_selection(channel),
            "lineageos" => self.launch_lineageos_install(channel),
            "eos" => self.launch_eos_install(channel),
            _ => log::warn!("No installer backend for '{}'", distro_id),
        }
    }

    /// Launch install for interface-based distros (Mobian, postmarketOS when
    /// reached via interfaces instead of channels).
    fn launch_interface_install(self: &Rc<Self>, distro_id: &str, interface_id: &str) {
        self.pause_detection();

        match distro_id {
            "mobian" => self.launch_mobian_install(distro_id, interface_id),
            "postmarketos" => {
                // postmarketOS via interface selection needs a default channel
                // This path is used when interfaces are defined directly in distros.yml
                self.launch_mobian_install(distro_id, interface_id);
            }
            _ => self.launch_mobian_install(distro_id, interface_id),
        }
    }

    fn launch_ubports_install(&self, channel: &ChannelConfig) {
        let Some(ref serial) = self.device.serial else {
            log::error!("No device serial available for UBports installation");
            return;
        };

        let Some(ref channel_path) = channel.path else {
            log::error!("No channel path defined for channel {}", channel.id);
            return;
        };
        let channel_path = channel_path.trim_end_matches('/');

        // Load firmware config from installer YAML
        let firmware = self.db.load_installer_config(&self.device, "ubuntutouch")
            .or_else(|| self.db.load_installer_config(&self.device, "ubports"))
            .map(|c| c.firmware)
            .unwrap_or_default();

        log::info!("Installing Ubuntu Touch from channel: {} ({}) with {} firmware images",
            channel.label, channel_path, firmware.len());

        let progress_page = FlashingPage::new();
        if let Some(ref menu_model) = self.menu_model {
            progress_page.set_menu_model(menu_model);
        }

        progress_page.start_ubports_installation("Ubuntu Touch", serial, channel_path, firmware);

        self.push_flashing_page(&progress_page);
    }

    fn launch_droidian_install(&self, channel: &ChannelConfig) {
        let Some(ref serial) = self.device.serial else {
            log::error!("No device serial available for Droidian installation");
            return;
        };

        let Some(ref release_url) = channel.release_url else {
            log::error!("No release_url defined for Droidian channel {}", channel.id);
            return;
        };

        let Some(ref artifact_match) = channel.artifact_match else {
            log::error!("No artifact_match defined for Droidian channel {}", channel.id);
            return;
        };

        // Load flash partitions config from installer YAML
        let flash_partitions = self.db.load_installer_config(&self.device, "droidian")
            .map(|c| c.flash_partitions)
            .unwrap_or_default();

        log::info!(
            "Installing Droidian from channel: {} (pattern: {}) with {} flash partitions",
            channel.label, artifact_match, flash_partitions.len()
        );

        let progress_page = FlashingPage::new();
        if let Some(ref menu_model) = self.menu_model {
            progress_page.set_menu_model(menu_model);
        }

        progress_page.start_droidian_installation("Droidian", serial, release_url, artifact_match, flash_partitions);

        self.push_flashing_page(&progress_page);
    }

    fn launch_lineageos_install(&self, channel: &ChannelConfig) {
        let Some(ref serial) = self.device.serial else {
            log::error!("No device serial available for LineageOS installation");
            return;
        };

        let Some(ref release_url) = channel.release_url else {
            log::error!("No release_url defined for LineageOS channel {}", channel.id);
            return;
        };

        log::info!(
            "Installing LineageOS from channel: {} ({})",
            channel.label,
            release_url
        );

        let progress_page = FlashingPage::new();
        if let Some(ref menu_model) = self.menu_model {
            progress_page.set_menu_model(menu_model);
        }

        progress_page.start_lineageos_installation("LineageOS", serial, release_url, false);

        self.push_flashing_page(&progress_page);
    }

    fn launch_eos_install(&self, channel: &ChannelConfig) {
        let Some(ref serial) = self.device.serial else {
            log::error!("No device serial available for /e/OS installation");
            return;
        };

        let distro_config = match self.load_distro_config("eos") {
            Some(c) => c,
            None => {
                log::error!("Could not load distro config for /e/OS");
                return;
            }
        };

        let base_url = match distro_config.base_url {
            Some(ref url) => url.clone(),
            None => {
                log::error!("No base_url defined for /e/OS");
                return;
            }
        };

        log::info!(
            "Installing /e/OS channel {} for {} from {}",
            channel.id,
            self.device.codename,
            base_url
        );

        let progress_page = FlashingPage::new();
        if let Some(ref menu_model) = self.menu_model {
            progress_page.set_menu_model(menu_model);
        }

        progress_page.start_eos_installation(serial, &base_url, &self.device.codename, &channel.id);

        self.push_flashing_page(&progress_page);
    }

    fn launch_mobian_install(&self, distro_id: &str, interface_id: &str) {
        let Some(ref serial) = self.device.serial else {
            log::error!("No device serial available for Mobian installation");
            return;
        };

        let distro_config = match self.load_distro_config(distro_id) {
            Some(c) => c,
            None => {
                log::error!("Could not load distro config for {}", distro_id);
                return;
            }
        };

        let base_url = match distro_config.base_url {
            Some(ref url) => url.clone(),
            None => {
                log::error!("No base_url defined for {}", distro_id);
                return;
            }
        };

        let chipset = distro_config.chipset.unwrap_or_else(|| "sdm670".to_string());
        let device_model = distro_config.device_model.unwrap_or_else(|| self.device.codename.clone());

        // Load flash operations config from installer YAML
        let flash_operations = self.db.load_installer_config(&self.device, distro_id)
            .map(|c| c.flash_operations)
            .unwrap_or_default();

        log::info!(
            "Installing {} ({}) for {}/{} with {} flash operations",
            distro_id, interface_id, chipset, device_model, flash_operations.len()
        );

        let progress_page = FlashingPage::new();
        if let Some(ref menu_model) = self.menu_model {
            progress_page.set_menu_model(menu_model);
        }

        progress_page.start_mobian_installation(
            "Mobian", serial, &base_url, interface_id, &chipset, &device_model, flash_operations,
        );

        self.push_flashing_page(&progress_page);
    }

    // ────────────────────────────────────────────────────────────────
    // postmarketOS: channel → interface → install
    // ────────────────────────────────────────────────────────────────

    fn launch_postmarketos_interface_selection(
        self: &Rc<Self>,
        channel: &ChannelConfig,
    ) {
        let interfaces = self.load_interfaces("postmarketos");
        if interfaces.is_empty() {
            log::error!("No interfaces found for postmarketOS");
            return;
        }

        let page = WizardChoicePage::new(
            "Select Interface",
            "Select Interface",
            "Choose which desktop interface to install:",
            "emblem-system-symbolic",
        );

        for iface in &interfaces {
            let btn = page.add_button(&iface.label, &["suggested-action", "pill"]);
            let flow = self.clone();
            let channel_clone = channel.clone();
            let iface_id = iface.id.clone();
            btn.connect_clicked(move |_| {
                flow.launch_postmarketos_install(&channel_clone, &iface_id);
            });
        }

        self.nav_view.push(&page);
    }

    fn launch_postmarketos_install(&self, channel: &ChannelConfig, interface_id: &str) {
        let Some(ref serial) = self.device.serial else {
            log::error!("No device serial available for postmarketOS installation");
            return;
        };

        let distro_config = match self.load_distro_config("postmarketos") {
            Some(c) => c,
            None => {
                log::error!("Could not load distro config for postmarketos");
                return;
            }
        };

        let base_url = match distro_config.base_url {
            Some(ref url) => url.clone(),
            None => {
                log::error!("No base_url defined for postmarketOS");
                return;
            }
        };

        let device_name = format!("{}-{}", self.device.maker.to_lowercase(), self.device.codename);

        // Load flash operations config from installer YAML
        let flash_operations = self.db.load_installer_config(&self.device, "postmarketos")
            .map(|c| c.flash_operations)
            .unwrap_or_default();

        log::info!(
            "Installing postmarketOS channel={} interface={} device={} with {} flash operations",
            channel.id, interface_id, device_name, flash_operations.len()
        );

        let progress_page = FlashingPage::new();
        if let Some(ref menu_model) = self.menu_model {
            progress_page.set_menu_model(menu_model);
        }

        progress_page.start_postmarketos_installation(
            "postmarketOS",
            serial,
            &base_url,
            &channel.id,
            interface_id,
            &device_name,
            flash_operations,
        );

        self.push_flashing_page(&progress_page);
    }

    // ────────────────────────────────────────────────────────────────
    // Factory image (stock Android) flashing
    // ────────────────────────────────────────────────────────────────

    fn show_flash_android_confirmation(
        self: &Rc<Self>,
        android_version: &str,
        url: &str,
        sha256: &str,
        message: &str,
    ) {
        let page = WizardChoicePage::new(
            android_version,
            &format!("Flash {}?", android_version),
            message,
            "android-symbolic",
        );
        let flash_btn = page.add_button(
            &format!("Flash {}", android_version),
            &["destructive-action", "pill"],
        );
        let cancel_btn = page.add_button("Cancel", &["pill"]);

        let flow = self.clone();
        let version_owned = android_version.to_string();
        let url_owned = url.to_string();
        let sha256_owned = sha256.to_string();
        flash_btn.connect_clicked(move |_| {
            flow.launch_factory_image_flash(&version_owned, &url_owned, &sha256_owned);
        });

        let nav_clone = self.nav_view.clone();
        cancel_btn.connect_clicked(move |_| {
            nav_clone.pop();
        });

        self.nav_view.push(&page);
    }

    fn launch_factory_image_flash(&self, android_version: &str, url: &str, sha256: &str) {
        let Some(ref serial) = self.device.serial else {
            log::error!("No device serial available for factory image flash");
            return;
        };

        log::info!("Flashing factory image: {} for {}", android_version, serial);

        self.pause_detection();

        let progress_page = FlashingPage::new();
        if let Some(ref menu_model) = self.menu_model {
            progress_page.set_menu_model(menu_model);
        }

        progress_page.start_factory_image_installation(android_version, serial, url, sha256);

        // On completion: show toast and reset (this is a prerequisite step)
        progress_page.connect_installation_complete(move |page| {
            if let Some(window) = page.root()
                .and_then(|w| w.downcast::<crate::window::SidestepWindow>().ok())
            {
                window.show_toast("Android flashed successfully!");
                window.reset_to_waiting();
            }
        });

        self.nav_view.push(&progress_page);
    }
}
