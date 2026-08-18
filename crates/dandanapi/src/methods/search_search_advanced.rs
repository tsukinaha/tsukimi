use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSearchAdvanced {
    pub params: SearchSearchAdvancedParams,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSearchAdvancedParams {
    #[doc = "数据源。anidb|tmdb。默认为anidb"]
    #[serde(rename = "source")]
    pub source: Option<String>,
    #[doc = "作品标题关键词"]
    #[serde(rename = "keyword")]
    pub keyword: Option<String>,
    #[doc = "作品类型"]
    #[serde(rename = "type")]
    pub r#type: Option<i32>,
    #[doc = "标签，一个或多个数字。若填写多个数字请用英文逗号隔开，例如 12,34,56 。设定多个数字时将搜索同时包含这些标签的作品。"]
    #[serde(rename = "tags")]
    pub tags: Option<String>,
    #[doc = "限定作品上映的年份"]
    #[serde(rename = "year")]
    pub year: Option<i32>,
    #[doc = "限定年份前提下继续限定作品月份"]
    #[serde(rename = "month")]
    pub month: Option<i32>,
    #[doc = "限定最低评分（包含）"]
    #[serde(rename = "minRate")]
    pub min_rate: i32,
    #[doc = "限定最高评分（包含）"]
    #[serde(rename = "maxRate")]
    pub max_rate: i32,
    #[doc = "只显示限制级别的内容。不提供此参数则不过滤结果，提供true或false都将过滤结果。"]
    #[serde(rename = "restricted")]
    pub restricted: Option<bool>,
    #[doc = "设定排序规则"]
    #[serde(rename = "sort")]
    pub sort: i32,
    #[doc = "提供 true 且数据源为 anidb 时使用新版搜索引擎。默认为`false`。"]
    #[serde(rename = "v2")]
    pub v2: bool,
}
impl Request for SearchSearchAdvanced {
    type Response = SearchBangumiResponse;
    type Body = ();
    type Params = SearchSearchAdvancedParams;
    const METHOD: Method = Method::GET;
    const PATH: &'static str = "/api/v2/search/adv";
    fn params(&self) -> Option<&Self::Params> {
        Some(&self.params)
    }
}
