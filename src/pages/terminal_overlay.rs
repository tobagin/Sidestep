// Terminal Overlay
// SPDX-License-Identifier: GPL-3.0-or-later

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
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl TerminalOverlay {
    /// Append one line to the terminal and scroll to the bottom.
    pub fn append(&self, line: &str) {
        let view = &*self.imp().terminal_view;
        let buffer = view.buffer();
        let mut end = buffer.end_iter();
        buffer.insert(&mut end, line);
        buffer.insert(&mut end, "\n");
        let mark = buffer.create_mark(None, &buffer.end_iter(), false);
        view.scroll_to_mark(&mark, 0.0, false, 0.0, 0.0);
    }
}

impl Default for TerminalOverlay {
    fn default() -> Self {
        glib::Object::new()
    }
}
