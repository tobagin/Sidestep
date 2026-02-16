// Distro Selection Page
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::models::{Distro, DistroTreeNode};
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use libadwaita as adw;
use adw::prelude::*;
use adw::subclass::prelude::*;
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/tobagin/Sidestep/ui/pages/distro_selection.ui")]
    pub struct DistroSelectionPage {
        #[template_child]
        pub distros_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub selected_info_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub selected_distro_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub download_size_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub install_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub main_menu_button: TemplateChild<gtk::MenuButton>,
        
        pub distros: RefCell<Vec<DistroTreeNode>>,
        pub selected_distro: RefCell<Option<Distro>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DistroSelectionPage {
        const NAME: &'static str = "DistroSelectionPage";
        type Type = super::DistroSelectionPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl DistroSelectionPage {
        #[template_callback]
        fn on_install_button_clicked(&self, _button: &gtk::Button) {
            // emitted signal or callback to main window
            // For now, let's just log
            if let Some(distro) = self.selected_distro.borrow().clone() {
                log::info!("Install clicked for {}", distro.name);
                let obj = self.obj();
                obj.emit_by_name::<()>("distro-selected", &[&distro.name]);
            }
        }


    }

    impl ObjectImpl for DistroSelectionPage {
        fn constructed(&self) {
            self.parent_constructed();
        }
        
        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: once_cell::sync::Lazy<Vec<glib::subclass::Signal>> =
                once_cell::sync::Lazy::new(|| {
                    vec![glib::subclass::Signal::builder("distro-selected")
                        .param_types([String::static_type()])
                        .build()]
                });
            &SIGNALS
        }
    }
    impl WidgetImpl for DistroSelectionPage {}
    impl NavigationPageImpl for DistroSelectionPage {}
}

glib::wrapper! {
    pub struct DistroSelectionPage(ObjectSubclass<imp::DistroSelectionPage>)
        @extends gtk::Widget, adw::NavigationPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl DistroSelectionPage {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn set_menu_model(&self, model: &gio::MenuModel) {
        self.imp().main_menu_button.set_menu_model(Some(model));
    }

    pub fn set_distros(&self, nodes: Vec<DistroTreeNode>) {
        let imp = self.imp();
        
        *imp.distros.borrow_mut() = nodes.clone();
        
        for node in nodes {
            let row = adw::ActionRow::builder()
                .title(node.name())
                .subtitle(node.description())
                .activatable(true)
                .selectable(false)
                .build();
                
            let page = self.clone();
            let node_clone = node.clone();
            
            match node_clone {
                DistroTreeNode::Group { name, children, .. } => {
                    row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
                    row.connect_activated(move |_| {
                        page.navigate_to_group(&name, children.clone());
                    });
                }
                DistroTreeNode::Item(distro) => {
                    row.connect_activated(move |_| {
                        page.select_distro(&distro);
                    });
                }
            }
            
            imp.distros_group.add(&row);
        }
    }

    fn navigate_to_group(&self, name: &str, children: Vec<DistroTreeNode>) {
        if let Some(nav_view) = self.ancestor(adw::NavigationView::static_type())
            .and_then(|w| w.downcast::<adw::NavigationView>().ok()) 
        {
            let page = DistroSelectionPage::new();
            page.set_title(name);
            page.set_distros(children);
            
            // Forward signal from child page
            let obj = self.clone();
            page.connect_closure(
                "distro-selected",
                false,
                glib::closure_local!(move |_: crate::pages::distro_selection::DistroSelectionPage, name: &str| {
                    obj.emit_by_name::<()>("distro-selected", &[&name]);
                }),
            );
            
            nav_view.push(&page);
        } else {
            log::error!("Could not find ancestor NavigationView");
        }
    }
    
    fn select_distro(&self, distro: &Distro) {
        let imp = self.imp();
        *imp.selected_distro.borrow_mut() = Some(distro.clone());
        
        // Update selection UI
        imp.selected_distro_row.set_title(&distro.name);
        imp.selected_distro_row.set_subtitle(&distro.description);
        imp.download_size_row.set_subtitle(&distro.download_size_string());
        
        imp.selected_info_group.set_visible(true);
        imp.install_button.set_sensitive(true);
    }
}
