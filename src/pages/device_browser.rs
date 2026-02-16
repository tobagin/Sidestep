// Device Browser Page
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::models::DeviceDatabase;
use gtk::{glib, prelude::*, subclass::prelude::*};
use libadwaita as adw;
use adw::prelude::*;
use adw::subclass::prelude::*;
use std::collections::BTreeMap;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct DeviceBrowserPage;

    #[glib::object_subclass]
    impl ObjectSubclass for DeviceBrowserPage {
        const NAME: &'static str = "DeviceBrowserPage";
        type Type = super::DeviceBrowserPage;
        type ParentType = adw::NavigationPage;
    }

    impl ObjectImpl for DeviceBrowserPage {
        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: std::sync::OnceLock<Vec<glib::subclass::Signal>> = std::sync::OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    glib::subclass::Signal::builder("device-selected")
                        .param_types([String::static_type()])
                        .build(),
                ]
            })
        }

        fn constructed(&self) {
            self.parent_constructed();
            self.obj().build_ui();
        }
    }

    impl WidgetImpl for DeviceBrowserPage {}
    impl NavigationPageImpl for DeviceBrowserPage {}
}

glib::wrapper! {
    pub struct DeviceBrowserPage(ObjectSubclass<imp::DeviceBrowserPage>)
        @extends gtk::Widget, adw::NavigationPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl DeviceBrowserPage {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("title", "Browse Devices")
            .property("tag", "device_browser")
            .build()
    }

    fn build_ui(&self) {
        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&adw::HeaderBar::new());

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();

        let clamp = adw::Clamp::builder()
            .maximum_size(600)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(12)
            .margin_end(12)
            .build();

        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(24)
            .build();

        // Title area
        let title_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .halign(gtk::Align::Center)
            .build();

        let title_label = gtk::Label::builder()
            .label("Supported Devices")
            .css_classes(vec!["title-1".to_string()])
            .build();

        let subtitle_label = gtk::Label::builder()
            .label("Select your device to see setup instructions")
            .css_classes(vec!["dim-label".to_string()])
            .wrap(true)
            .justify(gtk::Justification::Center)
            .build();

        title_box.append(&title_label);
        title_box.append(&subtitle_label);
        content_box.append(&title_box);

        // Load devices and group by manufacturer
        let db = DeviceDatabase::new();
        let devices = db.all_devices();

        let mut grouped: BTreeMap<String, Vec<_>> = BTreeMap::new();
        for device in &devices {
            grouped
                .entry(device.maker.clone())
                .or_default()
                .push(device.clone());
        }

        // Create a PreferencesGroup per manufacturer
        for (maker, devices) in &grouped {
            let group = adw::PreferencesGroup::builder()
                .title(maker)
                .build();

            // Flatten all display names (primary + variants) into a single sorted list
            let mut rows: Vec<(String, String, bool)> = Vec::new();
            for device in devices {
                rows.push((device.name.clone(), device.codename.clone(), device.experimental));
                for variant in &device.variants {
                    rows.push((variant.clone(), device.codename.clone(), device.experimental));
                }
            }
            rows.sort_by(|a, b| a.0.cmp(&b.0));

            for (display_name, codename, experimental) in &rows {
                let subtitle = if *experimental {
                    format!("{} (Experimental)", codename)
                } else {
                    codename.clone()
                };

                let row = adw::ActionRow::builder()
                    .title(display_name)
                    .subtitle(&subtitle)
                    .activatable(true)
                    .build();

                let icon = gtk::Image::from_icon_name("phone-symbolic");
                icon.set_pixel_size(32);
                row.add_prefix(&icon);

                let chevron = gtk::Image::from_icon_name("go-next-symbolic");
                row.add_suffix(&chevron);

                let page_clone = self.clone();
                let codename = codename.clone();
                row.connect_activated(move |_| {
                    page_clone.emit_by_name::<()>("device-selected", &[&codename]);
                });

                group.add(&row);
            }

            content_box.append(&group);
        }

        clamp.set_child(Some(&content_box));
        scrolled.set_child(Some(&clamp));
        toolbar_view.set_content(Some(&scrolled));

        self.set_child(Some(&toolbar_view));
    }

    pub fn connect_device_selected<F: Fn(&Self, String) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_closure(
            "device-selected",
            false,
            glib::closure_local!(move |obj: Self, codename: String| {
                f(&obj, codename);
            }),
        )
    }
}
