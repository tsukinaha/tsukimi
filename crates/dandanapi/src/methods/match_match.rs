use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
#[doc = "### 接口说明 此接口用于当用户打开某视频文件时，可以通过文件名称、Hash等信息查找此视频可能对应的节目信息。 此接口首先会使用Hash信息进行搜寻，如果有相应的记录，会返回“精确关联”的结果（即`isMatched`属性为`true`，此时列表中只包含一个搜索结果）。 如果Hash信息匹配失败，则会继续通过文件名进行模糊搜寻。 ### 返回值说明 一个包含节目信息的列表，节目在列表中排名越靠前，这个节目越有可能是视频文件的内容。 当列表中只有一个节目时（`isMatched`属性为`true`），视为“精确关联” —— 说明此视频已被人工关联了某一节目。客户端应自动选择这个唯一的结果，不必再让用户做出选择。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchMatch {
    pub body: MatchRequest,
}
impl Request for MatchMatch {
    type Response = MatchResponseV2;
    type Body = MatchRequest;
    type Params = ();
    const METHOD: Method = Method::POST;
    const PATH: &'static str = "/api/v2/match";
    fn body(&self) -> Option<&Self::Body> {
        Some(&self.body)
    }
}
