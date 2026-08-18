use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
use std::borrow::Cow;
#[doc = "### 接口说明 此接口用于为用户取消关注某一部作品。 ### 权限需求 此接口需要登录状态才能调用，同时应用应拥有取消关注的权限。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FavoriteDeleteFavorite {
    #[doc = "作品编号"]
    pub anime_id: i64,
}
impl Request for FavoriteDeleteFavorite {
    type Response = UserDeleteFavoriteResponse;
    type Body = ();
    type Params = ();
    const METHOD: Method = Method::DELETE;
    const PATH: &'static str = "/api/v2/favorite/{animeId}";
    fn path(&self) -> Cow<'static, str> {
        let path = Self::PATH.replace("{animeId}", &self.anime_id.to_string_or_empty());
        Cow::Owned(path)
    }
}
