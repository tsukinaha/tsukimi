use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
use std::borrow::Cow;
#[doc = "### 接口说明 此接口用于开放弹幕网络第三方应用开发者向指定弹幕库发送弹幕。 调用方通过 AppId/AppSecret 鉴权，可自行设置用户名，弹幕将与官方弹幕分开存储。 应用使用此接口发送弹幕后，使用 `GET /comment/{episodeId}` 接口获取弹幕时，返回的弹幕中将包含本应用发送的弹幕。 不同应用发送的弹幕将分别存储在不同的私有弹幕库中，互不干扰。 ### 权限说明 当前只有`社区合作`和`商业授权`层级的应用有此接口完整额度。其他层级的应用也可以调用此接口，但额度仅限于测试使用。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentSendAppComment {
    #[doc = "弹幕库ID"]
    pub episode_id: i64,
    pub body: SendAppCommentRequest,
}
impl Request for CommentSendAppComment {
    type Response = SendCommentResponseV2;
    type Body = SendAppCommentRequest;
    type Params = ();
    const METHOD: Method = Method::POST;
    const PATH: &'static str = "/api/v2/comment/{episodeId}/app";
    fn body(&self) -> Option<&Self::Body> {
        Some(&self.body)
    }
    fn path(&self) -> Cow<'static, str> {
        let path = Self::PATH.replace("{episodeId}", &self.episode_id.to_string_or_empty());
        Cow::Owned(path)
    }
}
