use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
#[doc = "### 接口说明 此接口用于获取用户当前关注的所有动画作品信息 ### 权限需求 此接口需要登录状态才能调用"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FavoriteGetUserFavorite {
    pub params: FavoriteGetUserFavoriteParams,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FavoriteGetUserFavoriteParams {
    #[doc = "只返回正在连载的作品"]
    #[serde(rename = "onlyOnAir")]
    pub only_on_air: bool,
}
impl Request for FavoriteGetUserFavorite {
    type Response = UserFavoriteResponse;
    type Body = ();
    type Params = FavoriteGetUserFavoriteParams;
    const METHOD: Method = Method::GET;
    const PATH: &'static str = "/api/v2/favorite";
    fn params(&self) -> Option<&Self::Params> {
        Some(&self.params)
    }
}
