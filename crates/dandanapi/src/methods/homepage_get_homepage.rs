use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
#[doc = "### 接口说明 此接口用于一次性获取系统公告、未看剧集列表、当季新番列表、热门种子等接口的数据，并合并为同一个文档进行返回。 ### 权限需求 当未提供jwt token时，将认为是匿名用户，返回的番剧列表中`isFavorited`始终为`false`。 当提供jwt token时（登录状态），返回的番剧列表中将按照当前用户对番剧关注状态设定`isFavorited`值。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomepageGetHomepage {
    pub params: HomepageGetHomepageParams,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomepageGetHomepageParams {
    #[doc = "是否过滤可能出现的成人内容"]
    #[serde(rename = "filterAdultContent")]
    pub filter_adult_content: bool,
}
impl Request for HomepageGetHomepage {
    type Response = HomepageResponseV2;
    type Body = ();
    type Params = HomepageGetHomepageParams;
    const METHOD: Method = Method::GET;
    const PATH: &'static str = "/api/v2/homepage";
    fn params(&self) -> Option<&Self::Params> {
        Some(&self.params)
    }
}
