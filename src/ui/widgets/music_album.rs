use std::collections::HashMap;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};

use crate::{
    client::network::get_songs,
    ui::provider::tu_item::TuItem,
    utils::{get_data_with_cache, get_image_with_cache, spawn},
};

use super::song_widget::format_duration;

mod imp {
    use std::cell::OnceCell;

    use crate::{ui::provider::tu_item::TuItem, utils::spawn_g_timeout};

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
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AlbumPage {
        const NAME: &'static str = "AlbumPage";
        type Type = super::AlbumPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
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
            spawn_g_timeout(glib::clone!(@weak obj => async move {
                obj.set_album().await;
                obj.get_songs().await;
            }));
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

        self.imp().title_label.set_text(&item.name());

        self.imp()
            .artist_label
            .set_text(&item.album_artist().unwrap_or(String::new()));

        let duration = item.run_time_ticks() / 10000000;
        let release = format!(
            "{} , {}",
            item.production_year(),
            format_duration(duration as i64)
        );
        self.imp().released_label.set_text(&release);

        let path = get_image_with_cache(&item.id(), "Primary", None)
            .await
            .unwrap_or_default();

        if !std::path::PathBuf::from(&path).is_file() {
            return;
        }

        let image = gtk::gio::File::for_path(path);
        self.imp().cover_image.set_file(Some(&image));

        spawn(glib::clone!(@weak self as obj=>async move {
            let window = obj.root().and_downcast::<super::window::Window>().unwrap();
            window.set_rootpic(image);
        }));
    }

    pub async fn get_songs(&self) {
        let item = self.item();
        let id = item.id();
        let songs = get_data_with_cache(item.id(), "audio", async move { get_songs(&id).await })
            .await
            .unwrap();

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
}
