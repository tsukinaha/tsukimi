use std::collections::HashMap;
use std::path::PathBuf;
use crate::client::client::EMBY_CLIENT;
use crate::utils::{spawn, spawn_tokio};
use gtk::gdk::Texture;
use gtk::glib::{self, clone};
use gtk::{prelude::*, IconPaintable, Revealer};
use tracing::{debug, warn};
use crate::bing_song_model;
use crate::ui::models::emby_cache_path;
use crate::ui::provider::core_song::CoreSong;
use crate::ui::widgets::song_widget::State;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::gio;
use gtk::gio::ListStore;
use gtk::{ template_callbacks, CompositeTemplate};

use super::song_widget::format_duration;

pub(crate) mod imp {
    use std::cell::{OnceCell, RefCell};

    use crate::ui::widgets::item_actionbox::ItemActionsBox;
    use crate::{
        ui::{
            provider::tu_item::TuItem,
            widgets::{hortu_scrolled::HortuScrolled, star_toggle::StarToggle},
        },
        utils::spawn_g_timeout,
    };

    use super::*;
    use glib::subclass::InitializingObject;
    use glib::SignalHandlerId;

    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/picture_loader.ui")]
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
        pub spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub broken: TemplateChild<gtk::Box>,
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
        glib::Object::builder().property("id", id).property("imagetype", image_type).property("tag", tag).build()
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
            imp.picture.set_file(Some(&file));
        } else {
            imp.broken.set_visible(true);
        }
        
        imp.spinner.stop();
        
        imp.revealer.set_reveal_child(true);
    }

    pub fn cache_file(&self) -> PathBuf {
        let cache_path = emby_cache_path();
        let path = format!("{}-{}-{}", self.id(), self.imagetype(), self.tag().unwrap_or("0".to_string()));
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
                    let mut retries = 0;
                    while retries < 3 {
                        let tag = tag.clone();
                        match EMBY_CLIENT.get_image(&id, &image_type, tag.and_then(|s| s.parse::<u8>().ok())).await {
                            Ok(_) => {
                                break;
                            }
                            Err(e) => {
                                warn!("Failed to get image: {}, retrying...", e);
                                retries += 1;
                            }
                        }
                    }
                })
                .await;
                debug!("Setting image: {}", &pathbuf.display());
                obj.reveal_picture(pathbuf);
            }
        ));
    }
    

}
