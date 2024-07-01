use std::collections::HashMap;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};

use crate::{
    client::{client::EMBY_CLIENT, error::UserFacingError, structs::List},
    toast,
    ui::provider::tu_item::TuItem,
    utils::{get_image_with_cache, req_cache, spawn},
};

use super::song_widget::format_duration;

pub(crate) mod imp {
    use std::cell::OnceCell;

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

    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/album_widget.ui")]
    #[properties(wrapper_type = super::AlbumPage)]
    pub struct AlbumPage {
        #[property(get, set, construct_only)]
        pub item: OnceCell<TuItem>,
        #[template_child]
        pub cover_image: TemplateChild<gtk::Picture>,
        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub artist_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub released_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub listbox: TemplateChild<gtk::Box>,
        #[template_child]
        pub recommendhortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub artisthortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub actionbox: TemplateChild<ItemActionsBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AlbumPage {
        const NAME: &'static str = "AlbumPage";
        type Type = super::AlbumPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            StarToggle::ensure_type();
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for AlbumPage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            let id = obj.item().id();

            spawn_g_timeout(
                glib::clone!(@weak self as this, @weak obj, @strong id => async move {
                    obj.set_toolbar();
                    obj.set_album().await;
                    obj.get_songs().await;
                    obj.set_lists().await;
                }),
            );
        }
    }

    impl WidgetImpl for AlbumPage {}
    impl AdwDialogImpl for AlbumPage {}
    impl NavigationPageImpl for AlbumPage {}
}

glib::wrapper! {
    /// Preference Window to display and update room details.
    pub struct AlbumPage(ObjectSubclass<imp::AlbumPage>)
        @extends gtk::Widget, adw::Dialog, adw::NavigationPage, @implements gtk::Accessible;
}

impl AlbumPage {
    pub fn new(item: TuItem) -> Self {
        glib::Object::builder().property("item", item).build()
    }

    pub async fn set_album(&self) {
        let item = self.item();

        let imp = self.imp();

        imp.actionbox.set_id(Some(item.id()));

        if item.is_favorite() {
            imp.actionbox.set_btn_active(true);
        } else {
            imp.actionbox.set_btn_active(false);
        }

        imp.title_label.set_text(&item.name());

        imp.artist_label.set_text(&item.albumartist_name());

        let duration = item.run_time_ticks() / 10000000;
        let release = format!(
            "{} , {}",
            item.production_year(),
            format_duration(duration as i64)
        );
        imp.released_label.set_text(&release);

        let path = if let Some(image_tags) = item.primary_image_item_id() {
            get_image_with_cache(&image_tags, "Primary", None)
                .await
                .unwrap_or_default()
        } else {
            get_image_with_cache(&item.id(), "Primary", None)
                .await
                .unwrap_or_default()
        };

        if !std::path::PathBuf::from(&path).is_file() {
            return;
        }

        let image = gtk::gio::File::for_path(path);
        imp.cover_image.set_file(Some(&image));

        spawn(glib::clone!(@weak self as obj=>async move {
            let window = obj.root().and_downcast::<super::window::Window>().unwrap();
            window.set_rootpic(image);
        }));
    }

    pub async fn get_songs(&self) {
        let item = self.item();
        let id = item.id();

        let songs = match req_cache(&format!("audio_{}", item.id()), async move {
            EMBY_CLIENT.get_songs(&id).await
        })
        .await
        {
            Ok(songs) => songs,
            Err(e) => {
                toast!(self, e.to_user_facing());
                List::default()
            }
        };

        let mut disc_boxes: HashMap<u32, super::disc_box::DiscBox> = HashMap::new();

        for song in songs.items {
            let item = TuItem::from_simple(&song, None);
            let parent_index_number = item.parent_index_number();

            let song_widget = disc_boxes.entry(parent_index_number).or_insert_with(|| {
                let new_disc_box = super::disc_box::DiscBox::new();
                new_disc_box.set_disc(parent_index_number);
                self.imp().listbox.append(&new_disc_box);
                new_disc_box
            });

            song_widget.add_song(item);
        }
    }

    pub fn set_toolbar(&self) {
        let window = self.root().and_downcast::<super::window::Window>().unwrap();
        window.set_player_toolbar();
    }

    pub async fn set_lists(&self) {
        self.sets("Recommend").await;
        self.sets("More From").await;
    }

    pub async fn sets(&self, types: &str) {
        let hortu = match types {
            "Recommend" => self.imp().recommendhortu.get(),
            "More From" => self.imp().artisthortu.get(),
            _ => return,
        };

        if types == "More From" {
            hortu.set_title(&format!("More From {}", self.item().albumartist_name()));
        } else {
            hortu.set_title(types);
        }

        let id = self.item().id();
        let artist_id = self.item().albumartist_id();
        let types = types.to_string();

        let results = match req_cache(&format!("item_{types}_{id}"), async move {
            match types.as_str() {
                "Recommend" => EMBY_CLIENT.get_similar(&id).await,
                "More From" => EMBY_CLIENT.get_artist_albums(&id, &artist_id).await,
                _ => Ok(List::default()),
            }
        })
        .await
        {
            Ok(history) => history,
            Err(e) => {
                toast!(self, e.to_user_facing());
                List::default()
            }
        };

        if results.items.is_empty() {
            hortu.set_visible(false);
            return;
        }

        hortu.set_items(&results.items);
    }
}
