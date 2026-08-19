use std::{
    cell::{
        Cell,
        OnceCell,
        RefCell,
    },
    sync::{
        Arc,
        OnceLock,
        atomic::{
            AtomicU64,
            Ordering,
        },
    },
};

use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gtk::{
    CompositeTemplate,
    gdk,
    glib,
};

use crate::{
    LyricsDocument,
    LyricsView,
    PlaybackAnchor,
};

use super::{
    animated_backdrop::{
        AnimatedBackdrop,
        BackgroundQuality,
        same_paintable,
    },
    cover_palette::CoverPalette,
};

const PALETTE_FOREGROUND_CLASS: &str = "lycoric-palette-foreground";
static PLAYER_PAGE_STYLE_ID: AtomicU64 = AtomicU64::new(1);

pub(super) struct PaletteCss {
    display: gdk::Display,
    provider: gtk::CssProvider,
}

mod imp {
    use glib::subclass::InitializingObject;

    use super::*;

    #[derive(Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/io/github/tsukimi/lycoric/ui/player_page.ui")]
    #[properties(wrapper_type = super::LyricPlayerPage)]
    pub struct LyricPlayerPage {
        #[template_child]
        pub backdrop: TemplateChild<AnimatedBackdrop>,
        #[template_child]
        pub cover_picture: TemplateChild<gtk::Picture>,
        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub artist_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub album_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub play_pause_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub position_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub duration_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub seek_scale: TemplateChild<gtk::Scale>,
        #[template_child]
        pub lyrics_view: TemplateChild<LyricsView>,

        #[property(get, set = Self::set_cover, explicit_notify, nullable)]
        pub cover: RefCell<Option<gdk::Paintable>>,
        #[property(get, set = Self::set_title, explicit_notify)]
        pub title: RefCell<String>,
        #[property(get, set = Self::set_artist, explicit_notify)]
        pub artist: RefCell<String>,
        #[property(get, set = Self::set_album, explicit_notify, nullable)]
        pub album: RefCell<Option<String>>,
        #[property(get, set = Self::set_playing, explicit_notify)]
        pub playing: Cell<bool>,
        #[property(get, set = Self::set_position_seconds, explicit_notify)]
        pub position_seconds: Cell<i64>,
        #[property(get, set = Self::set_duration_seconds, explicit_notify)]
        pub duration_seconds: Cell<i64>,
        #[property(get, set = Self::set_seekable, explicit_notify)]
        pub seekable: Cell<bool>,
        #[property(get, set = Self::set_reduced_motion, explicit_notify)]
        pub reduced_motion: Cell<bool>,
        #[property(get, set = Self::set_background_quality, explicit_notify, builder(BackgroundQuality::default()))]
        pub background_quality: Cell<BackgroundQuality>,

        pub seek_pointer_active: Cell<bool>,
        pub palette_css_class: OnceCell<String>,
        pub(super) palette_css: RefCell<Option<PaletteCss>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LyricPlayerPage {
        const NAME: &'static str = "LycoricPlayerPage";
        type Type = super::LyricPlayerPage;
        type ParentType = adw::BreakpointBin;

        fn class_init(klass: &mut Self::Class) {
            AnimatedBackdrop::ensure_type();
            LyricsView::ensure_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
            klass.set_css_name("lycoric-player-page");
            klass.set_accessible_role(gtk::AccessibleRole::Main);
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for LyricPlayerPage {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().finish_template_setup();
        }

        fn dispose(&self) {
            self.seek_pointer_active.set(false);
            self.obj().remove_palette_css_provider();
        }

        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: OnceLock<Vec<glib::subclass::Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    glib::subclass::Signal::builder("play-requested").build(),
                    glib::subclass::Signal::builder("pause-requested").build(),
                    glib::subclass::Signal::builder("previous-requested").build(),
                    glib::subclass::Signal::builder("next-requested").build(),
                    glib::subclass::Signal::builder("seek-requested")
                        .param_types([i64::static_type()])
                        .build(),
                    glib::subclass::Signal::builder("seek-preview-requested")
                        .param_types([i64::static_type()])
                        .build(),
                    glib::subclass::Signal::builder("close-requested").build(),
                ]
            })
        }
    }

    impl WidgetImpl for LyricPlayerPage {}
    impl BreakpointBinImpl for LyricPlayerPage {}

    impl LyricPlayerPage {
        fn set_cover(&self, cover: Option<gdk::Paintable>) {
            if same_paintable(self.cover.borrow().as_ref(), cover.as_ref()) {
                return;
            }
            let palette = cover
                .as_ref()
                .and_then(texture_from_paintable)
                .map(|texture| CoverPalette::from_texture(&texture))
                .unwrap_or_default();
            self.cover.replace(cover.clone());
            self.obj().apply_cover(cover.as_ref(), palette);
            self.obj().notify_cover();
        }

        fn set_title(&self, title: String) {
            if *self.title.borrow() == title {
                return;
            }
            self.title.replace(title.clone());
            self.title_label.set_label(&title);
            self.obj()
                .update_property(&[gtk::accessible::Property::Label(&title)]);
            self.obj().notify_title();
        }

        fn set_artist(&self, artist: String) {
            if *self.artist.borrow() == artist {
                return;
            }
            self.artist.replace(artist.clone());
            self.artist_label.set_label(&artist);
            self.obj().notify_artist();
        }

        fn set_album(&self, album: Option<String>) {
            let album = album.filter(|value| !value.is_empty());
            if *self.album.borrow() == album {
                return;
            }
            self.album.replace(album.clone());
            self.album_label
                .set_label(album.as_deref().unwrap_or_default());
            self.album_label.set_visible(album.is_some());
            self.obj().notify_album();
        }

        fn set_playing(&self, playing: bool) {
            if self.playing.replace(playing) == playing {
                return;
            }
            let icon = if playing {
                "media-playback-pause-symbolic"
            } else {
                "media-playback-start-symbolic"
            };
            let tooltip = if playing { "Pause" } else { "Play" };
            self.play_pause_button.set_icon_name(icon);
            self.play_pause_button.set_tooltip_text(Some(tooltip));
            self.backdrop.set_motion_active(playing);
            self.obj().notify_playing();
        }

        fn set_position_seconds(&self, seconds: i64) {
            let seconds = clamp_seconds(seconds, self.duration_seconds.get());
            if self.position_seconds.replace(seconds) == seconds {
                return;
            }
            if !self.seek_pointer_active.get() {
                self.seek_scale.set_value(seconds as f64);
                self.position_label.set_label(&format_seconds(seconds));
            }
            self.obj().notify_position_seconds();
        }

        fn set_duration_seconds(&self, seconds: i64) {
            let seconds = seconds.max(0);
            if self.duration_seconds.replace(seconds) == seconds {
                return;
            }
            self.seek_scale.set_range(0.0, seconds.max(1) as f64);
            self.duration_label.set_label(&format_seconds(seconds));
            if self.position_seconds.get() > seconds && seconds > 0 {
                self.obj().set_position_seconds(seconds);
            }
            self.obj().notify_duration_seconds();
        }

        fn set_seekable(&self, seekable: bool) {
            if self.seekable.replace(seekable) == seekable {
                return;
            }
            self.seek_scale.set_sensitive(seekable);
            self.obj().notify_seekable();
        }

        fn set_reduced_motion(&self, reduced: bool) {
            if self.reduced_motion.replace(reduced) == reduced {
                return;
            }
            self.backdrop.set_reduced_motion(reduced);
            self.lyrics_view.set_reduced_motion(reduced);
            self.obj().notify_reduced_motion();
        }

        fn set_background_quality(&self, quality: BackgroundQuality) {
            if self.background_quality.replace(quality) == quality {
                return;
            }
            self.backdrop.set_quality(quality);
            self.obj().notify_background_quality();
        }
    }
}

glib::wrapper! {
    pub struct LyricPlayerPage(ObjectSubclass<imp::LyricPlayerPage>)
        @extends gtk::Widget, adw::BreakpointBin,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

#[gtk::template_callbacks]
impl LyricPlayerPage {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_cover_with_palette(&self, cover: Option<&gdk::Paintable>, palette: CoverPalette) {
        let cover_changed = !same_paintable(self.cover().as_ref(), cover);
        if cover_changed {
            self.imp().cover.replace(cover.cloned());
        }
        self.apply_cover(cover, palette);
        if cover_changed {
            self.notify_cover();
        }
    }

    pub fn set_cover_palette(&self, palette: CoverPalette) {
        if self.backdrop().palette() == palette {
            return;
        }
        self.backdrop().set_palette(palette.clone());
        self.update_palette_css(&palette);
    }

    pub fn set_document(&self, document: Option<Arc<LyricsDocument>>) {
        self.lyrics_view().set_document(document);
    }

    pub fn set_playback_anchor(&self, anchor: PlaybackAnchor) {
        self.lyrics_view().set_playback_anchor(anchor);
    }

    pub fn set_lyrics_animation_fps(&self, frames_per_second: u32) {
        self.lyrics_view().set_max_animation_fps(frames_per_second);
    }

    pub fn lyrics_animation_fps(&self) -> u32 {
        self.lyrics_view().max_animation_fps()
    }

    pub fn set_duration_us(&self, duration_us: i64) {
        self.set_duration_seconds(micros_to_duration_seconds(duration_us));
    }

    pub fn set_position_us(&self, position_us: i64) {
        self.set_position_seconds(position_us.max(0) / 1_000_000);
    }

    pub fn is_playing(&self) -> bool {
        self.playing()
    }

    pub fn lyrics_view(&self) -> LyricsView {
        self.imp().lyrics_view.get()
    }

    fn finish_template_setup(&self) {
        let scope = format!(
            "lycoric-player-page-{}",
            PLAYER_PAGE_STYLE_ID.fetch_add(1, Ordering::Relaxed),
        );
        self.add_css_class(&scope);
        self.imp()
            .palette_css_class
            .set(scope)
            .expect("palette CSS scope is initialized once");
        self.update_palette_css(&CoverPalette::default());
        self.connect_seek_scale();
        self.connect_lyrics_seek();
        self.install_close_key();
    }

    fn apply_cover(&self, cover: Option<&gdk::Paintable>, palette: CoverPalette) {
        let cover_changed = !same_paintable(self.backdrop().cover().as_ref(), cover);
        let palette_changed = self.backdrop().palette() != palette;
        if !cover_changed && !palette_changed {
            return;
        }
        if cover_changed {
            self.imp().cover_picture.set_paintable(cover);
        }
        self.backdrop()
            .set_cover_and_palette(cover, palette.clone());
        if palette_changed {
            self.update_palette_css(&palette);
        }
    }

    fn connect_seek_scale(&self) {
        let scale = &self.imp().seek_scale;
        let weak = self.downgrade();
        scale.connect_change_value(move |_, scroll, value| {
            if let Some(obj) = weak.upgrade() {
                obj.preview_seek_seconds(value);
                let pointer_active = obj.imp().seek_pointer_active.get();
                if !pointer_active && scroll != gtk::ScrollType::Jump {
                    obj.emit_seek(seconds_to_micros(value));
                }
            }
            glib::Propagation::Proceed
        });

        let pointer = gtk::EventControllerLegacy::new();
        pointer.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = self.downgrade();
        pointer.connect_event(move |_, event| {
            if let Some(obj) = weak.upgrade() {
                obj.handle_seek_pointer_event(event.event_type());
            }
            glib::Propagation::Proceed
        });
        scale.add_controller(pointer);
    }

    fn handle_seek_pointer_event(&self, event_type: gdk::EventType) {
        match event_type {
            gdk::EventType::ButtonPress | gdk::EventType::TouchBegin => {
                self.imp().seek_pointer_active.set(true);
            }
            gdk::EventType::ButtonRelease | gdk::EventType::TouchEnd => {
                self.finish_pointer_seek();
            }
            gdk::EventType::TouchCancel | gdk::EventType::GrabBroken => {
                self.imp().seek_pointer_active.set(false);
            }
            _ => {}
        }
    }

    fn connect_lyrics_seek(&self) {
        let weak = self.downgrade();
        self.lyrics_view()
            .connect_local("line-activated", false, move |values| {
                let position_us = values
                    .get(1)
                    .and_then(|value| value.get::<i64>().ok())
                    .unwrap_or_default();
                if let Some(obj) = weak.upgrade() {
                    obj.emit_seek(position_us);
                }
                None
            });
    }

    fn install_close_key(&self) {
        let controller = gtk::EventControllerKey::new();
        let weak = self.downgrade();
        controller.connect_key_pressed(move |_, key, _, _| {
            if key != gdk::Key::Escape {
                return glib::Propagation::Proceed;
            }
            if let Some(obj) = weak.upgrade() {
                obj.emit_by_name::<()>("close-requested", &[]);
            }
            glib::Propagation::Stop
        });
        self.add_controller(controller);
    }

    fn preview_seek_seconds(&self, seconds: f64) {
        let seconds = clamp_seconds(seconds.round() as i64, self.duration_seconds());
        self.imp()
            .position_label
            .set_label(&format_seconds(seconds));
        let position_us = seconds.saturating_mul(1_000_000);
        self.emit_by_name::<()>("seek-preview-requested", &[&position_us]);
    }

    fn finish_pointer_seek(&self) {
        if !self.imp().seek_pointer_active.replace(false) {
            return;
        }
        self.emit_seek(seconds_to_micros(self.imp().seek_scale.value()));
    }

    fn emit_seek(&self, position_us: i64) {
        let maximum = self.duration_seconds().saturating_mul(1_000_000);
        let position_us = if maximum > 0 {
            position_us.clamp(0, maximum)
        } else {
            position_us.max(0)
        };
        self.emit_by_name::<()>("seek-requested", &[&position_us]);
    }

    fn update_palette_css(&self, palette: &CoverPalette) {
        let scope = self
            .imp()
            .palette_css_class
            .get()
            .expect("palette CSS scope is initialized during construction");
        let css = palette_foreground_css(scope, &palette.foreground);
        let mut state = self.imp().palette_css.borrow_mut();
        if let Some(state) = state.as_ref() {
            state.provider.load_from_string(&css);
            return;
        }
        let display = self.display();
        let provider = gtk::CssProvider::new();
        provider.load_from_string(&css);
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        *state = Some(PaletteCss { display, provider });
    }

    fn remove_palette_css_provider(&self) {
        let Some(state) = self.imp().palette_css.borrow_mut().take() else {
            return;
        };
        gtk::style_context_remove_provider_for_display(&state.display, &state.provider);
    }

    fn backdrop(&self) -> &AnimatedBackdrop {
        &self.imp().backdrop
    }

    #[template_callback]
    fn on_close_clicked(&self, _button: &gtk::Button) {
        self.emit_by_name::<()>("close-requested", &[]);
    }

    #[template_callback]
    fn on_previous_clicked(&self, _button: &gtk::Button) {
        self.emit_by_name::<()>("previous-requested", &[]);
    }

    #[template_callback]
    fn on_next_clicked(&self, _button: &gtk::Button) {
        self.emit_by_name::<()>("next-requested", &[]);
    }

    #[template_callback]
    fn on_play_pause_clicked(&self, _button: &gtk::Button) {
        let signal = if self.playing() {
            "pause-requested"
        } else {
            "play-requested"
        };
        self.emit_by_name::<()>(signal, &[]);
    }

    #[template_callback]
    fn on_compact_breakpoint_apply(&self, _breakpoint: &adw::Breakpoint) {
        self.imp().title_label.remove_css_class("title-1");
        self.imp().title_label.add_css_class("title-2");
    }

    #[template_callback]
    fn on_compact_breakpoint_unapply(&self, _breakpoint: &adw::Breakpoint) {
        self.imp().title_label.remove_css_class("title-2");
        self.imp().title_label.add_css_class("title-1");
    }
}

impl Default for LyricPlayerPage {
    fn default() -> Self {
        Self::new()
    }
}

fn texture_from_paintable(paintable: &gdk::Paintable) -> Option<gdk::Texture> {
    if let Some(texture) = paintable.downcast_ref::<gdk::Texture>() {
        return Some(texture.clone());
    }
    paintable.current_image().downcast::<gdk::Texture>().ok()
}

fn palette_foreground_css(scope: &str, foreground: &gdk::RGBA) -> String {
    format!(
        ".{scope} .{PALETTE_FOREGROUND_CLASS},\n\
         .{scope} .{PALETTE_FOREGROUND_CLASS} * {{\n\
             color: {foreground};\n\
         }}"
    )
}

fn micros_to_duration_seconds(microseconds: i64) -> i64 {
    let microseconds = microseconds.max(0);
    microseconds.saturating_add(999_999) / 1_000_000
}

fn seconds_to_micros(seconds: f64) -> i64 {
    if !seconds.is_finite() {
        return 0;
    }
    (seconds.max(0.0) * 1_000_000.0)
        .round()
        .clamp(0.0, i64::MAX as f64) as i64
}

fn clamp_seconds(seconds: i64, duration_seconds: i64) -> i64 {
    if duration_seconds > 0 {
        seconds.clamp(0, duration_seconds)
    } else {
        seconds.max(0)
    }
}

fn format_seconds(seconds: i64) -> String {
    let seconds = seconds.max(0);
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
