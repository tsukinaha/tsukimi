pub trait NotOptionToStringOrEmpty {
    fn to_string_or_empty(&self) -> String;
}

pub trait OptionToStringOrEmpty {
    fn to_string_or_empty(&self) -> String;
}

impl<T> NotOptionToStringOrEmpty for T
where
    T: ToString,
{
    fn to_string_or_empty(&self) -> String {
        self.to_string()
    }
}

impl<T> OptionToStringOrEmpty for Option<T>
where
    T: ToString,
{
    fn to_string_or_empty(&self) -> String {
        match self {
            Some(v) => v.to_string(),
            None => String::new(),
        }
    }
}
use super::types::*;
pub mod favorite_delete_favorite;
pub use favorite_delete_favorite::*;
pub mod comment_send_comment;
pub use comment_send_comment::*;
pub mod search_search_anime;
pub use search_search_anime::*;
pub mod match_batch_match;
pub use match_batch_match::*;
pub mod comment_get_comment;
pub use comment_get_comment::*;
pub mod bangumi_get_season_bangumi_of_anime;
pub use bangumi_get_season_bangumi_of_anime::*;
pub mod bangumi_get_queue_details;
pub use bangumi_get_queue_details::*;
pub mod search_search_episodes;
pub use search_search_episodes::*;
pub mod bangumi_get_bangumi_details_by_bgmtv_subject_id;
pub use bangumi_get_bangumi_details_by_bgmtv_subject_id::*;
pub mod search_search_anime_by_tag;
pub use search_search_anime_by_tag::*;
pub mod search_search_advanced;
pub use search_search_advanced::*;
pub mod play_history_get_user_play_history;
pub use play_history_get_user_play_history::*;
pub mod homepage_get_homepage;
pub use homepage_get_homepage::*;
pub mod play_history_add_play_history;
pub use play_history_add_play_history::*;
pub mod bangumi_get_bangumi_details;
pub use bangumi_get_bangumi_details::*;
pub mod login_login;
pub use login_login::*;
pub mod user_update_profile;
pub use user_update_profile::*;
pub mod register_register_main_user;
pub use register_register_main_user::*;
pub mod search_search_tmdb;
pub use search_search_tmdb::*;
pub mod bangumi_get_queue_intro;
pub use bangumi_get_queue_intro::*;
pub mod favorite_get_user_favorite;
pub use favorite_get_user_favorite::*;
pub mod trending_get_rising_bangumi;
pub use trending_get_rising_bangumi::*;
pub mod bangumi_get_seasons;
pub use bangumi_get_seasons::*;
pub mod favorite_add_favorite;
pub use favorite_add_favorite::*;
pub mod register_find_my_id;
pub use register_find_my_id::*;
pub mod homepage_get_banner;
pub use homepage_get_banner::*;
pub mod bangumi_get_bangumi_comments;
pub use bangumi_get_bangumi_comments::*;
pub mod register_reset_password;
pub use register_reset_password::*;
pub mod user_update_password;
pub use user_update_password::*;
pub mod user_update_user_email;
pub use user_update_user_email::*;
pub mod search_get_search_adv_config;
pub use search_get_search_adv_config::*;
pub mod login_renew_token;
pub use login_renew_token::*;
pub mod match_match;
pub use match_match::*;
pub mod bangumi_get_shin_bangumi;
pub use bangumi_get_shin_bangumi::*;
pub mod trending_get_hot_bangumi;
pub use trending_get_hot_bangumi::*;
pub mod trending_get_new_anime_hot_bangumi;
pub use trending_get_new_anime_hot_bangumi::*;
pub mod comment_send_app_comment;
pub use comment_send_app_comment::*;
