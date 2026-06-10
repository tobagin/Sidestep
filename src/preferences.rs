// Preferences Dialog
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::config;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use libadwaita as adw;
use adw::prelude::*;
use adw::subclass::prelude::*;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/tobagin/Sidestep/ui/preferences_dialog.ui")]
    pub struct PreferencesDialog {
        #[template_child]
        pub download_path_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub choose_folder_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub sync_interval_row: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub sync_now_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub show_terminal_row: TemplateChild<adw::SwitchRow>,

        pub settings: once_cell::sync::OnceCell<gio::Settings>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PreferencesDialog {
        const NAME: &'static str = "PreferencesDialog";
        type Type = super::PreferencesDialog;
        type ParentType = adw::PreferencesDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PreferencesDialog {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup();
        }
    }

    impl WidgetImpl for PreferencesDialog {}
    impl AdwDialogImpl for PreferencesDialog {}
    impl PreferencesDialogImpl for PreferencesDialog {}
}

glib::wrapper! {
    pub struct PreferencesDialog(ObjectSubclass<imp::PreferencesDialog>)
        @extends gtk::Widget, adw::Dialog, adw::PreferencesDialog,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl PreferencesDialog {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    fn setup(&self) {
        let imp = self.imp();
        let settings = gio::Settings::new(config::APP_ID);

        // Download location ----------------------------------------------------
        self.update_download_subtitle(&settings);

        let dialog_weak = self.downgrade();
        let settings_clone = settings.clone();
        imp.choose_folder_button.connect_clicked(move |btn| {
            let Some(dialog) = dialog_weak.upgrade() else { return };
            let settings = settings_clone.clone();
            let file_dialog = gtk::FileDialog::builder()
                .title("Select Download Folder")
                .modal(true)
                .build();

            let parent = btn.root().and_downcast::<gtk::Window>();
            let dialog_weak = dialog.downgrade();
            file_dialog.select_folder(
                parent.as_ref(),
                gio::Cancellable::NONE,
                move |result| {
                    let Some(dialog) = dialog_weak.upgrade() else { return };
                    match result {
                        Ok(folder) => {
                            if let Some(path) = folder.path() {
                                let _ = settings.set_string(
                                    "download-path",
                                    &path.to_string_lossy(),
                                );
                                dialog.update_download_subtitle(&settings);
                            }
                        }
                        Err(e) => log::debug!("Folder selection cancelled: {}", e),
                    }
                },
            );
        });

        // Sync interval --------------------------------------------------------
        let interval = settings.int("sync-interval-hours");
        imp.sync_interval_row.set_value(interval as f64);

        let settings_clone = settings.clone();
        imp.sync_interval_row.connect_value_notify(move |row| {
            let _ = settings_clone.set_int("sync-interval-hours", row.value() as i32);
        });

        // The device database is bundled with the application, so there is no
        // remote sync to perform — surface that clearly when the user asks.
        let dialog_weak = self.downgrade();
        imp.sync_now_button.connect_clicked(move |_| {
            if let Some(dialog) = dialog_weak.upgrade() {
                dialog.add_toast(adw::Toast::new(
                    "Device database is bundled with the app and is always up to date.",
                ));
            }
        });

        // Show terminal --------------------------------------------------------
        settings
            .bind("show-terminal", &*imp.show_terminal_row, "active")
            .build();

        imp.settings.set(settings).ok();
    }

    fn update_download_subtitle(&self, settings: &gio::Settings) {
        let path = settings.string("download-path");
        let subtitle = if path.is_empty() {
            "Default (Downloads folder)".to_string()
        } else {
            path.to_string()
        };
        self.imp().download_path_row.set_subtitle(&subtitle);
    }
}

impl Default for PreferencesDialog {
    fn default() -> Self {
        Self::new()
    }
}
