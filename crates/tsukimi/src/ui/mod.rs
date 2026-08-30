mod models;
mod mpv;
pub mod provider;
pub mod widgets;

use gtk::glib::prelude::StaticTypeExt;

pub use models::{
    SETTINGS,
    jellyfin_cache_path,
};
pub(crate) use mpv::options_matcher::{
    match_audio_channels,
    match_hwdec_interop,
    match_sub_border_style,
    match_video_upscale,
};
pub use widgets::{
    GlobalToast,
    window::Window,
};

pub fn init() {
    widgets::fixed_bin::FixedBin::ensure_type();
    widgets::lazy_diff_view::LazyDiffView::ensure_type();
    widgets::menu_info::MenuInfo::ensure_type();
    widgets::hover_scale::HoverScale::ensure_type();
    widgets::action_row::AActionRow::ensure_type();
    widgets::filter_panel::FiltersRow::ensure_type();
    widgets::image_dialog::ImageInfoCard::ensure_type();
    widgets::image_dialog::ImageDropRow::ensure_type();
    widgets::item_carousel::ItemCarousel::ensure_type();
    widgets::star_toggle::StarToggle::ensure_type();
    widgets::horbu_scrolled::HorbuScrolled::ensure_type();
    widgets::episode_switcher::EpisodeSwitcher::ensure_type();
    widgets::smooth_scale::SmoothScale::ensure_type();
    widgets::tuview_scrolled::TuViewScrolled::ensure_type();
    widgets::picture_loader::PictureLoader::ensure_type();
    widgets::hortu_scrolled::HortuScrolled::ensure_type();
    widgets::item_actionbox::ItemActionsBox::ensure_type();

    mpv::danmaku_scale_row::DanmakuScaleRow::ensure_type();
    mpv::sink::MPVPlaySink::ensure_type();
    mpv::video_scale::VideoScale::ensure_type();
    mpv::volume_bar::VolumeBar::ensure_type();
    mpv::danmaku_popover::DanmakuPopover::ensure_type();
    mutsumi::Danmakw::ensure_type();

    widgets::player_toolbar::PlayerToolbarBox::ensure_type();
    widgets::content_viewer::MediaContentViewer::ensure_type();
    widgets::media_viewer::MediaViewer::ensure_type();
    widgets::image_dialog::ImageDialog::ensure_type();
    widgets::home::HomePage::ensure_type();
    widgets::search::SearchPage::ensure_type();
    widgets::liked::LikedPage::ensure_type();
    mpv::page::MPVPage::ensure_type();
    mpv::control_sidebar::MPVControlSidebar::ensure_type();
    widgets::theme_switcher::ThemeSwitcher::ensure_type();
}
