use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiGetSeasons {}
impl Request for BangumiGetSeasons {
    type Response = BangumiSeasonListResponse;
    type Body = ();
    type Params = ();
    const METHOD: Method = Method::GET;
    const PATH: &'static str = "/api/v2/bangumi/season/anime";
}
