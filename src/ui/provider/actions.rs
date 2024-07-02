use crate::client::client::EMBY_CLIENT;
use crate::client::error::UserFacingError;
use crate::toast;
use crate::ui::widgets::song_widget::SongWidget;
use crate::ui::widgets::star_toggle::StarToggle;
use crate::utils::spawn;
use crate::utils::spawn_tokio;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::ObjectSubclassIsExt;

pub trait HasLikeAction {
    fn like_button(&self) -> StarToggle;
    async fn bind_like(&self, id: &str);
}

macro_rules! impl_has_likeaction {
    ($($t:ty),+) => {
        $(
            impl HasLikeAction for $t {
                fn like_button(&self) -> StarToggle {
                    self.imp().favourite_button.clone()
                }

                async fn bind_like(&self, id: &str) {
                    let like_button = self.like_button();
                    let id = id.to_string();

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
            }
        )+
    };
}

impl_has_likeaction!(SongWidget);
