// Success Page
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use libadwaita as adw;
use adw::prelude::*;
use adw::subclass::prelude::*;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/tobagin/Sidestep/ui/pages/success.ui")]
    pub struct SuccessPage {
        #[template_child]
        pub status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub main_menu_button: TemplateChild<gtk::MenuButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SuccessPage {
        const NAME: &'static str = "SuccessPage";
        type Type = super::SuccessPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.install_action("page.restart", None, move |page, _, _| {
                 page.emit_by_name::<()>("restart-clicked", &[]);
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl WidgetImpl for SuccessPage {}
    impl NavigationPageImpl for SuccessPage {}
}

glib::wrapper! {
    pub struct SuccessPage(ObjectSubclass<imp::SuccessPage>)
        @extends gtk::Widget, adw::NavigationPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SuccessPage {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn set_menu_model(&self, model: &gio::MenuModel) {
        self.imp().main_menu_button.set_menu_model(Some(model));
    }

    pub fn set_distro_name(&self, name: &str) {
        self.imp().status_page.set_description(Some(
            &format!("Your device is ready to boot into {}.\nDisconnect the USB cable and restart your device.", name),
        ));
    }
    
    pub fn connect_restart_clicked<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_closure(
            "restart-clicked",
            false,
            glib::closure_local!(move |obj: Self| {
                f(&obj);
            }),
        )
    }
}

impl ObjectImpl for imp::SuccessPage {
     fn signals() -> &'static [glib::subclass::Signal] {
        static SIGNALS: once_cell::sync::Lazy<Vec<glib::subclass::Signal>> =
            once_cell::sync::Lazy::new(|| {
                vec![
                    glib::subclass::Signal::builder("restart-clicked").build(),
                ]
            });
        &SIGNALS
    }
}
