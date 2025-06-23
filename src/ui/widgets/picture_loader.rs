use std::path::PathBuf;

use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gtk::{
    CompositeTemplate,
    gio,
    glib::{
        self,
        clone,
    },
};
use tracing::{
    debug,
    warn,
};

use super::{
    image_paintable::ImagePaintable,
    utils::{
        TU_ITEM_POST_SIZE,
        TU_ITEM_VIDEO_SIZE,
    },
};
use crate::{
    client::jellyfin_client::JELLYFIN_CLIENT,
    ui::models::jellyfin_cache_path,
    utils::{
        spawn,
        spawn_tokio,
    },
};

pub(crate) mod imp {
    use std::cell::{
        OnceCell,
        RefCell,
    };

    use glib::subclass::InitializingObject;

    use super::*;

    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/picture_loader.ui")]
    #[properties(wrapper_type = super::PictureLoader)]
    pub struct PictureLoader {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub imagetype: OnceCell<String>,
        #[property(get, set, nullable, construct_only)]
        pub tag: RefCell<Option<String>>,
        #[property(get, set, nullable, construct_only)]
        pub url: RefCell<Option<String>>,
        #[template_child]
        pub revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub picture: TemplateChild<gtk::Picture>,
        #[template_child]
        pub spinner: TemplateChild<adw::Spinner>,
        #[template_child]
        pub broken: TemplateChild<gtk::Box>,
        #[property(get, set, default = false, construct_only)]
        pub animated: std::cell::Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PictureLoader {
        const NAME: &'static str = "PictureLoader";
        type Type = super::PictureLoader;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for PictureLoader {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            if let Some(url) = obj.url() {
                obj.load_pic_for_url(url);
            } else {
                spawn(clone!(
                    #[weak]
                    obj,
                    async move {
                        obj.load_pic().await;
                    }
                ));
            }
        }
    }

    impl WidgetImpl for PictureLoader {}
    impl BinImpl for PictureLoader {}
}

glib::wrapper! {
    pub struct PictureLoader(ObjectSubclass<imp::PictureLoader>)
        @extends gtk::Widget, adw::Bin, @implements gtk::Accessible;
}

impl PictureLoader {
    pub fn new(id: &str, image_type: &str, tag: Option<String>) -> Self {
        glib::Object::builder()
            .property("id", id)
            .property("imagetype", image_type)
            .property("tag", tag)
            .build()
    }

    pub fn new_animated(id: &str, image_type: &str, tag: Option<String>) -> Self {
        glib::Object::builder()
            .property("id", id)
            .property("imagetype", image_type)
            .property("tag", tag)
            .property("animated", true)
            .build()
    }

    pub fn new_for_url(image_type: &str, url: &str) -> Self {
        glib::Object::builder()
            .property("id", "")
            .property("imagetype", image_type)
            .property("url", url)
            .build()
    }

    // for EuListItem
    pub fn load_pic_for_url(&self, url: String) {
        let size = match self.imagetype().as_str() {
            "Primary" => &TU_ITEM_POST_SIZE,
            _ => &TU_ITEM_VIDEO_SIZE,
        };

        self.imp().picture.set_width_request(size.0);
        self.imp().picture.set_height_request(size.1);
        self.imp().picture.set_content_fit(gtk::ContentFit::Contain);

        gio::File::for_uri(&url).read_async(
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
                                    obj.imp().picture.set_paintable(Some(
                                        &gtk::gdk::Texture::for_pixbuf(&pixbuf),
                                    ));
                                    obj.imp().spinner.set_visible(false);
                                    obj.imp().revealer.set_reveal_child(true);
                                }
                                Err(_) => {
                                    obj.imp().broken.set_visible(true);
                                }
                            },
                        );
                    }
                }
            ),
        );
    }

    pub async fn load_pic(&self) {
        let cache_file_path = self.cache_file().await;
        self.reveal_picture::<true>(&cache_file_path);
        self.get_file(cache_file_path);
    }

    pub fn reveal_picture<const R: bool>(&self, cache_file_path: &PathBuf) {
        let imp = self.imp();

        if cache_file_path.exists() {
            let file = gio::File::for_path(cache_file_path);
            if self.animated() {
                let paintable = ImagePaintable::from_file(&file);
                imp.picture.set_paintable(paintable.ok().as_ref());
            } else {
                imp.picture.set_file(Some(&file));
            }
        } else {
            if R {
                return;
            }
            imp.broken.set_visible(true);
        }

        imp.spinner.set_visible(false);

        spawn(clone!(
            #[weak]
            imp,
            async move {
                imp.revealer.set_reveal_child(true);
            }
        ));
    }

    pub async fn cache_file(&self) -> PathBuf {
        let cache_path = jellyfin_cache_path().await;
        let path = format!(
            "{}-{}-{}",
            self.id(),
            self.imagetype(),
            self.tag().unwrap_or("0".to_string())
        );
        cache_path.join(path)
    }

    pub fn get_file(&self, pathbuf: PathBuf) {
        let id = self.id();
        let image_type = self.imagetype();
        let tag = self.tag();
        spawn(clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                spawn_tokio(async move {
                    let tag = tag.to_owned();
                    if let Err(e) = JELLYFIN_CLIENT
                        .get_image(&id, &image_type, tag.and_then(|s| s.parse::<u8>().ok()))
                        .await
                    {
                        warn!("{}: {}-{}", e, id, image_type);
                    }
                })
                .await;
                debug!("Setting image: {}", &pathbuf.display());
                obj.reveal_picture::<false>(&pathbuf);
            }
        ));
    }
}
