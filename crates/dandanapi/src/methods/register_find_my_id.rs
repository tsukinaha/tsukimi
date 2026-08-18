use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
#[doc = "### 接口说明 通过此接口可以查找一个指定邮箱对应的用户名，查找结果将会发送到对应的邮箱中。 调用此接口需要有应用的AppId与AppSecret，您可以联系弹弹play开发方申请。 ### 请求说明 请求参数中`email`必须和注册时的信息完全一致，方能查找成功。 查找用户名的请求每`10`分钟只能发送一次，否则会返回错误信息。 ### Hash计算方法 Hash属性的计算方法为，将登录请求中 `appId` `email` `unixTimestamp` 属性的值加上您应用的 `AppSecret` 密钥的值按顺序拼接起来， 计算出32位MD5（不区分大小写）。举例来说，`appId`为`dandanplay`，AppSecret为`FFFFF`，邮箱为`test3@example.com`， 那么计算方法将会是 `hash=MD5(dandanplaytest3@example.com666666666FFFFF)`。 ### 错误代码 当调用接口发生错误时，例如参数不完整、验证错误、登录失败，`success`属性值将为`false`，`errorCode`代码将不为`0`， 同时`errorMessage`属性将包含错误的描述信息 。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterFindMyId {
    pub body: FindMyIdRequestV2,
}
impl Request for RegisterFindMyId {
    type Response = FindMyIdResponse;
    type Body = FindMyIdRequestV2;
    type Params = ();
    const METHOD: Method = Method::POST;
    const PATH: &'static str = "/api/v2/register/findmyid";
    fn body(&self) -> Option<&Self::Body> {
        Some(&self.body)
    }
}
