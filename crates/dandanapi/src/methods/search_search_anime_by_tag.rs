use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
#[doc = "### 接口说明 根据用户提供的标签列表搜索到对应的作品信息，搜索结果中不包含剧集信息。 ### 权限需求 不需要登录状态即可使用。返回中的`isFavorited`属性目前都为`false`。 ### 返回值 将返回根据提供的标签列表最匹配的作品列表。 ### 标签说明 支持查询多个标签，标签之间用英文逗号分隔。每个标签的长度不超过50个字符。标签数量不超过10个。 标签将区分大小写，且不支持模糊查询。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSearchAnimeByTag {
    pub params: SearchSearchAnimeByTagParams,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSearchAnimeByTagParams {
    #[doc = "标签列表。"]
    #[serde(rename = "tags")]
    pub tags: String,
}
impl Request for SearchSearchAnimeByTag {
    type Response = SearchBangumiResponse;
    type Body = ();
    type Params = SearchSearchAnimeByTagParams;
    const METHOD: Method = Method::GET;
    const PATH: &'static str = "/api/v2/search/tag";
    fn params(&self) -> Option<&Self::Params> {
        Some(&self.params)
    }
}
