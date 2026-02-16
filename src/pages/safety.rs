// Safety Page
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::models::Device;
use gtk::{glib, prelude::*, subclass::prelude::*};
use libadwaita as adw;
use adw::prelude::*;
use adw::subclass::prelude::*;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/tobagin/Sidestep/ui/pages/safety.ui")]
    pub struct SafetyPage {
        #[template_child]
        pub warning_banner: TemplateChild<adw::Banner>,
        #[template_child]
        pub device_name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub backup_check: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub warranty_check: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub risk_check: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub continue_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SafetyPage {
        const NAME: &'static str = "SafetyPage";
        type Type = super::SafetyPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SafetyPage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            // Wire checkboxes to update continue button sensitivity
            let obj_weak = obj.downgrade();
            let update_sensitivity = move || {
                if let Some(obj) = obj_weak.upgrade() {
                    obj.update_continue_sensitivity();
                }
            };

            let cb = update_sensitivity.clone();
            self.backup_check.connect_toggled(move |_| cb());

            let cb = update_sensitivity.clone();
            self.warranty_check.connect_toggled(move |_| cb());

            let cb = update_sensitivity;
            self.risk_check.connect_toggled(move |_| cb());

            // Wire continue button to emit confirmed signal
            let obj_weak = obj.downgrade();
            self.continue_button.connect_clicked(move |_| {
                if let Some(obj) = obj_weak.upgrade() {
                    obj.emit_by_name::<()>("confirmed", &[]);
                }
            });
        }

        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: once_cell::sync::Lazy<Vec<glib::subclass::Signal>> =
                once_cell::sync::Lazy::new(|| {
                    vec![
                        glib::subclass::Signal::builder("confirmed").build(),
                    ]
                });
            &SIGNALS
        }
    }

    impl WidgetImpl for SafetyPage {}
    impl NavigationPageImpl for SafetyPage {}
}

glib::wrapper! {
    pub struct SafetyPage(ObjectSubclass<imp::SafetyPage>)
        @extends gtk::Widget, adw::NavigationPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SafetyPage {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn set_device(&self, device: &Device) {
        let imp = self.imp();
        imp.device_name_label.set_label(&format!("Device: {}", device.name));
    }

    pub fn connect_confirmed<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_closure(
            "confirmed",
            false,
            glib::closure_local!(move |obj: Self| {
                f(&obj);
            }),
        )
    }

    fn update_continue_sensitivity(&self) {
        let imp = self.imp();
        let all_checked = imp.backup_check.is_active()
            && imp.warranty_check.is_active()
            && imp.risk_check.is_active();
        imp.continue_button.set_sensitive(all_checked);
    }
}
