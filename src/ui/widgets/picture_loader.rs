use std::{
    path::PathBuf,
    sync::LazyLock,
};

use super::{
    image_paintable::paintable_from_file,
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
use adw::{
    prelude::*,
    subclass::prelude::*,
};
use anyhow::{
    Result,
    bail,
};
use gtk::{
    CompositeTemplate,
    gdk,
    gio,
    glib,
};
use tracing::warn;

const IMAGE_LOAD_DELAY: std::time::Duration = std::time::Duration::from_millis(80);
static IMAGE_LOAD_SEMAPHORE: LazyLock<tokio::sync::Semaphore> = LazyLock::new(|| {
    tokio::sync::Semaphore::new(
        std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4),
    )
});

#[derive(Clone)]
struct LoadToken {
    cancellable: gio::Cancellable,
    generation: u64,
}

impl LoadToken {
    fn is_cancelled(&self) -> bool {
        self.cancellable.is_cancelled()
    }

    fn is_current_for(&self, loader: &PictureLoader) -> bool {
        !self.is_cancelled() && loader.imp().generation.get() == self.generation
    }
}

#[derive(Clone)]
enum ImageSource {
    Item {
        id: String,
        image_type: String,
        tag: Option<String>,
    },
    Url {
        image_type: String,
        url: String,
    },
}

pub(crate) mod imp {
    use std::cell::{
        Cell,
        RefCell,
    };

    use glib::subclass::InitializingObject;

    use super::*;

    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/picture_loader.ui")]
    #[properties(wrapper_type = super::PictureLoader)]
    pub struct PictureLoader {
        #[property(get, set)]
        pub id: RefCell<String>,
        #[property(get, set)]
        pub imagetype: RefCell<String>,
        #[property(get, set, nullable)]
        pub tag: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        pub url: RefCell<Option<String>>,
        #[template_child]
        pub revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub picture: TemplateChild<gtk::Picture>,
        #[template_child]
        pub spinner: TemplateChild<adw::Spinner>,
        #[template_child]
        pub broken: TemplateChild<gtk::Box>,
        pub cancellable: RefCell<Option<gio::Cancellable>>,
        pub generation: Cell<u64>,
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

            // Wait until builder properties are applied before reading the image source
            glib::idle_add_local_once(glib::clone!(
                #[weak(rename_to = obj)]
                self.obj(),
                move || {
                    obj.load_source(obj.image_source());
                }
            ));
        }

        fn dispose(&self) {
            self.obj().cancel_current_request();
        }
    }

    impl WidgetImpl for PictureLoader {}
    impl BinImpl for PictureLoader {}
}

glib::wrapper! {
    pub struct PictureLoader(ObjectSubclass<imp::PictureLoader>)
        @extends gtk::Widget, adw::Bin, @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl PictureLoader {
    pub fn new(id: &str, image_type: &str, tag: Option<String>) -> Self {
        glib::Object::builder()
            .property("id", id)
            .property("imagetype", image_type)
            .property("tag", tag)
            .build()
    }

    pub fn new_for_url(image_type: &str, url: &str) -> Self {
        glib::Object::builder()
            .property("id", "")
            .property("imagetype", image_type)
            .property("url", url)
            .build()
    }

    pub fn reload(&self, id: &str, image_type: &str, tag: Option<String>) {
        self.reset_view();
        self.set_id(id);
        self.set_imagetype(image_type);
        self.set_tag(tag);
        self.set_url(None::<String>);
        self.load_source(self.image_source());
    }

    pub fn reset(&self) {
        self.cancel_current_request();
        self.reset_view();
    }

    fn reset_view(&self) {
        let imp = self.imp();
        imp.revealer.set_reveal_child(false);
        imp.broken.set_visible(false);
        imp.spinner.set_visible(true);
        imp.picture.set_paintable(None::<&gdk::Paintable>);
    }

    pub fn reset_in(widget: &gtk::Widget) {
        if let Some(picture_loader) = widget.downcast_ref::<Self>() {
            picture_loader.reset();
            return;
        }

        if let Some(bin) = widget.downcast_ref::<adw::Bin>()
            && let Some(child) = bin.child()
        {
            Self::reset_in(&child);
        }
    }

    fn load_source(&self, source: ImageSource) {
        let load_token = self.new_request();
        if let ImageSource::Url { image_type, .. } = &source {
            self.configure_picture_size(image_type);
        }
        let weak_self = self.downgrade();
        spawn(async move {
            let paintable = Self::load_paintable(load_token.clone(), source).await;
            let Some(obj) = weak_self.upgrade() else {
                return;
            };
            if !load_token.is_current_for(&obj) {
                return;
            }
            if let Ok(paintable) = paintable {
                obj.show_paintable(&paintable, &load_token);
            } else {
                obj.show_broken(&load_token);
            }
        });
    }

    fn new_request(&self) -> LoadToken {
        let generation = self.cancel_current_request();
        let cancellable = gio::Cancellable::new();
        self.imp().cancellable.replace(Some(cancellable.clone()));
        LoadToken {
            cancellable,
            generation,
        }
    }

    fn cancel_current_request(&self) -> u64 {
        if let Some(cancellable) = self.imp().cancellable.borrow_mut().take() {
            cancellable.cancel();
        }
        let generation = self.imp().generation.get().wrapping_add(1);
        self.imp().generation.set(generation);
        generation
    }

    fn configure_picture_size(&self, image_type: &str) {
        let size = match image_type {
            "Primary" => &TU_ITEM_POST_SIZE,
            _ => &TU_ITEM_VIDEO_SIZE,
        };
        self.imp().picture.set_width_request(size.0);
        self.imp().picture.set_height_request(size.1);
        self.imp().picture.set_content_fit(gtk::ContentFit::Contain);
    }

    async fn load_paintable(load_token: LoadToken, source: ImageSource) -> Result<gdk::Paintable> {
        match source {
            ImageSource::Url { url, .. } => {
                Self::load_file(gio::File::for_uri(&url), &load_token).await
            }
            ImageSource::Item {
                id,
                image_type,
                tag,
            } => {
                glib::timeout_future(IMAGE_LOAD_DELAY).await;
                if load_token.is_cancelled() {
                    bail!("image load cancelled");
                }
                let cache_path = Self::cache_file(&id, &image_type, tag.as_deref()).await;
                let file = gio::File::for_path(&cache_path);

                if let Ok(paintable) = Self::load_file(file.clone(), &load_token).await {
                    Ok(paintable)
                } else {
                    Self::download_image(&id, &image_type, tag.as_deref()).await;
                    if load_token.is_cancelled() {
                        bail!("image load cancelled");
                    }
                    Self::load_file(file, &load_token).await
                }
            }
        }
    }

    async fn load_file(file: gio::File, load_token: &LoadToken) -> Result<gdk::Paintable> {
        let _permit = IMAGE_LOAD_SEMAPHORE.acquire().await?;
        paintable_from_file(file, Some(load_token.cancellable.clone())).await
    }

    async fn cache_file(id: &str, image_type: &str, tag: Option<&str>) -> PathBuf {
        let cache_path = jellyfin_cache_path().await;
        let path = format!("{}-{}-{}", id, image_type, tag.unwrap_or("0"));
        cache_path.join(path)
    }

    async fn download_image(id: &str, image_type: &str, tag: Option<&str>) {
        let id = id.to_string();
        let image_type = image_type.to_string();
        let tag = tag.map(str::to_string);
        let warn_id = id.clone();
        let warn_image_type = image_type.clone();

        let result = spawn_tokio(async move {
            JELLYFIN_CLIENT
                .get_image(&id, &image_type, tag.and_then(|s| s.parse::<u8>().ok()))
                .await
        })
        .await;

        if let Err(error) = result {
            warn!("{}: {}-{}", error, warn_id, warn_image_type);
        }
    }

    fn image_source(&self) -> ImageSource {
        if let Some(url) = self.url() {
            ImageSource::Url {
                image_type: self.imagetype(),
                url,
            }
        } else {
            ImageSource::Item {
                id: self.id(),
                image_type: self.imagetype(),
                tag: self.tag(),
            }
        }
    }

    fn show_paintable(&self, paintable: &gdk::Paintable, load_token: &LoadToken) {
        if !load_token.is_current_for(self) {
            return;
        }
        let imp = self.imp();
        imp.picture.set_paintable(Some(paintable));
        imp.spinner.set_visible(false);
        imp.revealer.set_reveal_child(true);
    }

    fn show_broken(&self, load_token: &LoadToken) {
        if !load_token.is_current_for(self) {
            return;
        }
        let imp = self.imp();
        imp.broken.set_visible(true);
        imp.spinner.set_visible(false);
        imp.revealer.set_reveal_child(true);
    }
}
