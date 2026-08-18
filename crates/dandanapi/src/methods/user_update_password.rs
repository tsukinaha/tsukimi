use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
#[doc = "### 接口说明 此接口用于为已经登录的用户修改当前的登录密码。 ### 权限需求 此接口需要登录后才可使用（请求中包含Authorization头）"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserUpdatePassword {
    pub body: UserUpdatePasswordRequest,
}
impl Request for UserUpdatePassword {
    type Response = UserUpdateProfileResponseV2;
    type Body = UserUpdatePasswordRequest;
    type Params = ();
    const METHOD: Method = Method::POST;
    const PATH: &'static str = "/api/v2/user/password";
    fn body(&self) -> Option<&Self::Body> {
        Some(&self.body)
    }
}
