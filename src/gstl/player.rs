use crate::{client::client::EMBY_CLIENT, ui::provider::core_song::CoreSong};
use gst::prelude::*;
use gtk::glib;

pub mod imp {
    use async_channel::{Receiver, Sender};
    use gtk::glib::Properties;
    use gtk::prelude::ObjectExt;
    use gtk::{prelude::*, subclass::prelude::*};
    use once_cell::sync::*;
    use std::cell::{Cell, RefCell};
    use std::sync::OnceLock;
    use tracing::debug;
    use anyhow::Result;

    use super::*;
    use crate::ui::widgets::song_widget::State;
    use glib::subclass::Signal;
    use gtk::glib;

    #[derive(Default, Hash, Eq, PartialEq, Clone, Copy, glib::Enum, Debug)]
    #[repr(u32)]
    #[enum_type(name = "ListRepeatMode")]
    pub enum ListRepeatMode {
        #[default]
        None,
        Repeat,
        RepeatOne,
    }

    impl ListRepeatMode {
        pub fn from_string(string: &str) -> Self {
            match string {
                "none" => ListRepeatMode::None,
                "repeat" => ListRepeatMode::Repeat,
                "repeat-one" => ListRepeatMode::RepeatOne,
                _ => ListRepeatMode::None,
            }
        }

        pub fn to_string(self) -> &'static str {
            match self {
                ListRepeatMode::None => "none",
                ListRepeatMode::Repeat => "repeat",
                ListRepeatMode::RepeatOne => "repeat-one",
            }
        }
    }

    struct AboutToFinish {
        tx: Sender<bool>,
        rx: Receiver<bool>,
    }

    static ABOUT_TO_FINISH: Lazy<AboutToFinish> = Lazy::new(|| {
        let (tx, rx) = async_channel::bounded::<bool>(1);

        AboutToFinish { tx, rx }
    });

    struct StreamStart {
        tx: Sender<bool>,
        rx: Receiver<bool>,
    }

    static STREAM_START: Lazy<StreamStart> = Lazy::new(|| {
        let (tx, rx) = async_channel::bounded::<bool>(1);

        StreamStart { tx, rx }
    });

    struct Eos {
        tx: Sender<bool>,
        rx: Receiver<bool>,
    }

    static EOS: Lazy<Eos> = Lazy::new(|| {
        let (tx, rx) = async_channel::bounded::<bool>(1);

        Eos { tx, rx }
    });

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::MusicPlayer)]
    pub struct MusicPlayer {
        pipeline: OnceCell<gst::Element>,
        #[property(get, set, nullable)]
        pub active_core_song: RefCell<Option<CoreSong>>,
        #[property(get, set, nullable)]
        pub active_model: RefCell<Option<gtk::gio::ListStore>>,
        #[property(get, set, builder(ListRepeatMode::default()))]
        pub repeat_mode: Cell<ListRepeatMode>,
        #[property(get, set, default_value = false)]
        pub gapless: RefCell<bool>,
    }

    #[glib::derived_properties]
    impl ObjectImpl for MusicPlayer {
        fn constructed(&self) {
            self.parent_constructed();

            // Initialize GStreamer
            gst::init().unwrap();

            // Build the pipeline
            let pipeline = gst::ElementFactory::make("playbin3").build().unwrap();
            // Start playing
            let bus = pipeline.bus().unwrap();
            bus.add_signal_watch();

            bus.connect_message(Some("eos"), {
                move |_, _| {
                    let _ = EOS.tx.send_blocking(true);
                }
            });
            bus.connect_message(Some("buffering"), {
                glib::clone!(
                    #[strong]
                    pipeline,
                    move |_bus, msg| {
                        if let gst::MessageView::Buffering(buffering) = msg.view() {
                            let percent = buffering.percent();
                            if percent < 100 {
                                let _ = pipeline.set_state(gst::State::Paused);
                            } else {
                                let _ = pipeline.set_state(gst::State::Playing);
                            }
                        }
                    }
                )
            });

            self.pipeline.set(pipeline).unwrap();

            self.connect_about_to_finish(move |_| {
                let _ = ABOUT_TO_FINISH.tx.send_blocking(true);
                None
            });

            self.connect_stream_start(move |_, _| {
                let _ = STREAM_START.tx.send_blocking(true);
            });

            glib::spawn_future_local(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                async move {
                    while let Ok(true) = ABOUT_TO_FINISH.rx.recv().await {
                        if let Some(core_song) = imp.next_song() {
                            imp.add_song(&core_song);
                            imp.obj().set_gapless(true);
                        }
                    }
                }
            ));

            glib::spawn_future_local(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                async move {
                    while let Ok(true) = STREAM_START.rx.recv().await {
                        let obj = imp.obj();
                        if obj.gapless() {
                            let _ = imp.playlist_next();
                        }
                        obj.set_gapless(false);
                        obj.emit_by_name::<()>("stream-start", &[]);
                    }
                }
            ));

            glib::spawn_future_local(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                async move {
                    while let Ok(true) = EOS.rx.recv().await {
                        let obj = imp.obj();
                        if imp.playlist_next().is_err() {
                            return;
                        };
                        imp.stop();
                        if obj.gapless() {
                            if let Some(core_song) = imp.obj().active_core_song() {
                                imp.play(&core_song);
                            };
                        }
                        obj.set_gapless(false);
                    }
                }
            ));
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("stream-start").build()])
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicPlayer {
        const NAME: &'static str = "MusicPlayer";
        type Type = super::MusicPlayer;
    }

    impl WidgetImpl for MusicPlayer {}

    impl MusicPlayer {
        fn pipeline(&self) -> &gst::Element {
            self.pipeline.get().unwrap()
        }

        pub fn connect_about_to_finish<F>(&self, cb: F)
        where
            F: Fn(&[gst::glib::Value]) -> Option<gst::glib::Value> + Send + Sync + 'static,
        {
            gst::prelude::ObjectExt::connect(self.pipeline(), "about-to-finish", false, cb);
        }

        pub fn connect_stream_start<F>(&self, cb: F)
        where
            F: Fn(&gst::Bus, &gst::Message) + Send + Sync + 'static,
        {
            self.pipeline()
                .bus()
                .unwrap()
                .connect_message(Some("stream-start"), cb);
        }

        pub fn playing(&self) {
            self.pipeline()
                .set_state(gst::State::Playing)
                .expect("Unable to set the pipeline to the `Playing` state");
        }

        pub fn play(&self, core_song: &CoreSong) {
            core_song.set_state(State::Playing);
            if let Some(core_song_old) = self.active_core_song.borrow().as_ref() {
                if core_song_old != core_song {
                    core_song_old.set_state(State::Played);
                }
            }

            self.stop();
            let uri = EMBY_CLIENT.get_song_streaming_uri(&core_song.id());

            gst::prelude::ObjectExt::set_property(self.pipeline(), "uri", uri);
            self.playing();
        }

        pub fn add_song(&self, core_song: &CoreSong) {
            let uri = EMBY_CLIENT.get_song_streaming_uri(&core_song.id());
            gst::prelude::ObjectExt::set_property(self.pipeline(), "uri", uri);
        }

        pub fn playlist_next(&self) -> Result<()> {
            if let Some(core_song) = self.active_core_song.borrow().as_ref() {
                core_song.set_state(State::Played);
            };
            if let Some(core_song) = self.next_song() {
                core_song.set_state(State::Playing);
                debug!("Next Song: {}", core_song.name());
                self.obj().set_active_core_song(Some(core_song));
                Ok(())
            } else {
                Err(anyhow::Error::msg("No next song"))
            }
        }

        pub fn stop(&self) {
            self.pipeline()
                .set_state(gst::State::Null)
                .expect("Unable to set the pipeline to the `Null` state");
        }

        pub fn next_song(&self) -> Option<CoreSong> {
            let obj = self.obj();
            if obj.repeat_mode() == ListRepeatMode::RepeatOne {
                return obj.active_core_song();
            }
            let model = self.active_model.borrow();
            let model = model.as_ref()?;
            let core_song_position = self.core_song_position()?;
            debug!("Core Song Position: {}", core_song_position);
            let next_position = core_song_position + 1;
            if next_position >= model.n_items() {
                if obj.repeat_mode() == ListRepeatMode::Repeat {
                    return model.item(0)?.downcast::<CoreSong>().ok();
                }
                return None;
            }
            let row = model.item(next_position)?;
            row.downcast::<CoreSong>().ok()
        }

        pub fn prev_song(&self) -> Option<CoreSong> {
            let obj = self.obj();
            if obj.repeat_mode() == ListRepeatMode::RepeatOne {
                return obj.active_core_song();
            }
            let model = self.active_model.borrow();
            let model = model.as_ref()?;
            let core_song_position = self.core_song_position()?;
            if core_song_position == 0 {
                if obj.repeat_mode() == ListRepeatMode::Repeat {
                    return model.item(model.n_items() - 1)?.downcast::<CoreSong>().ok();
                }
                return None;
            }
            let prev_position = core_song_position - 1;
            let row = model.item(prev_position)?;
            row.downcast::<CoreSong>().ok()
        }

        pub fn core_song_position(&self) -> Option<u32> {
            let core_song = self.obj().active_core_song()?;
            let model = self.active_model.borrow();
            let model = model.as_ref()?;
            model.find(&core_song)
        }

        pub fn get_position(&self) -> gst::ClockTime {
            if self.pipeline().current_state() != gst::State::Playing {
                return gst::ClockTime::from_mseconds(0);
            }
            let pipeline = &self.pipeline();
            let position = pipeline.query_position::<gst::ClockTime>();
            if let Some(position) = position {
                position
            } else {
                gst::ClockTime::from_seconds(0)
            }
        }

        pub fn position(&self) -> f64 {
            self.get_position().mseconds() as f64 / 1000.0
        }

        pub fn pause(&self) {
            self.pipeline()
                .set_state(gst::State::Paused)
                .expect("Unable to set the pipeline to the `Paused` state");
        }

        pub fn unpause(&self) {
            self.pipeline()
                .set_state(gst::State::Playing)
                .expect("Unable to set the pipeline to the `Playing` state");
        }

        pub fn state(&self) -> gst::State {
            self.pipeline().current_state()
        }

        pub fn set_position(&self, position: f64) {
            let position = gst::ClockTime::from_seconds(position as u64);
            if let Err(e) = self.pipeline()
                .seek_simple(gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT, position)
            {
                tracing::warn!("Failed to seek: {}", e);
            }
        }

        pub fn load_model(&self, active_model: gtk::gio::ListStore, active_core_song: CoreSong) {
            if let Some(core_song) = self.active_core_song.borrow().as_ref() {
                core_song.set_state(State::Played);
            };
            self.active_model.replace(Some(active_model));
            self.active_core_song.replace(Some(active_core_song));
            self.prepre_play();
        }

        pub fn prepre_play(&self) {
            let Some(active_core_song) = self.obj().active_core_song() else {
                return;
            };
            self.play(&active_core_song);
        }

        pub fn next(&self) {
            let _ = self.playlist_next();
            self.prepre_play();
        }

        pub fn prev(&self) {
            self.playlist_prev();
            self.prepre_play();
        }

        pub fn playlist_prev(&self) {
            if let Some(core_song) = self.active_core_song.borrow().as_ref() {
                core_song.set_state(State::Played);
            };
            if let Some(core_song) = self.prev_song() {
                core_song.set_state(State::Playing);
                debug!("Prev Song: {}", core_song.name());
                self.active_core_song.replace(Some(core_song));
            }
        }
    }
}

glib::wrapper! {
    pub struct MusicPlayer(ObjectSubclass<imp::MusicPlayer>);
}

impl Default for MusicPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl MusicPlayer {
    pub fn new() -> MusicPlayer {
        glib::Object::builder().build()
    }
}
