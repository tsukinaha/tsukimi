use glib::Object;
use gtk::{glib, subclass::prelude::*};
use tracing::info;

use crate::video::{
    backend::{TrackKind, TrackSelection},
    mpv::contexted::ContextedMPV,
};

use gtk::gdk;

mod imp {
    use crate::{
        FRAME_CHANNEL, create_mpv_proxy,
        video::{MutsumiMpvError, mpv::contexted::ContextedMPV},
    };
    use std::{
        cell::RefCell,
        os::fd::{AsRawFd, IntoRawFd},
        sync::OnceLock,
    };

    use super::*;

    use glib::subclass::Signal;
    use gtk::{glib, prelude::*};

    #[derive(Default)]
    pub struct MutsumiVideoSink {
        pub mpv: ContextedMPV,
        pub texture: RefCell<Option<gdk::Texture>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MutsumiVideoSink {
        const NAME: &'static str = "MutsumiVideoSink";
        type Type = super::MutsumiVideoSink;
        type Interfaces = (gdk::Paintable,);
    }

    impl ObjectImpl for MutsumiVideoSink {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            self.setup_mpv();

            glib::spawn_future_local(glib::clone!(
                #[weak]
                obj,
                async move {
                    while let Ok(frame) = FRAME_CHANNEL.rx.recv_async().await {
                        let mut builder = gdk::DmabufTextureBuilder::new()
                            .set_display(&gdk::Display::default().unwrap())
                            .set_width(frame.width)
                            .set_height(frame.height)
                            .set_fourcc(frame.format)
                            .set_modifier(frame.modifier)
                            .set_n_planes(frame.planes.len() as u32);

                        for (i, plane) in frame.planes.iter().enumerate() {
                            builder = unsafe { builder.set_fd(i as u32, plane.fd.as_raw_fd()) }
                                .set_offset(i as u32, plane.offset)
                                .set_stride(i as u32, plane.stride);
                        }

                        match unsafe { builder.build_with_release_func(move || drop(frame)) } {
                            Ok(texture) => {
                                let size_changed =
                                    obj.imp().texture.borrow().as_ref().is_none_or(|old| {
                                        (old.width(), old.height())
                                            != (texture.width(), texture.height())
                                    });
                                obj.imp().texture.replace(Some(texture));
                                if size_changed {
                                    obj.invalidate_size();
                                }
                                obj.invalidate_contents();
                            }
                            Err(e) => {
                                tracing::error!("dmabuf build failed: {e}");
                            }
                        }
                    }
                }
            ));
        }

        fn dispose(&self) {
            self.mpv.shutdown();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("mutsumi-error")
                        .param_types([glib::Type::I32])
                        .build(),
                ]
            })
        }
    }

    impl PaintableImpl for MutsumiVideoSink {
        fn intrinsic_width(&self) -> i32 {
            self.texture.borrow().as_ref().map_or(0, |t| t.width())
        }

        fn intrinsic_height(&self) -> i32 {
            self.texture.borrow().as_ref().map_or(0, |t| t.height())
        }

        fn snapshot(&self, snapshot: &gdk::Snapshot, width: f64, height: f64) {
            if let Some(texture) = self.texture.borrow().as_ref() {
                snapshot.append_texture(
                    texture,
                    &gtk::graphene::Rect::new(0.0, 0.0, width as f32, height as f32),
                );
            }
        }
    }

    impl MutsumiVideoSink {
        fn setup_mpv(&self) {
            let socket = create_mpv_proxy().expect("Failed to create Wayland proxy");

            unsafe { std::env::set_var("WAYLAND_SOCKET", socket.into_raw_fd().to_string()) };

            self.mpv.mpv.set_property("vo", "gpu-next".to_owned());
        }

        fn throw_error(&self, code: MutsumiMpvError) {
            self.obj().emit_by_name::<()>("mutsumi-error", &[&code]);
        }
    }
}

glib::wrapper! {
    pub struct MutsumiVideoSink(ObjectSubclass<imp::MutsumiVideoSink>)
        @implements gdk::Paintable;
}

impl Default for MutsumiVideoSink {
    fn default() -> Self {
        Self::new()
    }
}

impl MutsumiVideoSink {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn mpv(&self) -> &ContextedMPV {
        &self.imp().mpv
    }

    pub fn play(&self, url: &str, percentage: f64) {
        let url = url.to_owned();

        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                let mpv = obj.mpv();

                info!("Now Playing: {}", url);
                mpv.load_video(&url);

                mpv.set_start(percentage);
                mpv.pause(false);
            }
        ));
    }

    pub fn press_key(&self, key: u32, state: gtk::gdk::ModifierType) {
        self.mpv().press_key(key, state)
    }

    pub fn release_key(&self, key: u32, state: gtk::gdk::ModifierType) {
        self.mpv().release_key(key, state)
    }

    pub fn volume_scroll(&self, value: i64) {
        self.mpv().volume_scroll(value)
    }

    pub fn set_slang(&self, value: String) {
        self.mpv().set_slang(value)
    }

    pub fn shutdown(&self) {
        self.mpv().shutdown();
    }

    pub fn stop(&self) {
        self.mpv().stop();
    }

    pub fn load_video(&self, url: &str) {
        self.mpv().load_video(url);
    }

    pub fn pause(&self, pause: bool) {
        self.mpv().pause(pause);
    }

    pub fn command_pause(&self) {
        self.mpv().command_pause();
    }

    pub fn set_percent_position(&self, value: f64) {
        self.mpv().set_percent_position(value);
    }

    pub fn set_start(&self, percentage: f64) {
        self.mpv().set_start(percentage);
    }

    pub fn set_aid(&self, value: TrackSelection) {
        self.mpv().set_aid(value);
    }

    pub fn set_sid(&self, value: TrackSelection) {
        self.mpv().set_sid(value);
    }

    pub fn disable_aid(&self) {
        self.mpv().set_aid(TrackSelection::None);
    }

    pub fn disable_sid(&self) {
        self.mpv().set_sid(TrackSelection::None);
    }

    pub fn set_brightness(&self, value: f64) {
        self.mpv().mpv.set_property("brightness", value);
    }

    pub fn set_contrast(&self, value: f64) {
        self.mpv().mpv.set_property("contrast", value);
    }

    pub fn set_gamma(&self, value: f64) {
        self.mpv().mpv.set_property("gamma", value);
    }

    pub fn set_hue(&self, value: f64) {
        self.mpv().mpv.set_property("hue", value);
    }

    pub fn set_saturation(&self, value: f64) {
        self.mpv().mpv.set_property("saturation", value);
    }

    pub fn set_sub_pos(&self, value: f64) {
        self.mpv().mpv.set_property("sub-pos", value);
    }

    pub fn set_sub_font_size(&self, value: f64) {
        self.mpv().mpv.set_property("sub-font-size", value);
    }

    pub fn set_sub_scale(&self, value: f64) {
        self.mpv().mpv.set_property("sub-scale", value);
    }

    pub fn set_sub_speed(&self, value: f64) {
        self.mpv().mpv.set_property("sub-speed", value);
    }

    pub fn set_sub_delay(&self, value: f64) {
        self.mpv().mpv.set_property("sub-delay", value);
    }

    pub fn set_sub_bold(&self, value: bool) {
        self.mpv().mpv.set_property("sub-bold", value);
    }

    pub fn set_sub_italic(&self, value: bool) {
        self.mpv().mpv.set_property("sub-italic", value);
    }

    pub fn set_sub_font(&self, value: &str) {
        self.mpv().mpv.set_property("sub-font", value.to_owned());
    }

    pub fn set_sub_color(&self, value: &str) {
        self.mpv().mpv.set_property("sub-color", value.to_owned());
    }

    pub fn set_sub_border_color(&self, value: &str) {
        self.mpv()
            .mpv
            .set_property("sub-border-color", value.to_owned());
    }

    pub fn set_sub_back_color(&self, value: &str) {
        self.mpv()
            .mpv
            .set_property("sub-back-color", value.to_owned());
    }

    pub fn set_sub_border_style(&self, value: &str) {
        self.mpv()
            .mpv
            .set_property("sub-border-style", value.to_owned());
    }

    pub fn set_sub_border_size(&self, value: f64) {
        self.mpv().mpv.set_property("sub-border-size", value);
    }

    pub fn set_sub_shadow_offset(&self, value: f64) {
        self.mpv().mpv.set_property("sub-shadow-offset", value);
    }

    pub fn set_audio_delay(&self, value: f64) {
        self.mpv().mpv.set_property("audio-delay", value);
    }

    pub fn set_audio_channels(&self, value: &str) {
        self.mpv()
            .mpv
            .set_property("audio-channels", value.to_owned());
    }

    pub fn set_audio_pan(&self, value: &str) {
        self.mpv().mpv.set_property("af", value.to_owned());
    }

    pub fn clear_audio_pan(&self) {
        self.mpv().mpv.set_property("af", String::new());
    }

    pub fn set_scale(&self, value: &str) {
        self.mpv().mpv.set_property("scale", value.to_owned());
    }

    pub fn set_deband(&self, value: bool) {
        self.mpv().mpv.set_property("deband", value);
    }

    pub fn set_deband_iterations(&self, value: i64) {
        self.mpv().mpv.set_property("deband-iterations", value);
    }

    pub fn set_deband_threshold(&self, value: i64) {
        self.mpv().mpv.set_property("deband-threshold", value);
    }

    pub fn set_deband_range(&self, value: i64) {
        self.mpv().mpv.set_property("deband-range", value);
    }

    pub fn set_deband_grain(&self, value: i64) {
        self.mpv().mpv.set_property("deband-grain", value);
    }

    pub fn set_deinterlace(&self, value: bool) {
        self.mpv().mpv.set_property("deinterlace", value);
    }

    pub fn set_hwdec(&self, value: &str) {
        self.mpv().mpv.set_property("hwdec", value.to_owned());
    }

    pub fn set_panscan(&self, value: f64) {
        self.mpv().mpv.set_property("panscan", value);
    }

    pub fn set_stretch_image_subs_to_screen(&self, value: bool) {
        self.mpv()
            .mpv
            .set_property("stretch-image-subs-to-screen", value);
    }

    pub fn set_demuxer_max_bytes(&self, value: &str) {
        self.mpv()
            .mpv
            .set_property("demuxer-max-bytes", value.to_owned());
    }

    pub fn set_cache_secs(&self, value: f64) {
        self.mpv().mpv.set_property("cache-secs", value);
    }

    pub fn display_stats_toggle(&self) {
        self.mpv().display_stats_toggle();
    }

    pub fn add_sub(&self, url: &str) {
        self.mpv().add_sub(url);
    }

    pub fn set_position(&self, position: f64) {
        self.mpv().set_percent_position(position);
    }

    pub fn set_volume(&self, volume: i64) {
        self.mpv().set_volume(volume);
    }

    pub fn seek_forward(&self, seconds: i64) {
        self.mpv().seek_forward(seconds);
    }

    pub fn seek_backward(&self, seconds: i64) {
        self.mpv().seek_backward(seconds);
    }

    pub fn set_speed(&self, speed: f64) {
        self.mpv().set_speed(speed);
    }

    pub fn set_keep_aspect_ratio(&self, value: bool) {
        self.mpv().mpv.set_property("keepaspect", value);
    }

    pub async fn position(&self) -> f64 {
        self.mpv().position().await
    }

    pub async fn paused(&self) -> bool {
        self.mpv().paused().await
    }

    pub async fn duration(&self) -> f64 {
        self.mpv().duration().await
    }

    pub async fn get_track_id(&self, kind: TrackKind) -> i64 {
        let type_ = match kind {
            TrackKind::Video => "vid",
            TrackKind::Audio => "aid",
            TrackKind::Subtitle => "sid",
        };

        self.mpv().get_track_id(type_).await
    }
}
