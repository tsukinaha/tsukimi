use adw::subclass::prelude::*;
use gtk::{gio, prelude::*};
use gtk::{glib, template_callbacks, CompositeTemplate};

use crate::client::client::EMBY_CLIENT;
use crate::client::error::UserFacingError;
use crate::toast;
use crate::utils::spawn_tokio;

use super::star_toggle::StarToggle;

mod imp {
    use std::cell::RefCell;

    use crate::ui::widgets::star_toggle::StarToggle;

    use super::*;
    use glib::subclass::InitializingObject;

    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/item_actions.ui")]
    #[properties(wrapper_type = super::ItemActionsBox)]
    pub struct ItemActionsBox {
        #[template_child]
        pub favourite_button: TemplateChild<StarToggle>,
        #[property(get, set, nullable)]
        pub id: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ItemActionsBox {
        const NAME: &'static str = "ItemActionsBox";
        type Type = super::ItemActionsBox;
        type ParentType = gtk::Box;

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
    impl ObjectImpl for ItemActionsBox {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().bind_edit();
        }
    }

    impl WidgetImpl for ItemActionsBox {}
    impl BoxImpl for ItemActionsBox {}
}

glib::wrapper! {
    /// Preference Window to display and update room details.
    pub struct ItemActionsBox(ObjectSubclass<imp::ItemActionsBox>)
        @extends gtk::Widget, adw::Dialog, adw::NavigationPage, @implements gtk::Accessible;
}

impl Default for ItemActionsBox {
    fn default() -> Self {
        Self::new()
    }
}

#[template_callbacks]
impl ItemActionsBox {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("id", None::<String>)
            .build()
    }

    #[template_callback]
    pub async fn on_favourite_button_toggled(&self, btn: &StarToggle) {
        let id = self.id();

        if let Some(id) = id {
            let result = if btn.is_active() {
                spawn_tokio(async move { EMBY_CLIENT.like(&id).await }).await
            } else {
                spawn_tokio(async move { EMBY_CLIENT.unlike(&id).await }).await
            };

            match result {
                Ok(_) => {
                    toast!(self, "Success");
                }
                Err(e) => {
                    toast!(self, e.to_user_facing());
                }
            }
        }
    }

    pub fn edit_metadata_action(&self) -> gio::SimpleActionGroup {
        let action_group = gio::SimpleActionGroup::new();
        action_group.add_action_entries([gio::ActionEntry::builder("editm")
            .activate(glib::clone!(@weak self as obj => move |_, _, _| {
                use crate::ui::widgets::metadata_dialog::MetadataDialog;
                use crate::insert_editm_dialog;
                let id = obj.id();
                if let Some(id) = id {
                    let dialog = MetadataDialog::new(&id);
                    insert_editm_dialog!(obj, dialog);
                }
            }))
            .build()]);
        action_group
    }

    pub fn bind_edit(&self) {
        self.insert_action_group("item", Some(&self.edit_metadata_action()));
    }

    pub fn set_btn_active(&self, active: bool) {
        self.imp().favourite_button.set_active(active);
    }
}
