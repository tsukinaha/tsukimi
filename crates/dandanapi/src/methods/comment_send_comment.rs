use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
use std::borrow::Cow;
#[doc = "### 接口说明 此接口用于弹弹play客户端向服务器的指定弹幕库发送弹幕。 第三方开发者请使用 `/comment/{episodeId}/app` 接口发送弹幕。 ### 权限需求 此接口需要用户登录后才可使用"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentSendComment {
    #[doc = "弹幕库ID"]
    pub episode_id: i64,
    pub body: SendCommentRequest,
}
impl Request for CommentSendComment {
    type Response = SendCommentResponseV2;
    type Body = SendCommentRequest;
    type Params = ();
    const METHOD: Method = Method::POST;
    const PATH: &'static str = "/api/v2/comment/{episodeId}";
    fn body(&self) -> Option<&Self::Body> {
        Some(&self.body)
    }
    fn path(&self) -> Cow<'static, str> {
        let path = Self::PATH.replace("{episodeId}", &self.episode_id.to_string_or_empty());
        Cow::Owned(path)
    }
}
