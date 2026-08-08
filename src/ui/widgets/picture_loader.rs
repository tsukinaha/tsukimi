use std::sync::LazyLock;

use super::{
    image_paintable::paintable_from_file,
    utils::{
        TU_ITEM_POST_SIZE,
        TU_ITEM_VIDEO_SIZE,
    },
};
use crate::{
    client::picture_source::PictureSource,
    utils::{
        resolve_picture_file,
        spawn,
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

pub mod imp {
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
        #[property(get, set)]
        pub tag: RefCell<String>,
        pub image_index: Cell<Option<u8>>,
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
    pub fn new_for_url(image_type: &str, url: &str) -> Self {
        Self::new_for_source(PictureSource::Url {
            image_type: image_type.to_string(),
            url: url.to_string(),
        })
    }

    pub(crate) fn new_for_source(source: PictureSource) -> Self {
        let obj: Self = match source {
            PictureSource::Item {
                id,
                tag,
                image_type,
                image_index,
            } => {
                let obj: Self = glib::Object::builder()
                    .property("id", id)
                    .property("imagetype", image_type)
                    .property("tag", tag)
                    .build();
                obj.imp().image_index.replace(image_index);
                obj
            }
            PictureSource::Url { image_type, url } => glib::Object::builder()
                .property("id", "")
                .property("imagetype", image_type)
                .property("url", url)
                .build(),
            PictureSource::User { .. } => unreachable!(),
        };
        obj
    }

    pub fn reload_for_url(&self, image_type: &str, url: &str) {
        self.reload_source(PictureSource::Url {
            image_type: image_type.to_string(),
            url: url.to_string(),
        });
    }

    pub fn reload_source(&self, source: PictureSource) {
        self.reset_view();
        match &source {
            PictureSource::Item {
                id,
                tag,
                image_type,
                image_index,
            } => {
                self.set_id(id.as_str());
                self.set_imagetype(image_type.as_str());
                self.set_tag(tag.as_str());
                self.imp().image_index.replace(*image_index);
                self.set_url(None::<String>);
            }
            PictureSource::Url { image_type, url } => {
                self.set_id("");
                self.set_imagetype(image_type.as_str());
                self.imp().image_index.replace(None);
                self.set_url(Some(url.as_str()));
            }
            PictureSource::User { .. } => unreachable!(),
        }
        self.load_source(source);
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

    fn load_source(&self, source: PictureSource) {
        let load_token = self.new_request();
        if let PictureSource::Url { image_type, .. } = &source {
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
    }

    async fn load_paintable(
        load_token: LoadToken, source: PictureSource,
    ) -> Result<gdk::Paintable> {
        if load_token.is_cancelled() {
            bail!("image load cancelled");
        }

        if matches!(&source, PictureSource::Item { .. }) {
            glib::timeout_future(IMAGE_LOAD_DELAY).await;
            if load_token.is_cancelled() {
                bail!("image load cancelled");
            }
        }

        let file = resolve_picture_file(source).await?;
        if load_token.is_cancelled() {
            bail!("image load cancelled");
        }
        Self::load_file(file, &load_token).await
    }

    async fn load_file(file: gio::File, load_token: &LoadToken) -> Result<gdk::Paintable> {
        let _permit = IMAGE_LOAD_SEMAPHORE.acquire().await?;
        paintable_from_file(file, Some(load_token.cancellable.clone())).await
    }

    fn image_source(&self) -> PictureSource {
        if let Some(url) = self.url() {
            PictureSource::Url {
                image_type: self.imagetype(),
                url,
            }
        } else {
            PictureSource::Item {
                id: self.id(),
                tag: self.tag(),
                image_type: self.imagetype(),
                image_index: self.imp().image_index.get(),
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
