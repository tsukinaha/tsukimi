use std::collections::HashMap;

use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gettextrs::gettext;
use gtk::{
    glib,
    template_callbacks,
};

use crate::{
    client::{
        error::UserFacingError,
        jellyfin_client::JELLYFIN_CLIENT,
        structs::{
            ExternalIdInfo,
            RemoteSearchInfo,
            SearchInfo,
        },
    },
    ui::GlobalToast,
    utils::{
        spawn,
        spawn_tokio,
    },
};

mod imp {
    use std::cell::OnceCell;

    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib,
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
        pub entries_group: TemplateChild<adw::PreferencesGroup>,

        #[template_child]
        pub name_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub year_entry: TemplateChild<adw::EntryRow>,

        #[template_child]
        pub path_row: TemplateChild<adw::ActionRow>,

        #[template_child]
        pub navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
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
        glib::Object::builder()
            .property("id", id)
            .property("itemtype", itemtype)
            .build()
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
        let id_clone = id.to_owned();
        match spawn_tokio(async move { JELLYFIN_CLIENT.get_external_id_info(&id).await }).await {
            Ok(data) => {
                self.imp().stack.set_visible_child_name("page");
                self.load_data(data);
            }
            Err(e) => {
                self.toast(e.to_user_facing());
            }
        }
        match spawn_tokio(async move { JELLYFIN_CLIENT.get_item_info(&id_clone).await }).await {
            Ok(item) => {
                self.imp()
                    .path_row
                    .set_subtitle(&item.path.unwrap_or_default().replace("&", "&amp;"));
            }
            Err(_) => {
                self.imp()
                    .path_row
                    .set_subtitle(&gettext("Failed to get path"));
            }
        }
    }

    fn load_data(&self, data: Vec<ExternalIdInfo>) {
        for info in data {
            let entry = adw::EntryRow::builder().title(&info.name).build();

            if let Some(url) = &info.website {
                let button = gtk::Button::builder()
                    .icon_name("external-link-symbolic")
                    .valign(gtk::Align::Center)
                    .build();

                let url = url.to_owned();
                button.connect_clicked(move |_| {
                    let _ = gtk::gio::AppInfo::launch_default_for_uri(
                        &url,
                        Option::<&gtk::gio::AppLaunchContext>::None,
                    );
                });

                entry.add_suffix(&button);
            }

            self.imp().entries_group.add(&entry);
        }
    }

    pub fn add_toast(&self, toast: adw::Toast) {
        self.imp().toast_overlay.add_toast(toast);
    }

    #[template_callback]
    async fn on_search_button_clicked(&self) {
        let imp = self.imp();

        let mut provider_ids = HashMap::new();

        let name = imp.name_entry.text().to_string();
        let year = imp.year_entry.text().to_string().parse::<u32>().ok();

        imp.entries_group
            .observe_children()
            .into_iter()
            .for_each(|child| {
                if let Some(entry) = child.ok().and_downcast_ref::<adw::EntryRow>() {
                    let provider_id = entry.text().to_string();
                    let provider = entry.title().to_string();
                    provider_ids.insert(provider, provider_id);
                }
            });

        let searchinfo = SearchInfo {
            name: Some(name),
            year,
            provider_ids,
        };

        let remote_search_info = RemoteSearchInfo {
            item_id: self.id(),
            search_info: searchinfo,
        };

        let type_ = self.itemtype();

        imp.stack.set_visible_child_name("loading");

        match spawn_tokio(async move {
            JELLYFIN_CLIENT
                .remote_search(&type_, &remote_search_info)
                .await
        })
        .await
        {
            Ok(data) => {
                let search_page = super::IdentifyDialogSearchPage::new(&self.id());
                search_page.extend_item(data, self.itemtype());
                self.imp().stack.set_visible_child_name("page");
                self.imp().navigation_view.push(&search_page);
            }
            Err(e) => {
                self.toast(e.to_user_facing());
            }
        }
    }
}
