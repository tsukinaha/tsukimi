use adw::subclass::prelude::*;
use gtk::{
    CompositeTemplate,
    gio,
    glib,
    prelude::*,
    template_callbacks,
};

use super::{
    star_toggle::StarToggle,
    utils::GlobalToast,
};
use crate::{
    client::{
        error::UserFacingError,
        jellyfin_client::JELLYFIN_CLIENT,
    },
    utils::{
        spawn,
        spawn_tokio,
    },
};
mod imp {
    use std::cell::RefCell;

    use glib::subclass::InitializingObject;

    use super::*;
    use crate::ui::widgets::star_toggle::StarToggle;

    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/item_actions.ui")]
    #[properties(wrapper_type = super::ItemActionsBox)]
    pub struct ItemActionsBox {
        #[template_child]
        pub favourite_button: TemplateChild<StarToggle>,
        #[property(get, set, nullable)]
        pub id: RefCell<Option<String>>,
        #[property(get, set, construct, default = false)]
        pub is_playable: RefCell<bool>,
        #[property(get, set, default = false)]
        pub played: RefCell<bool>,
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
                spawn_tokio(async move { JELLYFIN_CLIENT.like(&id).await }).await
            } else {
                spawn_tokio(async move { JELLYFIN_CLIENT.unlike(&id).await }).await
            };

            match result {
                Ok(_) => {
                    self.toast("Success");
                }
                Err(e) => {
                    self.toast(e.to_user_facing());
                }
            }
        }
    }

    pub fn edit_metadata_action(&self) -> gio::SimpleActionGroup {
        let action_group = gio::SimpleActionGroup::new();
        action_group.add_action_entries([gio::ActionEntry::builder("editm")
            .activate(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |_, _, _| {
                    use crate::{
                        insert_editm_dialog,
                        ui::widgets::metadata_dialog::MetadataDialog,
                    };
                    let id = obj.id();
                    if let Some(id) = id {
                        let dialog = MetadataDialog::new(&id);
                        insert_editm_dialog!(obj, dialog);
                    }
                }
            ))
            .build()]);
        action_group.add_action_entries([gio::ActionEntry::builder("editi")
            .activate(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |_, _, _| {
                    use crate::{
                        insert_editm_dialog,
                        ui::widgets::image_dialog::ImageDialog,
                    };
                    let id = obj.id();
                    if let Some(id) = id {
                        let dialog = ImageDialog::new(&id);
                        insert_editm_dialog!(obj, dialog);
                    }
                }
            ))
            .build()]);
        if self.is_playable() {
            if self.played() {
                action_group.add_action_entries([gio::ActionEntry::builder("unplayed")
                    .activate(glib::clone!(
                        #[weak(rename_to = obj)]
                        self,
                        move |_, _, _| {
                            let id = obj.id();
                            if let Some(id) = id {
                                spawn(glib::clone!(
                                    #[weak]
                                    obj,
                                    async move {
                                        match spawn_tokio(async move {
                                            JELLYFIN_CLIENT.set_as_unplayed(&id).await
                                        })
                                        .await
                                        {
                                            Ok(_) => {
                                                obj.set_played(false);
                                                obj.toast("Success");
                                                obj.bind_edit();
                                            }
                                            Err(e) => {
                                                obj.toast(e.to_user_facing());
                                            }
                                        }
                                    }
                                ));
                            }
                        }
                    ))
                    .build()]);
            } else {
                action_group.add_action_entries([gio::ActionEntry::builder("played")
                    .activate(glib::clone!(
                        #[weak(rename_to = obj)]
                        self,
                        move |_, _, _| {
                            let id = obj.id();
                            if let Some(id) = id {
                                spawn(glib::clone!(
                                    #[weak]
                                    obj,
                                    async move {
                                        match spawn_tokio(async move {
                                            JELLYFIN_CLIENT.set_as_played(&id).await
                                        })
                                        .await
                                        {
                                            Ok(_) => {
                                                obj.set_played(true);
                                                obj.toast("Success");
                                                obj.bind_edit();
                                            }
                                            Err(e) => {
                                                obj.toast(e.to_user_facing());
                                            }
                                        }
                                    }
                                ));
                            }
                        }
                    ))
                    .build()]);
            }
        }
        action_group
    }

    pub fn bind_edit(&self) {
        self.insert_action_group("item", Some(&self.edit_metadata_action()));
    }

    pub fn set_btn_active(&self, active: bool) {
        self.imp().favourite_button.set_active(active);
    }
}
