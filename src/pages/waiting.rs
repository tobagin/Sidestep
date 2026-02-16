// Waiting Page
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::models::Device;
use gtk::{glib, prelude::*, subclass::prelude::*};
use libadwaita as adw;
use adw::prelude::*;
use adw::subclass::prelude::*;
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/tobagin/Sidestep/ui/pages/waiting.ui")]
    pub struct WaitingPage {
        #[template_child]
        pub waiting_spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub device_list: TemplateChild<gtk::Box>,

        pub devices: RefCell<Vec<Device>>,
        pub device_buttons: RefCell<Vec<gtk::Button>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WaitingPage {
        const NAME: &'static str = "WaitingPage";
        type Type = super::WaitingPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("waiting.select-device", Some(glib::VariantTy::UINT32), |page, _, param| {
                if let Some(index) = param.and_then(|v| v.get::<u32>()) {
                    let imp = page.imp();
                    let devices = imp.devices.borrow();
                    if let Some(device) = devices.get(index as usize) {
                        page.emit_by_name::<()>("device-selected", &[&(index as u32)]);
                        let _ = device;
                    }
                }
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for WaitingPage {
        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: std::sync::OnceLock<Vec<glib::subclass::Signal>> = std::sync::OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    glib::subclass::Signal::builder("device-selected")
                        .param_types([u32::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for WaitingPage {}
    impl NavigationPageImpl for WaitingPage {}
}

glib::wrapper! {
    pub struct WaitingPage(ObjectSubclass<imp::WaitingPage>)
        @extends gtk::Widget, adw::NavigationPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl WaitingPage {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn set_spinning(&self, spinning: bool) {
        self.imp().waiting_spinner.set_spinning(spinning);
    }

    pub fn set_devices(&self, devices: &[Device]) {
        let imp = self.imp();

        // Store devices
        *imp.devices.borrow_mut() = devices.to_vec();

        if devices.is_empty() {
            imp.stack.set_visible_child_name("connecting");
            imp.waiting_spinner.set_spinning(true);
            return;
        }

        // Clear existing buttons
        for btn in imp.device_buttons.borrow().iter() {
            imp.device_list.remove(btn);
        }
        imp.device_buttons.borrow_mut().clear();

        // Add a button per device
        for (i, device) in devices.iter().enumerate() {
            let btn = gtk::Button::builder()
                .label(&format!("{}\n{} \u{2022} {}", device.name, device.maker, device.codename))
                .action_name("waiting.select-device")
                .action_target(&(i as u32).to_variant())
                .width_request(250)
                .height_request(50)
                .build();
            btn.add_css_class("suggested-action");
            btn.add_css_class("pill");

            imp.device_list.append(&btn);
            imp.device_buttons.borrow_mut().push(btn);
        }

        imp.stack.set_visible_child_name("connected");
        imp.waiting_spinner.set_spinning(false);
    }

    pub fn get_device(&self, index: u32) -> Option<Device> {
        let devices = self.imp().devices.borrow();
        devices.get(index as usize).cloned()
    }
}
