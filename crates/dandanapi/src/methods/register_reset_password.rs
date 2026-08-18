use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
#[doc = "### 接口说明 通过此接口可以重置一个用户的密码至随机密码，重置成功后新的随机密码将会发送到对应的邮箱中。 调用此接口需要有应用的AppId与AppSecret，您可以联系弹弹play开发方申请。 ### 请求说明 请求参数中`userName`和`email`必须和注册时的信息完全一致，方能成功重置。 重置密码的请求每2分钟只能发送一次，否则会返回错误信息。 ### Hash计算方法 Hash属性的计算方法为，将登录请求中 `appId` `email` `unixTimestamp` `userName` 属性的值加上您应用的 `AppSecret` 密钥的值按顺序拼接起来， 计算出32位MD5（不区分大小写）。举例来说，`appId`为`dandanplay`，AppSecret为`FFFFF`，用户名为`test1`，邮箱为`test3@example.com`， 那么计算方法将会是 `hash=MD5(dandanplaytest3@example.com666666666test1FFFFF)`。 ### 错误代码 当调用接口发生错误时，例如参数不完整、验证错误、登录失败，`success`属性值将为`false`，`errorCode`代码将不为`0`， 同时`errorMessage`属性将包含错误的描述信息 。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResetPassword {
    pub body: ResetPasswordRequestV2,
}
impl Request for RegisterResetPassword {
    type Response = ResetPasswordResponseV2;
    type Body = ResetPasswordRequestV2;
    type Params = ();
    const METHOD: Method = Method::POST;
    const PATH: &'static str = "/api/v2/register/resetpassword";
    fn body(&self) -> Option<&Self::Body> {
        Some(&self.body)
    }
}
