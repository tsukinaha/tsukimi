use glib::Object;
use gtk::{
    gio,
    glib,
    prelude::*,
    subclass::prelude::*,
    template_callbacks,
};

use adw::prelude::*;

use crate::{
    client::{
        error::UserFacingError,
        jellyfin_client::JELLYFIN_CLIENT,
        structs::*,
    },
    ui::provider::tu_item::TuItem,
    utils::{
        spawn,
        spawn_tokio,
    },
};

use super::{
    filter_panel::FilterPanelDialog,
    utils::GlobalToast,
};

mod imp {

    use std::{
        cell::OnceCell,
        sync::atomic::Ordering,
    };

    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib,
        subclass::prelude::*,
    };

    use gtk::prelude::*;

    use crate::{
        ui::widgets::{
            filter_panel::FilterPanelDialog,
            tuview_scrolled::TuViewScrolled,
        },
        utils::spawn,
    };

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/search.ui")]
    pub struct SearchPage {
        #[template_child]
        pub searchentry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub searchscrolled: TemplateChild<TuViewScrolled>,
        #[template_child]
        pub recommend_group: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub movie: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub series: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub boxset: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub person: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub music: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub audio: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub video: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub episode: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,

        #[template_child]
        pub filter: TemplateChild<gtk::Button>,

        pub filter_panel: OnceCell<FilterPanelDialog>,
        pub selection: gtk::SingleSelection,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SearchPage {
        const NAME: &'static str = "SearchPage";
        type Type = super::SearchPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            TuViewScrolled::ensure_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SearchPage {
        fn constructed(&self) {
            let obj = self.obj();
            self.parent_constructed();
            self.searchscrolled.connect_end_edge_reached(glib::clone!(
                #[weak]
                obj,
                move |scrolled, lock| {
                    spawn(glib::clone!(
                        #[weak]
                        obj,
                        #[weak]
                        scrolled,
                        async move {
                            scrolled.reveal_spinner(true);

                            let search_results = obj.get_search_results::<true>().await;

                            scrolled.set_store::<false>(search_results.items, false);

                            scrolled.reveal_spinner(false);

                            lock.store(false, Ordering::SeqCst);
                        },
                    ))
                }
            ));
            obj.update();
        }
    }

    impl WidgetImpl for SearchPage {}

    impl WindowImpl for SearchPage {}

    impl ApplicationWindowImpl for SearchPage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for SearchPage {}
}

glib::wrapper! {
    pub struct SearchPage(ObjectSubclass<imp::SearchPage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Default for SearchPage {
    fn default() -> Self {
        Self::new()
    }
}

#[template_callbacks]
impl SearchPage {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn update(&self) {
        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                obj.setup_recommend().await;
            }
        ));
    }

    pub async fn setup_recommend(&self) {
        let recommend =
            match spawn_tokio(async { JELLYFIN_CLIENT.get_search_recommend().await }).await {
                Ok(list) => list,
                Err(e) => {
                    self.toast(e.to_user_facing());
                    List::default()
                }
            };

        let imp = self.imp();
        imp.recommend_group.remove_all();

        for item in recommend.items {
            let action_row = adw::ActionRow::builder()
                .title(&item.name)
                .subtitle(
                    item.overview
                        .to_owned()
                        .unwrap_or_else(|| gettextrs::gettext("No overview"))
                        .replace("\n", " ")
                        .replace("&", "&amp;"),
                )
                .subtitle_lines(2)
                .use_markup(false)
                .activatable(true)
                .build();

            let icon = gtk::Image::new();

            if item.item_type == "Movie" {
                icon.set_icon_name(Some("display-projector-symbolic"));
            } else {
                icon.set_icon_name(Some("video-reel-symbolic"));
            }

            action_row.add_prefix(&icon);

            action_row.connect_activated(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |_| {
                    let tu_item = TuItem::from_simple(&item, None);
                    tu_item.activate(&obj, None);
                }
            ));

            imp.recommend_group.append(&action_row);
        }

        imp.stack.set_visible_child_name("recommend");
    }

    #[template_callback]
    async fn on_search_activate(&self) {
        let imp = self.imp();

        let search_results = self.get_search_results::<false>().await;

        if search_results.items.is_empty() {
            imp.stack.set_visible_child_name("fallback");
            return;
        };

        imp.searchscrolled
            .set_store::<true>(search_results.items, false);

        imp.stack.set_visible_child_name("result");
    }

    pub async fn get_search_results<const F: bool>(&self) -> List {
        let imp = self.imp();

        let search_content = imp.searchentry.text().to_string();
        if search_content.len() < 2 {
            return List::default();
        }
        let search_filter = {
            let mut filter = Vec::new();
            if imp.movie.is_active() {
                filter.push("Movie");
            }
            if imp.series.is_active() {
                filter.push("Series");
            }
            if imp.boxset.is_active() {
                filter.push("BoxSet");
            }
            if imp.person.is_active() {
                filter.push("Person");
            }
            if imp.music.is_active() {
                filter.push("MusicAlbum");
            }
            if imp.audio.is_active() {
                filter.push("Audio");
            }
            if imp.video.is_active() {
                filter.push("Video");
            }
            if imp.episode.is_active() {
                filter.push("Episode");
            }
            if filter.is_empty() {
                return List::default();
            }
            filter
        };
        let n_items = if F {
            imp.searchscrolled.n_items()
        } else {
            imp.stack.set_visible_child_name("loading");
            0
        };

        let filters_list = imp
            .filter_panel
            .get()
            .map(|f| f.filters_list())
            .unwrap_or_default();

        if !filters_list.is_empty() {
            imp.filter.add_css_class("accent");
        } else {
            imp.filter.remove_css_class("accent");
        }

        match spawn_tokio(async move {
            JELLYFIN_CLIENT
                .search(
                    &search_content,
                    &search_filter,
                    &n_items.to_string(),
                    &filters_list,
                )
                .await
        })
        .await
        {
            Ok(list) => list,
            Err(e) => {
                self.toast(e.to_user_facing());
                List::default()
            }
        }
    }

    #[template_callback]
    fn filter_panel_cb(&self, _btn: &gtk::Button) {
        let panel = self.imp().filter_panel.get_or_init(|| {
            let dialog = FilterPanelDialog::new();
            dialog.connect_applied(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                #[weak]
                dialog,
                move |_| {
                    dialog.close();
                    obj.on_filter_applied();
                }
            ));
            dialog
        });
        panel.present(Some(self));
    }

    pub fn on_filter_applied(&self) {
        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                obj.on_search_activate().await;
            }
        ));
    }
}
