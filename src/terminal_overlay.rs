// Terminal Overlay
// SPDX-License-Identifier: GPL-3.0-or-later
//
// A simple read-only log view shown at the bottom of the window. Command output
// from the unlock and flashing flows is appended here when the terminal is
// enabled.

use gtk::{glib, prelude::*, subclass::prelude::*};
use libadwaita as adw;
use adw::subclass::prelude::*;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/tobagin/Sidestep/ui/terminal_overlay.ui")]
    pub struct TerminalOverlay {
        #[template_child]
        pub terminal_view: TemplateChild<gtk::TextView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TerminalOverlay {
        const NAME: &'static str = "TerminalOverlay";
        type Type = super::TerminalOverlay;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TerminalOverlay {}
    impl WidgetImpl for TerminalOverlay {}
    impl BinImpl for TerminalOverlay {}
}

glib::wrapper! {
    pub struct TerminalOverlay(ObjectSubclass<imp::TerminalOverlay>)
        @extends gtk::Widget, adw::Bin,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl TerminalOverlay {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    /// Append a line of output and scroll to the bottom.
    pub fn append_line(&self, line: &str) {
        let buffer = self.imp().terminal_view.buffer();
        let mut end = buffer.end_iter();

        if buffer.char_count() > 0 {
            buffer.insert(&mut end, "\n");
        }
        buffer.insert(&mut end, line);

        // Scroll the view so the newest line is visible.
        let mark = buffer.create_mark(None, &buffer.end_iter(), false);
        self.imp()
            .terminal_view
            .scroll_to_mark(&mark, 0.0, false, 0.0, 0.0);
    }

    /// Clear all output.
    pub fn clear(&self) {
        let buffer = self.imp().terminal_view.buffer();
        buffer.set_text("");
    }
}

impl Default for TerminalOverlay {
    fn default() -> Self {
        Self::new()
    }
}
