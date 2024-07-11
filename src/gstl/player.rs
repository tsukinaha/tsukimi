use gst::prelude::*;
use gtk::glib;
use gtk::glib::prelude::*;
use gtk::glib::subclass::prelude::*;
use crate::{client::client::EMBY_CLIENT, ui::provider::core_song::CoreSong};

pub mod imp {
    use std::cell::RefCell;
    use once_cell::sync::*;
    use gtk::{glib, prelude::ListModelExt};
    use gtk::glib::Properties;
    use std::sync::OnceLock;

    use super::*;
    use glib::subclass::{InitializingObject, Signal};
    use crate::ui::widgets::song_widget::State;

    use super::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::MusicPlayer)]
    pub struct MusicPlayer {
        pipeline: OnceCell<gst::Element>,
        #[property(get, set, nullable)]
        pub active_core_song: RefCell<Option<CoreSong>>,
        #[property(get, set, nullable)]
        pub active_model: RefCell<Option<gtk::gio::ListStore>>,
    }

    #[glib::derived_properties]
    impl ObjectImpl for MusicPlayer {
        fn constructed(&self) {
            self.parent_constructed();

            // Initialize GStreamer
            gst::init().unwrap();

            // Build the pipeline
            let pipeline = gst::ElementFactory::make("playbin").build().unwrap();
            // Start playing
            let bus = pipeline.bus().unwrap();
            bus.add_signal_watch();
            
            bus.connect_message(Some("eos"), {
                move |_bus, _msg| {
                    // Hard Reset
                    // Not Implemented
                }
            });
            bus.connect_message(Some("buffering"), {
                glib::clone!(@weak pipeline => move |_bus, msg| {
                    if let gst::MessageView::Buffering(buffering) = msg.view() {
                        let percent = buffering.percent();
                        if percent < 100 {
                            let _ = pipeline.set_state(gst::State::Paused);
                        } else {
                            let _ = pipeline.set_state(gst::State::Playing);
                        }
                    }
                })
            });
            self.connect_about_to_finish(
                glib::clone!(@strong self as imp => move |_| {
                    imp.next_song();
                    None
                }),
            );
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![Signal::builder("song-changed")
                    .param_types([CoreSong::static_type()])
                    .build()]
            })
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicPlayer {
        const NAME: &'static str = "MusicPlayer";
        type Type = super::MusicPlayer;
    }

    impl MusicPlayer {
        fn pipeline(&self) -> &gst::Element {
            self.pipeline.get().unwrap()
        }

        pub fn connect_about_to_finish<F>(&self, cb: F)
        where
            F: Fn(&[glib::Value]) -> Option<glib::Value> + Send + Sync + 'static,
        {
            self.pipeline().connect("about-to-finish", false, cb);
        }

        pub fn playing(&self) {
            self.pipeline()
                .set_state(gst::State::Playing)
                .expect("Unable to set the pipeline to the `Playing` state");
        }

        pub fn play(&self, core_song: &CoreSong) {
            if let Some(core_song_old) = self.active_core_song.borrow().as_ref() {
                if core_song_old != core_song {
                    core_song_old.set_state(State::Played);
                }
            }
            
            self.stop();
            let uri = EMBY_CLIENT.get_song_streaming_uri(&core_song.id());

            self.pipeline().set_property("uri", uri);
            self.playing();
        }

        pub fn stop(&self) {
            self.pipeline()
                .set_state(gst::State::Null)
                .expect("Unable to set the pipeline to the `Null` state");
        }

        pub fn next_song(&self) {
            let model = self.active_model.borrow();
            let Some(model) = model.as_ref() else {
                return;
            };
            let Some(core_song_position) = self.core_song_position() else {
                return;
            };
            let next_position = core_song_position + 1;
            if next_position >= model.n_items() {
                return;
            }
            let row = model.item(next_position).unwrap();
            let core_song = row.downcast_ref::<CoreSong>().unwrap();
            self.play(core_song);
        }

        pub fn core_song_position(&self) -> Option<u32> {
            let core_song = self.active_core_song.borrow();
            let Some(core_song) = core_song.as_ref() else {
                return None;
            };
            let model = self.active_model.borrow();
            let Some(model) = model.as_ref() else {
                return None;
            };
            model.find(core_song)
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
            self.pipeline()
                .seek_simple(gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT, position)
                .expect("Seek failed");
        }

        pub fn load_model(&self, active_model: gtk::gio::ListStore, active_core_song: CoreSong) {
            self.active_model.replace(Some(active_model));
            self.active_core_song.replace(Some(active_core_song));
            self.prepre_play();
        }

        pub fn prepre_play(&self) {
            let object = self.active_core_song.borrow();
            let Some(active_core_song) = object.as_ref() else {
                return;
            };
            self.play(active_core_song);
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