use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
#[doc = "### 接口说明 此接口用于为已经登录的用户修改当前的基本资料（如昵称、头像）。 当提供`screenName`时才更新昵称，提供`profileImageBase64`时才更新头像图片，否则不会产生变化。 ### 更新头像图片 头像图片需要转换成base64编码后放入`profileImageBase64`字段中。此字段长度不能超过1MB。 上传的图片将保留长宽比，转换为边长最长600px的长方形，并存储为jpg格式。 ### 权限需求 此接口需要登录后才可使用（请求中包含Authorization头）"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserUpdateProfile {
    pub body: UserUpdateProfileRequest,
}
impl Request for UserUpdateProfile {
    type Response = UserUpdateProfileResponseV2;
    type Body = UserUpdateProfileRequest;
    type Params = ();
    const METHOD: Method = Method::POST;
    const PATH: &'static str = "/api/v2/user/profile";
    fn body(&self) -> Option<&Self::Body> {
        Some(&self.body)
    }
}
