use gettextrs::gettext;
use glib::Object;
use gtk::{
    gio,
    glib,
    glib::subclass::types::ObjectSubclassIsExt,
    prelude::*,
    template_callbacks,
};

use crate::{
    client::jellyfin_client::JELLYFIN_CLIENT,
    ui::{
        GlobalToast,
        widgets::window::Window,
    },
    utils::spawn_tokio,
};

mod imp {
    use std::cell::{
        OnceCell,
        RefCell,
    };

    use adw::subclass::prelude::*;
    use gettextrs::gettext;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib,
        prelude::*,
    };

    use crate::ui::provider::IS_ADMIN;

    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/image_info_card.ui")]
    #[properties(wrapper_type = super::ImageInfoCard)]
    pub struct ImageInfoCard {
        #[property(get, set, construct_only)]
        pub imgtype: OnceCell<String>,
        #[property(get, set, construct_only, default_value = false)]
        pub searchable: OnceCell<bool>,

        #[property(get, set)]
        pub imgid: RefCell<String>,
        #[property(get, set, default_value = 0)]
        pub image_index: RefCell<u8>,

        #[template_child]
        pub label1: TemplateChild<gtk::Label>,
        #[template_child]
        pub label2: TemplateChild<gtk::Label>,

        #[template_child]
        pub edit_menu_button: TemplateChild<gtk::MenuButton>,

        #[template_child]
        pub picture: TemplateChild<gtk::Picture>,

        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageInfoCard {
        const NAME: &'static str = "ImageInfoCard";
        type Type = super::ImageInfoCard;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();

            if IS_ADMIN.load(std::sync::atomic::Ordering::Relaxed) {
                klass.install_action(
                    "image.edit",
                    None,
                    move |image_info_card, _action, _parameter| {
                        image_info_card.on_edit();
                    },
                );
                klass.install_action_async(
                    "image.delete",
                    None,
                    |image_info_card, _action, _parameter| async move {
                        image_info_card.on_delete().await;
                    },
                );
                klass.install_action(
                    "image.search",
                    None,
                    move |image_info_card, _action, _parameter| {
                        image_info_card.on_search();
                    },
                );
            }
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ImageInfoCard {
        fn constructed(&self) {
            self.parent_constructed();

            self.label1.set_text(&gettext(self.obj().imgtype()));

            self.obj()
                .action_set_enabled("image.search", self.obj().searchable());
            self.obj()
                .action_set_enabled("image.edit", self.obj().searchable());
        }
    }

    impl WidgetImpl for ImageInfoCard {}

    impl BinImpl for ImageInfoCard {}
}

use adw::prelude::AdwDialogExt;

use super::ImageDialog;
glib::wrapper! {
    pub struct ImageInfoCard(ObjectSubclass<imp::ImageInfoCard>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget, adw::NavigationPage, adw::Bin,
        @implements gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

#[template_callbacks]
impl ImageInfoCard {
    pub fn new(img_type: &str, id: &str) -> Self {
        Object::builder()
            .property("imgtype", img_type)
            .property("imgid", id)
            .build()
    }

    #[template_callback]
    fn on_view(&self) {
        let window = self
            .ancestor(Window::static_type())
            .and_downcast::<Window>()
            .unwrap();
        let paintable = self.picture_paintable();
        if paintable.is_none() {
            self.toast(gettext("No image to view"));
            return;
        }
        window.media_viewer_show_paintable(paintable);
        window.reveal_image(&self.imp().picture.get());
        let dialog = self
            .ancestor(ImageDialog::static_type())
            .and_downcast::<ImageDialog>()
            .unwrap();
        dialog.close();
    }

    #[template_callback]
    fn on_copy(&self) {
        let clipboard = self.clipboard();
        let Some(texture) = self.picture_texture() else {
            self.toast(gettext("No image to copy"));
            return;
        };
        clipboard.set_texture(&texture);
        self.toast(gettext("Image copied to clipboard"));
    }

    fn on_search(&self) {
        if let Some(view) = self.navigation_view() {
            let page = super::ImageDialogSearchPage::new(&self.imgid(), &self.imgtype());
            view.push(&page);
        }
    }

    fn on_edit(&self) {
        if let Some(view) = self.navigation_view() {
            let page =
                super::ImageDialogEditPage::new(&self.imgid(), &self.imgtype(), self.image_index());
            view.push(&page);
        }
    }

    async fn on_delete(&self) {
        self.set_loading_visible();

        let id = self.imgid();
        let img_type = self.imgtype();
        let image_index = self.image_index();

        match spawn_tokio(async move {
            JELLYFIN_CLIENT
                .delete_image(&id, &img_type, Some(image_index))
                .await
        })
        .await
        {
            Ok(_) => {
                self.toast(gettext("Image deleted"));
                self.set_fallback_visible();
            }
            Err(e) => {
                tracing::error!("Error deleting image: {}", e);
                self.toast(gettext("Error deleting image"));
                self.set_picture_visible();
            }
        }
    }

    pub fn set_size(&self, width: &Option<u32>, height: &Option<u32>, size: &Option<u64>) {
        let Some(width) = width else {
            return;
        };
        let Some(height) = height else {
            return;
        };
        let mut str = format!("{width}x{height}");
        if let Some(size) = size {
            str.push_str(format!(" {}", bytefmt::format(*size)).as_str());
        }
        self.imp().label2.set_text(&str);
    }

    pub async fn set_picture(&self, img_type: &str, id: &str, image_index: &Option<u32>) {
        let path = JELLYFIN_CLIENT
            .get_image_path(id, img_type, *image_index)
            .await;

        let picture = self.imp().picture.get();

        gio::File::for_uri(&path).read_async(
            glib::Priority::LOW,
            None::<&gio::Cancellable>,
            glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |res| {
                    if let Ok(stream) = res {
                        gtk::gdk_pixbuf::Pixbuf::from_stream_async(
                            &stream,
                            None::<&gio::Cancellable>,
                            move |r| match r {
                                Ok(pixbuf) => {
                                    picture.set_paintable(Some(&gtk::gdk::Texture::for_pixbuf(
                                        &pixbuf,
                                    )));
                                    obj.set_picture_visible();
                                }
                                Err(_) => {
                                    obj.toast(gettext("Error loading image"));
                                    obj.set_fallback_visible();
                                }
                            },
                        );
                    }
                }
            ),
        );
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

    pub fn navigation_view(&self) -> Option<adw::NavigationView> {
        self.ancestor(adw::NavigationView::static_type())
            .and_downcast_ref::<adw::NavigationView>()
            .cloned()
    }
}
