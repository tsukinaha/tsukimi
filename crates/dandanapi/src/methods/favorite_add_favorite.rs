use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
#[doc = "### 接口说明 此接口用于为用户增加关注某一部作品。 ### 权限需求 此接口需要登录状态才能调用，同时应用应拥有添加关注的权限。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FavoriteAddFavorite {
    pub body: UserAddFavoriteRequest,
}
impl Request for FavoriteAddFavorite {
    type Response = UserAddFavoriteResponse;
    type Body = UserAddFavoriteRequest;
    type Params = ();
    const METHOD: Method = Method::POST;
    const PATH: &'static str = "/api/v2/favorite";
    fn body(&self) -> Option<&Self::Body> {
        Some(&self.body)
    }
}
