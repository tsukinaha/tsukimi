use std::fmt::Debug;

use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{
    glib,
    prelude::*,
    template_callbacks,
};

use super::utils::GlobalToast;
use crate::{
    client::{
        error::UserFacingError,
        jellyfin_client::JELLYFIN_CLIENT,
    },
    utils::spawn_tokio,
};
mod imp {
    use std::cell::OnceCell;

    use adw::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib,
    };

    use super::*;
    use crate::utils::spawn;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/missing_episodes.ui")]
    #[properties(wrapper_type = super::MissingEpisodesDialog)]
    pub struct MissingEpisodesDialog {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,

        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,

        #[template_child]
        pub list: TemplateChild<gtk::ListBox>,

        #[template_child]
        pub specials_check: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub upcoming_check: TemplateChild<gtk::CheckButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MissingEpisodesDialog {
        const NAME: &'static str = "MissingEpisodesDialog";
        type Type = super::MissingEpisodesDialog;
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
    impl ObjectImpl for MissingEpisodesDialog {
        fn constructed(&self) {
            self.parent_constructed();
            self.init();
        }
    }

    impl WidgetImpl for MissingEpisodesDialog {}
    impl AdwDialogImpl for MissingEpisodesDialog {}

    impl MissingEpisodesDialog {
        fn init(&self) {
            let obj = self.obj();
            spawn(glib::clone!(
                #[weak]
                obj,
                async move {
                    obj.set_items().await;
                }
            ));
        }
    }
}

glib::wrapper! {
    pub struct MissingEpisodesDialog(ObjectSubclass<imp::MissingEpisodesDialog>)
        @extends gtk::Widget, adw::Dialog, adw::PreferencesDialog, @implements gtk::Accessible, gtk::Root;
}

#[template_callbacks]
impl MissingEpisodesDialog {
    const LOADING_STACK_PAGE: &'static str = "loading";
    const VIEW_STACK_PAGE: &'static str = "view";

    pub fn new(id: &str) -> Self {
        glib::Object::builder().property("id", id).build()
    }

    pub fn loading_page(&self) {
        self.imp()
            .stack
            .set_visible_child_name(Self::LOADING_STACK_PAGE);
    }

    pub fn view_page(&self) {
        self.imp()
            .stack
            .set_visible_child_name(Self::VIEW_STACK_PAGE);
    }

    pub async fn set_items(&self) {
        let id = self.id();

        self.loading_page();

        let special_checked = self.imp().specials_check.is_active();
        let upcoming_checked = self.imp().upcoming_check.is_active();

        let items = match spawn_tokio(async move {
            JELLYFIN_CLIENT
                .get_show_missing(&id, special_checked, upcoming_checked)
                .await
        })
        .await
        {
            Ok(list) => list.items,
            Err(e) => {
                self.toast(e.to_user_facing());
                self.view_page();
                return;
            }
        };

        self.imp().list.remove_all();

        for item in items {
            let header = {
                let parent_index_number = item.parent_index_number.unwrap_or_default();
                let index_number = item.index_number.unwrap_or_default();
                if parent_index_number == 0 {
                    gettext("Special")
                } else {
                    format!("S{parent_index_number}E{index_number}")
                }
            };

            let date = if let Some(date) = item.premiere_date {
                format!("{}\n", date.format("%Y-%m-%d"))
            } else {
                String::new()
            };

            let row = adw::ActionRow::builder()
                .title(format!("{} - {}", header, item.name))
                .subtitle(format!("{}{}", date, item.overview.unwrap_or_default()))
                .build();

            self.imp().list.append(&row);
        }

        self.view_page();
    }

    #[template_callback]
    async fn on_specials_check_toggled(&self) {
        self.set_items().await;
    }

    #[template_callback]
    async fn on_upcoming_check_toggled(&self) {
        self.set_items().await;
    }
}
