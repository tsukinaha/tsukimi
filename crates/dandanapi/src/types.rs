use serde::{
    Deserialize,
    Serialize,
};
use std::collections::HashMap;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiListResponse {
    #[serde(flatten)]
    pub response_base: ResponseBase,
    #[doc = "番剧列表"]
    #[serde(rename = "bangumiList")]
    pub bangumi_list: Option<Vec<BangumiIntro>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiIntro {
    #[doc = "作品编号"]
    #[serde(rename = "animeId")]
    pub anime_id: Option<i64>,
    #[doc = "作品ID（新）"]
    #[serde(rename = "bangumiId")]
    pub bangumi_id: Option<String>,
    #[doc = "作品标题"]
    #[serde(rename = "animeTitle")]
    pub anime_title: Option<String>,
    #[doc = "海报图片地址"]
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
    #[doc = "搜索关键词"]
    #[serde(rename = "searchKeyword")]
    pub search_keyword: Option<String>,
    #[doc = "是否正在连载中"]
    #[serde(rename = "isOnAir")]
    pub is_on_air: Option<bool>,
    #[doc = "周几上映，0代表周日，1-6代表周一至周六"]
    #[serde(rename = "airDay")]
    pub air_day: Option<i32>,
    #[doc = "当前用户是否已关注（无论是否为已弃番等附加状态）"]
    #[serde(rename = "isFavorited")]
    pub is_favorited: Option<bool>,
    #[doc = "是否为限制级别的内容（例如属于R18分级）"]
    #[serde(rename = "isRestricted")]
    pub is_restricted: Option<bool>,
    #[doc = "番剧综合评分（综合多个来源的评分求出的加权平均值，0-10分）"]
    pub rating: Option<f32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseBase {
    #[doc = "错误代码，0表示没有发生错误，非0表示有错误，详细信息会包含在errorMessage属性中"]
    #[serde(rename = "errorCode")]
    pub error_code: Option<i32>,
    #[doc = "接口是否调用成功"]
    pub success: Option<bool>,
    #[doc = "当发生错误时，说明错误具体原因"]
    #[serde(rename = "errorMessage")]
    pub error_message: Option<String>,
    #[doc = "当参数校验失败时，提供可供调用方定位问题字段的补充信息。"]
    #[serde(rename = "errorDetail")]
    pub error_detail: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiSeasonListResponse {
    #[serde(flatten)]
    pub response_base: ResponseBase,
    #[doc = "番剧季度列表"]
    pub seasons: Option<Vec<BangumiSeason>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiSeason {
    #[doc = "年份"]
    pub year: Option<i32>,
    #[doc = "月份"]
    pub month: Option<i32>,
    #[doc = "季度名称"]
    #[serde(rename = "seasonName")]
    pub season_name: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiQueueIntroResponseV2 {
    #[serde(flatten)]
    pub response_base: ResponseBase,
    #[doc = "是否有更多数据可以展示（显示界面上的“更多”按钮）"]
    #[serde(rename = "hasMore")]
    pub has_more: Option<bool>,
    #[doc = "未看剧集列表"]
    #[serde(rename = "bangumiList")]
    pub bangumi_list: Option<Vec<BangumiQueueIntroV2>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiQueueIntroV2 {
    #[doc = "作品编号"]
    #[serde(rename = "animeId")]
    pub anime_id: Option<i64>,
    #[doc = "作品标题"]
    #[serde(rename = "animeTitle")]
    pub anime_title: Option<String>,
    #[doc = "最新一集的剧集标题"]
    #[serde(rename = "episodeTitle")]
    pub episode_title: Option<String>,
    #[doc = "剧集上映日期（无小时分钟，当地时间）"]
    #[serde(rename = "airDate")]
    pub air_date: Option<String>,
    #[doc = "海报图片地址"]
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
    #[doc = "未看状态的说明，如“今天更新”，“昨天更新”，“有多集未看”等"]
    pub description: Option<String>,
    #[doc = "番剧是否在连载中"]
    #[serde(rename = "isOnAir")]
    pub is_on_air: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiQueueDetailsResponseV2 {
    #[serde(flatten)]
    pub response_base: ResponseBase,
    #[doc = "未看番剧剧集列表"]
    #[serde(rename = "bangumiList")]
    pub bangumi_list: Option<Vec<BangumiQueueDetailsV2>>,
    #[doc = "已关注但从未看过的番剧列表"]
    #[serde(rename = "unwatchedBangumiList")]
    pub unwatched_bangumi_list: Option<Vec<BangumiQueueDetailsV2>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiQueueDetailsV2 {
    #[doc = "作品编号"]
    #[serde(rename = "animeId")]
    pub anime_id: Option<i64>,
    #[doc = "作品标题"]
    #[serde(rename = "animeTitle")]
    pub anime_title: Option<String>,
    #[doc = "是否正在连载中"]
    #[serde(rename = "isOnAir")]
    pub is_on_air: Option<bool>,
    #[doc = "海报图片地址"]
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
    #[doc = "搜索资源的关键词"]
    #[serde(rename = "searchKeyword")]
    pub search_keyword: Option<String>,
    #[doc = "上次观看时间（null表示尚未看过）"]
    #[serde(rename = "lastWatched")]
    pub last_watched: Option<String>,
    #[doc = "未看剧集的列表"]
    pub episodes: Option<Vec<BangumiQueueEpisodeV2>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiQueueEpisodeV2 {
    #[doc = "剧集编号（弹幕库编号）"]
    #[serde(rename = "episodeId")]
    pub episode_id: Option<i64>,
    #[doc = "剧集标题"]
    #[serde(rename = "episodeTitle")]
    pub episode_title: Option<String>,
    #[doc = "上映日期（无小时分钟，当地时间），可能为null"]
    #[serde(rename = "airDate")]
    pub air_date: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiDetailsResponse {
    #[serde(flatten)]
    pub response_base: ResponseBase,
    #[doc = "番剧详情"]
    pub bangumi: Option<BangumiDetails>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiDetails {
    #[serde(flatten)]
    pub bangumi_intro: BangumiIntro,
    #[doc = "作品类型"]
    pub r#type: Option<AnimeType>,
    #[doc = "类型描述"]
    #[serde(rename = "typeDescription")]
    pub type_description: Option<String>,
    #[doc = "作品标题"]
    pub titles: Option<Vec<BangumiTitle>>,
    #[doc = "作品季度列表。可能为空，仅对部分源（如TMDB源）有效"]
    pub seasons: Option<Vec<BangumiEpisodeSeason>>,
    #[doc = "剧集列表"]
    pub episodes: Option<Vec<BangumiEpisode>>,
    #[doc = "番剧简介"]
    pub summary: Option<String>,
    #[doc = "短简介（Staff简介或剧情简介）"]
    pub intro: Option<String>,
    #[doc = "番剧元数据（名称、制作人员、配音人员等）"]
    pub metadata: Option<Vec<String>>,
    #[doc = "Bangumi.tv页面地址"]
    #[serde(rename = "bangumiUrl")]
    pub bangumi_url: Option<String>,
    #[doc = "用户个人评分（0-10）"]
    #[serde(rename = "userRating")]
    pub user_rating: Option<i32>,
    #[doc = "关注状态"]
    #[serde(rename = "favoriteStatus")]
    pub favorite_status: Option<FavoriteStatus>,
    #[doc = "用户对此番剧的备注/评论/标签"]
    pub comment: Option<String>,
    #[doc = "各个站点的评分详情"]
    #[serde(rename = "ratingDetails")]
    pub rating_details: Option<HashMap<String, f32>>,
    #[doc = "与此作品直接关联的其他作品（例如同一作品的不同季、剧场版、OVA等）"]
    pub relateds: Option<Vec<BangumiIntro>>,
    #[doc = "与此作品相似的其他作品"]
    pub similars: Option<Vec<BangumiIntro>>,
    #[doc = "标签列表"]
    pub tags: Option<Vec<BangumiTag>>,
    #[doc = "此作品在其他在线数据库/网站的对应url"]
    #[serde(rename = "onlineDatabases")]
    pub online_databases: Option<Vec<BangumiOnlineDatabase>>,
    #[doc = "预告片列表"]
    pub trailers: Option<Vec<BangumiTrailer>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnimeType {
    #[serde(rename = "tvseries")]
    Tvseries,
    #[serde(rename = "tvspecial")]
    Tvspecial,
    #[serde(rename = "ova")]
    Ova,
    #[serde(rename = "movie")]
    Movie,
    #[serde(rename = "musicvideo")]
    Musicvideo,
    #[serde(rename = "web")]
    Web,
    #[serde(rename = "other")]
    Other,
    #[serde(rename = "jpmovie")]
    Jpmovie,
    #[serde(rename = "jpdrama")]
    Jpdrama,
    #[serde(rename = "unknown")]
    Unknown,
    #[serde(rename = "tmdbtv")]
    Tmdbtv,
    #[serde(rename = "tmdbmovie")]
    Tmdbmovie,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiTitle {
    #[doc = "语言"]
    pub language: Option<String>,
    #[doc = "标题"]
    pub title: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiEpisodeSeason {
    #[doc = "季度ID"]
    pub id: Option<String>,
    #[doc = "上映日期"]
    #[serde(rename = "airDate")]
    pub air_date: Option<String>,
    #[doc = "季度名称"]
    pub name: Option<String>,
    #[doc = "剧集数量"]
    #[serde(rename = "episodeCount")]
    pub episode_count: Option<i32>,
    #[doc = "季度简介"]
    pub summary: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiEpisode {
    #[doc = "季度ID（如果为空表示只有一个季度）"]
    #[serde(rename = "seasonId")]
    pub season_id: Option<String>,
    #[doc = "剧集ID（弹幕库编号）"]
    #[serde(rename = "episodeId")]
    pub episode_id: Option<i64>,
    #[doc = "剧集完整标题"]
    #[serde(rename = "episodeTitle")]
    pub episode_title: Option<String>,
    #[doc = "剧集短标题（可以用来排序，非纯数字，可能包含字母）"]
    #[serde(rename = "episodeNumber")]
    pub episode_number: Option<String>,
    #[doc = "上次观看时间（服务器时间，即北京时间）"]
    #[serde(rename = "lastWatched")]
    pub last_watched: Option<String>,
    #[doc = "本集上映时间（当地时间）"]
    #[serde(rename = "airDate")]
    pub air_date: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FavoriteStatus {
    #[serde(rename = "favorited")]
    Favorited,
    #[serde(rename = "finished")]
    Finished,
    #[serde(rename = "abandoned")]
    Abandoned,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiTag {
    #[doc = "标签编号"]
    pub id: Option<i32>,
    #[doc = "标签内容"]
    pub name: Option<String>,
    #[doc = "观众为此标签+1次数"]
    pub count: Option<i32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiOnlineDatabase {
    #[doc = "网站名称"]
    pub name: Option<String>,
    #[doc = "网址"]
    pub url: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiTrailer {
    #[doc = "视频编号"]
    pub id: Option<i32>,
    #[doc = "视频播放页地址"]
    pub url: Option<String>,
    #[doc = "视频标题"]
    pub title: Option<String>,
    #[doc = "视频封面"]
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
    #[doc = "发布时间"]
    pub date: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiCommentsResponse {
    #[serde(flatten)]
    pub response_base: ResponseBase,
    #[doc = "当前页返回的评论数量"]
    pub count: Option<i32>,
    #[doc = "是否还有更多评论可以获取"]
    #[serde(rename = "hasMore")]
    pub has_more: Option<bool>,
    #[doc = "评论列表"]
    pub comments: Option<Vec<BangumiComment>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiComment {
    #[doc = "评论编号"]
    pub id: Option<i32>,
    #[doc = "弹弹play 用户ID。为 0 表示非本平台用户"]
    #[serde(rename = "userId")]
    pub user_id: Option<i32>,
    #[doc = "外部平台用户ID/主页标识"]
    #[serde(rename = "externalUserId")]
    pub external_user_id: Option<String>,
    #[doc = "用户名"]
    #[serde(rename = "userName")]
    pub user_name: Option<String>,
    #[doc = "用户头像地址"]
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
    #[doc = "评论来源，例如 Bangumi"]
    pub source: Option<String>,
    #[doc = "评论内容"]
    pub text: Option<String>,
    #[doc = "用户评分（0-10）"]
    pub rating: Option<i32>,
    #[doc = "记录更新时间"]
    #[serde(rename = "updatedTime")]
    pub updated_time: Option<String>,
}
#[doc = "弹幕列表"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentResponseV2 {
    #[doc = "弹幕数量"]
    pub count: Option<i32>,
    #[doc = "弹幕列表"]
    pub comments: Option<Vec<CommentData>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentData {
    #[doc = "弹幕ID"]
    pub cid: Option<i64>,
    #[doc = "弹幕参数（出现时间,模式,颜色,用户ID）"]
    pub p: Option<String>,
    #[doc = "弹幕内容"]
    pub m: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendCommentResponseV2 {
    #[serde(flatten)]
    pub response_base: ResponseBase,
    #[doc = "此弹幕库中的弹幕ID"]
    pub cid: Option<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendCommentRequest {
    #[doc = "弹幕出现时间，单位为秒"]
    pub time: Option<f64>,
    #[doc = "弹幕模式：1-普通弹幕，4-顶部弹幕，5-底部弹幕"]
    pub mode: Option<i32>,
    #[doc = "弹幕颜色，计算方式为 Rx255x255+Gx255+B"]
    pub color: Option<i32>,
    #[doc = "弹幕内容，不能长于100个字符"]
    pub comment: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendAppCommentRequest {
    #[serde(flatten)]
    pub send_comment_request: SendCommentRequest,
    #[doc = "弹幕发送者昵称，由调用方应用自行指定。"]
    #[serde(rename = "userName")]
    pub user_name: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFavoriteResponse {
    #[serde(flatten)]
    pub response_base: ResponseBase,
    #[doc = "关注列表"]
    pub favorites: Option<Vec<UserFavoriteItem>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFavoriteItem {
    #[doc = "作品编号"]
    #[serde(rename = "animeId")]
    pub anime_id: Option<i64>,
    #[doc = "作品编号"]
    #[serde(rename = "bangumiId")]
    pub bangumi_id: Option<String>,
    #[doc = "作品标题"]
    #[serde(rename = "animeTitle")]
    pub anime_title: Option<String>,
    #[doc = "作品类型"]
    pub r#type: Option<AnimeType>,
    #[doc = "上次关注的时间"]
    #[serde(rename = "lastFavoriteTime")]
    pub last_favorite_time: Option<String>,
    #[doc = "上次剧集更新的时间"]
    #[serde(rename = "lastAirDate")]
    pub last_air_date: Option<String>,
    #[doc = "上次播放作品相关剧集的时间"]
    #[serde(rename = "lastWatchTime")]
    pub last_watch_time: Option<String>,
    #[doc = "海报图片地址"]
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
    #[doc = "此作品的总集数"]
    #[serde(rename = "episodeTotal")]
    pub episode_total: Option<i32>,
    #[doc = "当前已看的集数"]
    #[serde(rename = "episodeWatched")]
    pub episode_watched: Option<i32>,
    #[doc = "番剧首话上映日期"]
    #[serde(rename = "startDate")]
    pub start_date: Option<String>,
    #[doc = "此作品是否正在连载中"]
    #[serde(rename = "isOnAir")]
    pub is_on_air: Option<bool>,
    #[doc = "关注状态"]
    #[serde(rename = "favoriteStatus")]
    pub favorite_status: Option<FavoriteStatus>,
    #[doc = "用户给此作品的评分（1-10分，0代表未评分）"]
    #[serde(rename = "userRating")]
    pub user_rating: Option<i32>,
    #[doc = "此番剧的综合评分（0-10分）"]
    pub rating: Option<f32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAddFavoriteResponse {
    #[serde(flatten)]
    pub response_base: ResponseBase,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAddFavoriteRequest {
    #[doc = "动画作品编号"]
    #[serde(rename = "animeId")]
    pub anime_id: Option<i64>,
    #[doc = "设定或刷新当前的关注状态。设置为null代表不修改当前状态。"]
    #[serde(rename = "favoriteStatus")]
    pub favorite_status: Option<FavoriteStatus>,
    #[doc = "给作品打分（1-10分），0代表不修改当前分数"]
    pub rating: Option<i32>,
    #[doc = "给作品添加评论，最长为500个字符。当值为null或空字符串时将不修改当前的值。 如果希望清空所有文字，请传入至少一个空格。"]
    pub comment: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDeleteFavoriteResponse {
    #[serde(flatten)]
    pub response_base: ResponseBase,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomepageResponseV2 {
    #[serde(flatten)]
    pub response_base: ResponseBase,
    #[doc = "公告列表"]
    pub banners: Option<Vec<BannerPageItem>>,
    #[doc = "未看剧集列表"]
    #[serde(rename = "bangumiQueueIntroList")]
    pub bangumi_queue_intro_list: Option<Vec<BangumiQueueIntroV2>>,
    #[doc = "新番列表"]
    #[serde(rename = "shinBangumiList")]
    pub shin_bangumi_list: Option<Vec<BangumiIntro>>,
    #[doc = "动画番剧季度列表"]
    #[serde(rename = "bangumiSeasons")]
    pub bangumi_seasons: Option<Vec<BangumiSeason>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BannerPageItem {
    #[doc = "公告ID"]
    pub id: Option<i32>,
    #[doc = "标题"]
    pub title: Option<String>,
    #[doc = "子标题、描述"]
    pub description: Option<String>,
    #[doc = "落地页链接"]
    pub url: Option<String>,
    #[doc = "图片地址"]
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BannerResponse {
    #[serde(flatten)]
    pub response_base: ResponseBase,
    #[doc = "公告列表"]
    pub banners: Option<Vec<BannerPageItem>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    #[serde(flatten)]
    pub response_base: ResponseBase,
    #[doc = "该用户是否需要先注册弹弹play账号才可正常登录。当此值为true时表示用户使用了QQ微博等第三方登录但没有注册弹弹play账号。"]
    #[serde(rename = "registerRequired")]
    pub register_required: Option<bool>,
    #[doc = "用户编号"]
    #[serde(rename = "userId")]
    pub user_id: Option<i32>,
    #[doc = "弹弹play用户名。如果用户使用第三方账号登录（如QQ微博）且没有关联弹弹play账号，此属性将为null"]
    #[serde(rename = "userName")]
    pub user_name: Option<String>,
    #[doc = "用户邮箱地址"]
    pub email: Option<String>,
    #[doc = "旧API中使用的数字形式的token，仅为兼容性设置，不要在新代码中使用此属性"]
    #[serde(rename = "legacyTokenNumber")]
    pub legacy_token_number: Option<i32>,
    #[doc = "字符串形式的JWT token。将来调用需要验证权限的接口时，需要在HTTP Authorization头中设置“Bearer token”。"]
    pub token: Option<String>,
    #[doc = "JWT token过期时间，默认为21天。如果是APP应用开发者账号使用自己的应用登录则为1年。"]
    #[serde(rename = "tokenExpireTime")]
    pub token_expire_time: Option<String>,
    #[doc = "用户注册来源类型"]
    #[serde(rename = "userType")]
    pub user_type: Option<String>,
    #[doc = "昵称"]
    #[serde(rename = "screenName")]
    pub screen_name: Option<String>,
    #[doc = "头像图片的地址"]
    #[serde(rename = "profileImage")]
    pub profile_image: Option<String>,
    #[doc = "当前登录会话内应用权限列表，可以由此判断能否调用哪些API"]
    #[serde(rename = "appScope")]
    pub app_scope: Option<String>,
    #[doc = "商品列表"]
    #[serde(rename = "payConfigs")]
    pub pay_configs: Option<Vec<PayConfig>>,
    #[doc = "用户权益过期时间（全部为北京时间）"]
    pub privileges: Option<UserPrivileges>,
    #[doc = "消息体验证码"]
    pub code: Option<String>,
    #[doc = "当前时间戳"]
    pub ts: Option<i64>,
    #[doc = "已关联的第三方账号信息（如 bangumi.tv 账号）"]
    #[serde(rename = "linkedAccounts")]
    pub linked_accounts: Option<LinkedAccounts>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayConfig {
    #[doc = "支付渠道（wechat,alipay）"]
    #[serde(rename = "providerId")]
    pub provider_id: Option<String>,
    #[doc = "支付渠道名称（微信支付，支付宝）"]
    #[serde(rename = "providerName")]
    pub provider_name: Option<String>,
    #[doc = "商品列表"]
    pub items: Option<Vec<PayConfigItem>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayConfigItem {
    #[doc = "商品ID"]
    pub id: Option<String>,
    #[doc = "商品名称（如：1个月会员）"]
    pub name: Option<String>,
    #[doc = "商品价格（单位：分）"]
    pub price: Option<i32>,
    #[doc = "货币单位（CNY）"]
    pub currency: Option<String>,
}
#[doc = "用户各类权益到期时间"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPrivileges {
    #[doc = "会员权益过期时间（北京时间）"]
    pub member: Option<String>,
    #[doc = "弹弹play资源监视器权益过期时间（北京时间）"]
    pub resmonitor: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedAccounts {
    #[doc = "bangumi.tv 用户"]
    pub bangumi: Option<LinkedAccountInfo>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedAccountInfo {
    #[doc = "bangumi.tv 用户ID"]
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
    #[doc = "bangumi.tv 用户名"]
    #[serde(rename = "userName")]
    pub user_name: Option<String>,
    #[doc = "显示名称（昵称）"]
    pub display: Option<String>,
    #[doc = "用户头像URL"]
    pub avatar: Option<String>,
    #[doc = "当前授权过期时间（北京时间）"]
    pub expires: Option<String>,
}
#[doc = "请求用户登录"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    #[doc = "弹弹play用户名"]
    #[serde(rename = "userName")]
    pub user_name: String,
    #[doc = "用户密码"]
    pub password: String,
    #[doc = "客户端ID"]
    #[serde(rename = "appId")]
    pub app_id: String,
    #[doc = "Unix时间戳：从协调世界时1970年1月1日0时0分0秒起至现在的总秒数，不考虑闰秒。"]
    #[serde(rename = "unixTimestamp")]
    pub unix_timestamp: Option<i64>,
    #[doc = "通过参数计算得到的32位MD5值，不区分大小写。计算方法请参考接口说明。"]
    pub hash: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResponseV2 {
    #[serde(flatten)]
    pub response_base: ResponseBase,
    #[doc = "是否已精确关联到某个弹幕库"]
    #[serde(rename = "isMatched")]
    pub is_matched: Option<bool>,
    #[doc = "搜索匹配的结果"]
    pub matches: Option<Vec<MatchResultV2>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResultV2 {
    #[doc = "弹幕库ID"]
    #[serde(rename = "episodeId")]
    pub episode_id: Option<i64>,
    #[doc = "作品ID"]
    #[serde(rename = "animeId")]
    pub anime_id: Option<i64>,
    #[doc = "作品标题"]
    #[serde(rename = "animeTitle")]
    pub anime_title: Option<String>,
    #[doc = "剧集标题"]
    #[serde(rename = "episodeTitle")]
    pub episode_title: Option<String>,
    #[doc = "作品类别"]
    pub r#type: Option<AnimeType>,
    #[doc = "类型描述"]
    #[serde(rename = "typeDescription")]
    pub type_description: Option<String>,
    #[doc = "弹幕偏移时间（弹幕应延迟多少秒出现）。此数字为负数时表示弹幕应提前多少秒出现。"]
    pub shift: Option<f64>,
    #[doc = "此作品的海报图片地址"]
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchRequest {
    #[doc = "视频文件名，不包含文件夹名称和扩展名，特殊字符需进行转义。"]
    #[serde(rename = "fileName")]
    pub file_name: Option<String>,
    #[doc = "文件前16MB (16x1024x1024 Byte) 数据的32位MD5结果，不区分大小写。"]
    #[serde(rename = "fileHash")]
    pub file_hash: Option<String>,
    #[doc = "文件总长度，单位为Byte。"]
    #[serde(rename = "fileSize")]
    pub file_size: Option<i64>,
    #[doc = "[可选]32位整数的视频时长，单位为秒。默认为0。"]
    #[serde(rename = "videoDuration")]
    pub video_duration: Option<i32>,
    #[doc = "[可选]匹配模式。"]
    #[serde(rename = "matchMode")]
    pub match_mode: Option<MatchMode>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchMode {
    #[serde(rename = "hashAndFileName")]
    HashAndFileName,
    #[serde(rename = "fileNameOnly")]
    FileNameOnly,
    #[serde(rename = "hashOnly")]
    HashOnly,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMatchResponse {
    #[serde(flatten)]
    pub response_base: ResponseBase,
    #[doc = "批量匹配的结果。将针对每个请求生成对应的结果。"]
    pub results: Option<Vec<BatchMatchResponseItem>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMatchResponseItem {
    pub success: Option<bool>,
    #[serde(rename = "fileHash")]
    pub file_hash: Option<String>,
    #[serde(rename = "matchResult")]
    pub match_result: Option<MatchResultV2>,
}
#[doc = "批量匹配的请求"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMatchRequest {
    #[doc = "匹配请求，列表中最多包括32个请求"]
    pub requests: Option<Vec<MatchRequest>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPlayHistoryResponse {
    #[serde(flatten)]
    pub response_base: ResponseBase,
    #[serde(rename = "playHistoryAnimes")]
    pub play_history_animes: Option<Vec<UserPlayHistoryAnime>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPlayHistoryAnime {
    #[serde(rename = "animeId")]
    pub anime_id: Option<i64>,
    #[serde(rename = "animeTitle")]
    pub anime_title: Option<String>,
    #[doc = "作品类别"]
    pub r#type: Option<AnimeType>,
    #[doc = "类型描述"]
    #[serde(rename = "typeDescription")]
    pub type_description: Option<String>,
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
    #[serde(rename = "isOnAir")]
    pub is_on_air: Option<bool>,
    pub episodes: Option<Vec<BangumiEpisode>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAddPlayHistoryResponse {
    #[serde(flatten)]
    pub response_base: ResponseBase,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAddPlayHistoryRequest {
    #[doc = "弹幕库编号列表（最多100项，必须都属于同一作品）"]
    #[serde(rename = "episodeIdList")]
    pub episode_id_list: Option<Vec<i64>>,
    #[doc = "关注此作品（弹幕库编号列表中必须只有一项）"]
    #[serde(rename = "addToFavorite")]
    pub add_to_favorite: Option<bool>,
    #[doc = "给此剧集打分（弹幕库编号列表中必须只有一项）。范围为1-10分，0代表不修改当前评分。"]
    pub rating: Option<i32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequestV2 {
    #[doc = "客户端ID"]
    #[serde(rename = "appId")]
    pub app_id: String,
    #[doc = "用户名。只能包含英文或数字，长度为5-20位，首位不能为数字。"]
    #[serde(rename = "userName")]
    pub user_name: String,
    #[doc = "密码。长度为5到20位之间。"]
    pub password: String,
    #[doc = "备用邮箱（找回密码用）。长度不能超过50个字符。"]
    pub email: String,
    #[doc = "昵称。长度不能超过50个字符。"]
    #[serde(rename = "screenName")]
    pub screen_name: String,
    #[doc = "Unix时间戳：从协调世界时1970年1月1日0时0分0秒起至现在的总秒数，不考虑闰秒。"]
    #[serde(rename = "unixTimestamp")]
    pub unix_timestamp: Option<i64>,
    #[doc = "通过参数计算得到的32位MD5值，不区分大小写。计算方法请参考接口说明。"]
    pub hash: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetPasswordResponseV2 {
    #[serde(flatten)]
    pub response_base: ResponseBase,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetPasswordRequestV2 {
    #[doc = "应用ID"]
    #[serde(rename = "appId")]
    pub app_id: String,
    #[doc = "用户名"]
    #[serde(rename = "userName")]
    pub user_name: String,
    #[doc = "注册此用户时填写的备用邮箱"]
    pub email: String,
    #[doc = "Unix时间戳：从协调世界时1970年1月1日0时0分0秒起至现在的总秒数，不考虑闰秒。"]
    #[serde(rename = "unixTimestamp")]
    pub unix_timestamp: Option<i64>,
    #[doc = "通过参数计算得到的32位MD5值，不区分大小写。计算方法请参考接口说明。"]
    pub hash: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindMyIdResponse {
    #[serde(flatten)]
    pub response_base: ResponseBase,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindMyIdRequestV2 {
    #[doc = "应用ID"]
    #[serde(rename = "appId")]
    pub app_id: String,
    #[doc = "注册此用户时填写的备用邮箱"]
    pub email: String,
    #[doc = "Unix时间戳：从协调世界时1970年1月1日0时0分0秒起至现在的总秒数，不考虑闰秒。"]
    #[serde(rename = "unixTimestamp")]
    pub unix_timestamp: Option<i64>,
    #[doc = "通过参数计算得到的32位MD5值，不区分大小写。计算方法请参考接口说明。"]
    pub hash: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchAnimeResponse {
    #[serde(flatten)]
    pub response_base: ResponseBase,
    #[doc = "作品列表"]
    pub animes: Option<Vec<SearchAnimeDetails>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchAnimeDetails {
    #[doc = "作品ID"]
    #[serde(rename = "animeId")]
    pub anime_id: Option<i64>,
    #[doc = "作品ID（新）"]
    #[serde(rename = "bangumiId")]
    pub bangumi_id: Option<String>,
    #[doc = "作品标题"]
    #[serde(rename = "animeTitle")]
    pub anime_title: Option<String>,
    #[doc = "作品类型"]
    pub r#type: Option<AnimeType>,
    #[doc = "类型描述"]
    #[serde(rename = "typeDescription")]
    pub type_description: Option<String>,
    #[doc = "海报图片地址"]
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
    #[doc = "上映日期"]
    #[serde(rename = "startDate")]
    pub start_date: Option<String>,
    #[doc = "剧集总数"]
    #[serde(rename = "episodeCount")]
    pub episode_count: Option<i32>,
    #[doc = "此作品的综合评分（0-10）"]
    pub rating: Option<f32>,
    #[doc = "当前用户是否已关注此作品"]
    #[serde(rename = "isFavorited")]
    pub is_favorited: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchEpisodesResponse {
    #[serde(flatten)]
    pub response_base: ResponseBase,
    #[doc = "是否有更多未显示的搜索结果。当返回的搜索结果过多时此值为`true`"]
    #[serde(rename = "hasMore")]
    pub has_more: Option<bool>,
    #[doc = "搜索结果（作品信息）列表"]
    pub animes: Option<Vec<SearchEpisodesAnime>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchEpisodesAnime {
    #[doc = "作品编号"]
    #[serde(rename = "animeId")]
    pub anime_id: Option<i64>,
    #[doc = "作品标题"]
    #[serde(rename = "animeTitle")]
    pub anime_title: Option<String>,
    #[doc = "作品类型"]
    pub r#type: Option<AnimeType>,
    #[doc = "类型描述"]
    #[serde(rename = "typeDescription")]
    pub type_description: Option<String>,
    #[doc = "此作品的剧集列表"]
    pub episodes: Option<Vec<SearchEpisodeDetails>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchEpisodeDetails {
    #[doc = "剧集ID（弹幕库编号）"]
    #[serde(rename = "episodeId")]
    pub episode_id: Option<i64>,
    #[doc = "剧集标题"]
    #[serde(rename = "episodeTitle")]
    pub episode_title: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchBangumiResponse {
    #[serde(flatten)]
    pub response_base: ResponseBase,
    #[doc = "搜索结果"]
    pub bangumis: Option<Vec<SearchBangumiDetails>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchBangumiDetails {
    #[serde(flatten)]
    pub search_anime_details: SearchAnimeDetails,
    #[doc = "搜索结果中的排名，用于界面中排序展示，从1开始递增"]
    pub rank: Option<i32>,
    #[doc = "搜索关键词"]
    #[serde(rename = "searchKeyword")]
    pub search_keyword: Option<String>,
    #[doc = "是否正在连载中"]
    #[serde(rename = "isOnAir")]
    pub is_on_air: Option<bool>,
    #[doc = "是否为限制级别的内容（例如属于R18分级）"]
    #[serde(rename = "isRestricted")]
    pub is_restricted: Option<bool>,
    #[doc = "短简介（剧情简介或Staff简介）"]
    pub intro: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchAdvancedConfigResponse {
    #[serde(flatten)]
    pub response_base: ResponseBase,
    #[doc = "类型列表"]
    pub types: Option<Vec<ConfigKey>>,
    #[doc = "可用标签列表"]
    pub tags: Option<Vec<ConfigKey>>,
    #[doc = "排序依据"]
    pub sorts: Option<Vec<ConfigKey>>,
    #[doc = "搜索允许的最早年份"]
    #[serde(rename = "minYear")]
    pub min_year: Option<i32>,
    #[doc = "搜索允许的最晚年份"]
    #[serde(rename = "maxYear")]
    pub max_year: Option<i32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigKey {
    #[doc = "搜索中使用的值"]
    pub key: Option<i32>,
    #[doc = "用户界面上显示的文字"]
    pub value: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendingBangumiResponse {
    #[serde(flatten)]
    pub response_base: ResponseBase,
    #[doc = "榜单元数据"]
    pub summary: Option<TrendingSummary>,
    #[doc = "榜单条目"]
    #[serde(rename = "bangumiList")]
    pub bangumi_list: Option<Vec<TrendingBangumiItem>>,
}
#[doc = "排行榜元数据"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendingSummary {
    #[doc = "榜单标题"]
    pub title: Option<String>,
    #[doc = "榜单类型。hot=热播榜，rising=飙升榜，new-anime-hot=新番热播榜"]
    #[serde(rename = "rankingType")]
    pub ranking_type: Option<String>,
    #[doc = "统计周期。week=周，month=月，quarter=季度，season=季度新番，year=年度新番"]
    pub period: Option<String>,
    #[doc = "榜单范围。all=全站，current-season=本季新番，previous-season=上一季度新番，current-year=今年新番"]
    pub scope: Option<String>,
    #[doc = "当前统计开始日期（服务器时区）"]
    #[serde(rename = "dateFrom")]
    pub date_from: Option<String>,
    #[doc = "当前统计结束日期（服务器时区）"]
    #[serde(rename = "dateTo")]
    pub date_to: Option<String>,
    #[doc = "对比统计开始日期（仅飙升榜有效）"]
    #[serde(rename = "compareDateFrom")]
    pub compare_date_from: Option<String>,
    #[doc = "对比统计结束日期（仅飙升榜有效）"]
    #[serde(rename = "compareDateTo")]
    pub compare_date_to: Option<String>,
    #[doc = "当前可用的最新完整数据日期"]
    #[serde(rename = "latestDataDate")]
    pub latest_data_date: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendingBangumiItem {
    #[serde(flatten)]
    pub bangumi_intro: BangumiIntro,
    #[doc = "当前排名"]
    pub rank: Option<i32>,
    #[doc = "当前统计周期内的脱敏热度值"]
    pub heat: Option<String>,
    #[doc = "当前周期内有热度的天数"]
    #[serde(rename = "activeDays")]
    pub active_days: Option<i32>,
    #[doc = "对比周期脱敏热度值（仅飙升榜有效）"]
    #[serde(rename = "previousHeat")]
    pub previous_heat: Option<String>,
    #[doc = "当前周期与对比周期的脱敏热度差值（仅飙升榜有效）"]
    #[serde(rename = "heatDelta")]
    pub heat_delta: Option<String>,
    #[doc = "热度增长率文本（仅飙升榜有效）"]
    #[serde(rename = "heatGrowthRate")]
    pub heat_growth_rate: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserUpdateProfileResponseV2 {
    #[serde(flatten)]
    pub response_base: ResponseBase,
    #[serde(rename = "updateScreenName")]
    pub update_screen_name: Option<String>,
    #[serde(rename = "updateProfileImage")]
    pub update_profile_image: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserUpdatePasswordRequest {
    #[doc = "旧密码（5-20位）"]
    #[serde(rename = "oldPassword")]
    pub old_password: String,
    #[doc = "新密码（5-20位）"]
    #[serde(rename = "newPassword")]
    pub new_password: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserUpdateProfileRequest {
    #[doc = "用户新的昵称（留空将不修改昵称）"]
    #[serde(rename = "screenName")]
    pub screen_name: Option<String>,
    #[doc = "用户头像图片使用Base64编码后的数据（jpg格式，长度不能超过1MB）。留空将不修改头像图片"]
    #[serde(rename = "profileImageBase64")]
    pub profile_image_base64: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserUpdateEmailRequest {
    #[doc = "当前的关联邮箱地址"]
    #[serde(rename = "oldEmail")]
    pub old_email: String,
    #[doc = "新的关联邮箱地址"]
    #[serde(rename = "newEmail")]
    pub new_email: String,
}
