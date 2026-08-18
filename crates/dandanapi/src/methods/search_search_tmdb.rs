use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
#[doc = "### 接口说明 根据用户提供的关键词，在TMDB数据库中搜索作品，搜索结果中不包含剧集信息。 ### 权限需求 不需要登录状态即可使用 ### 关键词说明 * 关键词长度至少为`2`。 * 可以通过中文、日文、罗马音、英文等条件对作品的别名进行搜索。 ### 返回结果 返回结果中将包含TMDB电视剧和电影的搜索结果。电视剧结果排列在前，电影将排列在后。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSearchTmdb {
    pub params: SearchSearchTmdbParams,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSearchTmdbParams {
    #[serde(rename = "keyword")]
    pub keyword: String,
}
impl Request for SearchSearchTmdb {
    type Response = SearchAnimeResponse;
    type Body = ();
    type Params = SearchSearchTmdbParams;
    const METHOD: Method = Method::GET;
    const PATH: &'static str = "/api/v2/search/tmdb";
    fn params(&self) -> Option<&Self::Params> {
        Some(&self.params)
    }
}
