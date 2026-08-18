use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
use std::borrow::Cow;
#[doc = "### 接口说明 此接口用于获取服务器上指定弹幕库的弹幕。获取到的弹幕包括弹弹play官方弹幕、第三方网站关联弹幕和开放弹幕网络应用发送的弹幕。 ### withRelated 参数 当`withRelated`参数为`true`时，接口将会返回此弹幕库对应的所有第三方关联网址的弹幕。推荐使用此参数获取整合后的弹幕。 ### 接口跳转 在调用此接口时，将会跳转到弹幕加速服务上获取弹幕。返回的状态码为302，Location头部包含了跳转的地址。 ### 开放弹幕网络应用 当应用使用 `POST /comment/{episodeId}/app` 接口发送弹幕后，再使用此接口获取弹幕时，返回的弹幕中将包含本应用发送的弹幕。 不同应用发送的弹幕将分别存储在不同的私有弹幕库中，互不干扰。 ### 返回值 字段`p`的说明：格式为`出现时间,模式,颜色,用户ID`，各个值之间使用英文逗号分隔 * 弹幕出现时间：格式为 0.00，单位为秒，精确到小数点后两位，例如12.34、445.6、789.01 * 弹幕模式：1-普通弹幕，4-底部弹幕，5-顶部弹幕 * 颜色：32位整数表示的颜色，算法为 Rx256x256+Gx256+B，R/G/B的范围应是0-255 * 用户ID：字符串形式表示的用户ID，通常为数字，不会包含特殊字符"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentGetComment {
    #[doc = "弹幕库编号"]
    pub episode_id: i64,
    pub params: CommentGetCommentParams,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentGetCommentParams {
    #[doc = "起始弹幕编号，忽略此编号以前的弹幕。默认值为`0`。"]
    #[serde(rename = "from")]
    pub from: i64,
    #[doc = "是否同时获取关联的第三方弹幕。默认值为`false`，推荐使用`true`。"]
    #[serde(rename = "withRelated")]
    pub with_related: bool,
    #[doc = "中文简繁转换。`0`-不转换，`1`-转换为简体，`2`-转换为繁体。"]
    #[serde(rename = "chConvert")]
    pub ch_convert: i32,
}
impl Request for CommentGetComment {
    type Response = CommentResponseV2;
    type Body = ();
    type Params = CommentGetCommentParams;
    const METHOD: Method = Method::GET;
    const PATH: &'static str = "/api/v2/comment/{episodeId}";
    fn params(&self) -> Option<&Self::Params> {
        Some(&self.params)
    }
    fn path(&self) -> Cow<'static, str> {
        let path = Self::PATH.replace("{episodeId}", &self.episode_id.to_string_or_empty());
        Cow::Owned(path)
    }
}
