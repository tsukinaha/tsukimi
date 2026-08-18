use super::*;
use crate::Request;
use reqwest::Method;
use serde::{
    Deserialize,
    Serialize,
};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomepageGetBanner {}
impl Request for HomepageGetBanner {
    type Response = BannerResponse;
    type Body = ();
    type Params = ();
    const METHOD: Method = Method::GET;
    const PATH: &'static str = "/api/v2/homepage/banner";
}
