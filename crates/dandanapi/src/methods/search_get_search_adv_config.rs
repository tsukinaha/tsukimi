use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
#[doc = "### 接口说明 获取高级搜索功能所需的配置项，用于初始化客户端搜索界面。例如类别、标签等。 ### 权限需求 不需要登录状态即可使用。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchGetSearchAdvConfig {
    pub params: SearchGetSearchAdvConfigParams,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchGetSearchAdvConfigParams {
    #[serde(rename = "source")]
    pub source: Option<String>,
}
impl Request for SearchGetSearchAdvConfig {
    type Response = SearchAdvancedConfigResponse;
    type Body = ();
    type Params = SearchGetSearchAdvConfigParams;
    const METHOD: Method = Method::GET;
    const PATH: &'static str = "/api/v2/search/adv/config";
    fn params(&self) -> Option<&Self::Params> {
        Some(&self.params)
    }
}
