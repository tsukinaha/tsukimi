use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gtk::{
    glib,
    template_callbacks,
};

use crate::{
    client::{
        client::EMBY_CLIENT,
        error::UserFacingError,
        structs::{
            ExternalIdInfo,
            RemoteSearchInfo,
            RemoteSearchResult,
            SearchInfo,
            SearchProviderId,
        },
    },
    toast,
    utils::{
        spawn,
        spawn_tokio,
    },
};

mod imp {
    use std::cell::OnceCell;

    use glib::subclass::InitializingObject;
    use gtk::{
        glib,
        CompositeTemplate,
    };

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/identify_dialog.ui")]
    #[properties(wrapper_type = super::IdentifyDialog)]
    pub struct IdentifyDialog {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub itemtype: OnceCell<String>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub vbox: TemplateChild<gtk::Box>,
        #[template_child]
        pub title_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub year_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub music_brainz_album_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub music_brainz_album_artist_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub music_brainz_release_group_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub tmdb_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub imdb_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub tvdb_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub zap2it_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub result_box: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for IdentifyDialog {
        const NAME: &'static str = "IdentifyDialog";
        type Type = super::IdentifyDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for IdentifyDialog {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().init();
        }
    }

    impl WidgetImpl for IdentifyDialog {}
    impl AdwDialogImpl for IdentifyDialog {}
}

glib::wrapper! {
    /// A identify dialog to search for external ids.
    pub struct IdentifyDialog(ObjectSubclass<imp::IdentifyDialog>)
        @extends gtk::Widget, adw::Dialog, adw::PreferencesDialog, @implements gtk::Accessible, gtk::Root;
}

#[template_callbacks]
impl IdentifyDialog {
    pub fn new(id: &str, itemtype: &str) -> Self {
        glib::Object::builder().property("id", id).property("itemtype", itemtype).build()
    }

    pub fn init(&self) {
        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move { obj.get_data().await }
        ));
    }

    async fn get_data(&self) {
        let id = self.id();
        match spawn_tokio(async move { EMBY_CLIENT.get_external_id_info(&id).await }).await {
            Ok(data) => {
                self.imp().stack.set_visible_child_name("page");
                self.load_data(data);
            }
            Err(e) => {
                toast!(self, e.to_user_facing());
            }
        }
    }

    fn load_data(&self, data: Vec<ExternalIdInfo>) {
        for info in data {
            let entry = match info.key.as_str() {
                "MusicBrainzAlbum" => Some(self.imp().music_brainz_album_entry.get()),
                "MusicBrainzAlbumArtist" => Some(self.imp().music_brainz_album_artist_entry.get()),
                "MusicBrainzReleaseGroup" => {
                    Some(self.imp().music_brainz_release_group_entry.get())
                }
                "Tmdb" => Some(self.imp().tmdb_entry.get()),
                "Imdb" => Some(self.imp().imdb_entry.get()),
                "Tvdb" => Some(self.imp().tvdb_entry.get()),
                "Zap2it" => Some(self.imp().zap2it_entry.get()),
                _ => None,
            };

            if let Some(entry) = entry {
                entry.set_visible(true);
            }
        }
    }

    #[template_callback]
    async fn on_search(&self) {
        let imp = self.imp();

        let mut provider_ids: Vec<SearchProviderId> = Vec::new();

        let title = imp.title_entry.text();
        let year = imp.year_entry.text().to_string().parse::<u32>().ok();
        if imp.music_brainz_album_entry.is_visible() {
            provider_ids.push(SearchProviderId {
                music_brainz_album: Some(imp.music_brainz_album_entry.text().to_string()),
                ..Default::default()
            });
        }
        if imp.music_brainz_album_artist_entry.is_visible() {
            provider_ids.push(SearchProviderId {
                music_brainz_album_artist: Some(
                    imp.music_brainz_album_artist_entry.text().to_string(),
                ),
                ..Default::default()
            });
        }
        if imp.music_brainz_release_group_entry.is_visible() {
            provider_ids.push(SearchProviderId {
                music_brainz_release_group: Some(
                    imp.music_brainz_release_group_entry.text().to_string(),
                ),
                ..Default::default()
            });
        }
        if imp.tmdb_entry.is_visible() {
            provider_ids.push(SearchProviderId {
                tmdb: Some(imp.tmdb_entry.text().to_string()),
                ..Default::default()
            });
        }
        if imp.imdb_entry.is_visible() {
            provider_ids.push(SearchProviderId {
                imdb: Some(imp.imdb_entry.text().to_string()),
                ..Default::default()
            });
        }
        if imp.tvdb_entry.is_visible() {
            provider_ids.push(SearchProviderId {
                tvdb: Some(imp.tvdb_entry.text().to_string()),
                ..Default::default()
            });
        }
        if imp.zap2it_entry.is_visible() {
            provider_ids.push(SearchProviderId {
                zap2it: Some(imp.zap2it_entry.text().to_string()),
                ..Default::default()
            });
        }

        let searchinfo = SearchInfo { name: Some(title.to_string()), year, provider_ids };

        let remote_search_info = RemoteSearchInfo { item_id: self.id(), search_info: searchinfo };

        let type_ = self.itemtype();

        imp.stack.set_visible_child_name("loading");

        match spawn_tokio(
            async move { EMBY_CLIENT.remote_search(&type_, &remote_search_info).await },
        )
        .await
        {
            Ok(data) => {
                imp.stack.set_visible_child_name("searchresult");
                self.load_search_result(data);
            }
            Err(e) => {
                toast!(self, e.to_user_facing());
            }
        }
    }

    fn load_search_result(&self, data: Vec<RemoteSearchResult>) {
        let imp = self.imp();
        for result in data {
            let row = adw::ActionRow::builder().title(&result.name).build();
            if let Some(year) = result.production_year {
                row.set_subtitle(&year.to_string());
            }
            imp.result_box.append(&row);
        }
    }
}
