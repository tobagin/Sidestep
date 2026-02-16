// Device Info Page — read-only device information for the browse flow
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::config;
use crate::models::Device;
use crate::models::device_info::DeviceInfo;
use crate::models::distro_config::{CompatibilityInfo, DistroConfig};
use crate::utils::yaml_parser::YamlParser;
use gtk::{glib, prelude::*, subclass::prelude::*};
use libadwaita as adw;
use adw::prelude::*;
use adw::subclass::prelude::*;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct DeviceInfoPage;

    #[glib::object_subclass]
    impl ObjectSubclass for DeviceInfoPage {
        const NAME: &'static str = "DeviceInfoPage";
        type Type = super::DeviceInfoPage;
        type ParentType = adw::NavigationPage;
    }

    impl ObjectImpl for DeviceInfoPage {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for DeviceInfoPage {}
    impl NavigationPageImpl for DeviceInfoPage {}
}

glib::wrapper! {
    pub struct DeviceInfoPage(ObjectSubclass<imp::DeviceInfoPage>)
        @extends gtk::Widget, adw::NavigationPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl DeviceInfoPage {
    pub fn new(device: &Device) -> Self {
        let obj: Self = glib::Object::builder()
            .property("title", &device.name)
            .property("tag", "device_info")
            .build();
        obj.build_ui(device);
        obj
    }

    fn build_ui(&self, device: &Device) {
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

        // Device header
        self.build_header(&content_box, device);

        // Load info.yml for specs/display/connectivity/cameras
        let info = self.load_device_info(device);

        if let Some(ref info) = info {
            self.build_specs_section(&content_box, info);
            self.build_display_section(&content_box, info);
            self.build_connectivity_section(&content_box, info);
            self.build_cameras_section(&content_box, info);
        }

        // Available Distributions
        self.build_distros_section(&content_box, device);

        clamp.set_child(Some(&content_box));
        scrolled.set_child(Some(&clamp));
        toolbar_view.set_content(Some(&scrolled));

        self.set_child(Some(&toolbar_view));
    }

    // ────────────────────────────────────────────────────────────────
    // Header
    // ────────────────────────────────────────────────────────────────

    fn build_header(&self, content_box: &gtk::Box, device: &Device) {
        let header_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .halign(gtk::Align::Center)
            .build();

        let icon = gtk::Image::from_icon_name("phone-symbolic");
        icon.set_pixel_size(96);
        icon.add_css_class("dim-label");
        header_box.append(&icon);

        let name_label = gtk::Label::builder()
            .label(&device.name)
            .css_classes(vec!["title-1".to_string()])
            .build();
        header_box.append(&name_label);

        let subtitle = format!("{} ({})", device.maker, device.codename);
        let subtitle_label = gtk::Label::builder()
            .label(&subtitle)
            .css_classes(vec!["dim-label".to_string()])
            .build();
        header_box.append(&subtitle_label);

        if !device.variants.is_empty() {
            let aka_text = format!("Also known as: {}", device.variants.join(", "));
            let aka_label = gtk::Label::builder()
                .label(&aka_text)
                .css_classes(vec!["dim-label".to_string(), "caption".to_string()])
                .wrap(true)
                .justify(gtk::Justification::Center)
                .build();
            header_box.append(&aka_label);
        }

        if device.experimental {
            let badge = gtk::Label::builder()
                .label("Experimental")
                .css_classes(vec!["caption".to_string(), "warning".to_string()])
                .build();
            header_box.append(&badge);
        }

        content_box.append(&header_box);
    }

    // ────────────────────────────────────────────────────────────────
    // Specifications
    // ────────────────────────────────────────────────────────────────

    fn build_specs_section(&self, content_box: &gtk::Box, info: &DeviceInfo) {
        let specs = &info.device.specs;

        let group = adw::PreferencesGroup::builder()
            .title("Specifications")
            .build();

        group.add(&make_info_row("SoC", &specs.soc));
        group.add(&make_info_row("CPU", &specs.cpu));
        group.add(&make_info_row("GPU", &specs.gpu));
        group.add(&make_info_row("RAM", &specs.ram));
        group.add(&make_info_row("Storage", &specs.storage));
        group.add(&make_info_row("Battery", &specs.battery));

        if let Some(ref arch) = specs.arch {
            group.add(&make_info_row("Architecture", arch));
        }

        content_box.append(&group);
    }

    // ────────────────────────────────────────────────────────────────
    // Display
    // ────────────────────────────────────────────────────────────────

    fn build_display_section(&self, content_box: &gtk::Box, info: &DeviceInfo) {
        let display = &info.device.display;

        let group = adw::PreferencesGroup::builder()
            .title("Display")
            .build();

        group.add(&make_info_row("Size", &display.size));
        group.add(&make_info_row("Resolution", &display.resolution));
        group.add(&make_info_row("Panel Type", &display.panel_type));
        group.add(&make_info_row("Density", &display.density));
        group.add(&make_info_row("Refresh Rate", &display.refresh_rate));

        content_box.append(&group);
    }

    // ────────────────────────────────────────────────────────────────
    // Connectivity
    // ────────────────────────────────────────────────────────────────

    fn build_connectivity_section(&self, content_box: &gtk::Box, info: &DeviceInfo) {
        let conn = &info.device.connectivity;

        let group = adw::PreferencesGroup::builder()
            .title("Connectivity")
            .build();

        group.add(&make_info_row("Network", &conn.network.join(", ")));
        group.add(&make_info_row("Bluetooth", &conn.bluetooth));
        group.add(&make_info_row("WiFi", &conn.wifi));
        group.add(&make_info_row("Peripherals", &conn.peripherals.join(", ")));
        group.add(&make_info_row("Sensors", &conn.sensors.join(", ")));
        group.add(&make_info_row("Location", &conn.location.join(", ")));

        content_box.append(&group);
    }

    // ────────────────────────────────────────────────────────────────
    // Cameras
    // ────────────────────────────────────────────────────────────────

    fn build_cameras_section(&self, content_box: &gtk::Box, info: &DeviceInfo) {
        if info.device.cameras.is_empty() {
            return;
        }

        let group = adw::PreferencesGroup::builder()
            .title("Cameras")
            .build();

        for cam in &info.device.cameras {
            let row = adw::ActionRow::builder()
                .title(&cam.label)
                .subtitle(&cam.features)
                .build();
            let res_label = gtk::Label::builder()
                .label(&cam.resolution)
                .css_classes(vec!["dim-label".to_string()])
                .build();
            row.add_suffix(&res_label);

            let icon = gtk::Image::from_icon_name("camera-photo-symbolic");
            row.add_prefix(&icon);

            group.add(&row);
        }

        content_box.append(&group);
    }

    // ────────────────────────────────────────────────────────────────
    // Available Distributions
    // ────────────────────────────────────────────────────────────────

    fn build_distros_section(&self, content_box: &gtk::Box, device: &Device) {
        let distros = self.load_all_distros(device);
        if distros.is_empty() {
            return;
        }

        let group = adw::PreferencesGroup::builder()
            .title("Available Distributions")
            .build();

        for distro in &distros {
            let row = adw::ActionRow::builder()
                .title(&distro.name)
                .subtitle(&distro.developer)
                .activatable(true)
                .build();

            let icon_name = format!("{}-symbolic", distro.id);
            let icon = gtk::Image::from_icon_name(&icon_name);
            icon.set_pixel_size(32);
            row.add_prefix(&icon);

            let chevron = gtk::Image::from_icon_name("go-next-symbolic");
            row.add_suffix(&chevron);

            let page_ref = self.clone();
            let distro_clone = distro.clone();
            let device_clone = device.clone();
            row.connect_activated(move |_| {
                page_ref.show_distro_detail(&device_clone, &distro_clone);
            });

            group.add(&row);
        }

        content_box.append(&group);
    }

    // ────────────────────────────────────────────────────────────────
    // Distro detail sub-page
    // ────────────────────────────────────────────────────────────────

    fn show_distro_detail(&self, device: &Device, distro: &DistroConfig) {
        let Some(nav_view) = self.ancestor(adw::NavigationView::static_type())
            .and_then(|w| w.downcast::<adw::NavigationView>().ok())
        else {
            return;
        };

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

        // Distro header
        let header_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .halign(gtk::Align::Center)
            .build();

        let icon_name = format!("{}-symbolic", distro.id);
        let icon = gtk::Image::from_icon_name(&icon_name);
        icon.set_pixel_size(96);
        header_box.append(&icon);

        let title_label = gtk::Label::builder()
            .label(&distro.name)
            .css_classes(vec!["title-1".to_string()])
            .build();
        header_box.append(&title_label);

        let dev_label = gtk::Label::builder()
            .label(&format!("by {}", distro.developer))
            .css_classes(vec!["dim-label".to_string()])
            .build();
        header_box.append(&dev_label);

        content_box.append(&header_box);

        // Details group
        let details_group = adw::PreferencesGroup::builder()
            .title("Details")
            .build();

        details_group.add(&make_info_row("Flash Method", &distro.flash_method));

        if let Some(ref fw) = distro.firmware_requirement {
            details_group.add(&make_info_row("Firmware Requirement", fw));
        }

        content_box.append(&details_group);

        // Channels
        if !distro.channels.is_empty() {
            let channels_group = adw::PreferencesGroup::builder()
                .title("Available Channels")
                .build();

            for channel in &distro.channels {
                let row = adw::ActionRow::builder()
                    .title(&channel.label)
                    .build();
                let icon = gtk::Image::from_icon_name("emblem-system-symbolic");
                row.add_prefix(&icon);
                channels_group.add(&row);
            }

            content_box.append(&channels_group);
        }

        // Interfaces
        if let Some(ref interfaces) = distro.interfaces {
            if !interfaces.is_empty() {
                let iface_group = adw::PreferencesGroup::builder()
                    .title("Available Interfaces")
                    .build();

                for iface in interfaces {
                    let row = adw::ActionRow::builder()
                        .title(&iface.label)
                        .build();
                    let icon = gtk::Image::from_icon_name("desktop-symbolic");
                    row.add_prefix(&icon);
                    iface_group.add(&row);
                }

                content_box.append(&iface_group);
            }
        }

        // Compatibility
        if let Some(ref compat) = distro.compatibility {
            self.build_compatibility_section(&content_box, compat);
        }

        // Install hint
        let hint_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .halign(gtk::Align::Center)
            .margin_top(12)
            .margin_bottom(12)
            .build();

        let hint_label = gtk::Label::builder()
            .label(&format!(
                "Connect your {} via USB to install {}",
                device.name, distro.name
            ))
            .css_classes(vec!["dim-label".to_string()])
            .wrap(true)
            .justify(gtk::Justification::Center)
            .build();
        hint_box.append(&hint_label);
        content_box.append(&hint_box);

        clamp.set_child(Some(&content_box));
        scrolled.set_child(Some(&clamp));
        toolbar_view.set_content(Some(&scrolled));

        let page = adw::NavigationPage::builder()
            .title(&distro.name)
            .child(&toolbar_view)
            .build();

        nav_view.push(&page);
    }

    // ────────────────────────────────────────────────────────────────
    // Compatibility section (reuses icon+color pattern)
    // ────────────────────────────────────────────────────────────────

    fn build_compatibility_section(&self, content_box: &gtk::Box, compat: &CompatibilityInfo) {
        if compat.working.is_empty()
            && compat.partial.is_empty()
            && compat.not_working.is_empty()
            && compat.untested.is_empty()
            && compat.notes.is_empty()
        {
            return;
        }

        // Working
        if !compat.working.is_empty() {
            let group = adw::PreferencesGroup::builder()
                .title("Working")
                .build();
            for item in &compat.working {
                let row = adw::ActionRow::builder().title(item).build();
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
                let row = adw::ActionRow::builder().title(item).build();
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
                let row = adw::ActionRow::builder().title(item).build();
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
                let row = adw::ActionRow::builder().title(item).build();
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
                let row = adw::ActionRow::builder().title(note).build();
                let icon = gtk::Image::from_icon_name("dialog-information-symbolic");
                row.add_prefix(&icon);
                group.add(&row);
            }
            content_box.append(&group);
        }
    }

    // ────────────────────────────────────────────────────────────────
    // Data loading (same path-probing as DeviceDetailsPage)
    // ────────────────────────────────────────────────────────────────

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
                log::debug!("No distros.yml for {}: {:#}", device.codename, e);
                Vec::new()
            }
        }
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

fn make_info_row(title: &str, value: &str) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(title)
        .build();
    let label = gtk::Label::builder()
        .label(value)
        .css_classes(vec!["dim-label".to_string()])
        .wrap(true)
        .xalign(1.0)
        .build();
    row.add_suffix(&label);
    row
}
