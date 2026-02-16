// Unsupported Device Page
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::{glib, prelude::*, subclass::prelude::*};
use libadwaita as adw;
use adw::subclass::prelude::*;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/tobagin/Sidestep/ui/pages/unsupported_device.ui")]
    pub struct UnsupportedDevicePage {
        #[template_child]
        pub back_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for UnsupportedDevicePage {
        const NAME: &'static str = "UnsupportedDevicePage";
        type Type = super::UnsupportedDevicePage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.install_action("page.back", None, move |page, _, _| {
                page.on_back_clicked();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl WidgetImpl for UnsupportedDevicePage {}
    impl NavigationPageImpl for UnsupportedDevicePage {}
}

glib::wrapper! {
    pub struct UnsupportedDevicePage(ObjectSubclass<imp::UnsupportedDevicePage>)
        @extends gtk::Widget, adw::NavigationPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl UnsupportedDevicePage {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    fn on_back_clicked(&self) {
        self.emit_by_name::<()>("back-clicked", &[]);
    }
}

// Signal definitions
impl UnsupportedDevicePage {
    pub fn connect_back_clicked<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_closure(
            "back-clicked",
            false,
            glib::closure_local!(move |obj: Self| {
                f(&obj);
            }),
        )
    }
}

impl Default for UnsupportedDevicePage {
    fn default() -> Self {
        Self::new()
    }
}

// Add signals to ObjectImpl
impl ObjectImpl for imp::UnsupportedDevicePage {
    fn constructed(&self) {
        self.parent_constructed();
    }
    
    fn signals() -> &'static [glib::subclass::Signal] {
        static SIGNALS: once_cell::sync::Lazy<Vec<glib::subclass::Signal>> =
            once_cell::sync::Lazy::new(|| {
                vec![
                    glib::subclass::Signal::builder("back-clicked").build(),
                ]
            });
        &SIGNALS
    }
}
