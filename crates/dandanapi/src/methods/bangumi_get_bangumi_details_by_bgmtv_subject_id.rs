use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
use std::borrow::Cow;
#[doc = "### 接口说明 此接口用于通过Bangumi.tv的subjectId获取番剧详情。 弹弹play和Bangumi.tv番剧条目间的映射关系由人工维护，可能会出现错误、缺失、变动或延迟更新的情况，在使用时请注意。 ### 参数说明 `bgmtvSubjectId`：Bangumi.tv 的 subjectId，通常是一个整数。例如，网址 https://bangumi.tv/subject/975 中的 `975` 就是subjectId。 ### 返回值说明 此接口返回和接口 `/bangumi/{bangumiId}` 相同的结构，包含番剧的详细信息。 当没有找到对应的番剧时，会返回资源未找到错误，bangumi字段将为null。 ### 所需权限 此接口无需登录状态即可调用。当提供了token时，返回的剧集列表中将包含当前用户的上次播放时间。"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiGetBangumiDetailsByBgmtvSubjectId {
    #[doc = "Bangumi.tv的subjectId"]
    pub bgmtv_subject_id: i32,
}
impl Request for BangumiGetBangumiDetailsByBgmtvSubjectId {
    type Response = BangumiDetailsResponse;
    type Body = ();
    type Params = ();
    const METHOD: Method = Method::GET;
    const PATH: &'static str = "/api/v2/bangumi/bgmtv/{bgmtvSubjectId}";
    fn path(&self) -> Cow<'static, str> {
        let path = Self::PATH.replace(
            "{bgmtvSubjectId}",
            &self.bgmtv_subject_id.to_string_or_empty(),
        );
        Cow::Owned(path)
    }
}
