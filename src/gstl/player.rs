use gst::{glib, prelude::*};

use crate::{client::client::EMBY_CLIENT, ui::provider::core_song::CoreSong};

pub struct MusicPlayer {
    pipeline: gst::Element,
}

impl MusicPlayer {
    pub fn new() -> Self {
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
                match msg.view() {
                    gst::MessageView::Buffering(buffering) => {
                        let percent = buffering.percent();
                        if percent < 100 {
                            let _ = pipeline.set_state(gst::State::Paused);
                        } else {
                            let _ = pipeline.set_state(gst::State::Playing);
                        }
                    }
                    _ => (),
                }
            })
        });
        Self { pipeline }
    }

    pub fn _connect_about_to_finish<F>(&self, cb: F)
    where
        F: Fn(&[glib::Value]) -> Option<glib::Value> + Send + Sync + 'static,
    {
        self.pipeline.connect("about-to-finish", false, cb);
    }


    pub fn playing(&self) {
        let pipeline = &self.pipeline;
        pipeline
            .set_state(gst::State::Playing)
            .expect("Unable to set the pipeline to the `Playing` state");
    }

    pub fn play(&self, core_song: &CoreSong) {
        self.stop();
        let uri = EMBY_CLIENT.get_song_streaming_uri(&core_song.id());

        self.pipeline.set_property("uri", uri);
        self.playing();
    }

    pub fn stop(&self) {
        let pipeline = &self.pipeline;
        pipeline
            .set_state(gst::State::Null)
            .expect("Unable to set the pipeline to the `Null` state");
    }

    pub fn get_position(&self) -> gst::ClockTime {
        if self.pipeline.current_state() != gst::State::Playing {
            return gst::ClockTime::from_mseconds(0);
        }
        let pipeline = &self.pipeline;
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
        let pipeline = &self.pipeline;
        pipeline
            .set_state(gst::State::Paused)
            .expect("Unable to set the pipeline to the `Paused` state");
    }

    pub fn unpause(&self) {
        let pipeline = &self.pipeline;
        pipeline
            .set_state(gst::State::Playing)
            .expect("Unable to set the pipeline to the `Playing` state");
    }

    pub fn state(&self) -> gst::State {
        let pipeline = &self.pipeline;
        pipeline.current_state()
    }

    pub fn set_position(&self, position: f64) {
        let pipeline = &self.pipeline;
        let position = gst::ClockTime::from_seconds(position as u64);
        pipeline
            .seek_simple(gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT, position)
            .expect("Seek failed");
    }
}
