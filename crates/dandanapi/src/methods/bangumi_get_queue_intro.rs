use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
#[doc = "### 接口说明 此接口用户获取用户近期关注但未看/未看完的番剧的列表。 ### 权限需求 此接口需要登录状态才可调用。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiGetQueueIntro {}
impl Request for BangumiGetQueueIntro {
    type Response = BangumiQueueIntroResponseV2;
    type Body = ();
    type Params = ();
    const METHOD: Method = Method::GET;
    const PATH: &'static str = "/api/v2/bangumi/queue/intro";
}
