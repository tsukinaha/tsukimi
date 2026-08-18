use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
use std::borrow::Cow;
#[doc = "### 接口说明 返回最近一个可用统计周期内的全站热播榜数据。 ### 数据口径 榜单的热度值来自弹幕库访问计数的按日汇总结果。 ### 所需权限 当未提供 jwt token 时，将认为是匿名用户，返回的番剧列表中 `isFavorited` 始终为 `false`。 当提供 jwt token 时（登录状态），返回的番剧列表中将按照当前用户对番剧关注状态设定 `isFavorited` 值。 ### 数据出处说明 使用此榜单数据时，请注明数据来源为`弹弹play开放弹幕网络`。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendingGetHotBangumi {
    #[doc = "统计周期。可选值：week、month、quarter"]
    pub period: String,
    pub params: TrendingGetHotBangumiParams,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendingGetHotBangumiParams {
    #[doc = "是否过滤成人内容"]
    #[serde(rename = "filterAdultContent")]
    pub filter_adult_content: bool,
    #[doc = "返回条目数量，默认20，最大50"]
    #[serde(rename = "limit")]
    pub limit: i32,
}
impl Request for TrendingGetHotBangumi {
    type Response = TrendingBangumiResponse;
    type Body = ();
    type Params = TrendingGetHotBangumiParams;
    const METHOD: Method = Method::GET;
    const PATH: &'static str = "/api/v2/trending/all/hot/{period}";
    fn params(&self) -> Option<&Self::Params> {
        Some(&self.params)
    }
    fn path(&self) -> Cow<'static, str> {
        let path = Self::PATH.replace("{period}", &self.period.to_string_or_empty());
        Cow::Owned(path)
    }
}
