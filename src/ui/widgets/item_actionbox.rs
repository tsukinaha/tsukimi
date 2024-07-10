use crate::utils::spawn;

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
        #[property(get, set, construct, default = false)]
        pub is_playable: RefCell<bool>,
        #[property(get, set, default = false)]
        pub played: RefCell<bool>,
        #[property(get, set, nullable)]
        pub episode_id: RefCell<Option<String>>,
        #[property(get, set, default = false)]
        pub episode_played: RefCell<bool>,
        #[property(get, set, default = false)]
        pub episode_liked: RefCell<bool>,
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
        action_group.add_action_entries([gio::ActionEntry::builder("editi")
            .activate(glib::clone!(@weak self as obj => move |_, _, _| {
                use crate::ui::widgets::image_dialog::ImagesDialog;
                use crate::insert_editm_dialog;
                let id = obj.id();
                if let Some(id) = id {
                    let dialog = ImagesDialog::new(&id);
                    insert_editm_dialog!(obj, dialog);
                }
            }))
            .build()]);
        if self.is_playable() {
            if self.played() {
                action_group.add_action_entries([gio::ActionEntry::builder("unplayed")
                    .activate(glib::clone!(@weak self as obj => move |_, _, _| {
                        let id = obj.id();
                        if let Some(id) = id {
                            spawn(glib::clone!(@weak obj => async move {
                                match spawn_tokio(async move { EMBY_CLIENT.set_as_unplayed(&id).await }).await
                                {
                                    Ok(_) => {
                                        obj.set_played(false);
                                        toast!(obj, "Success");
                                        obj.bind_edit();
                                    }
                                    Err(e) => {
                                        toast!(obj, e.to_user_facing());
                                    }
                                }
                            })); 
                        }
                    }))
                    .build()]);
            } else {
                action_group.add_action_entries([gio::ActionEntry::builder("played")
                    .activate(glib::clone!(@weak self as obj => move |_, _, _| {
                        let id = obj.id();
                        if let Some(id) = id {
                            spawn(glib::clone!(@weak obj => async move {
                                match spawn_tokio(async move { EMBY_CLIENT.set_as_played(&id).await }).await
                                {
                                    Ok(_) => {
                                        obj.set_played(true);
                                        toast!(obj, "Success");
                                        obj.bind_edit();
                                    }
                                    Err(e) => {
                                        toast!(obj, e.to_user_facing());
                                    }
                                }
                            })); 
                        }
                    }))
                    .build()]);
            }
        }

        if let Some(episode_id) = self.episode_id() {
            let episode_id_clone = episode_id.clone();
            if self.episode_played() {
                action_group.add_action_entries([gio::ActionEntry::builder("episodeunplayed")
                    .activate(glib::clone!(@weak self as obj => move |_, _, _| {
                        
                            spawn(glib::clone!(@weak obj,@strong episode_id => async move {
                                match spawn_tokio(async move { EMBY_CLIENT.set_as_unplayed(&episode_id).await }).await
                                {
                                    Ok(_) => {
                                        obj.set_episode_played(false);
                                        toast!(obj, "Success");
                                        obj.bind_edit();
                                    }
                                    Err(e) => {
                                        toast!(obj, e.to_user_facing());
                                    }
                                }
                            })); 
                        
                    }))
                    .build()]);
            } else {
                action_group.add_action_entries([gio::ActionEntry::builder("episodeplayed")
                    .activate(glib::clone!(@weak self as obj => move |_, _, _| {
                        
                            spawn(glib::clone!(@weak obj,@strong episode_id => async move {
                                match spawn_tokio(async move { EMBY_CLIENT.set_as_played(&episode_id).await }).await
                                {
                                    Ok(_) => {
                                        obj.set_episode_played(true);
                                        toast!(obj, "Success");
                                        obj.bind_edit();
                                    }
                                    Err(e) => {
                                        toast!(obj, e.to_user_facing());
                                    }
                                }
                            })); 
                        
                    }))
                    .build()]);
            };

            if self.episode_liked() {
                action_group.add_action_entries([gio::ActionEntry::builder("episodeunlike")
                    .activate(glib::clone!(@weak self as obj => move |_, _, _| {
                        
                            spawn(glib::clone!(@weak obj,@strong episode_id_clone => async move {
                                match spawn_tokio(async move { EMBY_CLIENT.unlike(&episode_id_clone).await }).await
                                {
                                    Ok(_) => {
                                        obj.set_episode_liked(false);
                                        toast!(obj, "Success");
                                        obj.bind_edit();
                                    }
                                    Err(e) => {
                                        toast!(obj, e.to_user_facing());
                                    }
                                }
                            })); 
                        
                    }))
                    .build()]);
            } else {
                action_group.add_action_entries([gio::ActionEntry::builder("episodelike")
                    .activate(glib::clone!(@weak self as obj => move |_, _, _| {
                            spawn(glib::clone!(@weak obj,@strong episode_id_clone => async move {
                                match spawn_tokio(async move { EMBY_CLIENT.like(&episode_id_clone).await }).await
                                {
                                    Ok(_) => {
                                        obj.set_episode_liked(true);
                                        toast!(obj, "Success");
                                        obj.bind_edit();
                                    }
                                    Err(e) => {
                                        toast!(obj, e.to_user_facing());
                                    }
                                }
                            })); 
                    }))
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
