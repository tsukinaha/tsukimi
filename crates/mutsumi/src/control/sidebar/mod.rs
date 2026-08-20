use adw::{prelude::*, subclass::prelude::*};
use gtk::{CompositeTemplate, gio, glib, template_callbacks};

use crate::MutsumiVideoPlayer;

use super::GlobalToast;

mod imp {
    use std::cell::Cell;

    use glib::subclass::InitializingObject;
    use gtk::glib;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/io/github/mutsumiuniverse/mutsumi/ui/control_sidebar.ui")]
    #[properties(wrapper_type = super::ControlSidebar)]
    pub struct ControlSidebar {
        #[property(get, set = Self::set_player, explicit_notify, nullable)]
        pub player: glib::WeakRef<MutsumiVideoPlayer>,

        #[property(get, set)]
        pub show_buffer_speed: Cell<bool>,

        #[template_child]
        pub playback_speed_adj: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub seek_backward_adj: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub seek_forward_adj: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub buffer_switchrow: TemplateChild<adw::SwitchRow>,

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
        pub stretch_image_subs_to_screen_switchrow: TemplateChild<adw::SwitchRow>,

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
        pub deband_iterations_adj: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub deband_threshold_adj: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub deband_range_adj: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub deband_grain_adj: TemplateChild<gtk::Adjustment>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ControlSidebar {
        const NAME: &'static str = "ControlSidebar";
        type Type = super::ControlSidebar;
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
    impl ObjectImpl for ControlSidebar {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.set_show_buffer_speed(true);
            obj.bind_actions();
        }
    }

    impl WidgetImpl for ControlSidebar {}
    impl NavigationPageImpl for ControlSidebar {}

    impl ControlSidebar {
        fn set_player(&self, player: Option<MutsumiVideoPlayer>) {
            if self.player.upgrade() == player {
                return;
            }
            self.player.set(player.as_ref());
        }
    }
}

glib::wrapper! {
    pub struct ControlSidebar(ObjectSubclass<imp::ControlSidebar>)
        @extends gtk::Widget, adw::NavigationPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for ControlSidebar {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalToast for ControlSidebar {}

#[template_callbacks]
impl ControlSidebar {
    pub fn new() -> Self {
        glib::Object::new()
    }

    fn with_player(&self, f: impl FnOnce(&MutsumiVideoPlayer)) {
        if let Some(player) = self.player() {
            f(&player);
        }
    }

    fn bind_actions(&self) {
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

                    let justify = match parameter {
                        0 => "left",
                        2 => "right",
                        _ => "center",
                    };
                    obj.with_player(|p| p.set_sub_justify(justify));

                    action.set_state(&parameter.to_variant());
                }
            ))
            .build();

        let action_hwdec = gio::ActionEntry::builder("hwdec")
            .parameter_type(Some(&i32::static_variant_type()))
            .state(0.to_variant())
            .activate(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |_, action, parameter| {
                    let parameter = parameter
                        .expect("Could not get parameter.")
                        .get::<i32>()
                        .expect("The variant needs to be of type `i32`.");

                    obj.with_player(|p| p.set_hwdec(match_hwdec_interop(parameter)));

                    action.set_state(&parameter.to_variant());
                }
            ))
            .build();

        action_group.add_action_entries([action_text, action_hwdec]);
        self.insert_action_group("sidebar", Some(&action_group));
    }

    pub fn set_playback_speed(&self, value: f64) {
        self.imp().playback_speed_adj.set_value(value);
    }

    pub fn seek_backward_step(&self) -> i64 {
        self.imp().seek_backward_adj.value() as i64
    }

    pub fn seek_forward_step(&self) -> i64 {
        self.imp().seek_forward_adj.value() as i64
    }

    #[template_callback]
    fn on_playback_speed(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.with_player(|p| p.set_speed(spin.value()));
    }

    #[template_callback]
    fn on_brightness_spinrow(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.with_player(|p| p.set_brightness(spin.value()));
    }

    #[template_callback]
    fn on_contrast_spinrow(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.with_player(|p| p.set_contrast(spin.value()));
    }

    #[template_callback]
    fn on_gamma_spinrow(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.with_player(|p| p.set_gamma(spin.value()));
    }

    #[template_callback]
    fn on_hue_spinrow(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.with_player(|p| p.set_hue(spin.value()));
    }

    #[template_callback]
    fn on_saturation_spinrow(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.with_player(|p| p.set_saturation(spin.value()));
    }

    #[template_callback]
    fn on_sub_position(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        // Default: 100
        self.with_player(|p| p.set_sub_pos(spin.value()));
    }

    #[template_callback]
    fn on_sub_size(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.with_player(|p| p.set_sub_font_size(spin.value()));
    }

    #[template_callback]
    fn on_sub_scale(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.with_player(|p| p.set_sub_scale(spin.value()));
    }

    #[template_callback]
    fn on_sub_speed(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.with_player(|p| p.set_sub_speed(spin.value()));
    }

    #[template_callback]
    fn on_sub_bold(&self, button: gtk::ToggleButton) {
        self.with_player(|p| p.set_sub_bold(button.is_active()));
    }

    #[template_callback]
    fn on_sub_italic(&self, button: gtk::ToggleButton) {
        self.with_player(|p| p.set_sub_italic(button.is_active()));
    }

    #[template_callback]
    fn on_sub_font(&self, _param: glib::ParamSpec, button: gtk::FontDialogButton) {
        let Some(font_desc) = button.font_desc() else {
            return;
        };
        self.with_player(|p| p.set_sub_font(&font_desc.to_string()));
    }

    #[template_callback]
    fn on_border_style(&self, _param: glib::ParamSpec, combo: adw::ComboRow) {
        let border_style = match_sub_border_style(combo.selected() as i32);
        self.with_player(|p| p.set_sub_border_style(border_style));
    }

    #[template_callback]
    fn on_border_size(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.with_player(|p| p.set_sub_border_size(spin.value()));
    }

    #[template_callback]
    fn on_shadow_offset(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.with_player(|p| p.set_sub_shadow_offset(spin.value()));
    }

    #[template_callback]
    fn on_stretch_image_subs_to_screen(&self, _param: glib::ParamSpec, switch: adw::SwitchRow) {
        self.with_player(|p| p.set_stretch_image_subs_to_screen(switch.is_active()));
    }

    #[template_callback]
    fn on_sub_text_color(&self, _param: glib::ParamSpec, color: gtk::ColorDialogButton) {
        self.with_player(|p| p.set_sub_color(&rgba_to_mpv_color(color.rgba())));
    }

    #[template_callback]
    fn on_sub_border_color(&self, _param: glib::ParamSpec, color: gtk::ColorDialogButton) {
        self.with_player(|p| p.set_sub_border_color(&rgba_to_mpv_color(color.rgba())));
    }

    #[template_callback]
    fn on_sub_background_color(&self, _param: glib::ParamSpec, color: gtk::ColorDialogButton) {
        self.with_player(|p| p.set_sub_back_color(&rgba_to_mpv_color(color.rgba())));
    }

    #[template_callback]
    fn on_sub_offset(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.with_player(|p| p.set_sub_delay(spin.value() / 1000.0));
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

        self.toast("Subtitle settings cleared.");
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

        self.toast("Subtitle color settings cleared.");
    }

    #[template_callback]
    fn on_sub_offset_clear(&self, _button: gtk::Button) {
        let imp = self.imp();
        imp.sub_offset_adj.set_value(0.0);
        imp.sub_speed_adj.set_value(1.0);

        self.toast("Subtitle offset settings cleared.");
    }

    #[template_callback]
    fn on_audio_offset(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.with_player(|p| p.set_audio_delay(spin.value() / 1000.0));
    }

    #[template_callback]
    fn on_audio_channel(&self, _param: glib::ParamSpec, combo: adw::ComboRow) {
        let selected = combo.selected();

        if selected == 4 {
            self.with_player(|p| p.set_audio_pan("pan=[stereo|c0=c1|c1=c0]"));
            return;
        }

        let channel = match_audio_channels(selected as i32);

        self.with_player(|p| {
            p.clear_audio_pan();
            p.set_audio_channels(channel);
        });
    }

    #[template_callback]
    fn on_audio_clear(&self, _button: gtk::Button) {
        let imp = self.imp();
        imp.audio_offset_adj.set_value(0.0);
        imp.audio_channel_combo.set_selected(0);

        self.toast("Audio settings cleared.");
    }

    #[template_callback]
    fn on_video_aspect(&self, _param: glib::ParamSpec, combo: adw::ComboRow) {
        let panscan = match combo.selected() {
            1 => 1.0,
            _ => 0.0,
        };

        self.with_player(|p| p.set_panscan(panscan));
    }

    #[template_callback]
    fn on_video_filter_clear(&self, _button: gtk::Button) {
        let imp = self.imp();
        imp.brightness_adj.set_value(0.0);
        imp.contrast_adj.set_value(0.0);
        imp.gamma_adj.set_value(0.0);
        imp.hue_adj.set_value(0.0);
        imp.saturation_adj.set_value(0.0);

        self.toast("Video filter settings cleared.");
    }

    #[template_callback]
    fn on_video_deband(&self, _param: glib::ParamSpec, switch: gtk::Switch) {
        self.with_player(|p| p.set_deband(switch.is_active()));
    }

    #[template_callback]
    fn on_deband_iterations_spinrow(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.with_player(|p| p.set_deband_iterations(spin.value() as i64));
    }

    #[template_callback]
    fn on_deband_threshold_spinrow(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.with_player(|p| p.set_deband_threshold(spin.value() as i64));
    }

    #[template_callback]
    fn on_deband_range_spinrow(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.with_player(|p| p.set_deband_range(spin.value() as i64));
    }

    #[template_callback]
    fn on_deband_grain_spinrow(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.with_player(|p| p.set_deband_grain(spin.value() as i64));
    }

    #[template_callback]
    fn on_video_deband_clear(&self, _button: gtk::Button) {
        let imp = self.imp();
        imp.deband_iterations_adj.set_value(1.0);
        imp.deband_threshold_adj.set_value(48.0);
        imp.deband_range_adj.set_value(16.0);
        imp.deband_grain_adj.set_value(32.0);

        self.toast("Deband settings cleared.");
    }

    #[template_callback]
    fn on_video_deinterlace(&self, _param: glib::ParamSpec, switch: adw::SwitchRow) {
        self.with_player(|p| p.set_deinterlace(switch.is_active()));
    }

    #[template_callback]
    fn on_video_upscale(&self, _param: glib::ParamSpec, combo: adw::ComboRow) {
        let upscaler = match_video_upscale(combo.selected() as i32);
        self.with_player(|p| p.set_scale(upscaler));
    }

    #[template_callback]
    fn on_cache_size(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.with_player(|p| p.set_demuxer_max_bytes(&format!("{}MiB", spin.value())));
    }

    #[template_callback]
    fn on_cache_time(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.with_player(|p| p.set_cache_secs(spin.value()));
    }
}

fn rgba_to_mpv_color(rgba: gtk::gdk::RGBA) -> String {
    format!(
        "{}/{}/{}/{}",
        rgba.red(),
        rgba.green(),
        rgba.blue(),
        rgba.alpha()
    )
}

pub fn match_video_upscale<'a>(matcher: i32) -> &'a str {
    match matcher {
        0 => "lanczos",
        1 => "bilinear",
        2 => "ewa_lanczos",
        3 => "mitchell",
        4 => "hermite",
        5 => "oversample",
        6 => "linear",
        7 => "ewa_hanning",
        _ => "ewa_lanczossharp",
    }
}

pub fn match_audio_channels<'a>(matcher: i32) -> &'a str {
    match matcher {
        1 => "auto-safe",
        2 => "mono",
        3 => "stereo",
        _ => "auto",
    }
}

pub fn match_sub_border_style<'a>(matcher: i32) -> &'a str {
    match matcher {
        1 => "opaque-box",
        2 => "background-box",
        _ => "outline-and-shadow",
    }
}

pub fn match_hwdec_interop<'a>(matcher: i32) -> &'a str {
    match matcher {
        1 => "auto-safe",
        2 => "vaapi",
        _ => "no",
    }
}
