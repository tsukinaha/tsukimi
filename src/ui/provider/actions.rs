use crate::ui::widgets::actor::imp::ActorPage;
use crate::ui::widgets::item::imp::ItemPage;
use crate::ui::widgets::{boxset::imp::BoxSetPage, music_album::imp::AlbumPage, song_widget::imp::SongWidget};
use crate::ui::widgets::star_toggle::StarToggle;
use gtk::prelude::*;
use gtk::glib;
use gtk::subclass::prelude::ObjectSubclassExt;
use crate::utils::spawn_tokio;
use crate::toast;
use crate::client::client::EMBY_CLIENT;
use crate::utils::spawn;
use crate::client::error::UserFacingError;

pub trait HasLikeAction {
    fn like_button(&self) -> StarToggle;
    async fn bind_actions(&self, id: &str);
}

macro_rules! impl_has_likeaction {
    ($($t:ty),+) => {
        $(
            impl HasLikeAction for $t {
                fn like_button(&self) -> StarToggle {
                    self.favourite_button.clone()
                }

                async fn bind_actions(&self, id: &str) {
                    let obj = self.obj();
                    let like_button = self.like_button();
                    let id = id.to_string();
                    like_button.connect_toggled(
                        glib::clone!(@weak obj => move |button| {
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

impl_has_likeaction!(SongWidget, AlbumPage, BoxSetPage, ActorPage, ItemPage);