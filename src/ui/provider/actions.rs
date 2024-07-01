use crate::client::client::EMBY_CLIENT;
use crate::client::error::UserFacingError;
use crate::toast;
use crate::ui::widgets::song_widget::SongWidget;
use crate::ui::widgets::star_toggle::StarToggle;
use crate::utils::spawn;
use crate::utils::spawn_tokio;
use gtk::prelude::*;
use gtk::subclass::prelude::ObjectSubclassIsExt;
use gtk::{gio, glib};

pub trait HasLikeAction {
    fn like_button(&self) -> StarToggle;
    async fn bind_actions(&self, id: &str);
    fn edit_metadata_action(&self) -> gio::SimpleActionGroup;
    fn bind_edit(&self, id: &str);
}

macro_rules! impl_has_likeaction {
    ($($t:ty),+) => {
        $(
            impl HasLikeAction for $t {
                fn like_button(&self) -> StarToggle {
                    self.imp().favourite_button.clone()
                }

                async fn bind_actions(&self, id: &str) {
                    let like_button = self.like_button();
                    let id = id.to_string();

                    self.bind_edit(&id);

                    like_button.connect_toggled(
                        glib::clone!(@weak self as obj => move |button| {
                            let active = button.is_active();
                            spawn(
                                glib::clone!(@weak obj, @strong id => async move {

                                    let result = if active {
                                        spawn_tokio(async move {EMBY_CLIENT.like(&id).await} ).await
                                    } else {
                                        spawn_tokio(async move {EMBY_CLIENT.unlike(&id).await} ).await
                                    };

                                    match result {
                                        Ok(_) => {
                                            toast!(obj, "Success");
                                        }
                                        Err(e) => {
                                            toast!(obj, e.to_user_facing());
                                        }
                                    }
                                })
                            );
                        })
                    );
                }

                fn edit_metadata_action(&self) -> gio::SimpleActionGroup {
                    let action_group = gio::SimpleActionGroup::new();
                    action_group.add_action_entries([gio::ActionEntry::builder("editm")
                        .activate(glib::clone!(@weak self as obj => move |_, _, _| {
                            use crate::ui::widgets::metadata_dialog::MetadataDialog;
                            use crate::insert_editm_dialog;
                            let dialog = MetadataDialog::new();
                            insert_editm_dialog!(obj, dialog);
                        }))
                        .build()]);
                    action_group
                }

                fn bind_edit(&self, id: &str) {
                    self.insert_action_group("item", Some(&self.edit_metadata_action()));
                }
            }
        )+
    };
}

impl_has_likeaction!(SongWidget);
