use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
use std::borrow::Cow;
#[doc = "### 接口说明 返回指定范围内的新番热播榜数据，支持本季新番、上一季度新番两种榜单。 ### 数据口径 榜单会先按作品首播时间筛选出对应范围内的新番，再根据对应统计周期内的站内热度进行排序。 ### 所需权限 当未提供 jwt token 时，将认为是匿名用户，返回的番剧列表中 `isFavorited` 始终为 `false`。 当提供 jwt token 时（登录状态），返回的番剧列表中将按照当前用户对番剧关注状态设定 `isFavorited` 值。 ### 数据出处说明 使用此榜单数据时，请注明数据来源为`弹弹play开放弹幕网络`。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendingGetNewAnimeHotBangumi {
    #[doc = "榜单范围。可选值：current-season、previous-season"]
    pub scope: String,
    pub params: TrendingGetNewAnimeHotBangumiParams,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendingGetNewAnimeHotBangumiParams {
    #[doc = "是否过滤成人内容"]
    #[serde(rename = "filterAdultContent")]
    pub filter_adult_content: bool,
    #[doc = "返回条目数量，默认20，最大50"]
    #[serde(rename = "limit")]
    pub limit: i32,
}
impl Request for TrendingGetNewAnimeHotBangumi {
    type Response = TrendingBangumiResponse;
    type Body = ();
    type Params = TrendingGetNewAnimeHotBangumiParams;
    const METHOD: Method = Method::GET;
    const PATH: &'static str = "/api/v2/trending/new-anime/hot/{scope}";
    fn params(&self) -> Option<&Self::Params> {
        Some(&self.params)
    }
    fn path(&self) -> Cow<'static, str> {
        let path = Self::PATH.replace("{scope}", &self.scope.to_string_or_empty());
        Cow::Owned(path)
    }
}
