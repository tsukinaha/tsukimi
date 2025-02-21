use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gettextrs::gettext;
use gtk::{
    CompositeTemplate,
    gio,
    glib,
    template_callbacks,
};
use libmpv2::SetData;

use super::options_matcher::{
    match_audio_channels,
    match_hwdec_interop,
    match_sub_border_style,
    match_video_upscale,
};
use crate::{
    toast,
    ui::models::SETTINGS,
};

mod imp {
    use glib::subclass::InitializingObject;
    use gtk::glib;

    use super::*;
    use crate::ui::mpv::mpvglarea::MPVGLArea;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/mpv_control_sidebar.ui")]
    #[properties(wrapper_type = super::MPVControlSidebar)]
    pub struct MPVControlSidebar {
        #[property(get, set = Self::set_player, explicit_notify, nullable)]
        pub player: glib::WeakRef<MPVGLArea>,

        #[template_child]
        pub seek_forward_adj: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub seek_backward_adj: TemplateChild<gtk::Adjustment>,

        #[template_child]
        pub buffer_switchrow: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub cache_size_adj: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub cache_time_adj: TemplateChild<gtk::Adjustment>,

        #[template_child]
        pub brightness_adj: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub contrast_adj: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub gamma_adj: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub hue_adj: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub saturation_adj: TemplateChild<gtk::Adjustment>,

        #[template_child]
        pub sub_bold_toggle: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub sub_italic_toggle: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub sub_position_adj: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub sub_size_adj: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub sub_scale_adj: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub sub_font_button: TemplateChild<gtk::FontDialogButton>,
        #[template_child]
        pub sub_border_style_combo: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub sub_border_size_adj: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub sub_shadow_offset_adj: TemplateChild<gtk::Adjustment>,

        #[template_child]
        pub sub_text_color: TemplateChild<gtk::ColorDialogButton>,
        #[template_child]
        pub sub_border_color: TemplateChild<gtk::ColorDialogButton>,
        #[template_child]
        pub sub_background_color: TemplateChild<gtk::ColorDialogButton>,

        #[template_child]
        pub sub_offset_adj: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub sub_speed_adj: TemplateChild<gtk::Adjustment>,

        #[template_child]
        pub audio_offset_adj: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub audio_channel_combo: TemplateChild<adw::ComboRow>,

        #[template_child]
        pub video_upsacle_filter_combo: TemplateChild<adw::ComboRow>,

        #[template_child]
        pub deband_switch: TemplateChild<gtk::Switch>,

        #[template_child]
        pub deband_iterations_adj: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub deband_threshold_adj: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub deband_range_adj: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub deband_grain_adj: TemplateChild<gtk::Adjustment>,

        #[template_child]
        pub stretch_image_subs_to_screen_switchrow: TemplateChild<adw::SwitchRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MPVControlSidebar {
        const NAME: &'static str = "MPVControlSidebar";
        type Type = super::MPVControlSidebar;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for MPVControlSidebar {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().bind_actions();
        }
    }

    impl WidgetImpl for MPVControlSidebar {}
    impl NavigationPageImpl for MPVControlSidebar {}

    impl MPVControlSidebar {
        fn set_player(&self, player: Option<MPVGLArea>) {
            if self.player.upgrade() == player {
                return;
            }
            self.player.set(player.as_ref());
        }
    }
}

glib::wrapper! {
    pub struct MPVControlSidebar(ObjectSubclass<imp::MPVControlSidebar>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::ActionRow, adw::PreferencesRow, @implements gtk::Accessible;
}

impl Default for MPVControlSidebar {
    fn default() -> Self {
        Self::new()
    }
}

#[template_callbacks]
impl MPVControlSidebar {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn bind_actions(&self) {
        let action_group = gio::SimpleActionGroup::new();

        let action_text = gio::ActionEntry::builder("text-justify")
            .parameter_type(Some(&i32::static_variant_type()))
            .state(1.to_variant())
            .activate(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |_, action, parameter| {
                    let parameter = parameter
                        .expect("Could not get parameter.")
                        .get::<i32>()
                        .expect("The variant needs to be of type `i32`.");

                    match parameter {
                        0 => obj.set_mpv_property("sub-justify", "left"),
                        1 => obj.set_mpv_property("sub-justify", "center"),
                        2 => obj.set_mpv_property("sub-justify", "right"),
                        _ => {}
                    }
                    action.set_state(&parameter.to_variant());
                }
            ))
            .build();

        let action_video_end = gio::ActionEntry::builder("video-end")
            .parameter_type(Some(&i32::static_variant_type()))
            .state(SETTINGS.mpv_action_after_video_end().to_variant())
            .activate(move |_, action, parameter| {
                let parameter = parameter
                    .expect("Could not get parameter.")
                    .get::<i32>()
                    .expect("The variant needs to be of type `i32`.");

                SETTINGS.set_mpv_action_after_video_end(parameter).unwrap();

                action.set_state(&parameter.to_variant());
            })
            .build();

        let action_hwdec = gio::ActionEntry::builder("hwdec")
            .parameter_type(Some(&i32::static_variant_type()))
            .state(SETTINGS.mpv_hwdec().to_variant())
            .activate(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |_, action, parameter| {
                    let parameter = parameter
                        .expect("Could not get parameter.")
                        .get::<i32>()
                        .expect("The variant needs to be of type `i32`.");

                    let option = match_hwdec_interop(parameter);
                    obj.set_mpv_property("hwdec", option);

                    SETTINGS.set_mpv_hwdec(parameter).unwrap();

                    action.set_state(&parameter.to_variant());
                }
            ))
            .build();

        action_group.add_action_entries([action_text, action_video_end, action_hwdec]);
        self.insert_action_group("mpv", Some(&action_group));

        let imp = self.imp();

        SETTINGS
            .bind(
                "mpv-seek-backward-step",
                &imp.seek_backward_adj.get(),
                "value",
            )
            .build();
        SETTINGS
            .bind(
                "mpv-seek-forward-step",
                &imp.seek_forward_adj.get(),
                "value",
            )
            .build();
        imp.buffer_switchrow
            .set_active(SETTINGS.mpv_show_buffer_speed());
        imp.sub_font_button
            .set_font_desc(&gtk::pango::FontDescription::from_string(
                &SETTINGS.mpv_subtitle_font(),
            ));
        SETTINGS
            .bind(
                "mpv-show-buffer-speed",
                &imp.buffer_switchrow.get(),
                "active",
            )
            .build();
        SETTINGS
            .bind("mpv-cache-size", &imp.cache_size_adj.get(), "value")
            .build();
        SETTINGS
            .bind("mpv-cache-time", &imp.cache_time_adj.get(), "value")
            .build();
        SETTINGS
            .bind("mpv-deband", &imp.deband_switch.get(), "active")
            .build();
        SETTINGS
            .bind(
                "mpv-audio-channel",
                &imp.audio_channel_combo.get(),
                "selected",
            )
            .build();
        SETTINGS
            .bind("mpv-subtitle-scale", &imp.sub_scale_adj.get(), "value")
            .build();
        SETTINGS
            .bind(
                "mpv-video-scale",
                &imp.video_upsacle_filter_combo.get(),
                "selected",
            )
            .build();
    }

    pub fn set_mpv_property<V>(&self, property: &str, value: V)
    where
        V: SetData + Send + 'static,
    {
        if let Some(player) = self.player() {
            player.set_property(property, value)
        }
    }

    #[template_callback]
    pub fn on_brightness_spinrow(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.set_mpv_property("brightness", spin.value());
    }

    #[template_callback]
    pub fn on_contrast_spinrow(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.set_mpv_property("contrast", spin.value());
    }

    #[template_callback]
    pub fn on_gamma_spinrow(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.set_mpv_property("gamma", spin.value());
    }

    #[template_callback]
    pub fn on_hue_spinrow(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.set_mpv_property("hue", spin.value());
    }

    #[template_callback]
    pub fn on_saturation_spinrow(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.set_mpv_property("saturation", spin.value());
    }

    #[template_callback]
    pub fn on_sub_position(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        // Default: 100
        self.set_mpv_property("sub-pos", spin.value());
    }

    #[template_callback]
    pub fn on_sub_size(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.set_mpv_property("sub-font-size", spin.value());
    }

    #[template_callback]
    pub fn on_sub_scale(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.set_mpv_property("sub-scale", spin.value());
    }

    #[template_callback]
    pub fn on_sub_speed(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.set_mpv_property("sub-speed", spin.value());
    }

    #[template_callback]
    pub fn on_cache_size(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.set_mpv_property("demuxer-max-bytes", format!("{}MiB", spin.value()));
    }

    #[template_callback]
    pub fn on_cache_time(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.set_mpv_property("cache-secs", spin.value());
    }

    #[template_callback]
    pub fn on_buffer_speed(&self, _param: glib::ParamSpec, switch: adw::SwitchRow) {
        SETTINGS
            .set_mpv_show_buffer_speed(switch.is_active())
            .unwrap();
    }

    #[template_callback]
    pub fn on_border_size(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.set_mpv_property("sub-border-size", spin.value());
    }

    #[template_callback]
    fn on_video_deband(&self, _param: glib::ParamSpec, switch: gtk::Switch) {
        self.set_mpv_property("deband", switch.is_active());
    }

    #[template_callback]
    pub fn on_sub_bold(&self, button: gtk::ToggleButton) {
        self.set_mpv_property("sub-bold", button.is_active());
    }

    #[template_callback]
    pub fn on_sub_italic(&self, button: gtk::ToggleButton) {
        self.set_mpv_property("sub-italic", button.is_active());
    }

    #[template_callback]
    pub fn on_stretch_image_subs_to_screen(&self, _param: glib::ParamSpec, switch: adw::SwitchRow) {
        self.set_mpv_property("stretch-image-subs-to-screen", switch.is_active());
    }

    #[template_callback]
    pub fn on_sub_font(&self, _param: glib::ParamSpec, button: gtk::FontDialogButton) {
        let font_desc = button.font_desc().unwrap();
        SETTINGS
            .set_mpv_subtitle_font(font_desc.to_string())
            .unwrap();
        self.set_mpv_property("sub-font", font_desc.to_string());
    }

    #[template_callback]
    pub fn on_sub_text_color(&self, _param: glib::ParamSpec, color: gtk::ColorDialogButton) {
        let rgba = color.rgba();
        self.set_mpv_property(
            "sub-color",
            rgba_to_mpv_color((rgba.red(), rgba.green(), rgba.blue(), rgba.alpha())),
        );
    }

    #[template_callback]
    pub fn on_sub_border_color(&self, _param: glib::ParamSpec, color: gtk::ColorDialogButton) {
        let rgba = color.rgba();
        self.set_mpv_property(
            "sub-border-color",
            rgba_to_mpv_color((rgba.red(), rgba.green(), rgba.blue(), rgba.alpha())),
        );
    }

    #[template_callback]
    pub fn on_sub_background_color(&self, _param: glib::ParamSpec, color: gtk::ColorDialogButton) {
        let rgba = color.rgba();
        self.set_mpv_property(
            "sub-back-color",
            rgba_to_mpv_color((rgba.red(), rgba.green(), rgba.blue(), rgba.alpha())),
        );
    }

    #[template_callback]
    pub fn on_shadow_offset(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.set_mpv_property("sub-shadow-offset", spin.value());
    }

    #[template_callback]
    pub fn on_sub_offset(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.set_mpv_property("sub-delay", spin.value() / 1000.0);
    }

    #[template_callback]
    pub fn on_border_style(&self, _param: glib::ParamSpec, combo: adw::ComboRow) {
        let border_style = match_sub_border_style(combo.selected() as i32);
        self.set_mpv_property("sub-border-style", border_style);
    }

    #[template_callback]
    fn on_video_filter_clear(&self, _button: gtk::Button) {
        let imp = self.imp();
        imp.brightness_adj.set_value(0.0);
        imp.contrast_adj.set_value(0.0);
        imp.gamma_adj.set_value(0.0);
        imp.hue_adj.set_value(0.0);
        imp.saturation_adj.set_value(0.0);

        toast!(self, gettext("Video filter settings cleared."));
    }

    #[template_callback]
    fn on_video_deinterlace(&self, _param: glib::ParamSpec, switch: adw::SwitchRow) {
        self.set_mpv_property("deinterlace", switch.is_active());
    }

    #[template_callback]
    fn on_sub_clear(&self, _button: gtk::Button) {
        let imp = self.imp();
        imp.sub_bold_toggle.set_active(false);
        imp.sub_italic_toggle.set_active(false);
        imp.sub_position_adj.set_value(100.0);
        imp.sub_size_adj.set_value(55.0);
        imp.sub_scale_adj.set_value(1.0);
        imp.sub_font_button
            .set_font_desc(&gtk::pango::FontDescription::from_string(""));
        imp.sub_border_style_combo.set_selected(0);
        imp.sub_border_size_adj.set_value(3.0);
        imp.sub_shadow_offset_adj.set_value(0.0);
        imp.stretch_image_subs_to_screen_switchrow.set_active(false);

        toast!(self, gettext("Subtitle settings cleared."));
    }

    #[template_callback]
    fn on_sub_color_clear(&self, _button: gtk::Button) {
        let imp = self.imp();
        imp.sub_text_color
            .set_rgba(&gtk::gdk::RGBA::new(1.0, 1.0, 1.0, 1.0));
        imp.sub_border_color
            .set_rgba(&gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 1.0));
        imp.sub_background_color
            .set_rgba(&gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));

        toast!(self, gettext("Subtitle color settings cleared."));
    }

    #[template_callback]
    fn on_sub_offset_clear(&self, _button: gtk::Button) {
        let imp = self.imp();
        imp.sub_offset_adj.set_value(0.0);
        imp.sub_speed_adj.set_value(0.0);

        toast!(self, gettext("Subtitle offset settings cleared."));
    }

    #[template_callback]
    fn on_audio_offset(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.set_mpv_property("audio-delay", spin.value() / 1000.0);
    }

    #[template_callback]
    fn on_deband_iterations_spinrow(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.set_mpv_property("deband-iterations", spin.value() as i64);
    }

    #[template_callback]
    fn on_deband_threshold_spinrow(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.set_mpv_property("deband-threshold", spin.value() as i64);
    }

    #[template_callback]
    fn on_deband_range_spinrow(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.set_mpv_property("deband-range", spin.value() as i64);
    }

    #[template_callback]
    fn on_deband_grain_spinrow(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.set_mpv_property("deband-grain", spin.value() as i64);
    }

    #[template_callback]
    fn on_video_upscale(&self, _param: glib::ParamSpec, combo: adw::ComboRow) {
        let upscaler = match_video_upscale(combo.selected() as i32);
        self.set_mpv_property("scale", upscaler);
    }

    #[template_callback]
    fn on_video_deband_clear(&self, _button: gtk::Button) {
        let imp = self.imp();
        imp.deband_iterations_adj.set_value(1.0);
        imp.deband_threshold_adj.set_value(48.0);
        imp.deband_range_adj.set_value(16.0);
        imp.deband_grain_adj.set_value(32.0);

        toast!(self, gettext("Deband settings cleared."));
    }

    #[template_callback]
    fn on_audio_channel(&self, _param: glib::ParamSpec, combo: adw::ComboRow) {
        let selected = combo.selected();

        if selected == 4 {
            self.set_mpv_property("af", "pan=[stereo|c0=c1|c1=c0]");
            return;
        }

        let channel = match_audio_channels(selected as i32);

        self.set_mpv_property("af", "");
        self.set_mpv_property("audio-channels", channel);
    }

    #[template_callback]
    fn on_video_aspect(&self, _param: glib::ParamSpec, combo: adw::ComboRow) {
        let panscan = match combo.selected() {
            1 => 1.0,
            _ => 0.0,
        };

        self.set_mpv_property("panscan", panscan);
    }

    #[template_callback]
    pub fn on_audio_clear(&self, _button: gtk::Button) {
        let imp = self.imp();
        imp.audio_offset_adj.set_value(0.0);
        imp.audio_channel_combo.set_selected(1);
    }
}

fn rgba_to_mpv_color(rgba: (f32, f32, f32, f32)) -> String {
    format!("{}/{}/{}/{}", rgba.0, rgba.1, rgba.2, rgba.3)
}
