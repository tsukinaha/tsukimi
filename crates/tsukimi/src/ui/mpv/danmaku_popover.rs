use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gettextrs::gettext;
use gtk::{
    CompositeTemplate,
    glib,
};

use crate::ui::{
    models::SETTINGS,
    mpv::{
        danmaku_client::DanmakuClient,
        danmaku_search_dialog::DanmakuSearchDialog,
        page::MPVPage,
    },
};

#[derive(Debug, Clone, Default, PartialEq, Eq, glib::Boxed)]
#[boxed_type(name = "TsukimiDanmakuPopoverStatus")]
pub enum DanmakuPopoverStatus {
    Searching,
    Loaded(usize, String),
    Loading,
    NoMatching,
    ManualLoaded(usize, String),
    SecretNotExist,
    #[default]
    Idle,
    Disabled,
    Unavailable,
}

impl DanmakuPopoverStatus {
    pub fn title(&self) -> String {
        match self {
            DanmakuPopoverStatus::Loaded(i, _) | DanmakuPopoverStatus::ManualLoaded(i, _) => {
                gettext("{count} Danmaku Loaded").replace("{count}", &i.to_string())
            }
            _ => gettext("Danmaku"),
        }
    }

    pub fn status_subtitle(&self) -> String {
        match self {
            DanmakuPopoverStatus::Searching => gettext("Searching"),
            DanmakuPopoverStatus::Loading => gettext("Loading"),
            DanmakuPopoverStatus::Loaded(_, item_name)
            | DanmakuPopoverStatus::ManualLoaded(_, item_name) => item_name.clone(),
            DanmakuPopoverStatus::NoMatching => gettext("No danmaku found"),
            DanmakuPopoverStatus::SecretNotExist => {
                gettext("This feature requires an official build")
            }
            DanmakuPopoverStatus::Idle => gettext("Not loaded"),
            DanmakuPopoverStatus::Disabled => gettext("Disabled"),
            DanmakuPopoverStatus::Unavailable => gettext("Maybe there is something wrong"),
        }
    }

    pub fn status_title(&self) -> String {
        match self {
            DanmakuPopoverStatus::Loaded(..) => gettext("From 弹弹play开放弹幕网络 (Auto Matched)"),
            DanmakuPopoverStatus::ManualLoaded(..) => {
                gettext("From 弹弹play开放弹幕网络 (Manually Matched)")
            }
            _ => String::new(),
        }
    }

    pub fn icon_name(&self) -> &'static str {
        match self {
            DanmakuPopoverStatus::Searching => "",
            DanmakuPopoverStatus::Loading => "",
            DanmakuPopoverStatus::Loaded(..) | DanmakuPopoverStatus::ManualLoaded(..) => {
                "check-round-outline-symbolic"
            }
            DanmakuPopoverStatus::NoMatching
            | DanmakuPopoverStatus::Idle
            | DanmakuPopoverStatus::Disabled => "minus-circle-outline-symbolic",
            DanmakuPopoverStatus::SecretNotExist => "cross-small-circle-outline-symbolic",
            DanmakuPopoverStatus::Unavailable => "question-round-outline-symbolic",
        }
    }

    pub fn stack_visible_child_name(&self) -> &'static str {
        match self {
            DanmakuPopoverStatus::Searching | DanmakuPopoverStatus::Loading => "loading",
            _ => "icon",
        }
    }

    pub fn status_css_class(&self) -> &'static [&'static str] {
        match self {
            DanmakuPopoverStatus::Searching => &["blink", "accent"],
            DanmakuPopoverStatus::Loading => &["blink"],
            DanmakuPopoverStatus::Loaded(..) => &["success"],
            DanmakuPopoverStatus::ManualLoaded(..) => &["success"],
            DanmakuPopoverStatus::NoMatching => &["warning"],
            DanmakuPopoverStatus::SecretNotExist => &["error"],
            DanmakuPopoverStatus::Idle
            | DanmakuPopoverStatus::Disabled
            | DanmakuPopoverStatus::Unavailable => &[],
        }
    }
}

pub mod imp {
    use std::cell::{
        Cell,
        RefCell,
    };

    use super::*;
    use glib::subclass::InitializingObject;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/danmaku_popover.ui")]
    #[properties(wrapper_type = super::DanmakuPopover)]
    pub struct DanmakuPopover {
        pub page: glib::WeakRef<MPVPage>,
        #[template_child]
        pub danmaku_status_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub danmaku_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub danmaku_status_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub danmaku_status_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub danmaku_status_icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub manual_search_button: TemplateChild<adw::ButtonRow>,
        #[template_child]
        pub danmaku_opacity_spin: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub danmaku_speed_spin: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub font_size_spin: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub font_weight_row: TemplateChild<crate::ui::mpv::DanmakuScaleRow>,
        #[template_child]
        pub intensity_row: TemplateChild<crate::ui::mpv::DanmakuScaleRow>,
        #[template_child]
        pub spacing_spin: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub outline_spin: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub shadow_spin: TemplateChild<adw::SpinRow>,

        #[property(get, set = Self::set_enabled, explicit_notify)]
        pub enabled: Cell<bool>,
        #[property(get, set = Self::set_status, explicit_notify)]
        pub status: RefCell<DanmakuPopoverStatus>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DanmakuPopover {
        const NAME: &'static str = "DanmakuPopover";
        type Type = super::DanmakuPopover;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for DanmakuPopover {
        fn constructed(&self) {
            self.parent_constructed();

            if DanmakuClient::instance().is_none() {
                self.set_status(DanmakuPopoverStatus::SecretNotExist);
            }
        }
    }

    impl DanmakuPopover {
        fn set_enabled(&self, enabled: bool) {
            if self.enabled.replace(enabled) == enabled {
                return;
            }
            self.update_status_view();
            self.obj().notify_enabled();
        }

        fn set_status(&self, status: DanmakuPopoverStatus) {
            // Controls are disabled by default, so update sensitivity even if the status is unchanged
            let sensitive = status != DanmakuPopoverStatus::SecretNotExist;
            self.danmaku_switch.set_sensitive(sensitive);
            self.manual_search_button.set_sensitive(sensitive);

            if self.status.replace(status.clone()) == status {
                return;
            }
            self.update_status_view();
            self.obj().notify_status();
        }

        fn update_status_view(&self) {
            let status = if *self.status.borrow() == DanmakuPopoverStatus::SecretNotExist {
                DanmakuPopoverStatus::SecretNotExist
            } else if !self.enabled.get() {
                DanmakuPopoverStatus::Disabled
            } else {
                self.status.borrow().clone()
            };
            self.danmaku_status_group.set_title(&status.title());
            self.danmaku_status_row.set_title(&status.status_title());
            self.danmaku_status_row
                .set_subtitle(&status.status_subtitle());
            self.danmaku_status_icon
                .set_icon_name(Some(status.icon_name()));
            self.danmaku_status_stack
                .set_visible_child_name(status.stack_visible_child_name());
            self.danmaku_status_stack
                .set_css_classes(status.status_css_class());
        }
    }

    impl WidgetImpl for DanmakuPopover {}

    impl BinImpl for DanmakuPopover {}
}

glib::wrapper! {
    pub struct DanmakuPopover(ObjectSubclass<imp::DanmakuPopover>)
        @extends gtk::Widget, adw::Bin,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

#[gtk::template_callbacks]
impl DanmakuPopover {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_page(&self, page: &MPVPage) {
        self.imp().page.set(Some(page));
        let danmakw = page.danmakw();
        let imp = self.imp();

        SETTINGS.bind_mpv_danmaku_enabled(&imp.danmaku_switch.get(), "active");
        SETTINGS.bind_mpv_danmaku_opacity(&imp.danmaku_opacity_spin.get(), "value");
        SETTINGS.bind_mpv_danmaku_speed_factor(&imp.danmaku_speed_spin.get(), "value");
        SETTINGS.bind_mpv_danmaku_font_size(&imp.font_size_spin.get(), "value");
        SETTINGS.bind_mpv_danmaku_font_weight(&imp.font_weight_row.get(), "value");
        SETTINGS.bind_mpv_danmaku_intensity(&imp.intensity_row.get(), "value");
        SETTINGS.bind_mpv_danmaku_spacing_factor(&imp.spacing_spin.get(), "value");
        SETTINGS.bind_mpv_danmaku_outline_size(&imp.outline_spin.get(), "value");
        SETTINGS.bind_mpv_danmaku_shadow_offset(&imp.shadow_spin.get(), "value");

        imp.danmaku_switch
            .bind_property("active", self, "enabled")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();
        imp.danmaku_opacity_spin
            .bind_property("value", &danmakw, "opacity")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();
        imp.danmaku_speed_spin
            .bind_property("value", &danmakw, "speed-factor")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();
        imp.font_size_spin
            .bind_property("value", &danmakw, "font-size")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();
        imp.font_weight_row
            .bind_property("value", &danmakw, "font-weight")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .transform_to(|_, value: f64| Some(value.round() as u32))
            .build();
        imp.intensity_row
            .bind_property("value", &danmakw, "intensity")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .transform_to(|_, value: f64| Some(mutsumi::Intensity::from(value.round() as u32)))
            .build();
        imp.spacing_spin
            .bind_property("value", &danmakw, "spacing-factor")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();
        imp.outline_spin
            .bind_property("value", &danmakw, "outline-px")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();
        imp.shadow_spin
            .bind_property("value", &danmakw, "shadow-offset")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();
    }

    #[template_callback]
    fn on_manual_search(&self) {
        let Some(client) = DanmakuClient::instance() else {
            return;
        };
        let Some(page) = self.imp().page.upgrade() else {
            return;
        };
        DanmakuSearchDialog::new(&page, client).present(Some(self));
    }

    #[template_callback]
    fn on_danmaku_switch_state_set(&self, state: bool, _switch: &gtk::Switch) -> bool {
        if let Some(page) = self.imp().page.upgrade() {
            page.on_danmaku_switch_state_set(state);
        }
        false
    }
}
