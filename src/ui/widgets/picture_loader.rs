use std::path::PathBuf;

use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gtk::{
    gio,
    glib::{
        self,
        clone,
    },
    CompositeTemplate,
};
use tracing::{
    debug,
    warn,
};

use super::image_paintable::ImagePaintable;
use crate::{
    client::emby_client::EMBY_CLIENT,
    ui::models::emby_cache_path,
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
            self.obj().load_pic();
        }
    }

    impl WidgetImpl for PictureLoader {}
    impl BinImpl for PictureLoader {}
}

glib::wrapper! {
    pub struct PictureLoader(ObjectSubclass<imp::PictureLoader>)
        @extends gtk::Widget, @implements gtk::Accessible;
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

    pub fn load_pic(&self) {
        let cache_file_path = self.cache_file();

        if cache_file_path.exists() {
            self.reveal_picture(cache_file_path);
        } else {
            self.get_file(cache_file_path);
        }
    }

    pub fn reveal_picture(&self, cache_file_path: PathBuf) {
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
            imp.broken.set_visible(true);
        }

        imp.spinner.set_visible(false);

        imp.revealer.set_reveal_child(true);
    }

    pub fn cache_file(&self) -> PathBuf {
        let cache_path = emby_cache_path();
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
                    let tag = tag.clone();
                    if let Err(e) = EMBY_CLIENT
                        .get_image(&id, &image_type, tag.and_then(|s| s.parse::<u8>().ok()))
                        .await
                    {
                        warn!("Failed to get image: {}", e);
                    }
                })
                .await;
                debug!("Setting image: {}", &pathbuf.display());
                obj.reveal_picture(pathbuf);
            }
        ));
    }
}
