// Wizard Choice Page
// SPDX-License-Identifier: GPL-3.0-or-later
//
// Reusable status-page-with-pill-buttons used throughout the install flow
// (bootloader check, prerequisites, channel/interface selection, confirmations).
// The shell lives in Blueprint; callers add the dynamic buttons from Rust.

use gtk::{glib, prelude::*, subclass::prelude::*};
use libadwaita as adw;
use adw::prelude::*;
use adw::subclass::prelude::*;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/tobagin/Sidestep/ui/pages/wizard_choice_page.ui")]
    pub struct WizardChoicePage {
        #[template_child]
        pub status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub buttons_box: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WizardChoicePage {
        const NAME: &'static str = "WizardChoicePage";
        type Type = super::WizardChoicePage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for WizardChoicePage {}
    impl WidgetImpl for WizardChoicePage {}
    impl NavigationPageImpl for WizardChoicePage {}
}

glib::wrapper! {
    pub struct WizardChoicePage(ObjectSubclass<imp::WizardChoicePage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl WizardChoicePage {
    /// `page_title` is the navigation title (shown in the header bar);
    /// `title`/`description`/`icon` populate the central status page.
    pub fn new(page_title: &str, title: &str, description: &str, icon: &str) -> Self {
        let obj: Self = glib::Object::new();
        obj.set_title(page_title);
        let imp = obj.imp();
        imp.status_page.set_title(title);
        imp.status_page.set_description(Some(description));
        imp.status_page.set_icon_name(Some(icon));
        obj
    }

    /// Append a pill button and return it so the caller can wire `clicked`.
    pub fn add_button(&self, label: &str, css_classes: &[&str]) -> gtk::Button {
        let btn = gtk::Button::builder()
            .label(label)
            .width_request(250)
            .height_request(50)
            .build();
        for class in css_classes {
            btn.add_css_class(class);
        }
        self.imp().buttons_box.append(&btn);
        btn
    }
}
