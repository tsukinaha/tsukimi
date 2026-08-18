use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
use std::borrow::Cow;
#[doc = "### 接口说明 此接口用于获取指定作品的用户短评论/吐槽列表。 ### 参数说明 `bangumiId`：支持传入数字形式的 animeId（如 18319）或字符串形式的 bangumiId（如 \"tmdb-movie-21832\"）。 `page`：页码，从0开始。每页固定返回最新20条评论，最多支持到第9页。 ### 所需权限 此接口无需登录状态即可调用。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiGetBangumiComments {
    #[doc = "作品编号"]
    pub bangumi_id: String,
    pub params: BangumiGetBangumiCommentsParams,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiGetBangumiCommentsParams {
    #[doc = "页码，从0开始，最大为9"]
    #[serde(rename = "page")]
    pub page: i32,
}
impl Request for BangumiGetBangumiComments {
    type Response = BangumiCommentsResponse;
    type Body = ();
    type Params = BangumiGetBangumiCommentsParams;
    const METHOD: Method = Method::GET;
    const PATH: &'static str = "/api/v2/bangumi/{bangumiId}/comments";
    fn params(&self) -> Option<&Self::Params> {
        Some(&self.params)
    }
    fn path(&self) -> Cow<'static, str> {
        let path = Self::PATH.replace("{bangumiId}", &self.bangumi_id.to_string_or_empty());
        Cow::Owned(path)
    }
}
