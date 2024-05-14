use crate::config::set_config;
use crate::ui::widgets::window::Window;
use glib::Object;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use gst::prelude::*;

mod imp {

    use std::cell::RefCell;
    
    use gtk::prelude::*;
    use clapper_gtk::*;
    use glib::subclass::InitializingObject;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/clapperpage.ui")]
    #[properties(wrapper_type = super::ClapperPage)]
    pub struct ClapperPage {
        #[property(get, set, nullable)]
        pub url: RefCell<Option<String>>,
        #[template_child]
        pub video: TemplateChild<clapper_gtk::Video>,
        pub buffering: RefCell<Option<glib::SignalHandlerId>>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for ClapperPage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "ClapperPage";
        type Type = super::ClapperPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    #[glib::derived_properties]
    impl ObjectImpl for ClapperPage {
        fn constructed(&self) {
            self.parent_constructed();
            
            let header = TitleHeader::new();
            header.set_valign(gtk::Align::Start);
            self.video.add_fading_overlay(&header);

            let controls = SimpleControls::new();
            controls.set_valign(gtk::Align::End);
            self.video.add_fading_overlay(&controls);

            let backbutton = gtk::Button::builder()
                .icon_name("go-previous-symbolic")
                .valign(gtk::Align::Start)
                .halign(gtk::Align::Start)
                .margin_top(7)
                .margin_start(7)
                .build();

            backbutton.add_css_class("osd");
            backbutton.add_css_class("circular");
            backbutton.connect_clicked(glib::clone!(@weak self as imp => move |_| {
                imp.obj().on_button_clicked();
            }));
            self.video.add_fading_overlay(&backbutton);
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for ClapperPage {}

    // Trait shared by all windows
    impl WindowImpl for ClapperPage {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for ClapperPage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for ClapperPage {}
}

glib::wrapper! {
    pub struct ClapperPage(ObjectSubclass<imp::ClapperPage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl ClapperPage {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn add_item(&self, url: &str, suburi: Option<&str>) {
        let server_info = set_config();
        let url = format!("{}:{}/emby{}", server_info.domain, server_info.port, url);
        let item = clapper::MediaItem::new(&url);
        if let Some(suburi) = suburi {
            let suburi = format!("{}:{}/emby{}", server_info.domain, server_info.port, suburi);
            item.set_suburi(&suburi);
        }
        self.imp()
            .video
            .player()
            .unwrap()
            .queue()
            .unwrap()
            .add_item(&item);
        self.imp()
            .video
            .player()
            .unwrap()
            .queue()
            .unwrap()
            .select_item(Some(&item));
        self.imp().video.player().unwrap().play();

    }

    pub fn bind_fullscreen(&self, window: &Window) {
        let window_clone = window.clone();
        self.imp().video.connect_toggle_fullscreen(move |_video| {
            window_clone.set_fullscreened(!window_clone.is_fullscreened());
        });
    }

    pub fn on_button_clicked(&self) {
        self.imp().video.player().unwrap().stop();
        let window = self.root().unwrap().downcast::<Window>().unwrap();
        window.mainpage();
    }

    pub fn add_msg(&self) {
        let player = self.imp().video.player().unwrap();
        let element = player.video_sink().unwrap();
        if let Some(buffering) = self.imp().buffering.take() {
            element.bus().unwrap().disconnect(buffering);
        }
        
            let bus = element.bus().unwrap();
            bus.add_signal_watch();
            let buffering = bus.connect_message(Some("buffering"), {
                move |_bus, msg| {
                    match msg.view() {
                        gst::MessageView::Buffering(buffering) => {
                            let percent = buffering.percent();
                            if percent < 100 {
                                let _ = element.set_state(gst::State::Paused);
                            } else {
                                let _ = element.set_state(gst::State::Playing);
                            }
                        }
                        _ => (),
                    }
                }
            });
            self.imp().buffering.replace(Some(buffering));
    }
}
