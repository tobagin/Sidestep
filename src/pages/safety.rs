// Safety Page
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::{glib, subclass::prelude::*};
use libadwaita as adw;
use adw::subclass::prelude::*;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/tobagin/Sidestep/ui/pages/safety.ui")]
    pub struct SafetyPage {
        #[template_child]
        pub warning_banner: TemplateChild<adw::Banner>,
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

    impl ObjectImpl for SafetyPage {}

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
}
