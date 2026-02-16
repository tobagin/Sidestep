// Flashing Progress Page
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::flashing::{DroidianInstaller, FactoryImageInstaller, InstallProgress, MobianInstaller, UbportsInstaller};
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use libadwaita as adw;
use adw::prelude::*;
use adw::subclass::prelude::*;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/tobagin/Sidestep/ui/pages/flashing.ui")]
    pub struct FlashingPage {
        #[template_child]
        pub status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub download_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub download_progress: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub decompress_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub decompress_progress: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub flash_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub flash_progress: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub verify_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub verify_icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub error_banner: TemplateChild<adw::Banner>,
        #[template_child]
        pub cancel_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub main_menu_button: TemplateChild<gtk::MenuButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FlashingPage {
        const NAME: &'static str = "FlashingPage";
        type Type = super::FlashingPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for FlashingPage {
        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: once_cell::sync::Lazy<Vec<glib::subclass::Signal>> =
                once_cell::sync::Lazy::new(|| {
                    vec![glib::subclass::Signal::builder("installation-complete").build()]
                });
            &SIGNALS
        }
    }
    impl WidgetImpl for FlashingPage {}
    impl NavigationPageImpl for FlashingPage {}
}

glib::wrapper! {
    pub struct FlashingPage(ObjectSubclass<imp::FlashingPage>)
        @extends gtk::Widget, adw::NavigationPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl FlashingPage {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn set_menu_model(&self, model: &gio::MenuModel) {
        self.imp().main_menu_button.set_menu_model(Some(model));
    }

    pub fn connect_installation_complete<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_closure(
            "installation-complete",
            false,
            glib::closure_local!(move |obj: Self| {
                f(&obj);
            }),
        )
    }

    pub fn set_distro_name(&self, name: &str) {
        log::info!("Starting installation for: {}", name);
    }

    /// Legacy mock-based installation (for non-UBports distros)
    pub fn start_installation(&self, distro_name: &str) {
        self.set_distro_name(distro_name);

        let imp = self.imp();
        imp.status_page.set_title(&format!("Installing {}", distro_name));
        imp.status_page.set_description(Some("Preparing..."));

        self.mock_progress();
    }

    /// Start real UBports installation with progress from background thread
    pub fn start_ubports_installation(&self, distro_name: &str, serial: &str, channel_path: &str) {
        self.set_distro_name(distro_name);

        let imp = self.imp();
        imp.status_page.set_title(&format!("Installing {}", distro_name));
        imp.status_page.set_description(Some("Preparing..."));

        // Repurpose the "Decompress" row for checksum verification
        imp.decompress_row.set_title("Verifying Checksums");
        #[allow(deprecated)]
        imp.decompress_row.set_icon_name(Some("channel-secure-symbolic"));

        let installer = UbportsInstaller::new(serial.to_string(), channel_path.to_string());
        let receiver = installer.spawn();

        // Poll receiver on the main thread
        let page = self.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            while let Ok(msg) = receiver.try_recv() {
                let should_stop = page.handle_progress(msg);
                if should_stop {
                    return glib::ControlFlow::Break;
                }
            }
            glib::ControlFlow::Continue
        });
    }

    /// Start real Droidian installation with progress from background thread
    pub fn start_droidian_installation(
        &self,
        distro_name: &str,
        serial: &str,
        release_url: &str,
        artifact_pattern: &str,
    ) {
        self.set_distro_name(distro_name);

        let imp = self.imp();
        imp.status_page.set_title(&format!("Installing {}", distro_name));
        imp.status_page.set_description(Some("Preparing..."));

        // Repurpose the "Decompress" row for ZIP extraction
        imp.decompress_row.set_title("Extracting");
        #[allow(deprecated)]
        imp.decompress_row
            .set_icon_name(Some("package-x-generic-symbolic"));

        let installer = DroidianInstaller::new(
            serial.to_string(),
            release_url.to_string(),
            artifact_pattern.to_string(),
        );
        let receiver = installer.spawn();

        // Poll receiver on the main thread
        let page = self.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            while let Ok(msg) = receiver.try_recv() {
                let should_stop = page.handle_progress(msg);
                if should_stop {
                    return glib::ControlFlow::Break;
                }
            }
            glib::ControlFlow::Continue
        });
    }

    /// Start real Mobian installation with progress from background thread
    pub fn start_mobian_installation(
        &self,
        distro_name: &str,
        serial: &str,
        base_url: &str,
        interface: &str,
        chipset: &str,
        device_model: &str,
    ) {
        self.set_distro_name(distro_name);

        let imp = self.imp();
        imp.status_page.set_title(&format!("Installing {}", distro_name));
        imp.status_page.set_description(Some("Preparing..."));

        // Repurpose the "Decompress" row for tar.xz extraction
        imp.decompress_row.set_title("Extracting");
        #[allow(deprecated)]
        imp.decompress_row
            .set_icon_name(Some("package-x-generic-symbolic"));

        let installer = MobianInstaller::new(
            serial.to_string(),
            base_url.to_string(),
            interface.to_string(),
            chipset.to_string(),
            device_model.to_string(),
        );
        let receiver = installer.spawn();

        // Poll receiver on the main thread
        let page = self.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            while let Ok(msg) = receiver.try_recv() {
                let should_stop = page.handle_progress(msg);
                if should_stop {
                    return glib::ControlFlow::Break;
                }
            }
            glib::ControlFlow::Continue
        });
    }

    /// Start factory image (stock Android) installation with progress from background thread
    pub fn start_factory_image_installation(
        &self,
        android_version: &str,
        serial: &str,
        url: &str,
        sha256: &str,
    ) {
        self.set_distro_name(android_version);

        let imp = self.imp();
        imp.status_page.set_title(&format!("Flashing {}", android_version));
        imp.status_page.set_description(Some("Preparing..."));

        // Repurpose the "Decompress" row for ZIP extraction
        imp.decompress_row.set_title("Extracting");
        #[allow(deprecated)]
        imp.decompress_row
            .set_icon_name(Some("package-x-generic-symbolic"));

        let installer = FactoryImageInstaller::new(
            serial.to_string(),
            url.to_string(),
            sha256.to_string(),
            android_version.to_string(),
        );
        let receiver = installer.spawn();

        let page = self.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            while let Ok(msg) = receiver.try_recv() {
                let should_stop = page.handle_progress(msg);
                if should_stop {
                    return glib::ControlFlow::Break;
                }
            }
            glib::ControlFlow::Continue
        });
    }

    /// Handle a progress message from the installer. Returns true if polling should stop.
    fn handle_progress(&self, msg: InstallProgress) -> bool {
        let imp = self.imp();

        match msg {
            InstallProgress::DownloadProgress {
                downloaded,
                total,
                file_name,
            } => {
                if total > 0 {
                    let fraction = downloaded as f64 / total as f64;
                    imp.download_progress.set_fraction(fraction.min(1.0));

                    let downloaded_mb = downloaded as f64 / 1_000_000.0;
                    let total_mb = total as f64 / 1_000_000.0;
                    imp.download_row.set_subtitle(&format!(
                        "{} — {:.1} / {:.1} MB",
                        file_name, downloaded_mb, total_mb
                    ));
                }
            }

            InstallProgress::VerifyProgress {
                verified,
                total,
                file_name,
            } => {
                if total > 0 {
                    let fraction = verified as f64 / total as f64;
                    imp.decompress_progress.set_fraction(fraction.min(1.0));
                    imp.decompress_row.set_subtitle(&format!(
                        "{} ({}/{})",
                        file_name, verified, total
                    ));
                }

                if verified >= total {
                    imp.decompress_row.set_subtitle("All checksums verified");
                }
            }

            InstallProgress::FlashProgress {
                current,
                total,
                description,
            } => {
                if total > 0 {
                    let fraction = current as f64 / total as f64;
                    imp.flash_progress.set_fraction(fraction.min(1.0));
                    imp.flash_row.set_subtitle(&format!(
                        "{} ({}/{})",
                        description, current, total
                    ));
                }
            }

            InstallProgress::StatusChanged(status) => {
                imp.status_page.set_description(Some(&status));
            }

            InstallProgress::WaitingForRecovery => {
                imp.status_page.set_description(Some("Waiting for Recovery Mode..."));
                imp.error_banner.set_title(
                    "On your phone: use Volume buttons to select \"Recovery mode\", then press Power",
                );
                imp.error_banner.set_revealed(true);
                // Remove the error styling — this is an info prompt, not an error
                imp.error_banner.remove_css_class("error");
            }

            InstallProgress::RecoveryDetected => {
                imp.error_banner.set_revealed(false);
                imp.status_page.set_description(Some("Recovery mode detected"));
            }

            InstallProgress::Complete => {
                imp.status_page.set_title("Installation Complete!");
                imp.download_row.set_subtitle("Complete");
                imp.decompress_row.set_subtitle("Complete");
                imp.flash_row.set_subtitle("Complete");

                imp.download_progress.set_fraction(1.0);
                imp.decompress_progress.set_fraction(1.0);
                imp.flash_progress.set_fraction(1.0);

                imp.verify_icon.set_icon_name(Some("emblem-ok-symbolic"));
                imp.verify_icon.set_visible(true);
                imp.verify_row.set_subtitle("Verified");

                self.emit_by_name::<()>("installation-complete", &[]);
                return true;
            }

            InstallProgress::Error(msg) => {
                log::error!("Installation error: {}", msg);
                imp.status_page.set_title("Installation Failed");
                imp.error_banner.set_title(&msg);
                imp.error_banner.add_css_class("error");
                imp.error_banner.set_revealed(true);
                return true;
            }
        }

        false
    }

    fn mock_progress(&self) {
        let page = self.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            let imp = page.imp();
            let fraction = imp.download_progress.fraction();
            if fraction < 1.0 {
                imp.download_progress.set_fraction(fraction + 0.05);
                imp.download_row.set_subtitle(&format!("{:.0}%", (fraction + 0.05) * 100.0));
                return glib::ControlFlow::Continue;
            }

            imp.download_row.set_subtitle("Complete");
            imp.download_row.add_suffix(&gtk::Image::from_icon_name("emblem-ok-symbolic"));

            // Decompress
            let fraction_dec = imp.decompress_progress.fraction();
            if fraction_dec < 1.0 {
                 imp.decompress_progress.set_fraction(fraction_dec + 0.05);
                 return glib::ControlFlow::Continue;
            }
             imp.decompress_row.set_subtitle("Complete");

            // Flash
            let fraction_flash = imp.flash_progress.fraction();
            if fraction_flash < 1.0 {
                 imp.flash_progress.set_fraction(fraction_flash + 0.05);
                 return glib::ControlFlow::Continue;
            }
             imp.flash_row.set_subtitle("Complete");

            imp.status_page.set_title("Installation Complete!");
            imp.verify_icon.set_visible(true);
            imp.verify_row.set_subtitle("Verified");

            page.emit_by_name::<()>("installation-complete", &[]);

            glib::ControlFlow::Break
        });
    }
}
