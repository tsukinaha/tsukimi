use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
#[doc = "### 接口说明 此接口用于获取官方的新番列表 ### 所需权限 当未提供jwt token时，将认为是匿名用户，返回的番剧列表中`isFavorited`始终为`false`。 当提供jwt token时（登录状态），返回的番剧列表中将按照当前用户对番剧关注状态设定`isFavorited`值。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiGetShinBangumi {
    pub params: BangumiGetShinBangumiParams,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiGetShinBangumiParams {
    #[doc = "是否过滤成人内容"]
    #[serde(rename = "filterAdultContent")]
    pub filter_adult_content: bool,
}
impl Request for BangumiGetShinBangumi {
    type Response = BangumiListResponse;
    type Body = ();
    type Params = BangumiGetShinBangumiParams;
    const METHOD: Method = Method::GET;
    const PATH: &'static str = "/api/v2/bangumi/shin";
    fn params(&self) -> Option<&Self::Params> {
        Some(&self.params)
    }
}
