use crate::client::network::{positionback, RUNTIME};
use crate::client::structs::Back;
use crate::config::set_config;
use crate::toast;
use crate::ui::widgets::song_widget::format_duration;
use crate::ui::widgets::window::Window;
use clapper::{AudioStream, VideoStream};
use glib::Object;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

mod imp {

    use std::cell::RefCell;

    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    use crate::client::structs::Back;

    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/clapperpage.ui")]
    #[properties(wrapper_type = super::ClapperPage)]
    pub struct ClapperPage {
        #[property(get, set, nullable)]
        pub url: RefCell<Option<String>>,
        #[template_child]
        pub video: TemplateChild<clapper_gtk::Video>,
        #[template_child]
        pub mediainfo: TemplateChild<gtk::Label>,
        #[template_child]
        pub title: TemplateChild<gtk::Label>,
        #[template_child]
        pub secondtitle: TemplateChild<gtk::Label>,
        #[template_child]
        pub header: TemplateChild<gtk::Box>,
        pub mediaitem: RefCell<Option<clapper::MediaItem>>,
        pub timeout: RefCell<Option<glib::source::SourceId>>,
        pub back: RefCell<Option<Back>>,
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
            klass.bind_template_instance_callbacks();
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

impl Default for ClapperPage {
    fn default() -> Self {
        Self::new()
    }
}

#[gtk::template_callbacks]
impl ClapperPage {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn add_item(
        &self,
        url: &str,
        suburi: Option<&str>,
        name: Option<&str>,
        line2: Option<&str>,
        back: Option<Back>,
    ) {
        let imp = self.imp();
        let server_info = set_config();
        let url = format!("{}:{}/emby{}", server_info.domain, server_info.port, url);
        let item = clapper::MediaItem::builder().uri(url).build();

        if suburi.is_some() {
            toast!(self,"External subtitles not supported, see https://gitlab.freedesktop.org/gstreamer/gst-plugins-base/-/issues/36")
        }

        imp.back.replace(back);

        imp.mediaitem.replace(Some(item.clone()));
        imp.title.set_text(name.unwrap_or("Unknown"));
        if let Some(line2) = line2 {
            imp.secondtitle.set_text(line2);
        } else {
            imp.secondtitle.set_visible(false);
        }

        imp.video.player().unwrap().queue().unwrap().add_item(&item);
        imp.video
            .player()
            .unwrap()
            .queue()
            .unwrap()
            .select_item(Some(&item));
        imp.video.player().unwrap().play();
        self.update_timeout();
    }

    pub fn bind_fullscreen(&self, window: &Window) {
        let window_clone = window.clone();
        self.imp().video.connect_toggle_fullscreen(move |_video| {
            window_clone.set_fullscreened(!window_clone.is_fullscreened());
        });
    }

    pub fn on_button_clicked(&self) {
        self.imp().video.player().unwrap().stop();
        self.remove_timeout();
        let window = self.root().unwrap().downcast::<Window>().unwrap();
        window.mainpage();
    }

    #[template_callback]
    fn on_play_pause_toggled(&self) {
        let imp = self.imp();
        let player = imp.video.player().unwrap();
        let item = player.video_streams().unwrap().item(0);
        if let Some(video_stream) = item.and_downcast_ref::<VideoStream>() {
            let item = player.audio_streams().unwrap().item(0);
            if let Some(audio_stream) = item.and_downcast_ref::<AudioStream>() {
                let mediaitembind = imp.mediaitem.borrow();
                let mediaitem = mediaitembind.as_ref().unwrap();

                let mediainfo = &self.imp().mediainfo;
                let text = &format!(
                    " <b>Media</b> \n Container Format: {} \n Duration: {} \n\n <b>Video</b> \n Bitrate: {} kbps \n Framerate: {} \n Codec: {} \n Pixel Format: {} \n Resolution: {}x{} \n\n <b>Audio</b> \n Audio Codec: {} \n Channels: {} \n Sample Rate: {} Hz \n Bitrate: {} kbps \n Sample Format: {} \n\n <b>Video Playback</b> \n Decoder: {} \n Filter: {} \n Sink: {} \n\n <b>Audio Playback</b> \n Decoder: {} \n Filter: {} \n Sink: {} ",
                    mediaitem.container_format().unwrap_or("Unknown".into()),
                    &format_duration(mediaitem.duration() as i64),
                    video_stream.bitrate() / 1000,
                    video_stream.fps(),
                    video_stream.codec().unwrap_or("Unknown".into()),
                    video_stream.pixel_format().unwrap_or("Unknown".into()),
                    video_stream.width(),
                    video_stream.height(),
                    audio_stream.codec().unwrap_or("Unknown".into()),
                    audio_stream.channels(),
                    audio_stream.sample_rate(),
                    audio_stream.bitrate() / 1000,
                    audio_stream.sample_format().unwrap_or("Unknown".into()),
                    player.current_video_decoder().map_or("Unknown".to_string(), |decoder| decoder.type_().to_string()),
                    player.video_filter().map_or("-".to_string(), |filter| filter.type_().to_string()),
                    player.video_sink().map_or("Unknown".to_string(), |sink| sink.type_().to_string()),
                    player.current_audio_decoder().map_or("Unknown".to_string(), |decoder| decoder.type_().to_string()),
                    player.audio_filter().map_or("-".to_string(), |filter| filter.type_().to_string()),
                    player.audio_sink().map_or("Unknown".to_string(), |sink| sink.type_().to_string())
                );
                mediainfo.set_markup(text);
                mediainfo.set_visible(!mediainfo.is_visible())
            }
        }
    }

    pub fn update_position_callback(&self) -> glib::ControlFlow {
        let position = &self.imp().video.player().unwrap().position();
        let back = self.imp().back.borrow();
        if *position > 0.0 {
            if let Some(back) = back.as_ref() {
                let duration = *position as u64 * 10000000;
                let mut back = back.clone();
                back.tick = duration;
                RUNTIME.spawn(async move {
                    positionback(back).await;
                });
            }
        }
        glib::ControlFlow::Continue
    }

    pub fn update_timeout(&self) {
        if let Some(timeout) = self.imp().timeout.borrow_mut().take() {
            glib::source::SourceId::remove(timeout);
        }
        self.imp().timeout.replace(Some(glib::timeout_add_local(
            std::time::Duration::from_secs(10),
            glib::clone!(@strong self as obj => move || obj.update_position_callback()),
        )));
    }

    pub fn remove_timeout(&self) {
        if let Some(timeout) = self.imp().timeout.borrow_mut().take() {
            glib::source::SourceId::remove(timeout);
        }
    }
}
