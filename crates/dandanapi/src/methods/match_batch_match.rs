use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
#[doc = "### 接口说明 此接口用于批量匹配（参考`/match`接口），可以通过Hash、文件名称等信息查找多个视频对应的节目信息。 每次批量匹配提供的文件信息不能多于`32`个，文件信息中不能有重复项。 此接口只会返回“精确关联”的结果，如果文件未能成功匹配上一个弹幕库，对应匹配结果的`success`将为`false`。 ### 返回值说明 一个包含匹配结果的列表，将与请求中的文件信息一一对应。例如请求中包含了20个文件信息，返回结果的列表中也将包含20个匹配结果。 如果某个文件匹配成功，对应结果的`success`属性将为`true`。如果某文件未匹配成功，或是某个请求未通过验证，对应结果的`success`属性将为`false`。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchBatchMatch {
    pub body: BatchMatchRequest,
}
impl Request for MatchBatchMatch {
    type Response = BatchMatchResponse;
    type Body = BatchMatchRequest;
    type Params = ();
    const METHOD: Method = Method::POST;
    const PATH: &'static str = "/api/v2/match/batch";
    fn body(&self) -> Option<&Self::Body> {
        Some(&self.body)
    }
}
