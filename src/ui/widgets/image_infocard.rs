use glib::Object;
use gst::glib::Priority;
use gtk::glib::subclass::types::ObjectSubclassIsExt;
use gtk::prelude::*;
use gtk::template_callbacks;
use gtk::{gio, glib};

use crate::client::client::EMBY_CLIENT;
use crate::toast;
use crate::utils::spawn;

use super::image_dialog::ImagesDialog;
use super::window::Window;

mod imp {
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::{glib, CompositeTemplate};
    use std::cell::OnceCell;

    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/image_info_card.ui")]
    #[properties(wrapper_type = super::ImageInfoCard)]
    pub struct ImageInfoCard {
        #[property(get, set, construct_only)]
        pub imgtype: OnceCell<String>,

        #[template_child]
        pub label1: TemplateChild<gtk::Label>,
        #[template_child]
        pub label2: TemplateChild<gtk::Label>,

        #[template_child]
        pub picture: TemplateChild<gtk::Picture>,

        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for ImageInfoCard {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "ImageInfoCard";
        type Type = super::ImageInfoCard;
        type ParentType = adw::Bin;

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
    impl ObjectImpl for ImageInfoCard {
        fn constructed(&self) {
            self.parent_constructed();
            self.label1.set_text(&self.obj().imgtype());
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for ImageInfoCard {}

    impl BinImpl for ImageInfoCard {}
}

use adw::prelude::AdwDialogExt;
glib::wrapper! {
    pub struct ImageInfoCard(ObjectSubclass<imp::ImageInfoCard>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

#[template_callbacks]
impl ImageInfoCard {
    pub fn new(img_type: &str) -> Self {
        Object::builder().property("imgtype", img_type).build()
    }

    #[template_callback]
    fn on_view(&self) {
        let window = self
            .ancestor(Window::static_type())
            .and_downcast::<Window>()
            .unwrap();
        let paintable = self.picture_paintable();
        if paintable.is_none() {
            toast!(self, "No image to view");
            return;
        }
        window.media_viewer_show_paintable(paintable);
        window.reveal_image(&self.imp().picture.get());
        let dialog = self
            .ancestor(ImagesDialog::static_type())
            .and_downcast::<ImagesDialog>()
            .unwrap();
        dialog.close();
    }

    #[template_callback]
    fn on_copy(&self) {
        let clipboard = self.clipboard();
        let Some(texture) = self.picture_texture() else {
            toast!(self, "No image to copy");
            return;
        };
        clipboard.set_texture(&texture);
    }

    pub fn set_size(&self, width: &Option<u32>, height: &Option<u32>, size: &Option<u64>) {
        let Some(width) = width else {
            return;
        };
        let Some(height) = height else {
            return;
        };
        let mut str = format!("{}x{}", width, height);
        if let Some(size) = size {
            str.push_str(format!(" {}", bytefmt::format(*size)).as_str());
        }
        self.imp().label2.set_text(&str);
    }

    pub fn set_picture(&self, img_type: &str, id: &str, image_index: &Option<u32>) {
        let path = EMBY_CLIENT.get_image_path(id, img_type, *image_index);

        let picture = self.imp().picture.get();
        spawn(glib::clone!(@weak picture, @weak self as obj =>async move {
            let file = gio::File::for_uri(&path).read_future(Priority::default()).await.ok();
            if let Some(stream) = file {
                match gtk::gdk_pixbuf::Pixbuf::from_stream(&stream, None::<&gio::Cancellable>) {
                    Ok(pixbuf) => {
                        picture.set_paintable(Some(&gtk::gdk::Texture::for_pixbuf(&pixbuf)));
                        obj.set_picture_visible();
                    },
                    Err(_) => {
                        toast!(obj, "Error loading image");
                        obj.set_fallback_visible();
                    }
                }
            }
        }));
    }

    pub fn set_picture_visible(&self) {
        self.imp().stack.set_visible_child_name("picture")
    }

    pub fn set_fallback_visible(&self) {
        self.imp().stack.set_visible_child_name("fallback")
    }

    pub fn set_loading_visible(&self) {
        self.imp().stack.set_visible_child_name("loading")
    }

    pub fn picture_paintable(&self) -> Option<gtk::gdk::Paintable> {
        self.imp().picture.paintable()
    }

    pub fn picture_texture(&self) -> Option<gtk::gdk::Texture> {
        self.imp()
            .picture
            .paintable()
            .and_then(|paintable| paintable.downcast::<gtk::gdk::Texture>().ok())
    }
}
