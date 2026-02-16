// Unlocking Page
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::{glib, subclass::prelude::*};
use libadwaita as adw;
use adw::subclass::prelude::*;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/tobagin/Sidestep/ui/pages/unlocking.ui")]
    pub struct UnlockingPage {
        #[template_child]
        pub continue_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for UnlockingPage {
        const NAME: &'static str = "UnlockingPage";
        type Type = super::UnlockingPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for UnlockingPage {}

    impl WidgetImpl for UnlockingPage {}
    impl NavigationPageImpl for UnlockingPage {}
}

glib::wrapper! {
    pub struct UnlockingPage(ObjectSubclass<imp::UnlockingPage>)
        @extends gtk::Widget, adw::NavigationPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl UnlockingPage {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
}
