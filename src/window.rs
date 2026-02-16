use crate::config;
use crate::hardware::{DeviceDetector, DeviceEvent};
use crate::models::{Device, DeviceDatabase};
use crate::pages::device_browser::DeviceBrowserPage;
use crate::pages::device_details::DeviceDetailsPage;
use crate::pages::success::SuccessPage;
use crate::pages::waiting::WaitingPage;
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
                        let db = DeviceDatabase::new();
                        let supported = db.find_by_codename(&device.codename).is_some();
                        window.start_wizard(&device, supported);
                    }
                }),
            );

            // Connect browse-requested signal
            let window_weak = obj.downgrade();
            waiting_page.connect_browse_requested(move |_| {
                if let Some(window) = window_weak.upgrade() {
                    window.show_device_browser();
                }
            });

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

    pub fn set_installing(&self, installing: bool) {
        self.imp().installing.set(installing);
    }

    /// Pause device detection entirely (no events will be sent).
    /// Also sets the installing flag to prevent any queued events from being handled.
    pub fn pause_detection(&self) {
        self.imp().installing.set(true);
        if let Some(ref detector) = *self.imp().device_detector.borrow() {
            detector.pause();
        }
    }

    /// Resume device detection and clear the installing flag.
    pub fn resume_detection(&self) {
        self.imp().installing.set(false);
        if let Some(ref detector) = *self.imp().device_detector.borrow() {
            detector.resume();
        }
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

        // Device is now shown on WaitingPage — user clicks to start wizard
    }

    fn on_device_disconnected(&self) {
        log::info!("Device disconnected");
        self.reset_to_waiting();
    }

    pub fn reset_to_waiting(&self) {
        let imp = self.imp();

        // Resume detection (safety net in case it was paused)
        self.resume_detection();
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

    fn start_wizard(&self, device: &Device, supported: bool) {
        log::info!("Starting wizard flow for device: {} (supported: {})", device.codename, supported);
        let imp = self.imp();

        let details_page = DeviceDetailsPage::new(device, supported);
        details_page.set_menu_model(&imp.primary_menu);

        details_page.connect_unlock_clicked(move |_| {
            log::info!("Unlock clicked - unlocking page not implemented yet");
        });

        imp.main_nav.push(&details_page);
    }

    pub fn show_success(nav_view: &adw::NavigationView, menu_model: &gio::MenuModel) {
        let success_page = SuccessPage::new();
        success_page.set_menu_model(menu_model);

        success_page.connect_restart_clicked(move |page| {
            // Reset fully: resume detection, clear stale devices, pop to waiting
            if let Some(window) = page.root()
                .and_then(|w| w.downcast::<SidestepWindow>().ok())
            {
                window.reset_to_waiting();
            }
        });

        nav_view.push(&success_page);
    }

    fn show_device_browser(&self) {
        let imp = self.imp();

        let browser_page = DeviceBrowserPage::new();

        let window_weak = self.downgrade();
        browser_page.connect_device_selected(move |_, codename| {
            let Some(window) = window_weak.upgrade() else { return };
            let db = DeviceDatabase::new();
            if let Some(device) = db.find_by_codename(&codename) {
                window.start_wizard(&device, true);
            }
        });

        imp.main_nav.push(&browser_page);
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
