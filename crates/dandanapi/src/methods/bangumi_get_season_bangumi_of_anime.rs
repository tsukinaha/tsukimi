use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
use std::borrow::Cow;
#[doc = "### 接口说明 此接口用于获取指定季度中上映的动画番剧列表。 ## 参数说明 Url中的`year`与`month`参数需要先通过`/season/anime`接口获取。 例如2018年只有1、4、7、10四个季度，如果`month`的值不为此四个数字之一将无法获取到对应季度的番剧。 ### 所需权限 当未提供jwt token时，将认为是匿名用户，返回的番剧列表中`isFavorited`始终为`false`。 当提供jwt token时（登录状态），返回的番剧列表中将按照当前用户对番剧关注状态设定`isFavorited`值。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiGetSeasonBangumiOfAnime {
    #[doc = "年份"]
    pub year: i32,
    #[doc = "季度月份（一般指1、4、7、10）"]
    pub month: i32,
    pub params: BangumiGetSeasonBangumiOfAnimeParams,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiGetSeasonBangumiOfAnimeParams {
    #[doc = "是否过滤成人内容"]
    #[serde(rename = "filterAdultContent")]
    pub filter_adult_content: bool,
}
impl Request for BangumiGetSeasonBangumiOfAnime {
    type Response = BangumiListResponse;
    type Body = ();
    type Params = BangumiGetSeasonBangumiOfAnimeParams;
    const METHOD: Method = Method::GET;
    const PATH: &'static str = "/api/v2/bangumi/season/anime/{year}/{month}";
    fn params(&self) -> Option<&Self::Params> {
        Some(&self.params)
    }
    fn path(&self) -> Cow<'static, str> {
        let path = Self::PATH
            .replace("{year}", &self.year.to_string_or_empty())
            .replace("{month}", &self.month.to_string_or_empty());
        Cow::Owned(path)
    }
}
