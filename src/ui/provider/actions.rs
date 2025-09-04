use gtk::{
    glib,
    prelude::*,
    subclass::prelude::ObjectSubclassIsExt,
};

use crate::{
    client::{
        error::UserFacingError,
        jellyfin_client::JELLYFIN_CLIENT,
    },
    ui::{
        GlobalToast,
        widgets::{
            song_widget::SongWidget,
            star_toggle::StarToggle,
        },
    },
    utils::{
        spawn,
        spawn_tokio,
    },
};

pub trait HasLikeAction {
    fn like_button(&self) -> StarToggle;
    async fn bind_like(&self, id: &str);
}

macro_rules! impl_has_likeaction {
    ($($t:ty),+) => {
        $(
            impl HasLikeAction for $t {
                fn like_button(&self) -> StarToggle {
                    self.imp().favourite_button.to_owned()
                }

                async fn bind_like(&self, id: &str) {
                    let like_button = self.like_button();
                    let id = id.to_string();

                    like_button.connect_toggled(
                        glib::clone!(#[weak(rename_to = obj)] self, move |button| {
                            let active = button.is_active();
                            spawn(
                                glib::clone!(#[weak] obj, #[strong] id, async move {

                                    let result = if active {
                                        spawn_tokio(async move {JELLYFIN_CLIENT.like(&id).await} ).await
                                    } else {
                                        spawn_tokio(async move {JELLYFIN_CLIENT.unlike(&id).await} ).await
                                    };

                                    match result {
                                        Ok(_) => {
                                            obj.toast("Success");
                                        }
                                        Err(e) => {
                                            obj.toast(e.to_user_facing());
                                        }
                                    }
                                })
                            );
                        })
                    );
                }
            }
        )+
    };
}

impl_has_likeaction!(SongWidget);
