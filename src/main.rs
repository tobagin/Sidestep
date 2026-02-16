// Sidestep - Mobile Linux Installer
// SPDX-License-Identifier: GPL-3.0-or-later

mod application;
mod config;
mod flashing;
mod hardware;
mod models;
mod window;
mod pages;
mod wizard;
mod wizards;
mod utils;

use application::SidestepApplication;
use gettextrs::LocaleCategory;
use gtk::{gio, glib, prelude::*};

fn main() -> glib::ExitCode {
    // Initialize logging
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("sidestep=info")
    ).init();

    log::info!("Starting Sidestep v{}", config::VERSION);

    // Set up gettext translations
    gettextrs::setlocale(LocaleCategory::LcAll, "");
    gettextrs::bindtextdomain(config::GETTEXT_PACKAGE, config::LOCALEDIR)
        .expect("Unable to bind text domain");
    gettextrs::textdomain(config::GETTEXT_PACKAGE)
        .expect("Unable to switch to text domain");

    // Load resources
    let resources = gio::Resource::load(config::PKGDATADIR.to_owned() + "/sidestep.gresource")
        .expect("Could not load resources");
    gio::resources_register(&resources);

    // Create and run the application
    let app = SidestepApplication::new();
    app.run()
}
