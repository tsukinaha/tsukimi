use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{gio, glib, template_callbacks, CompositeTemplate};
use libmpv2::SetData;

use crate::{toast, ui::models::SETTINGS};

mod imp {
    use super::*;
    use crate::ui::mpv::mpvglarea::MPVGLArea;
    use glib::subclass::InitializingObject;
    use gtk::glib;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsukimi/mpv_control_sidebar.ui")]
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

                    match parameter {
                        0 => obj.set_mpv_property("hwdec", "no"),
                        1 => obj.set_mpv_property("hwdec", "auto-safe"),
                        2 => obj.set_mpv_property("hwdec", "vaapi"),
                        _ => {}
                    }

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
    }

    pub fn set_mpv_property<V>(&self, property: &str, value: V)
    where
        V: SetData,
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
    pub fn on_sub_bold(&self, button: gtk::ToggleButton) {
        self.set_mpv_property("sub-bold", button.is_active());
    }

    #[template_callback]
    pub fn on_sub_italic(&self, button: gtk::ToggleButton) {
        self.set_mpv_property("sub-italic", button.is_active());
    }

    #[template_callback]
    pub fn on_sub_font(&self, _param: glib::ParamSpec, button: gtk::FontDialogButton) {
        let font_desc = button.font_desc().unwrap();
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
            "sub-background-color",
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
        let border_style = match combo.selected() {
            1 => "opaque-box",
            2 => "background-box",
            _ => "outline-and-shadow",
        };
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
    fn on_audio_channel(&self, _param: glib::ParamSpec, combo: adw::ComboRow) {
        let selected = combo.selected();

        if selected == 4 {
            self.set_mpv_property("af", "pan=[stereo|c0=c1|c1=c0]");
            return;
        }

        let channel = match combo.selected() {
            1 => "auto-safe",
            2 => "mono",
            3 => "stereo",
            _ => "auto",
        };

        self.set_mpv_property("af", "");
        self.set_mpv_property("audio-channels", channel);
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
