use crate::config;
use crate::hardware::{DeviceDetector, DeviceEvent};
use crate::models::Device;
use crate::pages::distro_selection::DistroSelectionPage;
use crate::pages::device_details::DeviceDetailsPage;
use crate::pages::success::SuccessPage;
use crate::pages::waiting::WaitingPage;
use crate::pages::flashing::FlashingPage;
use crate::pages::unsupported_device::UnsupportedDevicePage;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use libadwaita as adw;
use adw::subclass::prelude::*;
use std::cell::{Cell, RefCell};
use std::sync::mpsc::Receiver;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/tobagin/Sidestep/ui/window.ui")]
    pub struct SidestepWindow {
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub main_nav: TemplateChild<adw::NavigationView>,

        pub settings: once_cell::sync::OnceCell<gio::Settings>,
        pub device_detector: RefCell<Option<DeviceDetector>>,
        pub current_device: RefCell<Option<Device>>,
        pub device_serial: RefCell<Option<String>>,
        pub connected_devices: RefCell<Vec<Device>>,
        pub waiting_page: RefCell<Option<WaitingPage>>,
        pub terminal_visible: Cell<bool>,
        pub installing: Cell<bool>,

        #[template_child]
        pub primary_menu: TemplateChild<gio::MenuModel>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SidestepWindow {
        const NAME: &'static str = "SidestepWindow";
        type Type = super::SidestepWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl SidestepWindow {
    }

    impl ObjectImpl for SidestepWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            // Load settings
            let settings = gio::Settings::new(config::APP_ID);
            self.settings.set(settings.clone()).unwrap();

            // Restore window geometry
            obj.set_default_size(
                settings.int("window-width"),
                settings.int("window-height"),
            );
            if settings.boolean("window-maximized") {
                obj.maximize();
            }

            // Restore terminal visibility
            let show_terminal = settings.boolean("show-terminal");
            self.terminal_visible.set(show_terminal);

            // Setup actions
            obj.setup_actions();

            // Initialize Navigation
            let waiting_page = WaitingPage::new();
            self.main_nav.push(&waiting_page);
            *self.waiting_page.borrow_mut() = Some(waiting_page.clone());

            // Connect device-selected signal
            let window_weak = obj.downgrade();
            waiting_page.connect_closure(
                "device-selected",
                false,
                glib::closure_local!(move |page: WaitingPage, index: u32| {
                    let Some(window) = window_weak.upgrade() else { return };
                    if let Some(device) = page.get_device(index) {
                        window.start_wizard(&device);
                    }
                }),
            );

            // Start device detection
            obj.start_device_detection();
        }
    }

    impl WidgetImpl for SidestepWindow {}
    impl WindowImpl for SidestepWindow {
        fn close_request(&self) -> glib::Propagation {
            // Save window state
            let obj = self.obj();
            if let Some(settings) = self.settings.get() {
                let (width, height) = obj.default_size();
                settings.set_int("window-width", width).ok();
                settings.set_int("window-height", height).ok();
                settings.set_boolean("window-maximized", obj.is_maximized()).ok();
                settings.set_boolean("show-terminal", self.terminal_visible.get()).ok();
            }

            // Stop device detection
            if let Some(detector) = self.device_detector.borrow_mut().take() {
                detector.stop();
            }

            glib::Propagation::Proceed
        }
    }
    impl ApplicationWindowImpl for SidestepWindow {}
    impl AdwApplicationWindowImpl for SidestepWindow {}
}

glib::wrapper! {
    pub struct SidestepWindow(ObjectSubclass<imp::SidestepWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl SidestepWindow {
    pub fn new<A: IsA<gtk::Application>>(application: &A) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }

    fn setup_actions(&self) {
        let _action_group = gio::SimpleActionGroup::new();

        // Toggle terminal action
        let toggle_terminal = gio::SimpleAction::new_stateful(
            "show-terminal",
            None,
            &false.into()
        );
        
        let imp = self.imp();
        toggle_terminal.set_state(&imp.terminal_visible.get().into());

        let window_weak = self.downgrade();
        toggle_terminal.connect_activate(move |action, _| {
            let Some(window) = window_weak.upgrade() else { return; };
            let state = action.state().unwrap().get::<bool>().unwrap();
            action.set_state(&(!state).into());
            
            let imp = window.imp();
            imp.terminal_visible.set(!state);
            log::debug!("Terminal visibility: {}", !state);
            // TODO: Show/hide terminal overlay
        });
        
        // Register window actions
        self.add_action(&toggle_terminal);
    }

    fn start_device_detection(&self) {
        log::info!("Starting device detection");
        let imp = self.imp();

        // Create device detector
        let (detector, receiver) = DeviceDetector::new();

        // Start detection
        detector.start();
        *imp.device_detector.borrow_mut() = Some(detector);

        // Set up polling for events using glib timeout
        self.setup_event_polling(receiver);
    }

    fn setup_event_polling(&self, receiver: Receiver<DeviceEvent>) {
        // Poll for events every 100ms on the main thread
        let window = self.downgrade();
        
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            let Some(window) = window.upgrade() else {
                return glib::ControlFlow::Break;
            };

            // Try to receive all pending events
            while let Ok(event) = receiver.try_recv() {
                window.handle_device_event(event);
            }

            glib::ControlFlow::Continue
        });
    }

    fn handle_device_event(&self, event: DeviceEvent) {
        if self.imp().installing.get() {
            log::info!("Ignoring device event during installation: {:?}", event);
            return;
        }

        match event {
            DeviceEvent::Connected(device) => {
                self.on_device_detected(device);
            }
            DeviceEvent::Disconnected => {
                self.on_device_disconnected();
            }
        }
    }

    fn on_device_detected(&self, device: Device) {
        log::info!("Device detected: {} ({})", device.name, device.codename);
        let imp = self.imp();

        // Check if device is supported
        let supported_codenames = ["sargo", "enchilada", "zeta"]; // TODO: externalize this
        if !supported_codenames.contains(&device.codename.as_str()) {
             log::warn!("Unsupported device detected: {}", device.codename);
             self.show_unsupported_device();
             return;
        }

        // Store current device and serial
        *imp.device_serial.borrow_mut() = device.serial.clone();
        *imp.current_device.borrow_mut() = Some(device.clone());

        // Track connected device and update WaitingPage
        {
            let mut connected = imp.connected_devices.borrow_mut();
            if !connected.iter().any(|d| d.codename == device.codename) {
                connected.push(device.clone());
            }
        }
        self.update_waiting_page();

        // Start wizard flow
        self.start_wizard(&device);
    }

    fn show_unsupported_device(&self) {
        let imp = self.imp();
        let page = UnsupportedDevicePage::new();
        
        let nav_view = imp.main_nav.clone();
        let nav_view_weak = nav_view.downgrade();
        
        page.connect_back_clicked(move |_| {
            if let Some(nav) = nav_view_weak.upgrade() {
                nav.pop_to_tag("waiting");
            }
        });
        
        nav_view.push(&page);
    }

    fn on_device_disconnected(&self) {
        log::info!("Device disconnected");
        self.reset_to_waiting();
    }

    fn reset_to_waiting(&self) {
        let imp = self.imp();

        // Clear installing flag and current device, serial, and connected devices
        imp.installing.set(false);
        *imp.current_device.borrow_mut() = None;
        *imp.device_serial.borrow_mut() = None;
        imp.connected_devices.borrow_mut().clear();
        self.update_waiting_page();

        // Pop to root
        imp.main_nav.pop_to_tag("waiting");

        log::info!("Reset to waiting state");
    }

    fn update_waiting_page(&self) {
        let imp = self.imp();
        if let Some(ref waiting_page) = *imp.waiting_page.borrow() {
            let devices = imp.connected_devices.borrow();
            waiting_page.set_devices(&devices);
        }
    }

    fn start_wizard(&self, device: &Device) {
        log::info!("Starting wizard flow for device: {}", device.codename);
        let imp = self.imp();

        // Create Hub Page
        let details_page = DeviceDetailsPage::new(device);
        
        let menu_model = imp.primary_menu.clone();
        details_page.set_menu_model(&menu_model);
        
        let nav_view = imp.main_nav.clone();
        let device_install = device.clone();
        let menu_model_clone = menu_model.clone();
        
        let nav_view_weak = nav_view.downgrade();
        let window_weak = self.downgrade();

        let device_serial = imp.device_serial.borrow().clone();
        details_page.connect_install_clicked(move |_| {
            if let Some(nav_view) = nav_view_weak.upgrade() {
                Self::navigate_to_distro_selection(&nav_view, &device_install, &menu_model_clone, device_serial.as_deref(), window_weak.clone());
            }
        });

        // Connect Unlock Action
        details_page.connect_unlock_clicked(move |_| {
            log::info!("Unlock clicked - unlocking page not implemented yet");
        });

        // Push to main nav
        nav_view.push(&details_page);
    }

    fn navigate_to_distro_selection(nav_view: &adw::NavigationView, device: &Device, menu_model: &gio::MenuModel, serial: Option<&str>, window_weak: glib::WeakRef<SidestepWindow>) {
        let page = DistroSelectionPage::new();
        page.set_menu_model(menu_model);
        let nav_view = nav_view.clone(); // Clone for closure capture
        
        // Load distros logic...
        let possible_paths = vec![
            std::path::PathBuf::from(config::PKGDATADIR).join("devices"),
            std::path::PathBuf::from("/app/share/sidestep/devices"),
            std::path::PathBuf::from("devices"),
        ];

        let mut devices_path = None;
        for path in possible_paths {
            if path.exists() {
                devices_path = Some(path);
                break;
            }
        }
        let devices_path = devices_path.unwrap_or_else(|| std::path::PathBuf::from("devices"));

        let manufacturer = match device.codename.as_str() {
            "sargo" => "google",
            "enchilada" => "oneplus",
            "zeta" => "microsoft",
            _ => "unknown", 
        };

        let parser = crate::utils::yaml_parser::YamlParser::new(devices_path);
        match parser.parse_distros(manufacturer, &device.codename) {
            Ok(distros) => {
                 page.set_distros(distros);
            }
            Err(e) => {
                log::error!("Failed to load distros: {:#}", e);
            }
        }

        // Connect selection to install
        let nav_view_clone = nav_view.clone();
        let menu_model_clone = menu_model.clone();
        let serial_owned = serial.map(|s| s.to_string());
        let device_codename = device.codename.clone();
        let window_weak_distro = window_weak.clone();
        page.connect_closure(
            "distro-selected",
            false,
            glib::closure_local!(move |_: DistroSelectionPage, name: &str| {
                // Set installing flag on the window to suppress device events during install
                if let Some(window) = window_weak_distro.upgrade() {
                    log::info!("Setting installing flag — device events will be suppressed");
                    window.imp().installing.set(true);
                } else {
                    log::warn!("Could not upgrade window weak ref to set installing flag");
                }
                log::info!("Selected distro: {}", name);
                let progress_page = FlashingPage::new();
                progress_page.set_menu_model(&menu_model_clone);

                // Detect Ubuntu Touch and use the UBports installer
                if name == "Ubuntu Touch" {
                    if let Some(ref serial) = serial_owned {
                        let channel_path = match device_codename.as_str() {
                            "sargo" => "24.04-2.x/arm64/android9plus/daily/sargo",
                            _ => "24.04-2.x/arm64/android9plus/daily/sargo",
                        };
                        progress_page.start_ubports_installation(name, serial, channel_path);
                    } else {
                        log::error!("No device serial available for UBports installation");
                        progress_page.start_installation(name);
                    }
                } else {
                    progress_page.start_installation(name);
                }

                // Connect progress completion
                let nav_view_weak = nav_view_clone.downgrade();
                let menu_model_success = menu_model_clone.clone();
                let window_weak_complete = window_weak_distro.clone();
                progress_page.connect_closure(
                    "installation-complete",
                    false,
                    glib::closure_local!(move |_: FlashingPage| {
                         // Clear installing flag
                         if let Some(window) = window_weak_complete.upgrade() {
                             log::info!("Clearing installing flag — device events will resume");
                             window.imp().installing.set(false);
                         }
                         if let Some(nav) = nav_view_weak.upgrade() {
                             Self::show_success(&nav, &menu_model_success);
                         }
                    })
                );

                nav_view_clone.push(&progress_page);
            }),
        );

        nav_view.push(&page);
    }

    pub fn show_success(nav_view: &adw::NavigationView, menu_model: &gio::MenuModel) {
        let success_page = SuccessPage::new();
        success_page.set_menu_model(menu_model);
        let nav_view_weak = nav_view.downgrade();
        
        success_page.connect_restart_clicked(move |_| {
            if let Some(nav) = nav_view_weak.upgrade() {
                 nav.pop_to_tag("waiting");
            }
        });

        nav_view.push(&success_page);
    }

    pub fn show_toast(&self, message: &str) {
        let imp = self.imp();
        let toast = adw::Toast::new(message);
        imp.toast_overlay.add_toast(toast);
    }

    pub fn append_terminal_log(&self, line: &str) {
        // TODO: Append to terminal overlay
        log::debug!("[terminal] {}", line);
    }
}
