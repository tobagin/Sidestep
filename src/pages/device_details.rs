// Device Details Page
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::models::Device;
use crate::models::DeviceDatabase;
use crate::models::device_info::DeviceInfo;
use crate::models::distro_config::CompatibilityInfo;
use crate::pages::install_flow::InstallFlow;
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

        // Only show rows we actually have a value for — community devices may
        // carry partial specs.
        let mut push = |title: &str, value: &str| {
            if !value.trim().is_empty() {
                rows.push(self.make_action_row(title, value));
            }
        };
        push("SoC", &specs.soc);
        push("CPU", &specs.cpu);
        push("GPU", &specs.gpu);
        push("RAM", &specs.ram);
        push("Storage", &specs.storage);
        push("Battery", &specs.battery);
        let disp = format!("{} {} {}", display.size, display.panel_type, display.resolution)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        push("Display", &disp);

        for row in &rows {
            imp.specs_group.add(row);
        }
        let any = !rows.is_empty();
        *imp.specs_rows.borrow_mut() = rows;
        imp.specs_group.set_visible(any);
    }

    fn load_device_info(&self, device: &Device) -> Option<DeviceInfo> {
        let devices_dir = DeviceDatabase::bundled_devices_dir();
        let manufacturer = DeviceDatabase::maker_to_dir(&device.maker);
        let parser = YamlParser::new(&devices_dir);
        match parser.parse_device_info(&manufacturer, &device.codename) {
            Ok(info) => Some(info),
            Err(_) => {
                log::debug!("No info.yml found for {}/{}", device.maker, device.codename);
                None
            }
        }
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

        let db = DeviceDatabase::new();
        let distros = db.get_distro_configs(&device);
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

        let menu_model = self.imp().main_menu_button.menu_model();

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
            let menu_clone = menu_model.clone();
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
                            menu_clone.as_ref(),
                        );
                        return;
                    }
                }
                // No compatibility data — proceed directly
                let flow = InstallFlow::new(
                    nav_clone.clone(),
                    menu_clone.clone(),
                    device_clone.clone(),
                    DeviceDatabase::new(),
                    {
                        let s = self_clone.clone();
                        move || s.on_unlock_clicked()
                    },
                );
                flow.start(&distro_clone.id);
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

    // ────────────────────────────────────────────────────────────────
    // Compatibility page
    // ────────────────────────────────────────────────────────────────

    fn show_compatibility_page(
        &self,
        nav_view: &adw::NavigationView,
        device: &Device,
        distro_id: &str,
        distro_name: &str,
        compat: &CompatibilityInfo,
        menu_model: Option<&gio::MenuModel>,
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
        let nav_clone = nav_view.clone();
        let device_clone = device.clone();
        let distro_id_owned = distro_id.to_string();
        let menu_clone = menu_model.cloned();
        continue_btn.connect_clicked(move |_| {
            let flow = InstallFlow::new(
                nav_clone.clone(),
                menu_clone.clone(),
                device_clone.clone(),
                DeviceDatabase::new(),
                {
                    let s = self_clone.clone();
                    move || s.on_unlock_clicked()
                },
            );
            flow.start(&distro_id_owned);
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
