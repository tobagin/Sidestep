// Device Details Page
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::config;
use crate::models::Device;
use crate::models::device_info::DeviceInfo;
use crate::models::distro_config::{ChannelConfig, CompatibilityInfo, DistroConfig, InterfaceConfig};
use crate::models::installer::{InstallerConfig, Step};
use crate::pages::flashing::FlashingPage;
use crate::pages::safety::SafetyPage;
use crate::utils::yaml_parser::YamlParser;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use libadwaita as adw;
use adw::prelude::*;
use adw::subclass::prelude::*;
use std::cell::{Cell, RefCell};

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
        pub status_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub specs_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub install_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub unsupported_label: TemplateChild<gtk::Label>,

        pub device: RefCell<Option<Device>>,
        pub supported: Cell<bool>,
        pub status_rows: RefCell<Vec<adw::ActionRow>>,
        pub specs_rows: RefCell<Vec<adw::ActionRow>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DeviceDetailsPage {
        const NAME: &'static str = "DeviceDetailsPage";
        type Type = super::DeviceDetailsPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.install_action("page.show-distros", None, move |page, _, _| {
                page.show_distro_selection_page();
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
    pub fn new(device: &Device, supported: bool) -> Self {
        let obj: Self = glib::Object::builder().build();
        obj.set_device(device, supported);
        obj
    }

    pub fn set_device(&self, device: &Device, supported: bool) {
        let imp = self.imp();
        *imp.device.borrow_mut() = Some(device.clone());
        imp.supported.set(supported);

        let browse_mode = device.serial.is_none();

        imp.device_name_label.set_label(&device.name);
        imp.device_codename_label.set_label(&format!("{} ({})", device.maker, device.codename));

        // Show/hide Install vs Unsupported
        if supported {
            imp.install_button.set_visible(true);
            imp.unsupported_label.set_visible(false);

            if browse_mode {
                imp.install_button.set_label("Connect Device to Install");
                imp.install_button.set_sensitive(false);
            } else {
                imp.install_button.set_label("Install");
                imp.install_button.set_sensitive(true);
            }
        } else {
            imp.install_button.set_visible(false);
            imp.unsupported_label.set_visible(true);
        }

        // Adjust status group for browse mode
        if browse_mode {
            imp.status_group.set_title("Setup Instructions");
        } else {
            imp.status_group.set_title("Device Status");
        }

        // Populate status group
        self.populate_status_group(device);

        // Populate specs group from info.yml
        self.load_and_populate_specs(device);
    }

    pub fn set_menu_model(&self, model: &gio::MenuModel) {
        self.imp().main_menu_button.set_menu_model(Some(model));
    }

    // ────────────────────────────────────────────────────────────────
    // Status & Specs population
    // ────────────────────────────────────────────────────────────────

    fn populate_status_group(&self, device: &Device) {
        let imp = self.imp();
        let browse_mode = device.serial.is_none();

        // Clear any previous rows
        for row in imp.status_rows.borrow().iter() {
            imp.status_group.remove(row);
        }
        imp.status_rows.borrow_mut().clear();

        let mut rows = Vec::new();

        if browse_mode {
            // Browse mode: show setup info banner and bootloader hint
            let info_row = adw::ActionRow::builder()
                .title("Connect your device via USB to begin installation")
                .build();
            let info_icon = gtk::Image::from_icon_name("dialog-information-symbolic");
            info_row.add_prefix(&info_icon);
            rows.push(info_row);

            rows.push(self.make_action_row("Bootloader", "Connect device to check"));
        } else {
            // Live mode: show all runtime info
            if let Some(ref serial) = device.serial {
                rows.push(self.make_action_row("Serial", serial));
            }

            if let Some(ref ver) = device.android_version {
                rows.push(self.make_action_row("Android Version", ver));
            }

            if let Some(ref build) = device.build_id {
                rows.push(self.make_action_row("Build", build));
            }

            if let Some(level) = device.battery_level {
                let icon_name = if level >= 80 {
                    "battery-level-100-symbolic"
                } else if level >= 50 {
                    "battery-level-50-symbolic"
                } else if level >= 20 {
                    "battery-level-20-symbolic"
                } else {
                    "battery-level-0-symbolic"
                };
                let row = adw::ActionRow::builder()
                    .title("Battery")
                    .build();
                row.add_suffix(&gtk::Label::new(Some(&format!("{}%", level))));
                row.add_prefix(&gtk::Image::from_icon_name(icon_name));
                rows.push(row);
            }

            let lock_text = match device.is_locked {
                Some(true) => "Locked",
                Some(false) => "Unlocked",
                None => "Unknown",
            };
            rows.push(self.make_action_row("Bootloader", lock_text));
        }

        for row in &rows {
            imp.status_group.add(row);
        }
        *imp.status_rows.borrow_mut() = rows;
    }

    fn load_and_populate_specs(&self, device: &Device) {
        let imp = self.imp();

        // Clear previous specs rows
        for row in imp.specs_rows.borrow().iter() {
            imp.specs_group.remove(row);
        }
        imp.specs_rows.borrow_mut().clear();

        let info = match self.load_device_info(device) {
            Some(info) => info,
            None => {
                imp.specs_group.set_visible(false);
                return;
            }
        };

        let mut rows = Vec::new();
        let specs = &info.device.specs;
        let display = &info.device.display;

        rows.push(self.make_action_row("SoC", &specs.soc));
        rows.push(self.make_action_row("CPU", &specs.cpu));
        rows.push(self.make_action_row("GPU", &specs.gpu));
        rows.push(self.make_action_row("RAM", &specs.ram));
        rows.push(self.make_action_row("Storage", &specs.storage));
        rows.push(self.make_action_row("Battery", &specs.battery));
        rows.push(self.make_action_row("Display", &format!(
            "{} {} ({})", display.size, display.panel_type, display.resolution
        )));

        for row in &rows {
            imp.specs_group.add(row);
        }
        *imp.specs_rows.borrow_mut() = rows;
        imp.specs_group.set_visible(true);
    }

    fn load_device_info(&self, device: &Device) -> Option<DeviceInfo> {
        let possible_dirs = vec![
            std::path::PathBuf::from(config::PKGDATADIR).join("devices"),
            std::path::PathBuf::from("/app/share/sidestep/devices"),
            std::path::PathBuf::from("data/devices"),
            std::path::PathBuf::from("devices"),
        ];

        let manufacturer = maker_to_dir(&device.maker);

        for dir in possible_dirs {
            let parser = YamlParser::new(&dir);
            if let Ok(info) = parser.parse_device_info(&manufacturer, &device.codename) {
                return Some(info);
            }
        }

        log::debug!("No info.yml found for {}/{}", device.maker, device.codename);
        None
    }

    fn make_action_row(&self, title: &str, value: &str) -> adw::ActionRow {
        let row = adw::ActionRow::builder()
            .title(title)
            .build();
        let label = gtk::Label::builder()
            .label(value)
            .css_classes(vec!["dim-label".to_string()])
            .build();
        row.add_suffix(&label);
        row
    }

    // ────────────────────────────────────────────────────────────────
    // Distro selection page
    // ────────────────────────────────────────────────────────────────

    fn show_distro_selection_page(&self) {
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

        let distros = self.load_all_distros(&device);
        if distros.is_empty() {
            log::error!("No distros found for {}", device.codename);
            return;
        }

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&adw::HeaderBar::new());

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();

        let clamp = adw::Clamp::builder()
            .maximum_size(600)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(12)
            .margin_end(12)
            .build();

        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(24)
            .build();

        // Title area
        let title_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .halign(gtk::Align::Center)
            .build();

        let icon = gtk::Image::from_icon_name("system-software-install-symbolic");
        icon.set_pixel_size(96);
        title_box.append(&icon);

        let title_label = gtk::Label::builder()
            .label("Choose a Distribution")
            .css_classes(vec!["title-1".to_string()])
            .build();

        let subtitle_label = gtk::Label::builder()
            .label("Select which operating system to install on your device")
            .css_classes(vec!["dim-label".to_string()])
            .wrap(true)
            .justify(gtk::Justification::Center)
            .build();

        title_box.append(&title_label);
        title_box.append(&subtitle_label);
        content_box.append(&title_box);

        // Distro list group
        let distro_group = adw::PreferencesGroup::new();

        for distro in &distros {
            let row = adw::ActionRow::builder()
                .title(&distro.name)
                .subtitle(&distro.developer)
                .activatable(true)
                .build();

            // Add distro icon
            let icon_name = format!("{}-symbolic", distro.id);
            let icon = gtk::Image::from_icon_name(&icon_name);
            icon.set_pixel_size(32);
            row.add_prefix(&icon);

            // Add chevron
            let chevron = gtk::Image::from_icon_name("go-next-symbolic");
            row.add_suffix(&chevron);

            let self_clone = self.clone();
            let nav_clone = nav_view.clone();
            let device_clone = device.clone();
            let distro_clone = distro.clone();
            row.connect_activated(move |_| {
                // Check for compatibility data
                if let Some(ref compat) = distro_clone.compatibility {
                    if !compat.working.is_empty() || !compat.partial.is_empty() || !compat.not_working.is_empty() || !compat.untested.is_empty() {
                        self_clone.show_compatibility_page(
                            &nav_clone,
                            &device_clone,
                            &distro_clone.id,
                            &distro_clone.name,
                            compat,
                        );
                        return;
                    }
                }
                // No compatibility data — proceed directly
                self_clone.on_install_clicked(&distro_clone.id);
            });

            distro_group.add(&row);
        }

        content_box.append(&distro_group);
        clamp.set_child(Some(&content_box));
        scrolled.set_child(Some(&clamp));
        toolbar_view.set_content(Some(&scrolled));

        let page = adw::NavigationPage::builder()
            .title("Choose Distribution")
            .child(&toolbar_view)
            .build();

        nav_view.push(&page);
    }

    fn load_all_distros(&self, device: &Device) -> Vec<DistroConfig> {
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

        let manufacturer = maker_to_dir(&device.maker);

        let parser = YamlParser::new(devices_path);
        match parser.parse_device_config(&manufacturer, &device.codename) {
            Ok(config) => config.available_distros,
            Err(e) => {
                log::error!("Failed to load distros.yml for {}: {:#}", device.codename, e);
                Vec::new()
            }
        }
    }

    // ────────────────────────────────────────────────────────────────
    // Compatibility page
    // ────────────────────────────────────────────────────────────────

    fn show_compatibility_page(
        &self,
        nav_view: &adw::NavigationView,
        _device: &Device,
        distro_id: &str,
        distro_name: &str,
        compat: &CompatibilityInfo,
    ) {
        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&adw::HeaderBar::new());

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();

        let clamp = adw::Clamp::builder()
            .maximum_size(600)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(12)
            .margin_end(12)
            .build();

        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(24)
            .build();

        // Title
        let title_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .halign(gtk::Align::Center)
            .build();

        let icon_name = format!("{}-symbolic", distro_id);
        let icon = gtk::Image::from_icon_name(&icon_name);
        icon.set_pixel_size(96);
        title_box.append(&icon);

        let title_label = gtk::Label::builder()
            .label(&format!("{} Compatibility", distro_name))
            .css_classes(vec!["title-1".to_string()])
            .build();

        title_box.append(&title_label);
        content_box.append(&title_box);

        // Working
        if !compat.working.is_empty() {
            let group = adw::PreferencesGroup::builder()
                .title("Working")
                .build();
            for item in &compat.working {
                let row = adw::ActionRow::builder()
                    .title(item)
                    .build();
                let icon = gtk::Image::from_icon_name("object-select-symbolic");
                icon.add_css_class("success");
                row.add_prefix(&icon);
                group.add(&row);
            }
            content_box.append(&group);
        }

        // Partially Working
        if !compat.partial.is_empty() {
            let group = adw::PreferencesGroup::builder()
                .title("Partially Working")
                .build();
            for item in &compat.partial {
                let row = adw::ActionRow::builder()
                    .title(item)
                    .build();
                let icon = gtk::Image::from_icon_name("dialog-warning-symbolic");
                icon.add_css_class("warning");
                row.add_prefix(&icon);
                group.add(&row);
            }
            content_box.append(&group);
        }

        // Not Working
        if !compat.not_working.is_empty() {
            let group = adw::PreferencesGroup::builder()
                .title("Not Working")
                .build();
            for item in &compat.not_working {
                let row = adw::ActionRow::builder()
                    .title(item)
                    .build();
                let icon = gtk::Image::from_icon_name("process-stop-symbolic");
                icon.add_css_class("error");
                row.add_prefix(&icon);
                group.add(&row);
            }
            content_box.append(&group);
        }

        // Untested
        if !compat.untested.is_empty() {
            let group = adw::PreferencesGroup::builder()
                .title("Untested")
                .build();
            for item in &compat.untested {
                let row = adw::ActionRow::builder()
                    .title(item)
                    .build();
                let icon = gtk::Image::from_icon_name("dialog-question-symbolic");
                icon.add_css_class("dim-label");
                row.add_prefix(&icon);
                group.add(&row);
            }
            content_box.append(&group);
        }

        // Notes
        if !compat.notes.is_empty() {
            let group = adw::PreferencesGroup::builder()
                .title("Notes")
                .build();
            for note in &compat.notes {
                let row = adw::ActionRow::builder()
                    .title(note)
                    .build();
                let icon = gtk::Image::from_icon_name("dialog-information-symbolic");
                row.add_prefix(&icon);
                group.add(&row);
            }
            content_box.append(&group);
        }

        // Continue button
        let button_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .halign(gtk::Align::Center)
            .margin_top(12)
            .margin_bottom(12)
            .build();

        let continue_btn = gtk::Button::builder()
            .label("Continue to Install")
            .css_classes(vec!["suggested-action".to_string(), "pill".to_string()])
            .width_request(250)
            .height_request(50)
            .build();

        let self_clone = self.clone();
        let distro_id_owned = distro_id.to_string();
        continue_btn.connect_clicked(move |_| {
            self_clone.on_install_clicked(&distro_id_owned);
        });

        button_box.append(&continue_btn);
        content_box.append(&button_box);

        clamp.set_child(Some(&content_box));
        scrolled.set_child(Some(&clamp));
        toolbar_view.set_content(Some(&scrolled));

        let page = adw::NavigationPage::builder()
            .title(&format!("{} Compatibility", distro_name))
            .child(&toolbar_view)
            .build();

        nav_view.push(&page);
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
            self.show_safety_page(&nav_view, &device, distro_id);
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

        // "Yes" → show safety disclaimer before proceeding
        let self_clone = self.clone();
        let nav_clone = nav_view.clone();
        let device_clone = device.clone();
        let distro_id_owned = distro_id.to_string();
        yes_btn.connect_clicked(move |_| {
            self_clone.show_safety_page(&nav_clone, &device_clone, &distro_id_owned);
        });

        // "No / I don't know" → trigger unlock flow
        let self_clone2 = self.clone();
        no_btn.connect_clicked(move |_| {
            self_clone2.on_unlock_clicked();
        });

        nav_view.push(&page);
    }

    /// Show the safety disclaimer page before proceeding to install.
    fn show_safety_page(
        &self,
        nav_view: &adw::NavigationView,
        device: &Device,
        distro_id: &str,
    ) {
        let safety_page = SafetyPage::new();
        safety_page.set_device(device);

        let self_clone = self.clone();
        let nav_clone = nav_view.clone();
        let device_clone = device.clone();
        let distro_id_owned = distro_id.to_string();
        safety_page.connect_confirmed(move |_| {
            self_clone.proceed_to_installer(&nav_clone, &device_clone, &distro_id_owned);
        });

        nav_view.push(&safety_page);
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
                .join(maker_to_dir(&device.maker))
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
                let launcher = gtk::UriLauncher::new(&url_string);
                glib::spawn_future_local(async move {
                    if let Err(e) = launcher.launch_future(None::<&gtk::Window>).await {
                        log::warn!("Failed to launch URI: {}", e);
                    }
                });
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
        let config = match self.load_distro_config(device, distro_id) {
            Some(c) => c,
            None => return Vec::new(),
        };

        config.channels
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

        let manufacturer = maker_to_dir(&device.maker);

        let parser = YamlParser::new(devices_path);
        let config = match parser.parse_device_config(&manufacturer, &device.codename) {
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

    /// Connect completion and failure signals on a FlashingPage,
    /// then push it onto the NavigationView.
    fn push_flashing_page(
        &self,
        nav_view: &adw::NavigationView,
        progress_page: &FlashingPage,
    ) {
        // On success: show the success page
        let nav_view_weak = nav_view.downgrade();
        let menu_model = self.imp().main_menu_button.menu_model();
        progress_page.connect_installation_complete(move |page| {
            if let Some(nav) = nav_view_weak.upgrade() {
                if let Some(ref model) = menu_model {
                    let name = page.distro_name();
                    crate::window::SidestepWindow::show_success(&nav, model, &name);
                }
            }
        });

        // On failure: "Start Over" button resets to the waiting page
        progress_page.connect_installation_failed(move |page| {
            if let Some(window) = page.root()
                .and_then(|w| w.downcast::<crate::window::SidestepWindow>().ok())
            {
                window.reset_to_waiting();
            }
        });

        nav_view.push(progress_page);
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
            "ubuntutouch" | "ubports" => self.launch_ubports_install(nav_view, device, channel),
            "droidian" => self.launch_droidian_install(nav_view, device, channel),
            "postmarketos" => self.show_postmarketos_interface_selection(nav_view, device, channel),
            "lineageos" => self.launch_lineageos_install(nav_view, device, channel),
            "eos" => self.launch_eos_install(nav_view, device, channel),
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

        self.push_flashing_page(nav_view, &progress_page);
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

        self.push_flashing_page(nav_view, &progress_page);
    }

    fn launch_lineageos_install(
        &self,
        nav_view: &adw::NavigationView,
        device: &Device,
        channel: &ChannelConfig,
    ) {
        let Some(ref serial) = device.serial else {
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
        if let Some(menu_model) = self.imp().main_menu_button.menu_model() {
            progress_page.set_menu_model(&menu_model);
        }

        progress_page.start_lineageos_installation("LineageOS", serial, release_url, false);

        self.push_flashing_page(nav_view, &progress_page);
    }

    fn launch_eos_install(
        &self,
        nav_view: &adw::NavigationView,
        device: &Device,
        channel: &ChannelConfig,
    ) {
        let Some(ref serial) = device.serial else {
            log::error!("No device serial available for /e/OS installation");
            return;
        };

        let distro_config = match self.load_distro_config(device, "eos") {
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
            device.codename,
            base_url
        );

        let progress_page = FlashingPage::new();
        if let Some(menu_model) = self.imp().main_menu_button.menu_model() {
            progress_page.set_menu_model(&menu_model);
        }

        progress_page.start_eos_installation(serial, &base_url, &device.codename, &channel.id);

        self.push_flashing_page(nav_view, &progress_page);
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

        self.push_flashing_page(nav_view, &progress_page);
    }

    // ────────────────────────────────────────────────────────────────
    // postmarketOS install flow (channel → interface → install)
    // ────────────────────────────────────────────────────────────────

    fn show_postmarketos_interface_selection(
        &self,
        nav_view: &adw::NavigationView,
        device: &Device,
        channel: &ChannelConfig,
    ) {
        let interfaces = self.load_interfaces(device, "postmarketos");
        if interfaces.is_empty() {
            log::error!("No interfaces found for postmarketOS");
            return;
        }

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

        for iface in &interfaces {
            let btn = gtk::Button::builder()
                .label(&iface.label)
                .css_classes(vec!["suggested-action", "pill"])
                .width_request(250)
                .height_request(50)
                .build();

            let self_clone = self.clone();
            let nav_clone = nav_view.clone();
            let device_clone = device.clone();
            let channel_clone = channel.clone();
            let iface_id = iface.id.clone();
            btn.connect_clicked(move |_| {
                self_clone.launch_postmarketos_install(
                    &nav_clone,
                    &device_clone,
                    &channel_clone,
                    &iface_id,
                );
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

    fn launch_postmarketos_install(
        &self,
        nav_view: &adw::NavigationView,
        device: &Device,
        channel: &ChannelConfig,
        interface_id: &str,
    ) {
        let Some(ref serial) = device.serial else {
            log::error!("No device serial available for postmarketOS installation");
            return;
        };

        let distro_config = match self.load_distro_config(device, "postmarketos") {
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

        // Device name on the image server: manufacturer-codename (e.g. google-sargo)
        let device_name = format!("{}-{}", device.maker.to_lowercase(), device.codename);

        log::info!(
            "Installing postmarketOS channel={} interface={} device={}",
            channel.id,
            interface_id,
            device_name
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

        progress_page.start_postmarketos_installation(
            "postmarketOS",
            serial,
            &base_url,
            &channel.id,
            interface_id,
            &device_name,
        );

        self.push_flashing_page(nav_view, &progress_page);
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
            .icon_name("android-symbolic")
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

/// Sanitize a manufacturer name for use as a filesystem directory.
/// Strips characters that aren't alphanumeric, hyphen, or underscore,
/// then lowercases. e.g. "F(x)tec" → "fxtec".
fn maker_to_dir(maker: &str) -> String {
    maker
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>()
        .to_lowercase()
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
