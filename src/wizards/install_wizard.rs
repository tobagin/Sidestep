// Generic Install Wizard
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::models::{Device, installer::{InstallerConfig, Step}};
use gtk::{glib, prelude::*, subclass::prelude::*};
use libadwaita as adw;
use adw::prelude::*;
use adw::subclass::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use crate::config::SIDESTEP_DATA_DIR;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/tobagin/Sidestep/ui/wizards/install_wizard.ui")]
    pub struct InstallWizard {
        #[template_child]
        pub navigation_view: TemplateChild<adw::NavigationView>,
        
        pub config: RefCell<Option<Rc<InstallerConfig>>>,
        pub device: RefCell<Option<Device>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for InstallWizard {
        const NAME: &'static str = "InstallWizard";
        type Type = super::InstallWizard;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for InstallWizard {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for InstallWizard {}
    impl WindowImpl for InstallWizard {}
    impl AdwWindowImpl for InstallWizard {}
}

glib::wrapper! {
    pub struct InstallWizard(ObjectSubclass<imp::InstallWizard>)
        @extends gtk::Widget, gtk::Window, adw::Window,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl InstallWizard {
    pub fn new(parent: &gtk::Window, device: &Device, distro_id: &str) -> Self {
        let obj: Self = glib::Object::builder()
            .property("transient-for", parent)
            .build();
            
        obj.setup(device, distro_id);
        obj
    }

    fn setup(&self, device: &Device, distro_id: &str) {
        let imp = self.imp();
        *imp.device.borrow_mut() = Some(device.clone());

        // Load config
        let config_path = format!("{}/devices/{}/{}/installers/{}.yml", 
            SIDESTEP_DATA_DIR, 
            device.maker.to_lowercase(), 
            device.codename.to_lowercase(), 
            distro_id);
            
        if let Ok(file_content) = std::fs::read_to_string(&config_path) {
            if let Ok(config) = serde_yaml::from_str::<InstallerConfig>(&file_content) {
                let config_rc = Rc::new(config);
                *imp.config.borrow_mut() = Some(config_rc.clone());
                self.set_title(Some(&format!("Install {}", config_rc.name)));
                
                // Start flow
                self.show_prerequisites(config_rc);
            } else {
                log::error!("Failed to parse installer config: {}", config_path);
            }
        } else {
            log::error!("Failed to read installer config: {}", config_path);
        }
    }

    fn show_prerequisites(&self, config: Rc<InstallerConfig>) {
        if let Some(prereq) = config.prerequisites.first() {
            let page = adw::NavigationPage::builder()
                .title(&prereq.title)
                .build();

            let status_page = adw::StatusPage::builder()
                .title(&prereq.title)
                .description(&prereq.message)
                .icon_name("system-help-symbolic")
                .build();
            
            let box_container = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .spacing(24)
                .build();
                
            box_container.append(&status_page);

            // Actions
            let buttons_box = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .spacing(12)
                .halign(gtk::Align::Center)
                .build();
                
            let yes_btn = gtk::Button::builder()
                .label("Yes, I have it") // We can genericize the label from config if needed.
                .css_classes(vec!["suggested-action", "pill"])
                .width_request(200)
                .height_request(50)
                .build();
                
            let on_success = prereq.on_success.clone();
            let on_failure = prereq.on_failure.clone();
            let config_clone = config.clone();
            let self_clone = self.clone();
            
            yes_btn.connect_clicked(move |_| {
                self_clone.show_step(&on_success, config_clone.clone());
            });

            let no_btn = gtk::Button::builder()
                .label("No, I don't")
                .css_classes(vec!["pill"])
                .width_request(200)
                .height_request(50)
                .build();
                
            let config_clone2 = config.clone();
            let self_clone2 = self.clone();
            no_btn.connect_clicked(move |_| {
                 self_clone2.show_step(&on_failure, config_clone2.clone());
            });

            buttons_box.append(&yes_btn);
            buttons_box.append(&no_btn);
            
            // Hacky way to add buttons to status page since we can't easily add a child to StatusPage's child area programmatically if it's not set.
            // Actually StatusPage has a child property.
            status_page.set_child(Some(&buttons_box));
            
            page.set_child(Some(&box_container));
            self.imp().navigation_view.push(&page);
        }
    }

    fn show_step(&self, step_id: &str, config: Rc<InstallerConfig>) {
        if let Some(step) = config.steps.get(step_id) {
            match step {
                Step::Instruction { message, link, action_label } => {
                   self.show_instruction_step(step_id, message, link, action_label);
                },
                Step::Flash { url } => {
                    self.show_flash_step(step_id, url);
                }
            }
        }
    }

    fn show_instruction_step(&self, title: &str, message: &str, link: &Option<String>, action_label: &Option<String>) {
         let page = adw::NavigationPage::builder()
                .title(title)
                .build();

            let status_page = adw::StatusPage::builder()
                .title(title)
                .description(message)
                .icon_name("dialog-information-symbolic")
                .build();
            
             let box_container = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .spacing(24)
                .build();
                
            box_container.append(&status_page);
            
            if let (Some(url), Some(label)) = (link, action_label) {
                 let buttons_box = gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .spacing(12)
                    .halign(gtk::Align::Center)
                    .build();
                    
                let link_btn = gtk::Button::builder()
                    .label(label)
                    .css_classes(vec!["suggested-action", "pill"])
                    .width_request(200)
                    .height_request(50)
                    .build();
                    
                let url_string = url.clone();
                link_btn.connect_clicked(move |_| {
                    gtk::UriLauncher::new(&url_string).launch(None::<&gtk::Window>, None::<&gtk::gio::Cancellable>, |result| {
                        if let Err(e) = result {
                             log::warn!("Failed to launch URI: {}", e);
                        }
                    });
                });
                
                buttons_box.append(&link_btn);
                status_page.set_child(Some(&buttons_box));
            }
            
            page.set_child(Some(&box_container));
            self.imp().navigation_view.push(&page);
    }

    fn show_flash_step(&self, _title: &str, url: &str) {
         let page = adw::NavigationPage::builder()
                .title("Install")
                .build();

         let status_page = adw::StatusPage::builder()
                .title("Ready to Install")
                .description(&format!("Ready to download and flash from:\n{}", url))
                .icon_name("system-software-install-symbolic")
                .build();
        
         page.set_child(Some(&status_page));
         self.imp().navigation_view.push(&page);
         
         // TODO: Implement actual downloading and flashing logic here
         // For now just a placeholder
    }
}
