use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gettextrs::gettext;
use gtk::{
    CompositeTemplate,
    gio,
    glib,
};

use gtk::template_callbacks;
use serde_json::Value;

use crate::{
    client::{
        error::UserFacingError,
        jellyfin_client::JELLYFIN_CLIENT,
        structs::RemoteSearchResult,
    },
    ui::{
        GlobalToast,
        widgets::eu_item::{
            self,
            EuItem,
            EuObject,
        },
    },
    utils::{
        spawn,
        spawn_tokio,
    },
};

mod imp {

    use std::cell::OnceCell;

    use eu_item::EuListItemExt;
    use glib::subclass::InitializingObject;
    use gtk::gio;

    use crate::ui::widgets::eu_item::EuObject;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/identify_dialog_search_page.ui")]
    #[properties(wrapper_type = super::IdentifyDialogSearchPage)]
    pub struct IdentifyDialogSearchPage {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,

        #[template_child]
        pub grid: TemplateChild<gtk::GridView>,

        pub selection: gtk::SingleSelection,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for IdentifyDialogSearchPage {
        const NAME: &'static str = "IdentifyDialogSearchPage";
        type Type = super::IdentifyDialogSearchPage;
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
    impl ObjectImpl for IdentifyDialogSearchPage {
        fn constructed(&self) {
            self.parent_constructed();
            let store = gio::ListStore::new::<EuObject>();
            self.selection.set_model(Some(&store));
            self.grid
                .set_factory(Some(gtk::SignalListItemFactory::new().eu_item()));
            self.grid.set_model(Some(&self.selection));
        }
    }

    impl WidgetImpl for IdentifyDialogSearchPage {}

    impl NavigationPageImpl for IdentifyDialogSearchPage {}
}

glib::wrapper! {
    pub struct IdentifyDialogSearchPage(ObjectSubclass<imp::IdentifyDialogSearchPage>)
        @extends gtk::Widget, adw::NavigationPage, @implements gtk::Accessible;
}

#[template_callbacks]
impl IdentifyDialogSearchPage {
    pub fn new(id: &str) -> Self {
        glib::Object::builder().property("id", id).build()
    }

    #[template_callback]
    fn item_activated_cb(&self, pos: u32, gridview: &gtk::GridView) {
        let Some(item) = gridview.model().and_then(|m| {
            m.item(pos)
                .and_downcast::<EuObject>()
                .and_then(|o| o.item())
        }) else {
            return;
        };

        let Some(value): Option<Value> = item
            .json_value()
            .and_then(|v| serde_json::from_str(&v).ok())
        else {
            return;
        };

        let id = self.id();

        let alert_dialog = adw::AlertDialog::builder()
            .heading(gettext("Identify"))
            .title("Identify")
            .body(gettext("Are you sure you wish to continue?"))
            .build();

        alert_dialog.add_response("close", &gettext("Cancel"));
        alert_dialog.add_response("ok", &gettext("Ok"));
        alert_dialog.set_response_appearance("ok", adw::ResponseAppearance::Suggested);

        let check_button = gtk::CheckButton::with_label(&gettext("Replace existing images"));
        alert_dialog.set_extra_child(Some(&check_button));

        alert_dialog.connect_response(
            Some("ok"),
            glib::clone!(
                #[weak(rename_to = obj)]
                self,
                #[weak]
                check_button,
                move |_, _| {
                    let replace = check_button.is_active();
                    spawn(glib::clone!(
                        #[weak]
                        obj,
                        #[strong]
                        id,
                        #[strong]
                        value,
                        async move {
                            match spawn_tokio(async move {
                                JELLYFIN_CLIENT
                                    .apply_remote_search(&id, value, replace)
                                    .await
                            })
                            .await
                            {
                                Ok(_) => {
                                    obj.toast(gettext("Success"));
                                }
                                Err(e) => {
                                    obj.toast(e.to_user_facing());
                                }
                            }
                        }
                    ))
                }
            ),
        );

        alert_dialog.present(Some(self));
    }

    pub fn extend_item(&self, items: Value, item_type: String) {
        let Value::Array(ref items) = items else {
            return;
        };

        let Some(store) = self
            .imp()
            .selection
            .model()
            .and_downcast::<gio::ListStore>()
        else {
            return;
        };
        for item_value in items {
            let Ok(item) = serde_json::from_value::<RemoteSearchResult>(item_value.to_owned())
            else {
                continue;
            };
            let eu_item = EuItem::new(
                item.image_url,
                None,
                Some(item.name),
                item.production_year.map(|p| p.to_string()),
                None,
                Some(item_type.to_owned()),
                Some(item_value.to_string()),
            );
            let eu_object = EuObject::new(&eu_item);
            store.append(&eu_object);
        }
    }
}
