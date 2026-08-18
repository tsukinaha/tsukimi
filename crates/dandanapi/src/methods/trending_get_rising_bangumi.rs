use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
use std::borrow::Cow;
#[doc = "### 接口说明 返回最近一个可用统计周期内，相比上一对应周期热度增长最快的番剧列表。 ### 数据口径 飙升榜会综合当前周期热度值与相对上一周期的热度增量计算得分，用于识别最近快速升温的作品。 ### 所需权限 当未提供 jwt token 时，将认为是匿名用户，返回的番剧列表中 `isFavorited` 始终为 `false`。 当提供 jwt token 时（登录状态），返回的番剧列表中将按照当前用户对番剧关注状态设定 `isFavorited` 值。 ### 数据出处说明 使用此榜单数据时，请注明数据来源为`弹弹play开放弹幕网络`。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendingGetRisingBangumi {
    #[doc = "统计周期。可选值：week、month、quarter"]
    pub period: String,
    pub params: TrendingGetRisingBangumiParams,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendingGetRisingBangumiParams {
    #[doc = "是否过滤成人内容"]
    #[serde(rename = "filterAdultContent")]
    pub filter_adult_content: bool,
    #[doc = "返回条目数量，默认20，最大50"]
    #[serde(rename = "limit")]
    pub limit: i32,
}
impl Request for TrendingGetRisingBangumi {
    type Response = TrendingBangumiResponse;
    type Body = ();
    type Params = TrendingGetRisingBangumiParams;
    const METHOD: Method = Method::GET;
    const PATH: &'static str = "/api/v2/trending/all/rising/{period}";
    fn params(&self) -> Option<&Self::Params> {
        Some(&self.params)
    }
    fn path(&self) -> Cow<'static, str> {
        let path = Self::PATH.replace("{period}", &self.period.to_string_or_empty());
        Cow::Owned(path)
    }
}
