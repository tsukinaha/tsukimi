use std::ops::Deref;

use gtk::{
    gio,
    glib::{
        self,
        thread_guard::ThreadGuard,
    },
    prelude::*,
};

use crate::{
    APP_ID,
    client::Account,
    ui::provider::descriptor::{
        Descriptor,
        VecSerialize,
    },
};

pub struct Settings(ThreadGuard<gio::Settings>);

impl Settings {
    const KEY_IS_OVERLAY: &'static str = "is-overlay";
    const KEY_ROOT_PIC: &'static str = "root-pic";
    const KEY_IS_BACKGROUND_ENABLED: &'static str = "is-backgroundenabled";
    const KEY_THREADS: &'static str = "threads";
    const KEY_PIC_OPACITY: &'static str = "pic-opacity";
    const KEY_PIC_BLUR: &'static str = "pic-blur";
    const KEY_PREFERRED_SERVER: &'static str = "preferred-server";
    const KEY_IS_AUTO_SELECT_SERVER: &'static str = "is-auto-select-server";
    const KEY_FONT_SIZE: &'static str = "font-size";
    const KEY_FONT_NAME: &'static str = "font-name";
    const KEY_LIST_SORT_BY: &'static str = "list-sort-by";
    const KEY_LIST_SORT_ORDER: &'static str = "list-sort-order";
    const KEY_ACCENT_COLOR_CODE: &'static str = "accent-color-code";
    const KEY_MUSIC_REPEAT_MODE: &'static str = "music-repeat-mode";
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
    const KEY_MPV_SHOW_BUFFER_SPEED: &'static str = "mpv-show-buffer-speed"; // bool
    const KEY_MPV_VIDEO_OUTPUT: &'static str = "mpv-video-output"; // i32
    const KEY_MPV_ACTION_AFTER_VIDEO_END: &'static str = "mpv-action-after-video-end"; // i32
    const KEY_MPV_HWDEC: &'static str = "mpv-hwdec"; // i32
    const PREFERRED_VERSION_DESCRIPTORS: &'static str = "video-version-descriptors"; // String
    const ACCOUNTS: &'static str = "accounts"; // String
    const KEY_MPV_AUDIO_CHANNEL: &'static str = "mpv-audio-channel"; // i32
    const KEY_MPV_SUBTITLE_SCALE: &'static str = "mpv-subtitle-scale"; // f64
    const KEY_MPV_VIDEO_SCALE: &'static str = "mpv-video-scale"; // i32
    const KEY_MPV_CONFIG_DIR: &'static str = "mpv-config-path"; // String
    const KEY_POST_SCALE: &'static str = "post-scale"; // f64
    const KEY_IS_REFRESH: &'static str = "is-refresh"; // bool
    const KEY_DEVICE_UUID: &'static str = "device-uuid"; // String
    const KEY_MAIN_THEME: &'static str = "main-theme"; // i32
    const KEY_WINDOW_WIDTH: &'static str = "window-width"; // i32
    const KEY_WINDOW_HEIGHT: &'static str = "window-height"; // i32
    const KEY_IS_MAXIMIZED: &'static str = "is-maximized"; // bool
    const KEY_IS_FULLSCREEN: &'static str = "is-fullscreen"; // bool
    #[cfg(target_os = "windows")]
    const KEY_IS_FIRST_RUN: &'static str = "is-first-run"; // bool

    #[cfg(target_os = "windows")]
    pub fn is_first_run(&self) -> bool {
        self.boolean(Self::KEY_IS_FIRST_RUN)
    }

    #[cfg(target_os = "windows")]
    pub fn set_is_first_run(&self, is_first_run: bool) -> Result<(), glib::BoolError> {
        self.set_boolean(Self::KEY_IS_FIRST_RUN, is_first_run)
    }

    pub fn is_overlay(&self) -> bool {
        self.boolean(Self::KEY_IS_OVERLAY)
    }

    pub fn is_maximized(&self) -> bool {
        self.boolean(Self::KEY_IS_MAXIMIZED)
    }

    pub fn set_is_maximized(&self, is_maximized: bool) -> Result<(), glib::BoolError> {
        self.set_boolean(Self::KEY_IS_MAXIMIZED, is_maximized)
    }

    pub fn is_fullscreen(&self) -> bool {
        self.boolean(Self::KEY_IS_FULLSCREEN)
    }

    pub fn set_is_fullscreen(&self, is_fullscreen: bool) -> Result<(), glib::BoolError> {
        self.set_boolean(Self::KEY_IS_FULLSCREEN, is_fullscreen)
    }

    pub fn window_dismension(&self) -> (i32, i32) {
        (
            self.int(Self::KEY_WINDOW_WIDTH),
            self.int(Self::KEY_WINDOW_HEIGHT),
        )
    }

    pub fn set_window_dismension(&self, width: i32, height: i32) -> Result<(), glib::BoolError> {
        self.set_int(Self::KEY_WINDOW_WIDTH, width)?;
        self.set_int(Self::KEY_WINDOW_HEIGHT, height)
    }

    pub fn main_theme(&self) -> i32 {
        self.int(Self::KEY_MAIN_THEME)
    }

    pub fn set_main_theme(&self, main_theme: i32) -> Result<(), glib::BoolError> {
        self.set_int(Self::KEY_MAIN_THEME, main_theme)
    }

    pub fn device_uuid(&self) -> String {
        self.string(Self::KEY_DEVICE_UUID).to_string()
    }

    pub fn set_device_uuid(&self, device_uuid: &str) -> Result<(), glib::BoolError> {
        self.set_string(Self::KEY_DEVICE_UUID, device_uuid)
    }

    pub fn is_refresh(&self) -> bool {
        self.boolean(Self::KEY_IS_REFRESH)
    }

    pub fn post_scale(&self) -> f64 {
        self.double(Self::KEY_POST_SCALE)
    }

    pub fn mpv_config_dir(&self) -> String {
        self.string(Self::KEY_MPV_CONFIG_DIR).to_string()
    }

    pub fn mpv_subtitle_scale(&self) -> f64 {
        self.double(Self::KEY_MPV_SUBTITLE_SCALE)
    }

    pub fn mpv_video_scale(&self) -> i32 {
        self.int(Self::KEY_MPV_VIDEO_SCALE)
    }

    pub fn mpv_audio_channel(&self) -> i32 {
        self.int(Self::KEY_MPV_AUDIO_CHANNEL)
    }

    pub fn accounts(&self) -> Vec<Account> {
        serde_json::from_str(self.string(Self::ACCOUNTS).as_ref()).unwrap_or_default()
    }

    pub fn add_account(&self, account: Account) -> Result<(), glib::BoolError> {
        let mut accounts = self.accounts();
        if accounts.iter().any(|a| a.servername == account.servername) {
            accounts.retain(|a| a.servername != account.servername);
        }
        accounts.push(account);
        self.set_string(Self::ACCOUNTS, &accounts.to_string())
    }

    pub fn remove_account(&self, account: Account) -> Result<(), glib::BoolError> {
        let mut accounts = self.accounts();
        accounts.retain(|a| a != &account);
        self.set_string(Self::ACCOUNTS, &accounts.to_string())
    }

    pub fn edit_account(
        &self, old_account: Account, new_account: Account,
    ) -> Result<(), glib::BoolError> {
        let mut accounts = self.accounts();
        if accounts.contains(&new_account) {
            return Ok(());
        }
        if let Some(index) = accounts.iter().position(|a| a == &old_account) {
            accounts[index] = new_account;
        }
        self.set_string(Self::ACCOUNTS, &accounts.to_string())
    }

    pub fn set_accounts(&self, accounts: Vec<Account>) -> Result<(), glib::BoolError> {
        self.set_string(Self::ACCOUNTS, &accounts.to_string())
    }

    pub fn preferred_version_descriptors(&self) -> Vec<Descriptor> {
        serde_json::from_str(self.string(Self::PREFERRED_VERSION_DESCRIPTORS).as_ref())
            .unwrap_or_default()
    }

    pub fn add_preferred_version_descriptor(
        &self, descriptor: Descriptor,
    ) -> Result<(), glib::BoolError> {
        let mut descriptors = self.preferred_version_descriptors();
        if descriptors.contains(&descriptor) {
            return Ok(());
        }
        descriptors.push(descriptor);
        self.set_string(
            Self::PREFERRED_VERSION_DESCRIPTORS,
            &descriptors.to_string(),
        )
    }

    pub fn remove_preferred_version_descriptor(
        &self, descriptor: Descriptor,
    ) -> Result<(), glib::BoolError> {
        let mut descriptors = self.preferred_version_descriptors();
        descriptors.retain(|d| d != &descriptor);
        self.set_string(
            Self::PREFERRED_VERSION_DESCRIPTORS,
            &descriptors.to_string(),
        )
    }

    pub fn edit_preferred_version_descriptor(
        &self, old_descriptor: Descriptor, new_descriptor: Descriptor,
    ) -> Result<(), glib::BoolError> {
        let mut descriptors = self.preferred_version_descriptors();
        if descriptors.contains(&new_descriptor) {
            return Ok(());
        }
        if let Some(index) = descriptors.iter().position(|d| d == &old_descriptor) {
            descriptors[index] = new_descriptor;
        }
        self.set_string(
            Self::PREFERRED_VERSION_DESCRIPTORS,
            &descriptors.to_string(),
        )
    }

    pub fn set_preferred_version_descriptors(
        &self, descriptors: Vec<Descriptor>,
    ) -> Result<(), glib::BoolError> {
        self.set_string(
            Self::PREFERRED_VERSION_DESCRIPTORS,
            &descriptors.to_string(),
        )
    }

    pub fn set_mpv_hwdec(&self, mpv_hwdec: i32) -> Result<(), glib::BoolError> {
        self.set_int(Self::KEY_MPV_HWDEC, mpv_hwdec)
    }

    pub fn mpv_hwdec(&self) -> i32 {
        self.int(Self::KEY_MPV_HWDEC)
    }

    pub fn set_list_sord_order(&self, list_sort_order: i32) -> Result<(), glib::BoolError> {
        self.set_int(Self::KEY_LIST_SORT_ORDER, list_sort_order)
    }

    pub fn list_sort_order(&self) -> i32 {
        self.int(Self::KEY_LIST_SORT_ORDER)
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

    pub fn mpv_audio_preferred_lang(&self) -> i32 {
        self.int(Self::KEY_MPV_AUDIO_PREFERRED_LANG)
    }

    pub fn mpv_subtitle_preferred_lang(&self) -> i32 {
        self.int(Self::KEY_MPV_SUBTITLE_PREFERRED_LANG)
    }

    pub fn mpv_subtitle_preferred_lang_str(&self) -> String {
        match self.mpv_subtitle_preferred_lang() {
            1 => "eng",
            2 => "chs",
            3 => "jpn",
            4 => "chi",
            5 => "ara",
            6 => "nob",
            7 => "por",
            8 => "fre",
            _ => "",
        }
        .to_string()
    }

    pub fn mpv_default_volume(&self) -> i32 {
        self.int(Self::KEY_MPV_DEFAULT_VOLUME)
    }

    pub fn mpv_show_buffer_speed(&self) -> bool {
        self.boolean(Self::KEY_MPV_SHOW_BUFFER_SPEED)
    }

    pub fn set_mpv_show_buffer_speed(
        &self, mpv_show_buffer_speed: bool,
    ) -> Result<(), glib::BoolError> {
        self.set_boolean(Self::KEY_MPV_SHOW_BUFFER_SPEED, mpv_show_buffer_speed)
    }

    pub fn set_mpv_video_output(&self, mpv_video_output: i32) -> Result<(), glib::BoolError> {
        self.set_int(Self::KEY_MPV_VIDEO_OUTPUT, mpv_video_output)
    }

    pub fn mpv_video_output(&self) -> i32 {
        self.int(Self::KEY_MPV_VIDEO_OUTPUT)
    }

    pub fn set_mpv_action_after_video_end(
        &self, mpv_action_after_video_end: i32,
    ) -> Result<(), glib::BoolError> {
        self.set_int(
            Self::KEY_MPV_ACTION_AFTER_VIDEO_END,
            mpv_action_after_video_end,
        )
    }

    pub fn mpv_action_after_video_end(&self) -> i32 {
        self.int(Self::KEY_MPV_ACTION_AFTER_VIDEO_END)
    }

    pub fn mpv_cache_time(&self) -> i32 {
        self.int(Self::KEY_MPV_CACHE_TIME)
    }

    pub fn mpv_cache_size(&self) -> i32 {
        self.int(Self::KEY_MPV_CACHE_SIZE)
    }

    pub fn mpv_config(&self) -> bool {
        self.boolean(Self::KEY_MPV_CONFIG)
    }

    pub fn mpv_seek_forward_step(&self) -> i32 {
        self.int(Self::KEY_MPV_SEEK_FORWARD_STEP)
    }

    pub fn mpv_seek_backward_step(&self) -> i32 {
        self.int(Self::KEY_MPV_SEEK_BACKWARD_STEP)
    }

    pub fn set_music_repeat_mode(&self, music_repeat_mode: &str) -> Result<(), glib::BoolError> {
        self.set_string(Self::KEY_MUSIC_REPEAT_MODE, music_repeat_mode)
    }

    pub fn music_repeat_mode(&self) -> String {
        self.string(Self::KEY_MUSIC_REPEAT_MODE).to_string()
    }

    pub fn set_accent_color_code(&self, accent_color_code: &str) -> Result<(), glib::BoolError> {
        self.set_string(Self::KEY_ACCENT_COLOR_CODE, accent_color_code)
    }

    pub fn accent_color_code(&self) -> String {
        self.string(Self::KEY_ACCENT_COLOR_CODE).to_string()
    }

    pub fn set_list_sort_by(&self, list_sort: i32) -> Result<(), glib::BoolError> {
        self.set_int(Self::KEY_LIST_SORT_BY, list_sort)
    }

    pub fn list_sort_by(&self) -> i32 {
        self.int(Self::KEY_LIST_SORT_BY)
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

    pub fn auto_select_server(&self) -> bool {
        self.boolean(Self::KEY_IS_AUTO_SELECT_SERVER)
    }

    pub fn set_overlay(&self, overlay: bool) -> Result<(), glib::BoolError> {
        self.set_boolean(Self::KEY_IS_OVERLAY, overlay)
    }
    pub fn overlay(&self) -> bool {
        self.boolean(Self::KEY_IS_OVERLAY)
    }

    pub fn set_root_pic(&self, root_pic: &str) -> Result<(), glib::BoolError> {
        self.set_string(Self::KEY_ROOT_PIC, root_pic)
    }

    pub fn root_pic(&self) -> String {
        self.string(Self::KEY_ROOT_PIC).to_string()
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

    pub fn pic_blur(&self) -> i32 {
        self.int(Self::KEY_PIC_BLUR)
    }

    pub fn set_background_enabled(&self, background_enabled: bool) -> Result<(), glib::BoolError> {
        self.set_boolean(Self::KEY_IS_BACKGROUND_ENABLED, background_enabled)
    }

    pub fn background_enabled(&self) -> bool {
        self.boolean(Self::KEY_IS_BACKGROUND_ENABLED)
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
