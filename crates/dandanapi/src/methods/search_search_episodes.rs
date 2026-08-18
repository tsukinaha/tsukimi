use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
#[doc = "### 接口说明 此接口用于根据关键词搜索所有匹配的剧集信息。 当自动匹配失败或结果不理想时可以调用此接口，让用户手动通过关键词搜索到作品。 ### 参数说明 - anime：作品标题。支持通过中文、日语（含罗马音）、英语搜索，至少为2个字符。 - tmdbId：使用 TMDB 电视剧 ID 搜索作品，如果指定此参数，将仅返回此 TMDB TV ID 的关联作品（可能有多个）。 - tmdbIdType: 指定 tmdbId 的类型，0或不提供表示tmdbId为电视剧ID，1表示tmdbId为电影ID。 - episode：剧集编号，默认为空。支持正整数或 C1/S1/O1 格式，将仅保留指定集数的结果；其他值将被忽略。 - v2：提供 true 时使用新版搜索引擎。 必须提供`anime`和`tmdbId`中至少一个参数。 当同时提供`anime`和`tmdbId`参数时，会先尝试使用`anime`参数进行搜索，之后在搜索结果中匹配`tmdbId`的剧集。 ### 参数注意事项 * 参数可以包含空格，但空格将作为查询字符串的一部分而不是传统的“OR”查询。 * 未提供`episode`参数的情况下，如果`anime`参数中包含空格，且空格后为数字（如“EVA 10”），此数字将被认定为是`episode`参数。 * 如果参数中包含特殊字符，需要经过Url编码后才能传递。 ### 返回值说明 接口将返回包含节目信息的列表，当结果集过大时，`hasMore`属性为`true`，这时客户端应该提示用户填写更详细的信息以缩小搜索范围。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSearchEpisodes {
    pub params: SearchSearchEpisodesParams,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSearchEpisodesParams {
    #[doc = "作品标题。支持通过中文、日语（含罗马音）、英语搜索，至少为2个字符。"]
    #[serde(rename = "anime")]
    pub anime: Option<String>,
    #[doc = "TMDB ID，如果指定此参数，将仅返回此 TMDB ID 的关联作品（可能有多个）。"]
    #[serde(rename = "tmdbId")]
    pub tmdb_id: Option<i32>,
    #[doc = "指定 tmdbId 的类型，0或不提供表示tmdbId为电视剧ID，1表示tmdbId为电影ID。"]
    #[serde(rename = "tmdbIdType")]
    pub tmdb_id_type: i32,
    #[doc = "剧集编号，默认为空。支持正整数或 C1/S1/O1 格式，将仅保留指定集数的结果。 其他值将被忽略。"]
    #[serde(rename = "episode")]
    pub episode: Option<String>,
    #[doc = "提供 true 时使用新版搜索引擎。默认为`false`。"]
    #[serde(rename = "v2")]
    pub v2: bool,
}
impl Request for SearchSearchEpisodes {
    type Response = SearchEpisodesResponse;
    type Body = ();
    type Params = SearchSearchEpisodesParams;
    const METHOD: Method = Method::GET;
    const PATH: &'static str = "/api/v2/search/episodes";
    fn params(&self) -> Option<&Self::Params> {
        Some(&self.params)
    }
}
