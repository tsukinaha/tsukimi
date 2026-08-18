use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
#[doc = "### 接口说明 默认情况下Token的有效期为21天，此接口用于在此期间延长一个有效的JWT Token的有效时间。 ### 权限需求 此接口需要登录后才可使用（请求中包含Authorization头） ### 返回值说明 调用此接口后相当于重新使用当前用户的信息进行重新登录，将会返回最新的用户信息（包括已延长有效期的JWT Token）。 如果应用或用户的状态异常，将会返回相应的错误代码。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRenewToken {}
impl Request for LoginRenewToken {
    type Response = LoginResponse;
    type Body = ();
    type Params = ();
    const METHOD: Method = Method::GET;
    const PATH: &'static str = "/api/v2/login/renew";
}
