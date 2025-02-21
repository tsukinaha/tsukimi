use std::{
    io::{
        BufRead,
        BufReader,
        Cursor,
        Seek,
    },
    time::Duration,
};

use gtk::{
    gdk,
    gio,
    glib,
    graphene,
    prelude::*,
    subclass::prelude::*,
};
use image::{
    AnimationDecoder,
    DynamicImage,
    ImageFormat,
    codecs::{
        gif::GifDecoder,
        png::PngDecoder,
        webp::WebPDecoder,
    },
    flat::SampleLayout,
};
use tracing::error;

/// A single frame of an animation.
pub struct Frame {
    pub texture: gdk::Texture,
    pub duration: Duration,
}

impl From<image::Frame> for Frame {
    fn from(f: image::Frame) -> Self {
        let mut duration = Duration::from(f.delay());

        // The convention is to use 100 milliseconds duration if it is defined as 0.
        if duration.is_zero() {
            duration = Duration::from_millis(100);
        }

        let sample = f.into_buffer().into_flat_samples();
        let texture = texture_from_data(
            &sample.samples,
            sample.layout,
            gdk::MemoryFormat::R8g8b8a8,
            image::ColorType::Rgba8.bytes_per_pixel(),
        );

        Frame {
            texture: texture.upcast(),
            duration,
        }
    }
}

mod imp {
    use std::{
        cell::{
            Cell,
            RefCell,
        },
        marker::PhantomData,
    };

    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::ImagePaintable)]
    pub struct ImagePaintable {
        /// The frames of the animation, if any.
        pub frames: RefCell<Option<Vec<Frame>>>,
        /// The image if this is not an animation, otherwise this is the next
        /// frame to display.
        pub frame: RefCell<Option<gdk::Texture>>,
        /// The current index in the animation.
        pub current_idx: Cell<usize>,
        /// The source ID of the timeout to load the next frame, if any.
        pub timeout_source_id: RefCell<Option<glib::SourceId>>,
        /// Whether this image is an animation.
        #[property(get = Self::is_animation)]
        pub is_animation: PhantomData<bool>,
        /// The width of this image.
        #[property(get = Self::intrinsic_width, default = -1)]
        pub width: PhantomData<i32>,
        /// The height of this image.
        #[property(get = Self::intrinsic_height, default = -1)]
        pub height: PhantomData<i32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImagePaintable {
        const NAME: &'static str = "ImagePaintable";
        type Type = super::ImagePaintable;
        type Interfaces = (gdk::Paintable,);
    }

    #[glib::derived_properties]
    impl ObjectImpl for ImagePaintable {}

    impl PaintableImpl for ImagePaintable {
        fn intrinsic_height(&self) -> i32 {
            self.frame
                .borrow()
                .as_ref()
                .map(|texture| texture.height())
                .unwrap_or(-1)
        }

        fn intrinsic_width(&self) -> i32 {
            self.frame
                .borrow()
                .as_ref()
                .map(|texture| texture.width())
                .unwrap_or(-1)
        }

        fn snapshot(&self, snapshot: &gdk::Snapshot, width: f64, height: f64) {
            if let Some(texture) = &*self.frame.borrow() {
                texture.snapshot(snapshot, width, height);
            } else {
                let snapshot = snapshot.downcast_ref::<gtk::Snapshot>().unwrap();
                snapshot.append_color(
                    &gdk::RGBA::BLACK,
                    &graphene::Rect::new(0f32, 0f32, width as f32, height as f32),
                );
            }
        }

        fn flags(&self) -> gdk::PaintableFlags {
            if self.obj().is_animation() {
                gdk::PaintableFlags::SIZE
            } else {
                gdk::PaintableFlags::SIZE | gdk::PaintableFlags::CONTENTS
            }
        }

        fn current_image(&self) -> gdk::Paintable {
            self.frame
                .borrow()
                .clone()
                .map(|frame| frame.upcast())
                .or_else(|| {
                    let snapshot = gtk::Snapshot::new();
                    self.obj().snapshot(&snapshot, 1.0, 1.0);

                    snapshot.to_paintable(None)
                })
                .expect("there should be a fallback paintable")
        }
    }

    impl ImagePaintable {
        /// Whether this image is an animation.
        fn is_animation(&self) -> bool {
            self.frames.borrow().is_some()
        }
    }
}

glib::wrapper! {
    /// A paintable that loads images with the `image` crate.
    ///
    /// It handles more image types than GDK-Pixbuf and can also handle
    /// animations from GIF and APNG files.
    pub struct ImagePaintable(ObjectSubclass<imp::ImagePaintable>)
        @implements gdk::Paintable;
}

impl ImagePaintable {
    /// Load an image from the given reader in the optional format.
    ///
    /// The actual format will try to be guessed from the content.
    pub fn new<R: BufRead + Seek>(
        reader: R, format: Option<ImageFormat>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let obj = glib::Object::new::<Self>();

        let mut reader = image::ImageReader::new(reader);

        if let Some(format) = format {
            reader.set_format(format);
        }

        let reader = reader.with_guessed_format()?;

        obj.load_inner(reader)?;

        Ok(obj)
    }

    /// Load an image or animation from the given reader.
    fn load_inner<R: BufRead + Seek>(
        &self, reader: image::ImageReader<R>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let imp = self.imp();
        let format = reader.format().ok_or("Could not detect image format")?;

        let read = reader.into_inner();

        // Handle animations.
        match format {
            image::ImageFormat::Gif => {
                let decoder = GifDecoder::new(read)?;

                let frames = decoder
                    .into_frames()
                    .collect_frames()?
                    .into_iter()
                    .map(Frame::from)
                    .collect::<Vec<_>>();

                if frames.len() == 1 {
                    if let Some(frame) = frames.into_iter().next() {
                        imp.frame.replace(Some(frame.texture));
                    }
                } else {
                    imp.frames.replace(Some(frames));
                    self.update_frame();
                }
            }
            image::ImageFormat::Png => {
                let decoder = PngDecoder::new(read)?;

                if decoder.is_apng().unwrap_or_default() {
                    let decoder = decoder.apng()?;
                    let frames = decoder
                        .into_frames()
                        .collect_frames()?
                        .into_iter()
                        .map(Frame::from)
                        .collect::<Vec<_>>();
                    imp.frames.replace(Some(frames));
                    self.update_frame();
                } else {
                    let image = DynamicImage::from_decoder(decoder)?;
                    self.set_image(image);
                }
            }
            image::ImageFormat::WebP => {
                let decoder = WebPDecoder::new(read)?;

                if decoder.has_animation() {
                    let frames = decoder
                        .into_frames()
                        .collect_frames()?
                        .into_iter()
                        .map(Frame::from)
                        .collect::<Vec<_>>();
                    imp.frames.replace(Some(frames));
                    self.update_frame();
                } else {
                    let image = DynamicImage::from_decoder(decoder)?;
                    self.set_image(image);
                }
            }
            _ => {
                let image = image::load(read, format)?;
                self.set_image(image);
            }
        }

        Ok(())
    }

    /// Set the image that is displayed by this paintable.
    fn set_image(&self, image: DynamicImage) {
        let texture = match image.color() {
            image::ColorType::L8 | image::ColorType::Rgb8 => {
                let sample = image.into_rgb8().into_flat_samples();
                texture_from_data(
                    &sample.samples,
                    sample.layout,
                    gdk::MemoryFormat::R8g8b8,
                    image::ColorType::Rgb8.bytes_per_pixel(),
                )
            }
            image::ColorType::La8 | image::ColorType::Rgba8 => {
                let sample = image.into_rgba8().into_flat_samples();
                texture_from_data(
                    &sample.samples,
                    sample.layout,
                    gdk::MemoryFormat::R8g8b8a8,
                    image::ColorType::Rgba8.bytes_per_pixel(),
                )
            }
            image::ColorType::L16 | image::ColorType::Rgb16 => {
                let sample = image.into_rgb16().into_flat_samples();
                let bytes = sample
                    .samples
                    .into_iter()
                    .flat_map(|b| b.to_ne_bytes())
                    .collect::<Vec<_>>();
                texture_from_data(
                    &bytes,
                    sample.layout,
                    gdk::MemoryFormat::R16g16b16,
                    image::ColorType::Rgb16.bytes_per_pixel(),
                )
            }
            image::ColorType::La16 | image::ColorType::Rgba16 => {
                let sample = image.into_rgba16().into_flat_samples();
                let bytes = sample
                    .samples
                    .into_iter()
                    .flat_map(|b| b.to_ne_bytes())
                    .collect::<Vec<_>>();
                texture_from_data(
                    &bytes,
                    sample.layout,
                    gdk::MemoryFormat::R16g16b16a16,
                    image::ColorType::Rgba16.bytes_per_pixel(),
                )
            }
            image::ColorType::Rgb32F => {
                let sample = image.into_rgb32f().into_flat_samples();
                let bytes = sample
                    .samples
                    .into_iter()
                    .flat_map(|b| b.to_ne_bytes())
                    .collect::<Vec<_>>();
                texture_from_data(
                    &bytes,
                    sample.layout,
                    gdk::MemoryFormat::R32g32b32Float,
                    image::ColorType::Rgb32F.bytes_per_pixel(),
                )
            }
            image::ColorType::Rgba32F => {
                let sample = image.into_rgb32f().into_flat_samples();
                let bytes = sample
                    .samples
                    .into_iter()
                    .flat_map(|b| b.to_ne_bytes())
                    .collect::<Vec<_>>();
                texture_from_data(
                    &bytes,
                    sample.layout,
                    gdk::MemoryFormat::R32g32b32Float,
                    image::ColorType::Rgb32F.bytes_per_pixel(),
                )
            }
            c => {
                error!("Received image of unsupported color format: {c:?}");
                return;
            }
        };

        self.imp().frame.replace(Some(texture.upcast()));
    }

    /// Creates a new paintable by loading an image from the given file.
    pub fn from_file(file: &gio::File) -> Result<Self, Box<dyn std::error::Error>> {
        let stream = file.read(gio::Cancellable::NONE)?;
        let reader = BufReader::new(stream.into_read());
        let format = file
            .path()
            .and_then(|path| ImageFormat::from_path(path).ok());

        Self::new(reader, format)
    }

    /// Creates a new paintable by loading an image from memory.
    pub fn from_bytes(
        bytes: &[u8], content_type: Option<&str>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let reader = Cursor::new(bytes);
        let format = content_type.and_then(ImageFormat::from_mime_type);

        Self::new(reader, format)
    }

    /// Update the current frame of the animation.
    fn update_frame(&self) {
        let imp = self.imp();
        let frames_ref = imp.frames.borrow();

        // If it's not an animation, we return early.
        let frames = match &*frames_ref {
            Some(frames) => frames,
            None => return,
        };

        let idx = imp.current_idx.get();
        let next_frame = frames.get(idx).unwrap();
        imp.frame.replace(Some(next_frame.texture.clone()));

        // Invalidate the contents so that the new frame will be rendered.
        self.invalidate_contents();

        // Update the frame when the duration is elapsed.
        let update_frame_callback = glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move || {
                obj.imp().timeout_source_id.take();
                obj.update_frame();
            }
        );
        let source_id = glib::timeout_add_local_once(next_frame.duration, update_frame_callback);
        imp.timeout_source_id.replace(Some(source_id));

        // Update the index for the next call.
        let mut new_idx = idx + 1;
        if new_idx >= frames.len() {
            new_idx = 0;
        }
        imp.current_idx.set(new_idx);
    }

    /// Get the current frame of this `ImagePaintable`, if any.
    pub fn current_frame(&self) -> Option<gdk::Texture> {
        self.imp().frame.borrow().clone()
    }
}

fn texture_from_data(
    bytes: &[u8], layout: SampleLayout, format: gdk::MemoryFormat, bpp: u8,
) -> gdk::MemoryTexture {
    let bytes = glib::Bytes::from(bytes);

    let stride = layout.width * bpp as u32;

    gdk::MemoryTexture::new(
        layout.width as i32,
        layout.height as i32,
        format,
        &bytes,
        stride as usize,
    )
}
