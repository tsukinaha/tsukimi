use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
#[doc = "### 接口说明 通过此接口可以使用用户名/密码获取到后续接口需要的JWT Token。 调用此接口需要有应用的AppId与AppSecret，您可以联系弹弹play开发方申请。 ### Hash计算方法 Hash属性的计算方法为，将登录请求中 `appId` `password` `unixTimestamp` `userName` 属性的值以及您应用的 `AppSecret` 密钥的值依次拼接起来， 计算出32位MD5（不区分大小写）。举例来说，`appId`为`dandanplay`，AppSecret为`FFFFF`，用户名为`test1`，密码为`test2`， 那么计算方法将会是 `hash=MD5(dandanplaytest2666666666test1FFFFF)`。 ### 错误代码 当调用接口发生错误时，例如参数不完整、验证错误、登录失败，`success`属性值将为`false`，`errorCode`代码将不为`0`， 同时`errorMessage`属性将包含错误的描述信息 。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginLogin {
    pub body: LoginRequest,
}
impl Request for LoginLogin {
    type Response = LoginResponse;
    type Body = LoginRequest;
    type Params = ();
    const METHOD: Method = Method::POST;
    const PATH: &'static str = "/api/v2/login";
    fn body(&self) -> Option<&Self::Body> {
        Some(&self.body)
    }
}
