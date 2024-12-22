use adw::subclass::prelude::*;
use gtk::{
    gio,
    glib,
    CompositeTemplate,
    StringObject,
};

use gtk::{
    prelude::*,
    template_callbacks,
};

use crate::{
    client::{
        emby_client::EMBY_CLIENT,
        error::UserFacingError,
    },
    toast,
    ui::widgets::eu_item,
    utils::{
        spawn,
        spawn_tokio,
    },
};

use super::ImageDialogNavigtion;

mod imp {
    use std::cell::OnceCell;

    use eu_item::EuListItemExt;
    use glib::{
        subclass::InitializingObject,
        Properties,
    };
    use gtk::{
        gio,
        prelude::*,
    };

    use crate::ui::widgets::eu_item::EuObject;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/image_dialog_search_page.ui")]
    #[properties(wrapper_type = super::ImageDialogSearchPage)]
    pub struct ImageDialogSearchPage {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub image_type: OnceCell<String>,

        #[template_child]
        pub items_count_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub dropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub all_languages_check: TemplateChild<gtk::CheckButton>,

        #[template_child]
        pub dropdown_string_list: TemplateChild<gtk::StringList>,
        #[template_child]
        pub grid: TemplateChild<gtk::GridView>,

        pub selection: gtk::SingleSelection,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageDialogSearchPage {
        const NAME: &'static str = "ImageDialogSearchPage";
        type Type = super::ImageDialogSearchPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ImageDialogSearchPage {
        fn constructed(&self) {
            self.parent_constructed();
            let store = gio::ListStore::new::<EuObject>();
            self.selection.set_model(Some(&store));
            self.grid
                .set_factory(Some(gtk::SignalListItemFactory::new().eu_item()));
            self.grid.set_model(Some(&self.selection));

            self.obj().call_init::<true>();
        }
    }

    impl WidgetImpl for ImageDialogSearchPage {}

    impl NavigationPageImpl for ImageDialogSearchPage {}
}

glib::wrapper! {
    pub struct ImageDialogSearchPage(ObjectSubclass<imp::ImageDialogSearchPage>)
        @extends gtk::Widget, adw::NavigationPage, @implements gtk::Accessible;
}

#[template_callbacks]
impl ImageDialogSearchPage {
    pub fn new(id: &str, image_type: &str) -> Self {
        glib::Object::builder()
            .property("id", id)
            .property("image-type", image_type)
            .build()
    }

    pub async fn init<const FIRST_INIT: bool>(&self) {
        let id = self.id();
        let type_ = self.image_type();

        let Some(store) = self
            .imp()
            .selection
            .model()
            .and_downcast::<gio::ListStore>()
        else {
            return;
        };

        store.remove_all();

        let Some(dialog) = self.image_dialog() else {
            return;
        };

        dialog.loading_page();

        let if_all_language = self.imp().all_languages_check.is_active();
        let providers = self.providers();

        let remote_image_list = match spawn_tokio(async move {
            EMBY_CLIENT
                .get_remote_image_list(&id, 0, if_all_language, &type_, &providers)
                .await
        })
        .await
        {
            Ok(remote_image_list) => remote_image_list,
            Err(e) => {
                toast!(self, e.to_user_facing());
                return;
            }
        };

        dialog.view_page();

        self.imp()
            .items_count_label
            .set_text(&format!("{} Items", remote_image_list.total_record_count));

        if FIRST_INIT {
            for provider in remote_image_list.providers {
                self.imp().dropdown_string_list.append(&provider);
            }
        }

        for item in remote_image_list.images {
            let line2 = format!(
                "{}x{} - {}",
                item.width.unwrap_or(0),
                item.height.unwrap_or(0),
                item.language.unwrap_or_default()
            );
            let eu_item = eu_item::EuItem::new(
                item.thumbnail_url,
                Some(item.url),
                Some(item.provider_name),
                Some(line2),
                item.community_rating.map(|x| x.to_string()),
                Some(self.image_type().to_string()),
                None,
            );
            let eu_object = eu_item::EuObject::new(&eu_item);
            store.append(&eu_object);
        }
    }

    fn providers(&self) -> String {
        self.imp()
            .dropdown
            .selected_item()
            .and_downcast_ref::<StringObject>()
            .map(|object| object.string())
            .filter(|s| s != "All")
            .unwrap_or_default()
            .to_string()
    }

    #[template_callback]
    fn on_provider_changed(&self) {
        self.call_init::<false>();
    }

    #[template_callback]
    fn on_all_languages_check_toggled(&self) {
        self.call_init::<false>();
    }

    pub fn call_init<const FIRST_INIT: bool>(&self) {
        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                obj.init::<FIRST_INIT>().await;
            }
        ));
    }
}
