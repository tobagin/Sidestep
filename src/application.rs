// Sidestep Application
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::config;
use crate::window::SidestepWindow;
use gettextrs::gettext;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use libadwaita as adw;
use adw::prelude::*;
use adw::subclass::prelude::*;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct SidestepApplication {}

    #[glib::object_subclass]
    impl ObjectSubclass for SidestepApplication {
        const NAME: &'static str = "SidestepApplication";
        type Type = super::SidestepApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for SidestepApplication {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_actions();
            obj.setup_accels();
        }
    }

    impl ApplicationImpl for SidestepApplication {
        fn activate(&self) {
            log::debug!("Application activate");
            let application = self.obj();

            // Get the current window or create one if necessary
            let window = if let Some(window) = application.active_window() {
                window
            } else {
                let window = SidestepWindow::new(&*application);
                window.upcast()
            };

            window.present();
        }
    }

    impl GtkApplicationImpl for SidestepApplication {}
    impl AdwApplicationImpl for SidestepApplication {}
}

glib::wrapper! {
    pub struct SidestepApplication(ObjectSubclass<imp::SidestepApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl SidestepApplication {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("application-id", config::APP_ID)
            .property("flags", gio::ApplicationFlags::default())
            .build()
    }

    fn setup_actions(&self) {
        // Quit action
        let quit_action = gio::ActionEntry::builder("quit")
            .activate(|app: &Self, _, _| {
                app.quit();
            })
            .build();

        // About action
        let about_action = gio::ActionEntry::builder("about")
            .activate(|app: &Self, _, _| {
                app.show_about();
            })
            .build();

        // Preferences action
        let preferences_action = gio::ActionEntry::builder("preferences")
            .activate(|app: &Self, _, _| {
                app.show_preferences();
            })
            .build();

        self.add_action_entries([quit_action, about_action, preferences_action]);
    }

    fn setup_accels(&self) {
        self.set_accels_for_action("app.quit", &["<primary>q"]);
        self.set_accels_for_action("app.preferences", &["<primary>comma"]);
        self.set_accels_for_action("win.toggle-terminal", &["<primary>t"]);
    }

    fn show_about(&self) {
        let window = self.active_window().unwrap();
        
        let about = adw::AboutDialog::builder()
            .application_name(config::APP_NAME)
            .application_icon(config::APP_ID)
            .developer_name("tobagin")
            .version(config::VERSION)
            .developers(vec!["tobagin"])
            .copyright("© 2026 tobagin")
            .license_type(gtk::License::Gpl30)
            .website("https://github.com/tobagin/Sidestep")
            .issue_url("https://github.com/tobagin/Sidestep/issues")
            .comments(&gettext("A user-friendly wizard for installing mobile Linux distributions"))
            .build();

        about.present(Some(&window));
    }

    fn show_preferences(&self) {
        let _window = self.active_window().unwrap();
        
        // TODO: Implement preferences dialog
        log::info!("Preferences dialog not yet implemented");
    }
}

impl Default for SidestepApplication {
    fn default() -> Self {
        Self::new()
    }
}
