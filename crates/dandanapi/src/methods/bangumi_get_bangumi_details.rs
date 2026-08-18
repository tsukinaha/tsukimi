use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
use std::borrow::Cow;
#[doc = "### 接口说明 此接口用于获取指定编号的作品的详细数据，包括简介、评分、详细剧集等。 ### 参数说明 `bangumiId`：支持传入数字形式的 animeId（如 18319）或字符串形式的 bangumiId（如 \"tmdb-movie-21832\"）。 ### 所需权限 此接口无需登录状态即可调用。当提供了token时，返回的剧集列表中将包含当前用户的上次播放时间。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiGetBangumiDetails {
    #[doc = "作品编号"]
    pub bangumi_id: String,
}
impl Request for BangumiGetBangumiDetails {
    type Response = BangumiDetailsResponse;
    type Body = ();
    type Params = ();
    const METHOD: Method = Method::GET;
    const PATH: &'static str = "/api/v2/bangumi/{bangumiId}";
    fn path(&self) -> Cow<'static, str> {
        let path = Self::PATH.replace("{bangumiId}", &self.bangumi_id.to_string_or_empty());
        Cow::Owned(path)
    }
}
