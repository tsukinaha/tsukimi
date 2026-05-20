use std::{
    path::PathBuf,
    sync::{
        LazyLock,
        atomic::{
            AtomicUsize,
            Ordering,
        },
    },
};

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
    image_paintable::{
        DecodedPaintable,
        ImagePaintable,
    },
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

const IMAGE_LOAD_DELAY: std::time::Duration = std::time::Duration::from_millis(80);
const IMAGE_DECODE_RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(120);
static MAX_IMAGE_DECODE_TASKS: LazyLock<usize> = LazyLock::new(rayon::current_num_threads);
static IMAGE_DECODE_TASKS: AtomicUsize = AtomicUsize::new(0);

enum LoadedImage {
    Texture(gtk::gdk::Texture),
    Decoded(DecodedPaintable),
}

struct DecodePermit;

impl Drop for DecodePermit {
    fn drop(&mut self) {
        IMAGE_DECODE_TASKS.fetch_sub(1, Ordering::Release);
    }
}

fn try_acquire_decode_permit() -> Option<DecodePermit> {
    let mut current = IMAGE_DECODE_TASKS.load(Ordering::Relaxed);
    loop {
        if current >= *MAX_IMAGE_DECODE_TASKS {
            return None;
        }

        match IMAGE_DECODE_TASKS.compare_exchange_weak(
            current,
            current + 1,
            Ordering::Acquire,
            Ordering::Relaxed,
        ) {
            Ok(_) => return Some(DecodePermit),
            Err(next) => current = next,
        }
    }
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
        #[property(get, set, default = false)]
        pub animated: Cell<bool>,
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

            let obj = self.obj();

            if let Some(url) = obj.url() {
                obj.start_url_loading(url);
            } else {
                obj.start_loading();
            }
        }

        fn dispose(&self) {
            self.obj().cancel_loading();
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

    pub fn reload(&self, id: &str, image_type: &str, tag: Option<String>, animated: bool) {
        self.reset();
        self.set_id(id);
        self.set_imagetype(image_type);
        self.set_tag(tag);
        self.set_url(None::<String>);
        self.set_animated(animated);
        self.start_loading();
    }

    pub fn reset(&self) {
        self.cancel_loading();
        self.imp()
            .picture
            .set_paintable(None::<&gtk::gdk::Paintable>);
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

    fn start_loading(&self) {
        let (cancellable, generation) = self.new_request();
        spawn(clone!(
            #[weak(rename_to = obj)]
            self,
            #[strong]
            cancellable,
            async move {
                glib::timeout_future(IMAGE_LOAD_DELAY).await;
                if !obj.is_current(&cancellable, generation) {
                    return;
                }
                obj.load_pic(cancellable, generation).await;
            }
        ));
    }

    fn start_url_loading(&self, url: String) {
        let (cancellable, generation) = self.new_request();
        self.load_pic_for_url(url, cancellable, generation);
    }

    fn new_request(&self) -> (gio::Cancellable, u64) {
        if let Some(cancellable) = self.imp().cancellable.borrow_mut().take() {
            cancellable.cancel();
        }

        let generation = self.imp().generation.get().wrapping_add(1);
        self.imp().generation.set(generation);

        let cancellable = gio::Cancellable::new();
        self.imp().cancellable.replace(Some(cancellable.clone()));
        (cancellable, generation)
    }

    pub fn cancel_loading(&self) {
        if let Some(cancellable) = self.imp().cancellable.borrow_mut().take() {
            cancellable.cancel();
        }

        let generation = self.imp().generation.get().wrapping_add(1);
        self.imp().generation.set(generation);
    }

    fn is_current(&self, cancellable: &gio::Cancellable, generation: u64) -> bool {
        !cancellable.is_cancelled() && self.imp().generation.get() == generation
    }

    // for EuListItem
    pub fn load_pic_for_url(&self, url: String, cancellable: gio::Cancellable, generation: u64) {
        let size = match self.imagetype().as_str() {
            "Primary" => &TU_ITEM_POST_SIZE,
            _ => &TU_ITEM_VIDEO_SIZE,
        };

        self.imp().picture.set_width_request(size.0);
        self.imp().picture.set_height_request(size.1);
        self.imp().picture.set_content_fit(gtk::ContentFit::Contain);

        let file = gio::File::for_uri(&url);
        self.load_file_bytes(file, None, cancellable, generation, false);
    }

    pub async fn load_pic(&self, cancellable: gio::Cancellable, generation: u64) {
        if !self.is_current(&cancellable, generation) {
            return;
        }

        let cache_file_path = self.cache_file().await;
        self.reveal_picture(cache_file_path, cancellable, generation, true);
    }

    pub fn reveal_picture(
        &self, cache_file_path: PathBuf, cancellable: gio::Cancellable, generation: u64,
        fetch_on_error: bool,
    ) {
        let file = gio::File::for_path(&cache_file_path);
        self.load_file_bytes(
            file,
            Some(cache_file_path),
            cancellable,
            generation,
            fetch_on_error,
        );
    }

    fn set_picture_visible(&self, cancellable: gio::Cancellable, generation: u64) {
        if !self.is_current(&cancellable, generation) {
            return;
        }

        let imp = self.imp();
        imp.spinner.set_visible(false);
        spawn(clone!(
            #[weak]
            imp,
            #[strong]
            cancellable,
            async move {
                if cancellable.is_cancelled() {
                    return;
                }
                imp.revealer.set_reveal_child(true);
            }
        ));
    }

    fn set_broken(&self, cancellable: gio::Cancellable, generation: u64) {
        if !self.is_current(&cancellable, generation) {
            return;
        }

        let imp = self.imp();
        imp.broken.set_visible(true);
        imp.spinner.set_visible(false);
        spawn(clone!(
            #[weak]
            imp,
            #[strong]
            cancellable,
            async move {
                if cancellable.is_cancelled() {
                    return;
                }
                imp.revealer.set_reveal_child(true);
            }
        ));
    }

    fn load_file_bytes(
        &self, file: gio::File, cache_file_path: Option<PathBuf>, cancellable: gio::Cancellable,
        generation: u64, fetch_on_error: bool,
    ) {
        if !self.is_current(&cancellable, generation) {
            return;
        }

        let weak_self: glib::SendWeakRef<Self> = self.downgrade().into();
        let read_cancellable = cancellable.clone();
        file.load_bytes_async(Some(&cancellable), move |res| {
            let cancellable = read_cancellable;

            if cancellable.is_cancelled() {
                return;
            }

            let Ok((bytes, _etag)) = res else {
                let weak_self = weak_self.into_weak_ref();
                let Some(obj) = weak_self.upgrade() else {
                    return;
                };

                if !obj.is_current(&cancellable, generation) {
                    return;
                }

                if fetch_on_error {
                    if let Some(path) = cache_file_path {
                        obj.get_file(path, cancellable, generation);
                    }
                } else {
                    obj.set_broken(cancellable, generation);
                }
                return;
            };

            let animated = {
                let weak_self = weak_self.clone().into_weak_ref();
                let Some(obj) = weak_self.upgrade() else {
                    return;
                };

                if !obj.is_current(&cancellable, generation) {
                    return;
                }

                obj.animated()
            };

            Self::try_spawn_decode(
                weak_self,
                bytes,
                animated,
                cache_file_path,
                cancellable,
                generation,
                fetch_on_error,
            );
        });
    }

    fn try_spawn_decode(
        weak_self: glib::SendWeakRef<Self>, bytes: glib::Bytes, animated: bool,
        cache_file_path: Option<PathBuf>, cancellable: gio::Cancellable, generation: u64,
        fetch_on_error: bool,
    ) {
        let Some(decode_permit) = try_acquire_decode_permit() else {
            glib::timeout_add_local_once(IMAGE_DECODE_RETRY_DELAY, move || {
                let weak_ref = weak_self.clone().into_weak_ref();
                let Some(obj) = weak_ref.upgrade() else {
                    return;
                };
                if obj.is_current(&cancellable, generation) {
                    Self::try_spawn_decode(
                        weak_self,
                        bytes,
                        animated,
                        cache_file_path,
                        cancellable,
                        generation,
                        fetch_on_error,
                    );
                }
            });
            return;
        };

        rayon::spawn(move || {
            let decoded = Self::decode_bytes(bytes, animated);
            drop(decode_permit);
            glib::idle_add_once(move || {
                let weak_self = weak_self.into_weak_ref();
                let Some(obj) = weak_self.upgrade() else {
                    return;
                };

                if !obj.is_current(&cancellable, generation) {
                    return;
                }

                match decoded {
                    Ok(LoadedImage::Texture(texture)) => {
                        obj.imp().picture.set_paintable(Some(&texture));
                        obj.set_picture_visible(cancellable, generation);
                    }
                    Ok(LoadedImage::Decoded(decoded)) => {
                        let paintable = ImagePaintable::from_decoded(decoded);
                        obj.imp().picture.set_paintable(Some(&paintable));
                        obj.set_picture_visible(cancellable, generation);
                    }
                    Err(_) if fetch_on_error => {
                        if let Some(path) = cache_file_path {
                            obj.get_file(path, cancellable, generation);
                        } else {
                            obj.set_broken(cancellable, generation);
                        }
                    }
                    Err(_) => obj.set_broken(cancellable, generation),
                }
            });
        });
    }

    fn decode_bytes(
        bytes: glib::Bytes, animated: bool,
    ) -> Result<LoadedImage, Box<dyn std::error::Error + Send + Sync>> {
        if animated {
            return ImagePaintable::decode_bytes(bytes).map(LoadedImage::Decoded);
        }

        match gtk::gdk::Texture::from_bytes(&bytes) {
            Ok(texture) => Ok(LoadedImage::Texture(texture)),
            Err(_) => ImagePaintable::decode_bytes(bytes).map(LoadedImage::Decoded),
        }
    }

    pub async fn cache_file(&self) -> PathBuf {
        let cache_path = jellyfin_cache_path().await;
        let path = format!(
            "{}-{}-{}",
            self.id(),
            self.imagetype(),
            self.tag().as_deref().unwrap_or("0")
        );
        cache_path.join(path)
    }

    pub fn get_file(&self, pathbuf: PathBuf, cancellable: gio::Cancellable, generation: u64) {
        let id = self.id();
        let image_type = self.imagetype();
        let tag = self.tag();
        let weak_self = self.downgrade();
        spawn(async move {
            if cancellable.is_cancelled() {
                return;
            }

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

            let Some(obj) = weak_self.upgrade() else {
                return;
            };

            if obj.is_current(&cancellable, generation) {
                debug!("Setting image: {}", &pathbuf.display());
                obj.reveal_picture(pathbuf, cancellable, generation, false);
            }
        });
    }
}
