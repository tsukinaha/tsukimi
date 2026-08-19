use glib::Object;
use gtk::{
    gdk::prelude::PaintableExt,
    glib,
    subclass::prelude::*,
};

use crate::{
    PlayParams,
    video::{
        backend::{
            TrackKind,
            TrackSelection,
        },
        mpv::contexted::ContextedMPV,
    },
};

use gtk::gdk;

mod imp {
    use crate::{
        BufferTransport,
        CapturedFrame,
        FrameCallbacks,
        MpvProxy,
        ShmMemoryFormat,
        SurfaceContentUpdate,
        SurfaceUpdate,
        create_mpv_proxy_with_upstream,
        video::{
            MutsumiMpvError,
            mpv::contexted::ContextedMPV,
        },
    };
    use std::{
        cell::{
            Cell,
            RefCell,
        },
        collections::HashSet,
        os::fd::AsRawFd,
        sync::OnceLock,
    };

    #[cfg(feature = "profiling")]
    use crate::video::mpv::proxy::profiling::{
        self,
        Stage,
    };

    use super::*;

    use glib::subclass::Signal;
    use gtk::{
        glib,
        prelude::*,
    };

    #[derive(Default)]
    pub struct MutsumiVideoSink {
        pub mpv: ContextedMPV,
        pub proxy: RefCell<Option<MpvProxy>>,
        pub texture: RefCell<Option<gdk::Texture>>,
        pub frame_callbacks: RefCell<Vec<FrameCallbacks>>,
        pub validated_pairs: RefCell<HashSet<(u64, u64, u32, u64)>>,
        pub validated_generation: Cell<Option<u64>>,
        #[cfg(feature = "profiling")]
        pub pending_snapshot_frame_id: Cell<Option<u64>>,
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

            let proxy = self.create_proxy();
            let frame_rx = proxy.frame_receiver();
            self.mpv.mpv.configure_proxy(proxy.config());
            self.proxy.replace(Some(proxy));

            glib::spawn_future_local(glib::clone!(
                #[weak]
                obj,
                async move {
                    while let Ok(update) = frame_rx.recv_async().await {
                        let SurfaceUpdate {
                            generation,
                            load_id,
                            surface_id,
                            content,
                            frame_callbacks,
                        } = update;

                        if generation != obj.mpv().mpv.current_generation()
                            || load_id != obj.mpv().mpv.current_load_id()
                        {
                            if let Some(callbacks) = frame_callbacks {
                                callbacks.done(0);
                            }
                            continue;
                        }

                        tracing::trace!(
                            generation,
                            load_id,
                            surface_id,
                            "received synthetic Wayland surface update"
                        );

                        match content {
                            SurfaceContentUpdate::Frame(CapturedFrame::Dmabuf(frame)) => {
                                #[cfg(feature = "profiling")]
                                let profile_frame_id = frame.profile_frame_id;
                                #[cfg(feature = "profiling")]
                                profiling::mark(profile_frame_id, Stage::TextureBuildStarted);

                                let fourcc = frame.format;
                                let modifier = frame.modifier;
                                let mut builder = gdk::DmabufTextureBuilder::new()
                                    .set_display(
                                        &gdk::Display::default()
                                            .expect("Could not connect to display"),
                                    )
                                    .set_width(frame.width)
                                    .set_height(frame.height)
                                    .set_fourcc(fourcc)
                                    .set_modifier(modifier)
                                    .set_n_planes(frame.planes.len() as u32);

                                for (index, plane) in frame.planes.iter().enumerate() {
                                    builder = unsafe {
                                        builder.set_fd(index as u32, plane.fd.as_raw_fd())
                                    }
                                    .set_offset(index as u32, plane.offset)
                                    .set_stride(index as u32, plane.stride);
                                }

                                match unsafe {
                                    builder.build_with_release_func(move || drop(frame))
                                } {
                                    Ok(texture) => {
                                        let validation_key =
                                            (generation, load_id, fourcc, modifier);
                                        if obj
                                            .imp()
                                            .validated_pairs
                                            .borrow_mut()
                                            .insert(validation_key)
                                        {
                                            // Force the first import through GDK's downloader. A
                                            // successful builder alone may defer the real import.
                                            let downloader = gdk::TextureDownloader::new(&texture);
                                            let _ = downloader.download_bytes();
                                            obj.imp().validated_generation.set(Some(generation));
                                        }

                                        #[cfg(feature = "profiling")]
                                        {
                                            profiling::mark(profile_frame_id, Stage::TextureBuilt);
                                            obj.imp()
                                                .pending_snapshot_frame_id
                                                .set(profile_frame_id);
                                        }
                                        obj.imp().texture.replace(Some(texture));
                                        obj.mpv().mpv.report_frame_imported(
                                            generation,
                                            load_id,
                                            BufferTransport::Dmabuf,
                                        );
                                    }
                                    Err(error) => {
                                        tracing::error!(
                                            generation,
                                            surface_id,
                                            fourcc,
                                            modifier,
                                            %error,
                                            "dmabuf import failed"
                                        );
                                        obj.mpv().mpv.report_dmabuf_import_failed(
                                            generation,
                                            load_id,
                                            fourcc,
                                            modifier,
                                            error.to_string(),
                                        );
                                    }
                                }
                            }
                            SurfaceContentUpdate::Frame(CapturedFrame::Shm(frame)) => {
                                let format = match frame.format {
                                    ShmMemoryFormat::Argb8888 => {
                                        gdk::MemoryFormat::B8g8r8a8Premultiplied
                                    }
                                    ShmMemoryFormat::Xrgb8888 => gdk::MemoryFormat::B8g8r8x8,
                                };
                                let bytes = glib::Bytes::from_owned(frame.data);
                                let texture = gdk::MemoryTexture::new(
                                    frame.width,
                                    frame.height,
                                    format,
                                    &bytes,
                                    frame.stride,
                                );
                                obj.imp().validated_generation.set(Some(generation));
                                obj.imp().texture.replace(Some(gdk::Texture::from(texture)));
                                obj.mpv().mpv.report_frame_imported(
                                    generation,
                                    load_id,
                                    BufferTransport::Shm,
                                );
                            }
                            SurfaceContentUpdate::Clear => {
                                // Keep the last valid frame visible while a replacement
                                // candidate is bootstrapping.
                                if obj.imp().validated_generation.get() == Some(generation) {
                                    #[cfg(feature = "profiling")]
                                    obj.imp().pending_snapshot_frame_id.set(None);
                                    obj.imp().texture.take();
                                }
                            }
                            SurfaceContentUpdate::Unchanged => {}
                        }

                        if let Some(frame_callbacks) = frame_callbacks {
                            obj.imp().frame_callbacks.borrow_mut().push(frame_callbacks);
                        }

                        obj.invalidate_contents();
                    }
                }
            ));
        }

        fn dispose(&self) {
            self.texture.take();
            self.frame_callbacks.take();
            self.mpv.shutdown();
            self.proxy.take();
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

                #[cfg(feature = "profiling")]
                profiling::mark(self.pending_snapshot_frame_id.take(), Stage::Snapshot);
            }

            let time_ms = (glib::monotonic_time() / 1_000) as u32;
            for frame_callbacks in self.frame_callbacks.take() {
                frame_callbacks.done(time_ms);
            }
        }
    }

    impl MutsumiVideoSink {
        fn create_proxy(&self) -> MpvProxy {
            let display = gdk::Display::default().expect("Could not connect to display");
            let formats = display.dmabuf_formats();

            // GTK < 4.24 has no memory-format mapping for 10-bit packed RGB
            // formats (AR30/XR30/AB30/XB30 = *RGB/*BGR 2101010), but the host
            // compositor may still advertise them in dmabuf_formats().
            // See https://gitlab.gnome.org/GNOME/gtk/-/issues/8148
            const PACKED_10BIT: &[u32] = &[
                0x30335241, // DRM_FORMAT_ARGB2101010 (AR30)
                0x30335258, // DRM_FORMAT_XRGB2101010 (XR30)
                0x30334241, // DRM_FORMAT_ABGR2101010 (AB30)
                0x30334258, // DRM_FORMAT_XBGR2101010 (XB30)
            ];
            let skip_packed_10bit = gtk::minor_version() < 24;

            let format_pairs: Vec<(u32, u64)> = (0..formats.n_formats())
                .map(|i| formats.format(i))
                .filter(|(fourcc, _)| !skip_packed_10bit || !PACKED_10BIT.contains(fourcc))
                .collect();

            let upstream_display = display
                .is::<gdk_wayland::WaylandDisplay>()
                .then(|| display.name().to_string());
            create_mpv_proxy_with_upstream(format_pairs, upstream_display)
        }

        pub fn throw_error(&self, code: MutsumiMpvError) {
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

    pub fn update_viewport(&self, width: i32, height: i32, scale: f64) {
        if let Some(proxy) = self.imp().proxy.borrow().as_ref() {
            proxy.update_viewport(width, height, scale);
        }
    }

    pub fn play(&self, source: &PlayParams) {
        let url = source.url().into_owned();
        let start_time = source.start_time;

        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                let mpv = obj.mpv();

                tracing::info!(target: "mutsumi::mpv", source = url, "loading media");
                mpv.load_video(&url);

                if let Some(start_time) = start_time {
                    mpv.set_start_time(start_time);
                }

                mpv.pause(false);
            }
        ));
    }

    // This would be useful for clearing the video area when stopping playback, to avoid showing the last frame.
    pub fn push_an_empty_texture(&self) {
        let bytes = glib::Bytes::from_static(&[0, 0, 0, 0]);

        let texture = gdk::MemoryTexture::new(1, 1, gdk::MemoryFormat::R8g8b8a8, &bytes, 4);

        let texture = gdk::Texture::from(texture);

        self.imp().texture.replace(Some(texture));
        self.invalidate_contents();
    }

    pub fn set_loop_playlist(&self, loop_: &str) {
        self.mpv().set_loop_playlist(loop_);
    }

    pub fn set_loop_file(&self, loop_: &str) {
        self.mpv().set_loop_file(loop_);
    }

    pub fn playlist_shuffle(&self) {
        self.mpv().playlist_shuffle();
    }

    pub fn playlist_unshuffle(&self) {
        self.mpv().playlist_unshuffle();
    }

    pub fn set_playlist(&self, urls: &[String]) {
        self.mpv().set_playlist(urls);
    }

    pub fn set_playlist_pos(&self, pos: i64) {
        self.mpv().set_playlist_pos(pos);
    }

    pub fn playlist_add(&self, url: &str, index: i64) {
        self.mpv().playlist_add(url, index);
    }

    pub fn playlist_remove(&self, index: i64) {
        self.mpv().playlist_remove(index);
    }

    pub fn playlist_move(&self, from: i64, to: i64) {
        self.mpv().playlist_move(from, to);
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

    pub fn set_start_time(&self, second: u64) {
        self.mpv().set_start_time(second);
    }

    pub fn set_start(&self, second: f64) {
        self.mpv().set_start(second);
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

    pub fn set_sub_justify(&self, value: &str) {
        self.mpv().mpv.set_property("sub-justify", value.to_owned());
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

    pub fn set_vo(&self, value: &str) {
        self.mpv().mpv.set_property("vo", value.to_owned());
    }

    pub fn set_property<V>(&self, property: &str, value: V)
    where
        V: Into<crate::video::MpvValue>,
    {
        self.mpv().mpv.set_property(property, value);
    }

    pub fn display_stats_toggle(&self) {
        self.mpv().display_stats_toggle();
    }

    pub fn add_sub(&self, url: &str) {
        self.mpv().add_sub(url);
    }

    pub fn set_position(&self, position: f64) {
        self.mpv().set_position(position);
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
