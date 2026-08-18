use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
#[doc = "### 接口说明 此接口用于提交用户的播放历史数据，同时可以更新用户对某剧集的评分。 ### 权限需求 此接口需要登录权限才可以调用。 ### 参数限制说明 接口支持单个或批量增加历史数据。 提交的请求中，如果`episodeIdList`数组只包含一条数据，则`addToFavorite`参数（关注此作品）和`rating`参数（更新评分）可以生效。 如果`episodeIdList`数组包含不止一条数据，则会忽略`addToFavorite`和`rating`参数。 在批量添加历史记录时，`episodeIdList`数组最多只能包含100条数据，而且其中的episodeId必须全部属于同一部作品。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayHistoryAddPlayHistory {
    pub body: UserAddPlayHistoryRequest,
}
impl Request for PlayHistoryAddPlayHistory {
    type Response = UserAddPlayHistoryResponse;
    type Body = UserAddPlayHistoryRequest;
    type Params = ();
    const METHOD: Method = Method::POST;
    const PATH: &'static str = "/api/v2/playhistory";
    fn body(&self) -> Option<&Self::Body> {
        Some(&self.body)
    }
}
