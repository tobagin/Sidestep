// Unlocking Page
// SPDX-License-Identifier: GPL-3.0-or-later
//
// Guides the user through unlocking their device's bootloader. Manual steps are
// checked off by the user; automated steps run adb/fastboot commands in a
// background thread and report success/failure back to the UI.

use crate::models::unlock_guide::unlock_steps;
use crate::models::unlocking_step::UnlockingStep;
use crate::models::{Device, DeviceDatabase};
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use libadwaita as adw;
use adw::prelude::*;
use adw::subclass::prelude::*;
use std::cell::{Cell, RefCell};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/tobagin/Sidestep/ui/pages/unlocking.ui")]
    pub struct UnlockingPage {
        #[template_child]
        pub main_menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub steps_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub progress_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub progress_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub command_progress: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub back_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub continue_button: TemplateChild<gtk::Button>,

        pub device: RefCell<Option<Device>>,
        pub steps: RefCell<Vec<UnlockingStep>>,
        pub done: RefCell<Vec<bool>>,
        pub rows: RefCell<Vec<adw::ActionRow>>,
        pub status_icons: RefCell<Vec<gtk::Image>>,
        pub running: Cell<bool>,
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

    impl ObjectImpl for UnlockingPage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            // Back button pops the navigation stack.
            let obj_weak = obj.downgrade();
            self.back_button.connect_clicked(move |_| {
                let Some(obj) = obj_weak.upgrade() else { return };
                if let Some(nav) = obj
                    .ancestor(adw::NavigationView::static_type())
                    .and_downcast::<adw::NavigationView>()
                {
                    nav.pop();
                }
            });

            // Continue emits a signal once the bootloader has been unlocked.
            let obj_weak = obj.downgrade();
            self.continue_button.connect_clicked(move |_| {
                if let Some(obj) = obj_weak.upgrade() {
                    obj.emit_by_name::<()>("unlock-complete", &[]);
                }
            });
        }

        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: once_cell::sync::Lazy<Vec<glib::subclass::Signal>> =
                once_cell::sync::Lazy::new(|| {
                    vec![glib::subclass::Signal::builder("unlock-complete").build()]
                });
            &SIGNALS
        }
    }

    impl WidgetImpl for UnlockingPage {}
    impl NavigationPageImpl for UnlockingPage {}
}

glib::wrapper! {
    pub struct UnlockingPage(ObjectSubclass<imp::UnlockingPage>)
        @extends gtk::Widget, adw::NavigationPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl UnlockingPage {
    pub fn new(device: &Device) -> Self {
        let obj: Self = glib::Object::builder().build();
        obj.set_device(device);
        obj
    }

    pub fn set_menu_model(&self, model: &gio::MenuModel) {
        self.imp().main_menu_button.set_menu_model(Some(model));
    }

    pub fn connect_unlock_complete<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_closure(
            "unlock-complete",
            false,
            glib::closure_local!(move |obj: Self| {
                f(&obj);
            }),
        )
    }

    fn set_device(&self, device: &Device) {
        let imp = self.imp();
        *imp.device.borrow_mut() = Some(device.clone());

        // Prefer the curated per-device steps from the database; fall back to a
        // manufacturer-generic guide for devices without curated entries.
        let mut steps = DeviceDatabase::new().get_unlocking_steps(&device.codename);
        if steps.is_empty() {
            steps = unlock_steps(device);
        }
        let count = steps.len();
        *imp.done.borrow_mut() = vec![false; count];
        *imp.steps.borrow_mut() = steps;

        self.build_rows();
        self.update_continue_sensitivity();
    }

    fn build_rows(&self) {
        let imp = self.imp();

        // Clear any previously added rows.
        for row in imp.rows.borrow().iter() {
            imp.steps_group.remove(row);
        }
        imp.rows.borrow_mut().clear();
        imp.status_icons.borrow_mut().clear();

        let steps = imp.steps.borrow().clone();
        let mut rows = Vec::with_capacity(steps.len());
        let mut status_icons = Vec::with_capacity(steps.len());

        for (index, step) in steps.iter().enumerate() {
            let mut subtitle = step.description.clone();
            if let Some(ref warning) = step.warning {
                subtitle.push_str(&format!("\n⚠ {}", warning));
            }

            let row = adw::ActionRow::builder()
                .title(format!("{}. {}", step.order, step.title))
                .subtitle(subtitle)
                .build();
            row.set_subtitle_lines(0);

            // Leading status icon (updated to a checkmark when the step is done).
            let status_icon = gtk::Image::from_icon_name(if step.is_automated() {
                "system-run-symbolic"
            } else {
                "checkbox-symbolic"
            });
            status_icon.set_valign(gtk::Align::Center);
            row.add_prefix(&status_icon);
            status_icons.push(status_icon);

            // Optional helper link (e.g. a vendor unlock portal).
            if let Some(ref link) = step.link {
                let label = step.link_label.clone().unwrap_or_else(|| "Open Website".to_string());
                let link_btn = gtk::Button::builder()
                    .label(label)
                    .valign(gtk::Align::Center)
                    .css_classes(vec!["flat".to_string()])
                    .build();
                let url = link.clone();
                link_btn.connect_clicked(move |btn| {
                    let launcher = gtk::UriLauncher::new(&url);
                    let window = btn.root().and_downcast::<gtk::Window>();
                    launcher.launch(window.as_ref(), gio::Cancellable::NONE, |res| {
                        if let Err(e) = res {
                            log::warn!("Failed to open URL: {}", e);
                        }
                    });
                });
                row.add_suffix(&link_btn);
            }

            if step.is_automated() {
                let run_btn = gtk::Button::builder()
                    .label("Run")
                    .valign(gtk::Align::Center)
                    .css_classes(vec!["suggested-action".to_string()])
                    .build();
                let page = self.clone();
                run_btn.connect_clicked(move |_| {
                    page.run_step(index);
                });
                row.add_suffix(&run_btn);
            } else {
                let check = gtk::CheckButton::builder()
                    .valign(gtk::Align::Center)
                    .build();
                let page = self.clone();
                check.connect_toggled(move |c| {
                    page.set_step_done(index, c.is_active());
                });
                row.add_suffix(&check);
                row.set_activatable_widget(Some(&check));
            }

            imp.steps_group.add(&row);
            rows.push(row);
        }

        *imp.rows.borrow_mut() = rows;
        *imp.status_icons.borrow_mut() = status_icons;
    }

    fn set_step_done(&self, index: usize, done: bool) {
        {
            let mut flags = self.imp().done.borrow_mut();
            if let Some(flag) = flags.get_mut(index) {
                *flag = done;
            }
        }

        // Reflect completion with a checkmark on the row.
        if let Some(row) = self.imp().rows.borrow().get(index) {
            if done {
                row.add_css_class("success");
            } else {
                row.remove_css_class("success");
            }
        }
        if let Some(icon) = self.imp().status_icons.borrow().get(index) {
            if done {
                icon.set_icon_name(Some("emblem-ok-symbolic"));
                icon.add_css_class("success");
            } else {
                icon.set_icon_name(Some("checkbox-symbolic"));
                icon.remove_css_class("success");
            }
        }

        self.update_continue_sensitivity();
    }

    fn update_continue_sensitivity(&self) {
        let imp = self.imp();
        let steps = imp.steps.borrow();
        let done = imp.done.borrow();

        let all_required_done = steps
            .iter()
            .enumerate()
            .all(|(i, step)| step.optional || done.get(i).copied().unwrap_or(false));

        imp.continue_button.set_sensitive(all_required_done && !imp.running.get());
    }

    fn run_step(&self, index: usize) {
        let imp = self.imp();
        if imp.running.get() {
            return;
        }

        let (command, title) = {
            let steps = imp.steps.borrow();
            let Some(step) = steps.get(index) else { return };
            let Some(command) = step.command.clone() else { return };
            (command, step.title.clone())
        };

        let serial = imp
            .device
            .borrow()
            .as_ref()
            .and_then(|d| d.serial.clone());

        imp.running.set(true);
        imp.progress_box.set_visible(true);
        imp.progress_label.set_label(&format!("{} — running: {}", title, command));
        imp.command_progress.set_fraction(0.0);
        self.update_continue_sensitivity();
        self.log_to_terminal(&format!("$ {}", command));

        // Run the command on a background thread with its own tokio runtime.
        let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
        let command_thread = command.clone();
        std::thread::spawn(move || {
            let result = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt.block_on(run_command(&command_thread, serial.as_deref())),
                Err(e) => Err(anyhow::anyhow!("Failed to start runtime: {}", e)),
            };
            let _ = tx.send(result.map_err(|e| format!("{:#}", e)));
        });

        // Poll for completion on the main thread while pulsing the progress bar.
        let page = self.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(120), move || {
            page.imp().command_progress.pulse();

            match rx.try_recv() {
                Ok(result) => {
                    page.on_command_finished(index, result);
                    glib::ControlFlow::Break
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    page.on_command_finished(index, Err("Command thread stopped unexpectedly".to_string()));
                    glib::ControlFlow::Break
                }
            }
        });
    }

    fn on_command_finished(&self, index: usize, result: Result<String, String>) {
        let imp = self.imp();
        imp.running.set(false);
        imp.progress_box.set_visible(false);

        match result {
            Ok(output) => {
                if !output.trim().is_empty() {
                    for line in output.lines() {
                        self.log_to_terminal(line);
                    }
                }
                self.log_to_terminal("✓ Command completed");
                self.set_step_done(index, true);
            }
            Err(err) => {
                log::error!("Unlock command failed: {}", err);
                self.log_to_terminal(&format!("✗ {}", err));
                if let Some(window) = self
                    .root()
                    .and_downcast::<crate::window::SidestepWindow>()
                {
                    window.show_toast(&format!("Command failed: {}", err));
                }
            }
        }

        self.update_continue_sensitivity();
    }

    fn log_to_terminal(&self, line: &str) {
        if let Some(window) = self.root().and_downcast::<crate::window::SidestepWindow>() {
            window.append_terminal_log(line);
        }
    }
}

/// Run an adb/fastboot command string, substituting the configured binary path
/// and inserting the device serial. Returns combined stdout+stderr on success.
async fn run_command(command: &str, serial: Option<&str>) -> anyhow::Result<String> {
    use tokio::process::Command;

    let tokens: Vec<&str> = command.split_whitespace().collect();
    let Some((&program, rest)) = tokens.split_first() else {
        anyhow::bail!("Empty command");
    };

    // Only adb/fastboot may be invoked. Command strings come from the device DB,
    // which is auto-synced from a remote tarball (models/sync.rs) and is therefore
    // untrusted — allowing arbitrary program names here would be remote code execution.
    let binary = match program {
        "adb" => std::env::var("ADB_PATH").unwrap_or_else(|_| "adb".to_string()),
        "fastboot" => std::env::var("FASTBOOT_PATH").unwrap_or_else(|_| "fastboot".to_string()),
        other => anyhow::bail!("Refusing to run non-allowlisted program: {other}"),
    };

    let mut args: Vec<String> = Vec::new();
    if program == "adb" || program == "fastboot" {
        if let Some(s) = serial {
            args.push("-s".to_string());
            args.push(s.to_string());
        }
    }
    args.extend(rest.iter().map(|s| s.to_string()));

    let output = Command::new(&binary)
        .args(&args)
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to run {}: {}", binary, e))?;

    let mut combined = String::new();
    combined.push_str(&String::from_utf8_lossy(&output.stdout));
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.trim().is_empty() {
        if !combined.is_empty() && !combined.ends_with('\n') {
            combined.push('\n');
        }
        combined.push_str(&stderr);
    }

    if !output.status.success() {
        anyhow::bail!(
            "{}",
            if combined.trim().is_empty() {
                "command exited with an error".to_string()
            } else {
                combined.trim().to_string()
            }
        );
    }

    Ok(combined)
}
