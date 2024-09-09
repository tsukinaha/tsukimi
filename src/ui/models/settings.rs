use std::ops::Deref;

use gtk::{
    gio,
    glib::{self, thread_guard::ThreadGuard},
    prelude::*,
};

use crate::APP_ID;

pub struct Settings(ThreadGuard<gio::Settings>);

impl Settings {
    const KEY_IS_OVERLAY: &'static str = "is-overlay";
    const KEY_IS_RESUME: &'static str = "is-resume";
    const KEY_IS_BLUR_ENABLED: &'static str = "is-blurenabled";
    const KEY_PROXY: &'static str = "proxy";
    const KEY_ROOT_PIC: &'static str = "root-pic";
    const KEY_IS_BACKGROUND_ENABLED: &'static str = "is-backgroundenabled";
    const KEY_THREADS: &'static str = "threads";
    const KEY_PIC_OPACITY: &'static str = "pic-opacity";
    const KEY_PIC_BLUR: &'static str = "pic-blur";
    const KEY_PREFERRED_SERVER: &'static str = "preferred-server";
    const KEY_IS_AUTO_SELECT_SERVER: &'static str = "is-auto-select-server";
    const KEY_FONT_SIZE: &'static str = "font-size";
    const KEY_FONT_NAME: &'static str = "font-name";
    const KEY_DAILY_RECOMMEND: &'static str = "is-daily-recommend";
    const KEY_LIST_SORT: &'static str = "list-sort";
    const KEY_ACCENT_COLOR_CODE: &'static str = "accent-color-code";
    const KEY_ACCENT_FG_COLOR_CODE: &'static str = "accent-fg-color-code";
    const KEY_MUSIC_REPEAT_MODE: &'static str = "music-repeat-mode";
    const KEY_MPV_ESTIMATE: &'static str = "mpv-estimate";
    const KEY_MPV_ESTIMATE_TARGET_FRAME: &'static str = "mpv-estimate-target-frame";
    const KEY_MPV_SEEK_FORWARD_STEP: &'static str = "mpv-seek-forward-step";
    const KEY_MPV_SEEK_BACKWARD_STEP: &'static str = "mpv-seek-backward-step";
    const KEY_MPV_CONFIG: &'static str = "mpv-config";
    const KEY_MPV_CACHE_SIZE: &'static str = "mpv-cache-size";
    const KEY_MPV_CACHE_TIME: &'static str = "mpv-cache-time";
    const KEY_MPV_SUBTITLE_SIZE: &'static str = "mpv-subtitle-size"; // i32
    const KEY_MPV_SUBTITLE_FONT: &'static str = "mpv-subtitle-font"; // String
    const KEY_MPV_AUDIO_PREFERRED_LANG: &'static str = "mpv-audio-preferred-lang"; // i32
    const KEY_MPV_SUBTITLE_PREFERRED_LANG: &'static str = "mpv-subtitle-preferred-lang"; // i32
    const KEY_MPV_DEFAULT_VOLUME: &'static str = "mpv-default-volume"; // i32
    const KEY_MPV_FORCE_STEREO: &'static str = "mpv-force-stereo"; // bool
    const KEY_MPV_SHOW_BUFFER_SPEED: &'static str = "mpv-show-buffer-speed"; // bool
    const KEY_MPV_VIDEO_OUTPUT: &'static str = "mpv-video-output"; // i32
    const KEY_MPV_ACTION_AFTER_VIDEO_END: &'static str = "mpv-action-after-video-end"; // i32

    pub fn set_mpv_subtitle_size(&self, mpv_subtitle_size: i32) -> Result<(), glib::BoolError> {
        self.set_int(Self::KEY_MPV_SUBTITLE_SIZE, mpv_subtitle_size)
    }

    pub fn mpv_subtitle_size(&self) -> i32 {
        self.int(Self::KEY_MPV_SUBTITLE_SIZE)
    }

    pub fn set_mpv_subtitle_font(&self, mpv_subtitle_font: String) -> Result<(), glib::BoolError> {
        self.set_string(Self::KEY_MPV_SUBTITLE_FONT, &mpv_subtitle_font)
    }

    pub fn mpv_subtitle_font(&self) -> String {
        self.string(Self::KEY_MPV_SUBTITLE_FONT).to_string()
    }

    pub fn set_mpv_audio_preferred_lang(&self, mpv_audio_preferred_lang: i32) -> Result<(), glib::BoolError> {
        self.set_int(Self::KEY_MPV_AUDIO_PREFERRED_LANG, mpv_audio_preferred_lang)
    }

    pub fn mpv_audio_preferred_lang(&self) -> i32 {
        self.int(Self::KEY_MPV_AUDIO_PREFERRED_LANG)
    }

    pub fn set_mpv_subtitle_preferred_lang(&self, mpv_subtitle_preferred_lang: i32) -> Result<(), glib::BoolError> {
        self.set_int(Self::KEY_MPV_SUBTITLE_PREFERRED_LANG, mpv_subtitle_preferred_lang)
    }

    pub fn mpv_subtitle_preferred_lang(&self) -> i32 {
        self.int(Self::KEY_MPV_SUBTITLE_PREFERRED_LANG)
    }

    pub fn set_mpv_default_volume(&self, mpv_default_volume: i32) -> Result<(), glib::BoolError> {
        self.set_int(Self::KEY_MPV_DEFAULT_VOLUME, mpv_default_volume)
    }

    pub fn mpv_default_volume(&self) -> i32 {
        self.int(Self::KEY_MPV_DEFAULT_VOLUME)
    }

    pub fn set_mpv_force_stereo(&self, mpv_force_stereo: bool) -> Result<(), glib::BoolError> {
        self.set_boolean(Self::KEY_MPV_FORCE_STEREO, mpv_force_stereo)
    }

    pub fn mpv_force_stereo(&self) -> bool {
        self.boolean(Self::KEY_MPV_FORCE_STEREO)
    }

    pub fn set_mpv_show_buffer_speed(&self, mpv_show_buffer_speed: bool) -> Result<(), glib::BoolError> {
        self.set_boolean(Self::KEY_MPV_SHOW_BUFFER_SPEED, mpv_show_buffer_speed)
    }

    pub fn mpv_show_buffer_speed(&self) -> bool {
        self.boolean(Self::KEY_MPV_SHOW_BUFFER_SPEED)
    }

    pub fn set_mpv_video_output(&self, mpv_video_output: i32) -> Result<(), glib::BoolError> {
        self.set_int(Self::KEY_MPV_VIDEO_OUTPUT, mpv_video_output)
    }

    pub fn mpv_video_output(&self) -> i32 {
        self.int(Self::KEY_MPV_VIDEO_OUTPUT)
    }

    pub fn set_mpv_action_after_video_end(&self, mpv_action_after_video_end: i32) -> Result<(), glib::BoolError> {
        self.set_int(Self::KEY_MPV_ACTION_AFTER_VIDEO_END, mpv_action_after_video_end)
    }

    pub fn mpv_action_after_video_end(&self) -> i32 {
        self.int(Self::KEY_MPV_ACTION_AFTER_VIDEO_END)
    }

    pub fn set_mpv_cache_time(&self, mpv_cache_time: i32) -> Result<(), glib::BoolError> {
        self.set_int(Self::KEY_MPV_CACHE_TIME, mpv_cache_time)
    }

    pub fn mpv_cache_time(&self) -> i32 {
        self.int(Self::KEY_MPV_CACHE_TIME)
    }

    pub fn set_mpv_cache_size(&self, mpv_cache_size: i32) -> Result<(), glib::BoolError> {
        self.set_int(Self::KEY_MPV_CACHE_SIZE, mpv_cache_size)
    }

    pub fn mpv_cache_size(&self) -> i32 {
        self.int(Self::KEY_MPV_CACHE_SIZE)
    }

    pub fn set_mpv_config(&self, mpv_config: bool) -> Result<(), glib::BoolError> {
        self.set_boolean(Self::KEY_MPV_CONFIG, mpv_config)
    }

    pub fn mpv_config(&self) -> bool {
        self.boolean(Self::KEY_MPV_CONFIG)
    }

    pub fn set_mpv_seek_forward_step(
        &self,
        mpv_seek_forward_step: i32,
    ) -> Result<(), glib::BoolError> {
        self.set_int(Self::KEY_MPV_SEEK_FORWARD_STEP, mpv_seek_forward_step)
    }

    pub fn mpv_seek_forward_step(&self) -> i32 {
        self.int(Self::KEY_MPV_SEEK_FORWARD_STEP)
    }

    pub fn set_mpv_seek_backward_step(
        &self,
        mpv_seek_backward_step: i32,
    ) -> Result<(), glib::BoolError> {
        self.set_int(Self::KEY_MPV_SEEK_BACKWARD_STEP, mpv_seek_backward_step)
    }

    pub fn mpv_seek_backward_step(&self) -> i32 {
        self.int(Self::KEY_MPV_SEEK_BACKWARD_STEP)
    }


    pub fn set_mpv_estimate(&self, mpv_estimate: bool) -> Result<(), glib::BoolError> {
        self.set_boolean(Self::KEY_MPV_ESTIMATE, mpv_estimate)
    }

    pub fn mpv_estimate(&self) -> bool {
        self.boolean(Self::KEY_MPV_ESTIMATE)
    }

    pub fn set_mpv_estimate_target_frame(
        &self,
        mpv_estimate_target_frame: i32,
    ) -> Result<(), glib::BoolError> {
        self.set_int(
            Self::KEY_MPV_ESTIMATE_TARGET_FRAME,
            mpv_estimate_target_frame,
        )
    }

    pub fn mpv_estimate_target_frame(&self) -> i32 {
        self.int(Self::KEY_MPV_ESTIMATE_TARGET_FRAME)
    }

    pub fn set_music_repeat_mode(&self, music_repeat_mode: &str) -> Result<(), glib::BoolError> {
        self.set_string(Self::KEY_MUSIC_REPEAT_MODE, music_repeat_mode)
    }

    pub fn music_repeat_mode(&self) -> String {
        self.string(Self::KEY_MUSIC_REPEAT_MODE).to_string()
    }

    pub fn set_accent_fg_color_code(
        &self,
        accent_fg_color_code: &str,
    ) -> Result<(), glib::BoolError> {
        self.set_string(Self::KEY_ACCENT_FG_COLOR_CODE, accent_fg_color_code)
    }

    pub fn accent_fg_color_code(&self) -> String {
        self.string(Self::KEY_ACCENT_FG_COLOR_CODE).to_string()
    }

    pub fn set_accent_color_code(&self, accent_color_code: &str) -> Result<(), glib::BoolError> {
        self.set_string(Self::KEY_ACCENT_COLOR_CODE, accent_color_code)
    }

    pub fn accent_color_code(&self) -> String {
        self.string(Self::KEY_ACCENT_COLOR_CODE).to_string()
    }

    pub fn set_list_sort(&self, list_sort: &u32) -> Result<(), glib::BoolError> {
        self.set_int(Self::KEY_LIST_SORT, *list_sort as i32)
    }

    pub fn list_sort(&self) -> i32 {
        self.int(Self::KEY_LIST_SORT)
    }

    pub fn set_daily_recommend(&self, daily_recommend: bool) -> Result<(), glib::BoolError> {
        self.set_boolean(Self::KEY_DAILY_RECOMMEND, daily_recommend)
    }

    pub fn daily_recommend(&self) -> bool {
        self.boolean(Self::KEY_DAILY_RECOMMEND)
    }

    pub fn set_font_name(&self, font_name: &str) -> Result<(), glib::BoolError> {
        self.set_string(Self::KEY_FONT_NAME, font_name)
    }

    pub fn font_name(&self) -> String {
        self.string(Self::KEY_FONT_NAME).to_string()
    }

    pub fn set_font_size(&self, font_size: i32) -> Result<(), glib::BoolError> {
        self.set_int(Self::KEY_FONT_SIZE, font_size)
    }

    pub fn font_size(&self) -> i32 {
        self.int(Self::KEY_FONT_SIZE)
    }

    pub fn set_preferred_server(&self, preferred_server: &str) -> Result<(), glib::BoolError> {
        self.set_string(Self::KEY_PREFERRED_SERVER, preferred_server)
    }

    pub fn preferred_server(&self) -> String {
        self.string(Self::KEY_PREFERRED_SERVER).to_string()
    }

    pub fn set_auto_select_server(&self, auto_select_server: bool) -> Result<(), glib::BoolError> {
        self.set_boolean(Self::KEY_IS_AUTO_SELECT_SERVER, auto_select_server)
    }

    pub fn auto_select_server(&self) -> bool {
        self.boolean(Self::KEY_IS_AUTO_SELECT_SERVER)
    }

    pub fn set_overlay(&self, overlay: bool) -> Result<(), glib::BoolError> {
        self.set_boolean(Self::KEY_IS_OVERLAY, overlay)
    }
    pub fn overlay(&self) -> bool {
        self.boolean(Self::KEY_IS_OVERLAY)
    }

    pub fn resume(&self) -> bool {
        self.boolean(Self::KEY_IS_RESUME)
    }

    pub fn set_proxy(&self, proxy: &str) -> Result<(), glib::BoolError> {
        self.set_string(Self::KEY_PROXY, proxy)
    }

    pub fn proxy(&self) -> String {
        self.string(Self::KEY_PROXY).to_string()
    }

    pub fn set_root_pic(&self, root_pic: &str) -> Result<(), glib::BoolError> {
        self.set_string(Self::KEY_ROOT_PIC, root_pic)
    }

    pub fn root_pic(&self) -> String {
        self.string(Self::KEY_ROOT_PIC).to_string()
    }

    pub fn set_threads(&self, threads: i32) -> Result<(), glib::BoolError> {
        self.set_int(Self::KEY_THREADS, threads)
    }

    pub fn threads(&self) -> i32 {
        self.int(Self::KEY_THREADS)
    }

    pub fn set_pic_opacity(&self, pic_opacity: i32) -> Result<(), glib::BoolError> {
        self.set_int(Self::KEY_PIC_OPACITY, pic_opacity)
    }

    pub fn pic_opacity(&self) -> i32 {
        self.int(Self::KEY_PIC_OPACITY)
    }

    pub fn set_pic_blur(&self, pic_blur: i32) -> Result<(), glib::BoolError> {
        self.set_int(Self::KEY_PIC_BLUR, pic_blur)
    }

    pub fn pic_blur(&self) -> i32 {
        self.int(Self::KEY_PIC_BLUR)
    }

    pub fn set_background_enabled(&self, background_enabled: bool) -> Result<(), glib::BoolError> {
        self.set_boolean(Self::KEY_IS_BACKGROUND_ENABLED, background_enabled)
    }

    pub fn background_enabled(&self) -> bool {
        self.boolean(Self::KEY_IS_BACKGROUND_ENABLED)
    }

    pub fn set_blur_enabled(&self, is_blur_enabled: bool) -> Result<(), glib::BoolError> {
        self.set_boolean(Self::KEY_IS_BLUR_ENABLED, is_blur_enabled)
    }

    pub fn is_blur_enabled(&self) -> bool {
        self.boolean(Self::KEY_IS_BLUR_ENABLED)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self(ThreadGuard::new(gio::Settings::new(APP_ID)))
    }
}

impl Deref for Settings {
    type Target = gio::Settings;

    fn deref(&self) -> &Self::Target {
        self.0.get_ref()
    }
}

unsafe impl Send for Settings {}
unsafe impl Sync for Settings {}
