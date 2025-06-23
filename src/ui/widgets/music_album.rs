use std::collections::BTreeMap;

use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gettextrs::gettext;
use gtk::{
    CompositeTemplate,
    gio,
    gio::ListStore,
    glib,
    template_callbacks,
};

use super::{
    song_widget::format_duration,
    utils::GlobalToast,
};
use crate::{
    bing_song_model,
    client::{
        error::UserFacingError,
        jellyfin_client::JELLYFIN_CLIENT,
        structs::List,
    },
    ui::{
        provider::{
            core_song::CoreSong,
            tu_item::TuItem,
        },
        widgets::song_widget::{
            SongWidget,
            State,
        },
    },
    utils::{
        CachePolicy,
        fetch_with_cache,
        get_image_with_cache,
        spawn,
    },
};

pub(crate) mod imp {
    use std::cell::{
        OnceCell,
        RefCell,
    };

    use glib::{
        SignalHandlerId,
        subclass::InitializingObject,
    };

    use super::*;
    use crate::{
        ui::{
            provider::tu_item::TuItem,
            widgets::{
                hortu_scrolled::HortuScrolled,
                item_actionbox::ItemActionsBox,
                star_toggle::StarToggle,
            },
        },
        utils::spawn_g_timeout,
    };

    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/album_widget.ui")]
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
        pub signal_id: RefCell<Option<SignalHandlerId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AlbumPage {
        const NAME: &'static str = "AlbumPage";
        type Type = super::AlbumPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            StarToggle::ensure_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
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

            spawn_g_timeout(glib::clone!(
                #[weak]
                obj,
                async move {
                    obj.set_album().await;
                    obj.get_songs().await;
                    obj.set_lists().await;
                }
            ));
        }
    }

    impl WidgetImpl for AlbumPage {}
    impl AdwDialogImpl for AlbumPage {}
    impl NavigationPageImpl for AlbumPage {}
}

glib::wrapper! {
    /// A page for displaying an album.
    pub struct AlbumPage(ObjectSubclass<imp::AlbumPage>)
        @extends gtk::Widget, adw::Dialog, adw::NavigationPage, @implements gtk::Accessible;
}

use crate::ui::widgets::disc_box::DiscBox;

#[template_callbacks]
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
            get_image_with_cache(image_tags, "Primary".to_string(), None)
                .await
                .unwrap_or_default()
        } else {
            get_image_with_cache(item.id(), "Primary".to_string(), None)
                .await
                .unwrap_or_default()
        };

        if !std::path::PathBuf::from(&path).is_file() {
            return;
        }

        let image = gtk::gio::File::for_path(path);
        imp.cover_image.set_file(Some(&image));

        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                let window = obj.root().and_downcast::<super::window::Window>().unwrap();
                window.set_rootpic(image);
            }
        ));
    }

    pub async fn get_songs(&self) {
        let item = self.item();
        let id = item.id();

        let mut songs = match fetch_with_cache(
            &format!("audio_{}", item.id()),
            CachePolicy::ReadCacheAndRefresh,
            async move { JELLYFIN_CLIENT.get_songs(&id).await },
        )
        .await
        {
            Ok(songs) => songs,
            Err(e) => {
                self.toast(e.to_user_facing());
                return;
            }
        };

        let mut disc_boxes: BTreeMap<u32, super::disc_box::DiscBox> = BTreeMap::new();

        songs.items.sort_by_key(|song| song.index_number);
        for song in songs.items {
            let item = TuItem::from_simple(&song, None);
            let parent_index_number = item.parent_index_number();

            let song_widget = disc_boxes.entry(parent_index_number).or_insert_with(|| {
                let new_disc_box = super::disc_box::DiscBox::new();
                new_disc_box.set_disc(parent_index_number);
                new_disc_box.connect_closure(
                    "song-activated",
                    true,
                    glib::closure_local!(
                        #[watch(rename_to = obj)]
                        self,
                        move |_: DiscBox, song_widget| {
                            obj.song_activated(song_widget);
                        }
                    ),
                );
                new_disc_box
            });
            song_widget.add_song(item);
        }

        for disc_box in disc_boxes.values() {
            self.imp().listbox.append(disc_box);
        }
    }

    fn song_activated(&self, song_widget: SongWidget) {
        song_widget.set_state(State::Playing);
        let active_model = self.song_model();
        let active_core_song = song_widget.coresong();
        bing_song_model!(self, active_model, active_core_song);
    }

    fn song_model(&self) -> ListStore {
        let imp = self.imp();
        let listbox = imp.listbox.get();
        let liststore = gio::ListStore::new::<CoreSong>();
        for child in listbox.observe_children().into_iter().flatten() {
            let Ok(discbox) = child.downcast::<DiscBox>() else {
                continue;
            };
            for child in discbox
                .imp()
                .listbox
                .observe_children()
                .into_iter()
                .flatten()
            {
                let Ok(song_widget) = child.downcast::<SongWidget>() else {
                    continue;
                };
                let item = song_widget.coresong();
                liststore.append(&item)
            }
        }
        liststore
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
            hortu.set_title(format!(
                "{} {}",
                gettext("More From"),
                self.item().albumartist_name()
            ));
        } else {
            hortu.set_title(gettext(types));
        }

        let id = self.item().id();
        let artist_id = self.item().albumartist_id();
        let types = types.to_string();

        let results = match fetch_with_cache(
            &format!("item_{types}_{id}"),
            CachePolicy::ReadCacheAndRefresh,
            async move {
                match types.as_str() {
                    "Recommend" => JELLYFIN_CLIENT.get_similar(&id).await,
                    "More From" => JELLYFIN_CLIENT.get_artist_albums(&id, &artist_id).await,
                    _ => Ok(List::default()),
                }
            },
        )
        .await
        {
            Ok(history) => history,
            Err(e) => {
                self.toast(e.to_user_facing());
                List::default()
            }
        };

        if results.items.is_empty() {
            hortu.set_visible(false);
            return;
        }

        hortu.set_items(&results.items);
    }

    #[template_callback]
    fn on_play_button_clicked(&self, _btn: gtk::Button) {
        let imp = self.imp();
        let active_model = self.song_model();
        let Some(object) = imp.listbox.get().first_child() else {
            return;
        };
        let Some(widget) = object
            .downcast::<DiscBox>()
            .unwrap()
            .imp()
            .listbox
            .first_child()
        else {
            return;
        };
        let active_core_song = widget.downcast::<SongWidget>().unwrap().coresong();
        bing_song_model!(self, active_model, active_core_song);
    }
}
