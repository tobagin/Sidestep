// Device Details Page
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::config;
use crate::models::Device;
use crate::models::distro_config::{ChannelConfig, DistroConfig, InterfaceConfig};
use crate::models::installer::{InstallerConfig, Step};
use crate::pages::flashing::FlashingPage;
use crate::utils::yaml_parser::YamlParser;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use libadwaita as adw;
use adw::subclass::prelude::*;
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/tobagin/Sidestep/ui/pages/device_details.ui")]
    pub struct DeviceDetailsPage {
        #[template_child]
        pub device_name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub main_menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub device_codename_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub unlock_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub distro_buttons_box: TemplateChild<gtk::Box>,

        pub device: RefCell<Option<Device>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DeviceDetailsPage {
        const NAME: &'static str = "DeviceDetailsPage";
        type Type = super::DeviceDetailsPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.install_action("page.install", Some(glib::VariantTy::STRING), move |page, _, target| {
                 if let Some(distro_id) = target.and_then(|v| v.get::<String>()) {
                    page.on_install_clicked(&distro_id);
                 }
            });
            klass.install_action("page.unlock", None, move |page, _, _| {
                page.on_unlock_clicked();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }


    impl WidgetImpl for DeviceDetailsPage {}
    impl NavigationPageImpl for DeviceDetailsPage {}
}

glib::wrapper! {
    pub struct DeviceDetailsPage(ObjectSubclass<imp::DeviceDetailsPage>)
        @extends gtk::Widget, adw::NavigationPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl DeviceDetailsPage {
    pub fn new(device: &Device) -> Self {
        let obj: Self = glib::Object::builder().build();
        obj.set_device(device);
        obj
    }

    pub fn set_device(&self, device: &Device) {
        let imp = self.imp();
        *imp.device.borrow_mut() = Some(device.clone());
        imp.device_name_label.set_label(&device.name);
        imp.device_codename_label.set_label(&format!("{} ({})", device.maker, device.codename));

        // Show/hide unlock button based on lock status
        let is_locked = device.is_locked.unwrap_or(true);

        if is_locked {
            imp.unlock_button.set_visible(true);
            imp.distro_buttons_box.set_visible(false);
        } else {
            imp.unlock_button.set_visible(false);
            imp.distro_buttons_box.set_visible(true);
        }

        self.load_device_info(device);
    }

    pub fn set_menu_model(&self, model: &gio::MenuModel) {
        self.imp().main_menu_button.set_menu_model(Some(model));
    }

    fn load_device_info(&self, _device: &Device) {
        // Device info loading logic removed as per request
    }

    // ────────────────────────────────────────────────────────────────
    // Inline wizard flow
    // ────────────────────────────────────────────────────────────────

    fn on_install_clicked(&self, distro_id: &str) {
        let device = match self.imp().device.borrow().clone() {
            Some(d) => d,
            None => return,
        };

        let Some(nav_view) = self.ancestor(adw::NavigationView::static_type())
            .and_then(|w| w.downcast::<adw::NavigationView>().ok())
        else {
            log::error!("Could not find ancestor NavigationView");
            return;
        };

        // If bootloader is confirmed unlocked, skip the confirmation prompt
        if device.is_locked == Some(false) {
            self.proceed_to_installer(&nav_view, &device, distro_id);
        } else {
            // Locked or unknown — ask the user to confirm
            self.show_unlock_check_page(&nav_view, &device, distro_id);
        }
    }

    fn show_unlock_check_page(
        &self,
        nav_view: &adw::NavigationView,
        device: &Device,
        distro_id: &str,
    ) {
        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&adw::HeaderBar::new());

        let status_page = adw::StatusPage::builder()
            .title("Bootloader Unlocked?")
            .description("Your device's bootloader must be unlocked before installing a distribution. Is your bootloader unlocked?")
            .icon_name("system-lock-screen-symbolic")
            .build();

        let buttons_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .halign(gtk::Align::Center)
            .build();

        let yes_btn = gtk::Button::builder()
            .label("Yes, it's unlocked")
            .css_classes(vec!["suggested-action", "pill"])
            .width_request(250)
            .height_request(50)
            .build();

        let no_btn = gtk::Button::builder()
            .label("No / I don't know")
            .css_classes(vec!["pill"])
            .width_request(250)
            .height_request(50)
            .build();

        buttons_box.append(&yes_btn);
        buttons_box.append(&no_btn);
        status_page.set_child(Some(&buttons_box));
        toolbar_view.set_content(Some(&status_page));

        let page = adw::NavigationPage::builder()
            .title("Bootloader Check")
            .child(&toolbar_view)
            .build();

        // "Yes" → proceed to installer prerequisites / channel selection
        let self_clone = self.clone();
        let nav_clone = nav_view.clone();
        let device_clone = device.clone();
        let distro_id_owned = distro_id.to_string();
        yes_btn.connect_clicked(move |_| {
            self_clone.proceed_to_installer(&nav_clone, &device_clone, &distro_id_owned);
        });

        // "No / I don't know" → trigger unlock flow
        let self_clone2 = self.clone();
        no_btn.connect_clicked(move |_| {
            self_clone2.on_unlock_clicked();
        });

        nav_view.push(&page);
    }

    /// Continue the install flow after bootloader is confirmed unlocked.
    fn proceed_to_installer(
        &self,
        nav_view: &adw::NavigationView,
        device: &Device,
        distro_id: &str,
    ) {
        // Try loading installer YAML config for prerequisites
        if let Some(config) = self.load_installer_config(device, distro_id) {
            if let Some(prereq) = config.prerequisites.first() {
                self.show_prerequisite_page(nav_view, device, distro_id, &config, prereq);
                return;
            }
        }

        // No prerequisites — go straight to channel selection / install
        self.proceed_after_prerequisites(nav_view, device, distro_id);
    }

    fn load_installer_config(&self, device: &Device, distro_id: &str) -> Option<InstallerConfig> {
        let possible_dirs = vec![
            std::path::PathBuf::from(config::SIDESTEP_DATA_DIR),
            std::path::PathBuf::from(config::PKGDATADIR),
            std::path::PathBuf::from("/app/share/sidestep"),
            std::path::PathBuf::from("data"),
        ];

        for dir in possible_dirs {
            let config_path = dir
                .join("devices")
                .join(device.maker.to_lowercase())
                .join(device.codename.to_lowercase())
                .join("installers")
                .join(format!("{}.yml", distro_id));

            if let Ok(content) = std::fs::read_to_string(&config_path) {
                match serde_yaml::from_str::<InstallerConfig>(&content) {
                    Ok(config) => return Some(config),
                    Err(e) => {
                        log::error!("Failed to parse {}: {}", config_path.display(), e);
                        return None;
                    }
                }
            }
        }

        log::debug!("No installer config found for {}/{}", device.codename, distro_id);
        None
    }

    fn show_prerequisite_page(
        &self,
        nav_view: &adw::NavigationView,
        device: &Device,
        distro_id: &str,
        config: &InstallerConfig,
        prereq: &crate::models::installer::Prerequisite,
    ) {
        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&adw::HeaderBar::new());

        let status_page = adw::StatusPage::builder()
            .title(&prereq.title)
            .description(&prereq.message)
            .icon_name("system-help-symbolic")
            .build();

        let buttons_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .halign(gtk::Align::Center)
            .build();

        let yes_btn = gtk::Button::builder()
            .label("Yes, I have it")
            .css_classes(vec!["suggested-action", "pill"])
            .width_request(200)
            .height_request(50)
            .build();

        let no_btn = gtk::Button::builder()
            .label("No, I don't")
            .css_classes(vec!["pill"])
            .width_request(200)
            .height_request(50)
            .build();

        buttons_box.append(&yes_btn);
        buttons_box.append(&no_btn);
        status_page.set_child(Some(&buttons_box));
        toolbar_view.set_content(Some(&status_page));

        let page = adw::NavigationPage::builder()
            .title(&prereq.title)
            .child(&toolbar_view)
            .build();

        // "Yes" → proceed to channel selection / install
        let self_clone = self.clone();
        let nav_clone = nav_view.clone();
        let device_clone = device.clone();
        let distro_id_owned = distro_id.to_string();
        let on_success = prereq.on_success.clone();
        let config_clone = config.clone();
        yes_btn.connect_clicked(move |_| {
            // Check if "on_success" points to a step or directly to flash
            if let Some(step) = config_clone.steps.get(&on_success) {
                match step {
                    Step::Flash { .. } => {
                        self_clone.proceed_after_prerequisites(&nav_clone, &device_clone, &distro_id_owned);
                    }
                    Step::Instruction { message, link, action_label } => {
                        self_clone.show_instruction_page(&nav_clone, message, link.as_deref(), action_label.as_deref());
                    }
                    Step::FlashAndroid { android_version, url, sha256, message } => {
                        self_clone.show_flash_android_confirmation(
                            &nav_clone, &device_clone, android_version, url, sha256, message,
                        );
                    }
                }
            } else {
                self_clone.proceed_after_prerequisites(&nav_clone, &device_clone, &distro_id_owned);
            }
        });

        // "No" → show instruction page
        let self_clone2 = self.clone();
        let nav_clone2 = nav_view.clone();
        let on_failure = prereq.on_failure.clone();
        let config_clone2 = config.clone();
        let device_clone2 = device.clone();
        no_btn.connect_clicked(move |_| {
            if let Some(step) = config_clone2.steps.get(&on_failure) {
                match step {
                    Step::Instruction { message, link, action_label } => {
                        self_clone2.show_instruction_page(&nav_clone2, message, link.as_deref(), action_label.as_deref());
                    }
                    Step::Flash { .. } => {
                        log::warn!("on_failure pointed to a flash step, ignoring");
                    }
                    Step::FlashAndroid { android_version, url, sha256, message } => {
                        self_clone2.show_flash_android_confirmation(
                            &nav_clone2, &device_clone2, android_version, url, sha256, message,
                        );
                    }
                }
            }
        });

        nav_view.push(&page);
    }

    fn show_instruction_page(
        &self,
        nav_view: &adw::NavigationView,
        message: &str,
        link: Option<&str>,
        action_label: Option<&str>,
    ) {
        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&adw::HeaderBar::new());

        let status_page = adw::StatusPage::builder()
            .title("Additional Steps Required")
            .description(message)
            .icon_name("dialog-information-symbolic")
            .build();

        if let (Some(url), Some(label)) = (link, action_label) {
            let buttons_box = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .spacing(12)
                .halign(gtk::Align::Center)
                .build();

            let link_btn = gtk::Button::builder()
                .label(label)
                .css_classes(vec!["suggested-action", "pill"])
                .width_request(200)
                .height_request(50)
                .build();

            let url_string = url.to_string();
            link_btn.connect_clicked(move |_| {
                gtk::UriLauncher::new(&url_string).launch(
                    None::<&gtk::Window>,
                    None::<&gtk::gio::Cancellable>,
                    |result| {
                        if let Err(e) = result {
                            log::warn!("Failed to launch URI: {}", e);
                        }
                    },
                );
            });

            buttons_box.append(&link_btn);
            status_page.set_child(Some(&buttons_box));
        }

        toolbar_view.set_content(Some(&status_page));

        let page = adw::NavigationPage::builder()
            .title("Instructions")
            .child(&toolbar_view)
            .build();

        nav_view.push(&page);
    }

    fn show_channel_selection_page(
        &self,
        nav_view: &adw::NavigationView,
        device: &Device,
        distro_id: &str,
        channels: &[ChannelConfig],
    ) {
        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&adw::HeaderBar::new());

        let status_page = adw::StatusPage::builder()
            .title("Select Channel")
            .description("Choose which channel to install:")
            .icon_name("emblem-system-symbolic")
            .build();

        let buttons_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .halign(gtk::Align::Center)
            .build();

        for channel in channels {
            let btn = gtk::Button::builder()
                .label(&channel.label)
                .css_classes(vec!["suggested-action", "pill"])
                .width_request(250)
                .height_request(50)
                .build();

            let self_clone = self.clone();
            let nav_clone = nav_view.clone();
            let device_clone = device.clone();
            let distro_id_owned = distro_id.to_string();
            let channel_clone = channel.clone();
            btn.connect_clicked(move |_| {
                self_clone.launch_install(&nav_clone, &device_clone, &distro_id_owned, &channel_clone);
            });

            buttons_box.append(&btn);
        }

        status_page.set_child(Some(&buttons_box));
        toolbar_view.set_content(Some(&status_page));

        let page = adw::NavigationPage::builder()
            .title("Select Channel")
            .child(&toolbar_view)
            .build();

        nav_view.push(&page);
    }

    fn proceed_after_prerequisites(
        &self,
        nav_view: &adw::NavigationView,
        device: &Device,
        distro_id: &str,
    ) {
        let channels = self.load_channels(device, distro_id);

        // If no channels, check for interfaces (e.g. Mobian)
        if channels.is_empty() {
            let interfaces = self.load_interfaces(device, distro_id);
            if !interfaces.is_empty() {
                self.show_interface_selection_page(nav_view, device, distro_id, &interfaces);
                return;
            }
            log::error!("No channels or interfaces found for distro '{}'", distro_id);
            return;
        }

        if channels.len() == 1 {
            self.launch_install(nav_view, device, distro_id, &channels[0]);
        } else {
            self.show_channel_selection_page(nav_view, device, distro_id, &channels);
        }
    }

    // ────────────────────────────────────────────────────────────────
    // Channel loading
    // ────────────────────────────────────────────────────────────────

    fn load_channels(&self, device: &Device, distro_id: &str) -> Vec<ChannelConfig> {
        let possible_paths = vec![
            std::path::PathBuf::from(config::PKGDATADIR).join("devices"),
            std::path::PathBuf::from("/app/share/sidestep/devices"),
            std::path::PathBuf::from("data/devices"),
            std::path::PathBuf::from("devices"),
        ];
        let devices_path = possible_paths
            .into_iter()
            .find(|p| p.exists())
            .unwrap_or_else(|| std::path::PathBuf::from("devices"));

        let manufacturer = match device.codename.as_str() {
            "sargo" => "google",
            "enchilada" => "oneplus",
            "zeta" => "microsoft",
            _ => "unknown",
        };

        let parser = YamlParser::new(devices_path);
        let config = match parser.parse_device_config(manufacturer, &device.codename) {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to load distros.yml for {}: {:#}", device.codename, e);
                return Vec::new();
            }
        };

        // Map distro_id to the id used in distros.yml
        let yml_distro_id = match distro_id {
            "ubuntutouch" => "ubports",
            other => other,
        };

        config
            .available_distros
            .iter()
            .find(|d| d.id == yml_distro_id)
            .map(|d| d.channels.clone())
            .unwrap_or_default()
    }

    fn load_interfaces(&self, device: &Device, distro_id: &str) -> Vec<InterfaceConfig> {
        match self.load_distro_config(device, distro_id) {
            Some(distro) => distro.interfaces.unwrap_or_default(),
            None => Vec::new(),
        }
    }

    /// Load the full DistroConfig for a given distro_id from distros.yml.
    fn load_distro_config(&self, device: &Device, distro_id: &str) -> Option<DistroConfig> {
        let possible_paths = vec![
            std::path::PathBuf::from(config::PKGDATADIR).join("devices"),
            std::path::PathBuf::from("/app/share/sidestep/devices"),
            std::path::PathBuf::from("data/devices"),
            std::path::PathBuf::from("devices"),
        ];
        let devices_path = possible_paths
            .into_iter()
            .find(|p| p.exists())
            .unwrap_or_else(|| std::path::PathBuf::from("devices"));

        let manufacturer = match device.codename.as_str() {
            "sargo" => "google",
            "enchilada" => "oneplus",
            "zeta" => "microsoft",
            _ => "unknown",
        };

        let parser = YamlParser::new(devices_path);
        let config = match parser.parse_device_config(manufacturer, &device.codename) {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to load distros.yml for {}: {:#}", device.codename, e);
                return None;
            }
        };

        config
            .available_distros
            .into_iter()
            .find(|d| d.id == distro_id)
    }

    fn show_interface_selection_page(
        &self,
        nav_view: &adw::NavigationView,
        device: &Device,
        distro_id: &str,
        interfaces: &[InterfaceConfig],
    ) {
        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&adw::HeaderBar::new());

        let status_page = adw::StatusPage::builder()
            .title("Select Interface")
            .description("Choose which desktop interface to install:")
            .icon_name("emblem-system-symbolic")
            .build();

        let buttons_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .halign(gtk::Align::Center)
            .build();

        for iface in interfaces {
            let btn = gtk::Button::builder()
                .label(&iface.label)
                .css_classes(vec!["suggested-action", "pill"])
                .width_request(250)
                .height_request(50)
                .build();

            let self_clone = self.clone();
            let nav_clone = nav_view.clone();
            let device_clone = device.clone();
            let distro_id_owned = distro_id.to_string();
            let iface_id = iface.id.clone();
            btn.connect_clicked(move |_| {
                self_clone.launch_mobian_install(&nav_clone, &device_clone, &distro_id_owned, &iface_id);
            });

            buttons_box.append(&btn);
        }

        status_page.set_child(Some(&buttons_box));
        toolbar_view.set_content(Some(&status_page));

        let page = adw::NavigationPage::builder()
            .title("Select Interface")
            .child(&toolbar_view)
            .build();

        nav_view.push(&page);
    }

    // ────────────────────────────────────────────────────────────────
    // Install launchers
    // ────────────────────────────────────────────────────────────────

    fn launch_install(
        &self,
        nav_view: &adw::NavigationView,
        device: &Device,
        distro_id: &str,
        channel: &ChannelConfig,
    ) {
        // Pause device detection to prevent disconnect events during install
        if let Some(window) = self.root()
            .and_then(|w| w.downcast::<crate::window::SidestepWindow>().ok())
        {
            window.pause_detection();
        }

        match distro_id {
            "ubuntutouch" => self.launch_ubports_install(nav_view, device, channel),
            "droidian" => self.launch_droidian_install(nav_view, device, channel),
            _ => log::warn!("No installer backend for '{}'", distro_id),
        }
    }

    fn launch_ubports_install(
        &self,
        nav_view: &adw::NavigationView,
        device: &Device,
        channel: &ChannelConfig,
    ) {
        let Some(ref serial) = device.serial else {
            log::error!("No device serial available for UBports installation");
            return;
        };

        let Some(ref channel_path) = channel.path else {
            log::error!("No channel path defined for channel {}", channel.id);
            return;
        };
        let channel_path = channel_path.trim_end_matches('/');

        log::info!("Installing Ubuntu Touch from channel: {} ({})", channel.label, channel_path);

        let progress_page = FlashingPage::new();
        if let Some(menu_model) = self.imp().main_menu_button.menu_model() {
            progress_page.set_menu_model(&menu_model);
        }

        progress_page.start_ubports_installation("Ubuntu Touch", serial, channel_path);

        let nav_view_weak = nav_view.downgrade();
        let menu_model = self.imp().main_menu_button.menu_model();
        progress_page.connect_installation_complete(move |_page| {
            if let Some(nav) = nav_view_weak.upgrade() {
                if let Some(ref model) = menu_model {
                    crate::window::SidestepWindow::show_success(&nav, model);
                }
            }
        });

        nav_view.push(&progress_page);
    }

    fn launch_droidian_install(
        &self,
        nav_view: &adw::NavigationView,
        device: &Device,
        channel: &ChannelConfig,
    ) {
        let Some(ref serial) = device.serial else {
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

        log::info!(
            "Installing Droidian from channel: {} (pattern: {})",
            channel.label,
            artifact_match
        );

        let progress_page = FlashingPage::new();
        if let Some(menu_model) = self.imp().main_menu_button.menu_model() {
            progress_page.set_menu_model(&menu_model);
        }

        progress_page.start_droidian_installation("Droidian", serial, release_url, artifact_match);

        let nav_view_weak = nav_view.downgrade();
        let menu_model = self.imp().main_menu_button.menu_model();
        progress_page.connect_installation_complete(move |_page| {
            if let Some(nav) = nav_view_weak.upgrade() {
                if let Some(ref model) = menu_model {
                    crate::window::SidestepWindow::show_success(&nav, model);
                }
            }
        });

        nav_view.push(&progress_page);
    }

    fn launch_mobian_install(
        &self,
        nav_view: &adw::NavigationView,
        device: &Device,
        distro_id: &str,
        interface_id: &str,
    ) {
        let Some(ref serial) = device.serial else {
            log::error!("No device serial available for Mobian installation");
            return;
        };

        let distro_config = match self.load_distro_config(device, distro_id) {
            Some(c) => c,
            None => {
                log::error!("Could not load distro config for mobian");
                return;
            }
        };

        let base_url = match distro_config.base_url {
            Some(ref url) => url.clone(),
            None => {
                log::error!("No base_url defined for Mobian");
                return;
            }
        };

        let chipset = distro_config.chipset.unwrap_or_else(|| "sdm670".to_string());
        let device_model = distro_config.device_model.unwrap_or_else(|| device.codename.clone());

        log::info!(
            "Installing Mobian ({}) for {}/{}",
            interface_id, chipset, device_model
        );

        // Pause device detection to prevent disconnect events during install
        if let Some(window) = self.root()
            .and_then(|w| w.downcast::<crate::window::SidestepWindow>().ok())
        {
            window.pause_detection();
        }

        let progress_page = FlashingPage::new();
        if let Some(menu_model) = self.imp().main_menu_button.menu_model() {
            progress_page.set_menu_model(&menu_model);
        }

        progress_page.start_mobian_installation(
            "Mobian", serial, &base_url, interface_id, &chipset, &device_model,
        );

        let nav_view_weak = nav_view.downgrade();
        let menu_model = self.imp().main_menu_button.menu_model();
        progress_page.connect_installation_complete(move |_page| {
            if let Some(nav) = nav_view_weak.upgrade() {
                if let Some(ref model) = menu_model {
                    crate::window::SidestepWindow::show_success(&nav, model);
                }
            }
        });

        nav_view.push(&progress_page);
    }

    // ────────────────────────────────────────────────────────────────
    // Factory image (stock Android) flashing
    // ────────────────────────────────────────────────────────────────

    fn show_flash_android_confirmation(
        &self,
        nav_view: &adw::NavigationView,
        device: &Device,
        android_version: &str,
        url: &str,
        sha256: &str,
        message: &str,
    ) {
        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&adw::HeaderBar::new());

        let status_page = adw::StatusPage::builder()
            .title(&format!("Flash {}?", android_version))
            .description(message)
            .icon_name("drive-harddisk-solidstate-symbolic")
            .build();

        let buttons_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .halign(gtk::Align::Center)
            .build();

        let flash_btn = gtk::Button::builder()
            .label(&format!("Flash {}", android_version))
            .css_classes(vec!["destructive-action", "pill"])
            .width_request(250)
            .height_request(50)
            .build();

        let cancel_btn = gtk::Button::builder()
            .label("Cancel")
            .css_classes(vec!["pill"])
            .width_request(200)
            .height_request(50)
            .build();

        buttons_box.append(&flash_btn);
        buttons_box.append(&cancel_btn);
        status_page.set_child(Some(&buttons_box));
        toolbar_view.set_content(Some(&status_page));

        let page = adw::NavigationPage::builder()
            .title(android_version)
            .child(&toolbar_view)
            .build();

        let self_clone = self.clone();
        let nav_clone = nav_view.clone();
        let device_clone = device.clone();
        let version_owned = android_version.to_string();
        let url_owned = url.to_string();
        let sha256_owned = sha256.to_string();
        flash_btn.connect_clicked(move |_| {
            self_clone.launch_factory_image_flash(
                &nav_clone,
                &device_clone,
                &version_owned,
                &url_owned,
                &sha256_owned,
            );
        });

        let nav_clone2 = nav_view.clone();
        cancel_btn.connect_clicked(move |_| {
            nav_clone2.pop();
        });

        nav_view.push(&page);
    }

    fn launch_factory_image_flash(
        &self,
        nav_view: &adw::NavigationView,
        device: &Device,
        android_version: &str,
        url: &str,
        sha256: &str,
    ) {
        let Some(ref serial) = device.serial else {
            log::error!("No device serial available for factory image flash");
            return;
        };

        log::info!("Flashing factory image: {} for {}", android_version, serial);

        // Pause device detection during flash
        if let Some(window) = self.root()
            .and_then(|w| w.downcast::<crate::window::SidestepWindow>().ok())
        {
            window.pause_detection();
        }

        let progress_page = FlashingPage::new();
        if let Some(menu_model) = self.imp().main_menu_button.menu_model() {
            progress_page.set_menu_model(&menu_model);
        }

        progress_page.start_factory_image_installation(android_version, serial, url, sha256);

        // On completion: show toast and reset to waiting (not success page —
        // this is a prerequisite step, the user still needs to install the Linux distro)
        let self_weak = self.downgrade();
        progress_page.connect_installation_complete(move |_page| {
            if let Some(page) = self_weak.upgrade() {
                if let Some(window) = page.root()
                    .and_then(|w| w.downcast::<crate::window::SidestepWindow>().ok())
                {
                    window.show_toast("Android flashed successfully!");
                    window.reset_to_waiting();
                }
            }
        });

        nav_view.push(&progress_page);
    }

    fn on_unlock_clicked(&self) {
        self.emit_by_name::<()>("unlock-clicked", &[]);
    }
}

impl Default for DeviceDetailsPage {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl DeviceDetailsPage {
    pub fn connect_unlock_clicked<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_closure(
            "unlock-clicked",
            false,
            glib::closure_local!(move |obj: Self| {
                f(&obj);
            }),
        )
    }
}

impl ObjectImpl for imp::DeviceDetailsPage {
    fn constructed(&self) {
        self.parent_constructed();
    }

    fn signals() -> &'static [glib::subclass::Signal] {
        static SIGNALS: once_cell::sync::Lazy<Vec<glib::subclass::Signal>> =
            once_cell::sync::Lazy::new(|| {
                vec![
                    glib::subclass::Signal::builder("unlock-clicked").build(),
                ]
            });
        &SIGNALS
    }
}
