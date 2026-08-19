use std::{
    cell::{
        Cell,
        RefCell,
    },
    env,
    ffi::OsString,
    path::PathBuf,
    process::ExitCode,
    rc::Rc,
    sync::Arc,
    time::Duration,
};

use adw::prelude::*;
use gst::prelude::*;
use gtk::{
    gdk,
    gio,
    glib,
};
use lycoric::{
    LyricPlayerPage,
    LyricTime,
    LyricsDocument,
    ParseOptions,
    PlaybackAnchor,
    PlaybackState,
    parse_lrc,
};

const MICROS_PER_SECOND: i64 = 1_000_000;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("gst_player: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let inputs = PlayerInputs::parse()?;
    gst::init().map_err(|error| format!("could not initialize GStreamer: {error}"))?;
    let lyrics = LyricsSource::load(&inputs.lrc_path)?;
    let document = Arc::new(lyrics.parse(None)?);

    let app = adw::Application::builder()
        .application_id("io.github.tsukimi.lycoric.GstPlayerExample")
        .build();
    app.connect_activate(move |app| {
        if let Err(error) = build_window(app, &inputs, &lyrics, document.clone()) {
            eprintln!("gst_player: {error}");
            app.quit();
        }
    });
    let _status = app.run_with_args::<&str>(&[]);
    Ok(())
}

#[derive(Clone)]
struct PlayerInputs {
    audio: OsString,
    lrc_path: PathBuf,
    cover_path: Option<PathBuf>,
}

impl PlayerInputs {
    fn parse() -> Result<Self, String> {
        let mut arguments = env::args_os();
        let program = arguments
            .next()
            .unwrap_or_else(|| OsString::from("gst_player"));
        let usage = || {
            format!(
                "usage: {} AUDIO_URI_OR_PATH LRC_PATH [COVER_PATH]",
                program.to_string_lossy()
            )
        };
        let audio = arguments.next().ok_or_else(&usage)?;
        let lrc_path = arguments.next().map(PathBuf::from).ok_or_else(&usage)?;
        let cover_path = arguments.next().map(PathBuf::from);
        if arguments.next().is_some() {
            return Err(usage());
        }
        Ok(Self {
            audio,
            lrc_path,
            cover_path,
        })
    }
}

#[derive(Clone)]
struct LyricsSource {
    path: PathBuf,
    source: Arc<str>,
}

impl LyricsSource {
    fn load(path: &PathBuf) -> Result<Self, String> {
        let source = std::fs::read_to_string(path)
            .map_err(|error| format!("could not read lyrics {}: {error}", path.display()))?;
        Ok(Self {
            path: path.clone(),
            source: Arc::from(source),
        })
    }

    fn parse(&self, media_duration_us: Option<i64>) -> Result<LyricsDocument, String> {
        let options = ParseOptions {
            media_duration: media_duration_us.map(LyricTime::from_micros),
            ..ParseOptions::default()
        };
        let report = parse_lrc(&self.source, &options);
        for diagnostic in &report.diagnostics {
            eprintln!("{}: {diagnostic}", self.path.display());
        }
        report.into_result().map_err(|diagnostics| {
            let details = diagnostics
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ");
            format!("lyrics {} contain errors: {details}", self.path.display())
        })
    }
}

fn build_window(
    app: &adw::Application, inputs: &PlayerInputs, lyrics: &LyricsSource,
    document: Arc<LyricsDocument>,
) -> Result<(), String> {
    lycoric::init();
    let page = build_page(document);
    load_cover(&page, inputs.cover_path.as_ref())?;
    let pipeline = build_pipeline(&inputs.audio)?;
    let controller = GstController::new(&page, pipeline, lyrics.clone())?;
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Lycoric GStreamer player")
        .default_width(1180)
        .default_height(760)
        .content(&page)
        .build();

    connect_page_close(&page, &window);
    let close_controller = controller.clone();
    window.connect_close_request(move |_| {
        close_controller.shutdown();
        glib::Propagation::Proceed
    });

    controller.start()?;
    window.present();
    controller.refresh_clock();
    Ok(())
}

fn build_page(document: Arc<LyricsDocument>) -> LyricPlayerPage {
    let page = LyricPlayerPage::new();
    page.set_title(document.metadata.title.as_deref().unwrap_or("Local audio"));
    page.set_artist(
        document
            .metadata
            .artist
            .as_deref()
            .unwrap_or("Unknown artist"),
    );
    page.set_album(document.metadata.album.as_deref());
    let duration_us = document
        .duration
        .map(LyricTime::as_micros)
        .unwrap_or_default();
    page.set_duration_us(duration_us);
    page.set_seekable(false);
    page.set_document(Some(document));
    page
}

fn load_cover(page: &LyricPlayerPage, path: Option<&PathBuf>) -> Result<(), String> {
    let Some(path) = path else {
        return Ok(());
    };
    let file = gio::File::for_path(path);
    let texture = gdk::Texture::from_file(&file)
        .map_err(|error| format!("could not load cover {}: {error}", path.display()))?;
    page.set_cover(Some(texture.upcast_ref::<gdk::Paintable>()));
    Ok(())
}

fn build_pipeline(audio: &OsString) -> Result<gst::Element, String> {
    let uri = gio::File::for_commandline_arg(audio).uri();
    gst::ElementFactory::make("playbin3")
        .property("uri", uri.as_str())
        .build()
        .map_err(|error| format!("could not create playbin3 for {uri}: {error}"))
}

fn connect_page_close(page: &LyricPlayerPage, window: &adw::ApplicationWindow) {
    let weak_window = window.downgrade();
    page.connect_local("close-requested", false, move |_| {
        if let Some(window) = weak_window.upgrade() {
            window.close();
        }
        None
    });
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PendingSeek {
    target_us: i64,
    generation: u64,
}

struct GstController {
    pipeline: gst::Element,
    page: glib::WeakRef<LyricPlayerPage>,
    lyrics: LyricsSource,
    bus_watch: RefCell<Option<gst::bus::BusWatchGuard>>,
    tick_source: RefCell<Option<glib::SourceId>>,
    playback_state: Cell<PlaybackState>,
    position_us: Cell<i64>,
    duration_us: Cell<i64>,
    last_published_second: Cell<Option<i64>>,
    lyrics_media_duration_us: Cell<Option<i64>>,
    pending_seek: Cell<Option<PendingSeek>>,
    seek_generation: Cell<u64>,
    serial: Cell<u64>,
    shutting_down: Cell<bool>,
}

impl GstController {
    fn new(
        page: &LyricPlayerPage, pipeline: gst::Element, lyrics: LyricsSource,
    ) -> Result<Rc<Self>, String> {
        let controller = Rc::new(Self {
            pipeline,
            page: page.downgrade(),
            lyrics,
            bus_watch: RefCell::new(None),
            tick_source: RefCell::new(None),
            playback_state: Cell::new(PlaybackState::Paused),
            position_us: Cell::new(0),
            duration_us: Cell::new(0),
            last_published_second: Cell::new(None),
            lyrics_media_duration_us: Cell::new(None),
            pending_seek: Cell::new(None),
            seek_generation: Cell::new(0),
            serial: Cell::new(0),
            shutting_down: Cell::new(false),
        });
        controller.connect_page_controls(page);
        controller.install_bus_watch()?;
        controller.install_clock_timer();
        Ok(controller)
    }

    fn connect_page_controls(self: &Rc<Self>, page: &LyricPlayerPage) {
        let weak = Rc::downgrade(self);
        page.connect_local("play-requested", false, move |_| {
            if let Some(controller) = weak.upgrade() {
                controller.play();
            }
            None
        });

        let weak = Rc::downgrade(self);
        page.connect_local("pause-requested", false, move |_| {
            if let Some(controller) = weak.upgrade() {
                controller.pause();
            }
            None
        });

        let weak = Rc::downgrade(self);
        page.connect_local("seek-requested", false, move |values| {
            let position_us = values.get(1).and_then(|value| value.get::<i64>().ok());
            if let (Some(controller), Some(position_us)) = (weak.upgrade(), position_us) {
                controller.seek(position_us);
            }
            None
        });
    }

    fn install_bus_watch(self: &Rc<Self>) -> Result<(), String> {
        let bus = self
            .pipeline
            .bus()
            .ok_or_else(|| "playbin3 did not provide a bus".to_owned())?;
        let weak = Rc::downgrade(self);
        let guard = bus
            .add_watch_local(move |_, message| {
                let Some(controller) = weak.upgrade() else {
                    return glib::ControlFlow::Break;
                };
                controller.handle_bus_message(message);
                glib::ControlFlow::Continue
            })
            .map_err(|error| format!("could not install GStreamer bus watch: {error}"))?;
        self.bus_watch.replace(Some(guard));
        Ok(())
    }

    fn install_clock_timer(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        let source = glib::timeout_add_local(Duration::from_secs(1), move || {
            let Some(controller) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            controller.refresh_clock();
            glib::ControlFlow::Continue
        });
        self.tick_source.replace(Some(source));
    }

    fn start(&self) -> Result<(), String> {
        self.pipeline
            .set_state(gst::State::Playing)
            .map_err(|error| format!("could not start playback: {error}"))?;
        Ok(())
    }

    fn play(&self) {
        if self.duration_us.get() > 0 && self.position_us.get() >= self.duration_us.get() {
            self.seek(0);
        }
        self.request_state(gst::State::Playing);
    }

    fn pause(&self) {
        self.request_state(gst::State::Paused);
    }

    fn request_state(&self, target: gst::State) {
        match self.pipeline.set_state(target) {
            Ok(_) => {
                let state = playback_state(target);
                let state_changed = self.playback_state.replace(state) != state;
                if self.pending_seek.get().is_some() {
                    self.publish_pending(state, state_changed);
                } else {
                    self.publish(self.position_us.get(), state, state_changed);
                }
            }
            Err(error) => eprintln!("could not change GStreamer state to {target:?}: {error}"),
        }
    }

    fn seek(&self, position_us: i64) {
        let position_us = position_us.max(0);
        let raw_position = match u64::try_from(position_us) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("seek position is outside GStreamer range: {error}");
                return;
            }
        };
        let target = gst::ClockTime::from_useconds(raw_position);
        let flags = gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE;
        let generation = self.seek_generation.get().wrapping_add(1);
        match self.pipeline.seek_simple(flags, target) {
            Ok(()) => {
                self.seek_generation.set(generation);
                self.pending_seek.set(Some(PendingSeek {
                    target_us: position_us,
                    generation,
                }));
                self.position_us.set(position_us);
                self.publish_discontinuity(position_us, self.playback_state.get());
            }
            Err(error) => eprintln!("GStreamer seek to {position_us}µs failed: {error}"),
        }
    }

    fn handle_bus_message(&self, message: &gst::Message) {
        match message.view() {
            gst::MessageView::StateChanged(changed) if self.is_pipeline_message(message) => {
                eprintln!(
                    "GStreamer state: {:?} -> {:?}",
                    changed.old(),
                    changed.current()
                );
                self.playback_state.set(playback_state(changed.current()));
                self.refresh_seekability();
                self.refresh_clock_for_event();
            }
            gst::MessageView::StreamStart(..) => self.handle_stream_start(),
            gst::MessageView::DurationChanged(..) => {
                self.refresh_seekability();
                self.refresh_clock();
            }
            gst::MessageView::AsyncDone(..) if self.is_pipeline_message(message) => {
                self.handle_async_done();
            }
            gst::MessageView::ClockLost(..) => self.handle_clock_lost(),
            gst::MessageView::Eos(..) => self.handle_eos(),
            gst::MessageView::Error(error) => self.handle_error(error),
            _ => {}
        }
    }

    fn is_pipeline_message(&self, message: &gst::Message) -> bool {
        message
            .src()
            .is_some_and(|source| source == self.pipeline.upcast_ref::<gst::Object>())
    }

    fn handle_stream_start(&self) {
        self.pending_seek.set(None);
        self.refresh_media_info();
        let position_us = self
            .pipeline
            .query_position::<gst::ClockTime>()
            .map(clock_time_us)
            .unwrap_or(0);
        let state = playback_state(self.pipeline.current_state());
        self.position_us.set(position_us);
        self.playback_state.set(state);
        self.publish_discontinuity(position_us, state);
    }

    fn handle_async_done(&self) {
        self.refresh_media_info();
        let Some(pending) = self.pending_seek.get() else {
            self.refresh_clock_for_event();
            return;
        };
        let Some(position) = self.pipeline.query_position::<gst::ClockTime>() else {
            eprintln!(
                "could not query position after seek generation {} to {}µs completed",
                pending.generation, pending.target_us
            );
            if self.clear_pending_seek(pending.generation) {
                self.serial.set(self.serial.get().wrapping_add(1));
                self.refresh_clock_for_event();
            }
            return;
        };
        if !self.clear_pending_seek(pending.generation) {
            return;
        }
        let position_us = clock_time_us(position);
        let state = playback_state(self.pipeline.current_state());
        self.position_us.set(position_us);
        self.playback_state.set(state);
        self.publish_discontinuity(position_us, state);
    }

    fn clear_pending_seek(&self, generation: u64) -> bool {
        let Some(pending) = self.pending_seek.get() else {
            return false;
        };
        if pending.generation != generation {
            return false;
        }
        self.pending_seek.set(None);
        true
    }

    fn handle_clock_lost(&self) {
        if !self.playback_state.get().is_playing() {
            return;
        }
        if let Err(error) = self.pipeline.set_state(gst::State::Paused) {
            eprintln!("could not pause playbin3 after clock loss: {error}");
            return;
        }
        if let Err(error) = self.pipeline.set_state(gst::State::Playing) {
            eprintln!("could not resume playbin3 after clock loss: {error}");
        }
    }

    fn handle_eos(&self) {
        let position_us = self
            .pipeline
            .query_position::<gst::ClockTime>()
            .map(clock_time_us)
            .unwrap_or_else(|| self.duration_us.get().max(self.position_us.get()));
        if let Err(error) = self.pipeline.set_state(gst::State::Paused) {
            eprintln!("could not pause playbin3 at EOS: {error}");
        }
        self.position_us.set(position_us);
        self.playback_state.set(PlaybackState::Paused);
        self.publish(position_us, PlaybackState::Paused, true);
    }

    fn handle_error(&self, error: &gst::message::Error) {
        eprintln!(
            "GStreamer error: {} (source: {:?}, debug: {:?})",
            error.error(),
            error.src().map(|source| source.path_string()),
            error.debug()
        );
        if let Err(state_error) = self.pipeline.set_state(gst::State::Null) {
            eprintln!("could not reset playbin3 after error: {state_error}");
        }
        self.pending_seek.set(None);
        self.playback_state.set(PlaybackState::Paused);
        if let Some(page) = self.page.upgrade() {
            page.set_seekable(false);
        }
        self.publish_discontinuity(self.position_us.get(), PlaybackState::Paused);
    }

    fn refresh_clock(&self) {
        self.refresh_clock_with(false);
    }

    fn refresh_clock_for_event(&self) {
        self.refresh_clock_with(true);
    }

    fn refresh_clock_with(&self, force: bool) {
        if self.shutting_down.get() {
            return;
        }
        let duration_changed = self.refresh_duration();
        let state = playback_state(self.pipeline.current_state());
        let state_changed = self.playback_state.replace(state) != state;
        let force = force || duration_changed || state_changed;
        if self.pending_seek.get().is_some() {
            self.publish_pending(state, force);
            return;
        }
        self.refresh_position();
        self.publish(self.position_us.get(), state, force);
    }

    fn publish_pending(&self, state: PlaybackState, force: bool) {
        if force {
            self.publish(self.position_us.get(), state, true);
        }
    }

    fn refresh_position(&self) {
        if let Some(position) = self.pipeline.query_position::<gst::ClockTime>() {
            self.position_us.set(clock_time_us(position));
        }
    }

    fn refresh_media_info(&self) {
        self.refresh_duration();
        self.refresh_seekability();
    }

    fn refresh_duration(&self) -> bool {
        let Some(duration) = self.pipeline.query_duration::<gst::ClockTime>() else {
            return false;
        };
        let duration_us = clock_time_us(duration);
        if self.duration_us.replace(duration_us) == duration_us {
            return false;
        }
        if let Some(page) = self.page.upgrade() {
            page.set_duration_us(duration_us);
            self.refresh_lyrics_document(&page, duration_us);
        }
        true
    }

    fn refresh_lyrics_document(&self, page: &LyricPlayerPage, duration_us: i64) {
        if duration_us <= 0 || self.lyrics_media_duration_us.get() == Some(duration_us) {
            return;
        }
        self.lyrics_media_duration_us.set(Some(duration_us));
        match self.lyrics.parse(Some(duration_us)) {
            Ok(document) => page.set_document(Some(Arc::new(document))),
            Err(error) => eprintln!("could not update lyrics for media duration: {error}"),
        }
    }

    fn refresh_seekability(&self) {
        let mut query = gst::query::Seeking::new(gst::Format::Time);
        let seekable = self.pipeline.query(&mut query) && query.result().0;
        if let Some(page) = self.page.upgrade() {
            page.set_seekable(seekable);
        }
    }

    fn publish_discontinuity(&self, position_us: i64, state: PlaybackState) {
        self.serial.set(self.serial.get().wrapping_add(1));
        self.publish(position_us, state, true);
    }

    fn publish(&self, position_us: i64, state: PlaybackState, force: bool) {
        let Some(page) = self.page.upgrade() else {
            return;
        };
        let mut last_second = self.last_published_second.get();
        if !mark_second_for_publication(&mut last_second, position_us, force) {
            return;
        }
        self.last_published_second.set(last_second);
        page.set_position_us(position_us);
        page.set_playing(state.is_playing());
        page.set_playback_anchor(PlaybackAnchor::new(
            LyricTime::from_micros(position_us),
            glib::monotonic_time(),
            1.0,
            state,
            self.serial.get(),
        ));
    }

    fn shutdown(&self) {
        if self.shutting_down.replace(true) {
            return;
        }
        self.pending_seek.set(None);
        if let Some(source) = self.tick_source.borrow_mut().take() {
            source.remove();
        }
        self.bus_watch.borrow_mut().take();
        if let Err(error) = self.pipeline.set_state(gst::State::Null) {
            eprintln!("could not set playbin3 to Null during shutdown: {error}");
        }
        self.playback_state.set(PlaybackState::Paused);
        if let Some(page) = self.page.upgrade() {
            page.set_playing(false);
        }
    }
}

impl Drop for GstController {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn playback_state(state: gst::State) -> PlaybackState {
    if state == gst::State::Playing {
        PlaybackState::Playing
    } else {
        PlaybackState::Paused
    }
}

fn clock_time_us(time: gst::ClockTime) -> i64 {
    i64::try_from(time.useconds()).unwrap_or(i64::MAX)
}

fn mark_second_for_publication(
    last_second: &mut Option<i64>, position_us: i64, force: bool,
) -> bool {
    let second = position_us.div_euclid(MICROS_PER_SECOND);
    if !force && *last_second == Some(second) {
        return false;
    }
    *last_second = Some(second);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_publication_waits_for_a_new_second() {
        let mut last_second = None;

        assert!(mark_second_for_publication(&mut last_second, 0, false));
        assert!(!mark_second_for_publication(
            &mut last_second,
            999_999,
            false
        ));
        assert!(mark_second_for_publication(
            &mut last_second,
            1_000_000,
            false
        ));
        assert!(!mark_second_for_publication(
            &mut last_second,
            1_999_999,
            false
        ));
        assert!(mark_second_for_publication(
            &mut last_second,
            1_999_999,
            true
        ));
    }

    #[test]
    fn media_duration_replaces_the_final_line_fallback() {
        let lyrics = LyricsSource {
            path: PathBuf::from("test.lrc"),
            source: Arc::from("[00:01.00]first\n[00:04.00]last"),
        };

        let document = lyrics.parse(Some(12_000_000)).unwrap();
        let final_line = document.tracks[0].lines.last().unwrap();

        assert_eq!(
            final_line.range.end,
            Some(LyricTime::from_micros(12_000_000))
        );
        assert_eq!(document.duration, Some(LyricTime::from_micros(12_000_000)));
    }
}
