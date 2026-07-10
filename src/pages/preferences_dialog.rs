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
        pub last_sync_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub show_terminal_row: TemplateChild<adw::SwitchRow>,
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
        @extends adw::PreferencesDialog, adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl PreferencesDialog {
    pub fn new() -> Self {
        glib::Object::new()
    }

    fn setup(&self) {
        let imp = self.imp();
        let settings = gio::Settings::new(config::APP_ID);

        // Interface: terminal switch <-> gschema (bool <-> bool)
        settings
            .bind("show-terminal", &*imp.show_terminal_row, "active")
            .build();

        // Database: sync interval (int key <-> double property, bind manually)
        imp.sync_interval_row
            .set_value(settings.int("sync-interval-hours") as f64);
        let s = settings.clone();
        imp.sync_interval_row.connect_value_notify(move |row| {
            s.set_int("sync-interval-hours", row.value() as i32).ok();
        });

        // Database: show last sync time; Sync Now fetches latest configs.
        self.refresh_last_sync();
        let dialog = self.downgrade();
        imp.sync_now_button.connect_clicked(move |btn| {
            let Some(dialog) = dialog.upgrade() else { return };
            dialog.start_sync(btn);
        });

        // Downloads: show current path, folder chooser writes it
        self.refresh_download_subtitle(&settings);
        let s = settings.clone();
        let dialog = self.downgrade();
        imp.choose_folder_button.connect_clicked(move |btn| {
            let Some(dialog) = dialog.upgrade() else { return };
            let parent = btn.root().and_downcast::<gtk::Window>();
            let file_dialog = gtk::FileDialog::builder().title("Download Location").build();
            let s = s.clone();
            let dialog = dialog.downgrade();
            file_dialog.select_folder(
                parent.as_ref(),
                gio::Cancellable::NONE,
                move |res| {
                    let (Ok(folder), Some(dialog)) = (res, dialog.upgrade()) else { return };
                    if let Some(path) = folder.path() {
                        s.set_string("download-path", &path.to_string_lossy()).ok();
                        dialog.refresh_download_subtitle(&s);
                    }
                },
            );
        });
    }

    fn refresh_last_sync(&self) {
        let subtitle = match crate::models::sync::last_sync() {
            None => "Never".to_string(),
            Some(ts) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(ts);
                let secs = now.saturating_sub(ts);
                if secs < 60 {
                    "Just now".to_string()
                } else if secs < 3600 {
                    format!("{} minutes ago", secs / 60)
                } else if secs < 86400 {
                    format!("{} hours ago", secs / 3600)
                } else {
                    format!("{} days ago", secs / 86400)
                }
            }
        };
        self.imp().last_sync_row.set_subtitle(&subtitle);
    }

    fn start_sync(&self, btn: &gtk::Button) {
        btn.set_sensitive(false);
        btn.set_label("Syncing…");

        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(crate::models::sync::run_blocking());
        });

        let dialog = self.downgrade();
        let btn = btn.downgrade();
        glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
            let (Some(dialog), Some(btn)) = (dialog.upgrade(), btn.upgrade()) else {
                return glib::ControlFlow::Break;
            };
            match rx.try_recv() {
                Ok(result) => {
                    btn.set_sensitive(true);
                    btn.set_label("Sync Now");
                    match result {
                        Ok(n) => {
                            dialog.refresh_last_sync();
                            dialog.toast(&format!("Synced {} device files", n));
                        }
                        Err(e) => dialog.toast(&format!("Sync failed: {}", e)),
                    }
                    glib::ControlFlow::Break
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(_) => {
                    btn.set_sensitive(true);
                    btn.set_label("Sync Now");
                    glib::ControlFlow::Break
                }
            }
        });
    }

    fn toast(&self, message: &str) {
        self.add_toast(adw::Toast::new(message));
    }

    fn refresh_download_subtitle(&self, settings: &gio::Settings) {
        let path = settings.string("download-path");
        let subtitle = if path.is_empty() {
            "Default cache directory".to_string()
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
