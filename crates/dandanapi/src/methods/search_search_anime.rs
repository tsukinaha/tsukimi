use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
#[doc = "### 接口说明 根据用户提供的关键词，在弹弹play数据库中搜索对应的作品信息，搜索结果中不包含剧集信息。 ### 权限需求 不需要登录状态即可使用 ### 关键词说明 * 关键词长度至少为`2`。 * 关键词中的空格将被认定为 AND 条件，其他字符将被作为原始字符去搜索。 * 可以通过中文、日文、罗马音、英文等条件对作品的别名进行搜索，繁体中文关键词将被统一为简体中文。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSearchAnime {
    pub params: SearchSearchAnimeParams,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSearchAnimeParams {
    #[doc = "作品标题关键词。"]
    #[serde(rename = "keyword")]
    pub keyword: String,
    #[doc = "可选的作品类型。"]
    #[serde(rename = "type")]
    pub r#type: AnimeType,
    #[doc = "提供 true 时使用新版搜索引擎。默认为`false`。"]
    #[serde(rename = "v2")]
    pub v2: bool,
}
impl Request for SearchSearchAnime {
    type Response = SearchAnimeResponse;
    type Body = ();
    type Params = SearchSearchAnimeParams;
    const METHOD: Method = Method::GET;
    const PATH: &'static str = "/api/v2/search/anime";
    fn params(&self) -> Option<&Self::Params> {
        Some(&self.params)
    }
}
