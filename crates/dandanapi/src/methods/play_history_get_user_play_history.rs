use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
#[doc = "### 接口说明 此接口用于获取用户的播放历史（作品+剧集）。只能获取到用户已关注作品的播放历史。 ### 权限需求 此接口需要登录状态才可以调用。 ### 开始结束日期参数说明 开始日期不能晚于结束日期； 开始日期与结束日期不能相差大于一年（最多查询一年的数据）； 当没有提供`toDate`参数时，默认将使用当前日期； 当没有提供`fromDate`参数时，默认将使用`toDate`减去三个月的日期。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayHistoryGetUserPlayHistory {
    pub params: PlayHistoryGetUserPlayHistoryParams,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayHistoryGetUserPlayHistoryParams {
    #[doc = "开始日期"]
    #[serde(rename = "fromDate")]
    pub from_date: Option<String>,
    #[doc = "结束日期"]
    #[serde(rename = "toDate")]
    pub to_date: Option<String>,
}
impl Request for PlayHistoryGetUserPlayHistory {
    type Response = UserPlayHistoryResponse;
    type Body = ();
    type Params = PlayHistoryGetUserPlayHistoryParams;
    const METHOD: Method = Method::GET;
    const PATH: &'static str = "/api/v2/playhistory";
    fn params(&self) -> Option<&Self::Params> {
        Some(&self.params)
    }
}
