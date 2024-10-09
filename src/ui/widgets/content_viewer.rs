use adw::{prelude::*, subclass::prelude::*};
use gtk::{gdk, gio, glib, glib::clone};
use tracing::warn;

use crate::utils::spawn;

use super::image_paintable::ImagePaintable;

mod imp {
    use glib::subclass::InitializingObject;
    use gtk::{glib, CompositeTemplate};
    use std::cell::Cell;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsukimi/content_viewer.ui")]
    #[properties(wrapper_type = super::MediaContentViewer)]
    pub struct MediaContentViewer {
        /// Whether to play the media content automatically.
        #[property(get, construct_only)]
        pub autoplay: Cell<bool>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub viewer: TemplateChild<adw::Bin>,
        #[template_child]
        pub fallback: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub spinner: TemplateChild<gtk::Spinner>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MediaContentViewer {
        const NAME: &'static str = "MediaContentViewer";
        type Type = super::MediaContentViewer;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.set_css_name("media-content-viewer");
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for MediaContentViewer {}

    impl WidgetImpl for MediaContentViewer {}
    impl BinImpl for MediaContentViewer {}
}

glib::wrapper! {
    /// Widget to view any media file.
    pub struct MediaContentViewer(ObjectSubclass<imp::MediaContentViewer>)
        @extends gtk::Widget, adw::Bin, @implements gtk::Accessible;
}

impl MediaContentViewer {
    pub fn new(autoplay: bool) -> Self {
        glib::Object::builder()
            .property("autoplay", autoplay)
            .build()
    }

    pub fn stop_playback(&self) {
        if let Some(stream) = self
            .imp()
            .viewer
            .child()
            .and_downcast::<gtk::Video>()
            .and_then(|v| v.media_stream())
        {
            if stream.is_playing() {
                stream.pause();
                stream.seek(0);
            }
        }
    }

    /// Show the loading screen.
    pub fn show_loading(&self) {
        self.imp().stack.set_visible_child_name("loading");
    }

    /// Show the viewer.
    fn show_viewer(&self) {
        self.imp().stack.set_visible_child_name("viewer");
    }

    /// Show the fallback message for the given content type.
    pub fn show_fallback(&self) {
        let imp = self.imp();
        let fallback = &imp.fallback;

        fallback.set_title("Image not Viewable");
        fallback.set_icon_name(Some("image-missing"));

        imp.stack.set_visible_child_name("fallback");
    }

    /// View the given image as bytes.
    ///
    /// If you have an image file, you can also use
    /// [`MediaContentViewer::view_file()`].
    pub fn view_image(&self, image: &impl IsA<gdk::Paintable>) {
        self.show_loading();

        let imp = self.imp();

        let picture = if let Some(picture) = imp.viewer.child().and_downcast::<gtk::Picture>() {
            picture
        } else {
            let picture = gtk::Picture::builder()
                .valign(gtk::Align::Fill)
                .halign(gtk::Align::Center)
                .build();
            imp.viewer.set_child(Some(&picture));
            picture
        };

        picture.set_paintable(Some(image));
        self.show_viewer();
    }

    /// View the given file.
    pub fn view_file(&self, file: gio::File) {
        self.show_loading();

        spawn(clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                obj.view_file_inner(file).await;
            }
        ));
    }

    async fn view_file_inner(&self, file: gio::File) {
        match ImagePaintable::from_file(&file) {
            Ok(texture) => {
                self.view_image(&texture);
                return;
            }
            Err(error) => {
                warn!("Could not load GdkTexture from file: {error}");
            }
        }

        self.show_fallback();
    }

    /// Get the texture displayed by this widget, if any.
    pub fn texture(&self) -> Option<gdk::Texture> {
        self.imp()
            .viewer
            .child()
            .and_downcast::<gtk::Picture>()
            .and_then(|p| p.paintable())
            .and_then(|p| p.downcast::<gdk::Texture>().ok())
    }
}
