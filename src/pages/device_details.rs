// Device Details Page
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::config;
use crate::models::Device;
use crate::models::distro_config::ChannelConfig;
use crate::pages::flashing::FlashingPage;
use crate::utils::yaml_parser::YamlParser;
use crate::wizards::install_wizard::InstallWizard;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use libadwaita as adw;
use adw::prelude::*;
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
        let is_locked = device.is_locked.unwrap_or(true); // Default to locked/unsafe

        if is_locked {
            // Locked (or unknown) -> Show Unlock, Hide Distros
            imp.unlock_button.set_visible(true);
            imp.distro_buttons_box.set_visible(false);
        } else {
            // Unlocked -> Hide Unlock, Show Distros
            imp.unlock_button.set_visible(false);
            imp.distro_buttons_box.set_visible(true);
        }
        
        // Load info from info.yml
        self.load_device_info(device);
    }

    pub fn set_menu_model(&self, model: &gio::MenuModel) {
        self.imp().main_menu_button.set_menu_model(Some(model));
    }

    fn load_device_info(&self, _device: &Device) {
        // Device info loading logic removed as per request
    }

    fn on_install_clicked(&self, distro_id: &str) {
        if let Some(device) = self.imp().device.borrow().as_ref() {
            // Route Ubuntu Touch through the UBports installer
            if distro_id == "ubuntutouch" {
                self.start_ubports_install(device);
                return;
            }

            if let Some(root) = self.root().and_then(|w| w.downcast::<gtk::Window>().ok()) {
                let wizard = InstallWizard::new(&root, device, distro_id);
                wizard.present();
            }
        }
    }

    /// Look up UBports channels from distros.yml for this device.
    fn load_ubports_channels(&self, device: &Device) -> Vec<ChannelConfig> {
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

        config
            .available_distros
            .iter()
            .find(|d| d.id == "ubports")
            .map(|d| d.channels.clone())
            .unwrap_or_default()
    }

    fn start_ubports_install(&self, device: &Device) {
        let channels = self.load_ubports_channels(device);
        if channels.is_empty() {
            log::error!("No UBports channels found in distros.yml");
            return;
        }

        // If only one channel, use it directly. Otherwise show a picker.
        if channels.len() == 1 {
            self.launch_ubports_install(device, &channels[0]);
        } else {
            self.show_channel_picker(device, &channels);
        }
    }

    fn show_channel_picker(&self, device: &Device, channels: &[ChannelConfig]) {
        let dialog = adw::AlertDialog::builder()
            .heading("Select Channel")
            .body("Choose which Ubuntu Touch channel to install:")
            .build();

        for channel in channels {
            dialog.add_response(&channel.id, &channel.label);
            dialog.set_response_appearance(
                &channel.id,
                adw::ResponseAppearance::Suggested,
            );
        }
        dialog.add_response("cancel", "Cancel");

        let page = self.clone();
        let device = device.clone();
        let channels: Vec<ChannelConfig> = channels.to_vec();
        dialog.connect_response(None, move |_, response| {
            if let Some(channel) = channels.iter().find(|c| c.id == response) {
                page.launch_ubports_install(&device, channel);
            }
        });

        dialog.present(Some(self));
    }

    fn launch_ubports_install(&self, device: &Device, channel: &ChannelConfig) {
        let Some(nav_view) = self.ancestor(adw::NavigationView::static_type())
            .and_then(|w| w.downcast::<adw::NavigationView>().ok())
        else {
            log::error!("Could not find ancestor NavigationView for UBports install");
            return;
        };

        let Some(ref serial) = device.serial else {
            log::error!("No device serial available for UBports installation");
            return;
        };

        let Some(ref channel_path) = channel.path else {
            log::error!("No channel path defined for channel {}", channel.id);
            return;
        };
        // Strip trailing slash if present
        let channel_path = channel_path.trim_end_matches('/');

        log::info!("Installing Ubuntu Touch from channel: {} ({})", channel.label, channel_path);

        let progress_page = FlashingPage::new();
        if let Some(menu_model) = self.imp().main_menu_button.menu_model() {
            progress_page.set_menu_model(&menu_model);
        }

        progress_page.start_ubports_installation("Ubuntu Touch", serial, channel_path);

        let nav_view_weak = nav_view.downgrade();
        let menu_model = self.imp().main_menu_button.menu_model();
        progress_page.connect_installation_complete(move |_| {
            if let Some(nav) = nav_view_weak.upgrade() {
                if let Some(ref model) = menu_model {
                    crate::window::SidestepWindow::show_success(&nav, model);
                }
            }
        });

        nav_view.push(&progress_page);
    }

    fn on_unlock_clicked(&self) {
        // Emit signal to parent to navigate
        self.emit_by_name::<()>("unlock-clicked", &[]);
    }
}

impl Default for DeviceDetailsPage {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

// Signal definitions
impl DeviceDetailsPage {
    pub fn connect_install_clicked<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_closure(
            "install-clicked",
            false,
            glib::closure_local!(move |obj: Self| {
                f(&obj);
            }),
        )
    }

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

// Add signals to ObjectImpl
impl ObjectImpl for imp::DeviceDetailsPage {
    fn constructed(&self) {
        self.parent_constructed();
    }
    
    fn signals() -> &'static [glib::subclass::Signal] {
        static SIGNALS: once_cell::sync::Lazy<Vec<glib::subclass::Signal>> =
            once_cell::sync::Lazy::new(|| {
                vec![
                    glib::subclass::Signal::builder("install-clicked").build(),
                    glib::subclass::Signal::builder("unlock-clicked").build(),
                ]
            });
        &SIGNALS
    }
}
