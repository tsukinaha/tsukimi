use std::{
    io::{
        BufRead,
        BufReader,
        Cursor,
        Error as IoError,
        ErrorKind,
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
    ImageReader,
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
    texture: gdk::Texture,
    duration: Duration,
}

pub struct DecodedPaintable {
    frames: Vec<Frame>,
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
    impl ObjectImpl for ImagePaintable {
        fn dispose(&self) {
            if let Some(source_id) = self.timeout_source_id.borrow_mut().take() {
                source_id.remove();
            }
        }
    }

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
                gdk::PaintableFlags::STATIC_SIZE
            } else {
                gdk::PaintableFlags::STATIC_SIZE | gdk::PaintableFlags::STATIC_CONTENTS
            }
        }

        fn current_image(&self) -> gdk::Paintable {
            self.frame
                .borrow()
                .to_owned()
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
    pub fn decode_bytes(
        bytes: glib::Bytes,
    ) -> Result<DecodedPaintable, Box<dyn std::error::Error + Send + Sync>> {
        let reader = Cursor::new(bytes);
        let reader = ImageReader::new(reader).with_guessed_format()?;
        Self::decode_reader(reader)
    }

    pub fn from_decoded(decoded: DecodedPaintable) -> Self {
        let obj = glib::Object::new::<Self>();
        obj.load_decoded(decoded);
        obj
    }

    fn decode_reader<R: BufRead + Seek>(
        reader: ImageReader<R>,
    ) -> Result<DecodedPaintable, Box<dyn std::error::Error + Send + Sync>> {
        let format = reader
            .format()
            .ok_or_else(|| IoError::new(ErrorKind::InvalidData, "Could not detect image format"))?;

        let read = reader.into_inner();
        let frames = match format {
            image::ImageFormat::Gif => {
                let decoder = GifDecoder::new(read)?;
                decoder
                    .into_frames()
                    .collect_frames()?
                    .into_iter()
                    .map(Frame::from)
                    .collect()
            }
            image::ImageFormat::Png => {
                let decoder = PngDecoder::new(read)?;
                if decoder.is_apng().unwrap_or_default() {
                    decoder
                        .apng()?
                        .into_frames()
                        .collect_frames()?
                        .into_iter()
                        .map(Frame::from)
                        .collect()
                } else {
                    vec![Frame::from(DynamicImage::from_decoder(decoder)?)]
                }
            }
            image::ImageFormat::WebP => {
                let decoder = WebPDecoder::new(read)?;
                if decoder.has_animation() {
                    decoder
                        .into_frames()
                        .collect_frames()?
                        .into_iter()
                        .map(Frame::from)
                        .collect()
                } else {
                    vec![Frame::from(DynamicImage::from_decoder(decoder)?)]
                }
            }
            _ => vec![Frame::from(image::load(read, format)?)],
        };

        Ok(DecodedPaintable { frames })
    }

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

        let decoded =
            Self::decode_reader(reader).map_err(|error| -> Box<dyn std::error::Error> { error })?;
        obj.load_decoded(decoded);

        Ok(obj)
    }

    fn load_decoded(&self, decoded: DecodedPaintable) {
        let imp = self.imp();
        let frames = decoded.frames;

        if frames.len() == 1 {
            if let Some(frame) = frames.into_iter().next() {
                imp.frame.replace(Some(frame.texture));
            }
        } else {
            imp.frames.replace(Some(frames));
            self.update_frame();
        }
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
        imp.frame.replace(Some(next_frame.texture.to_owned()));

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
        self.imp().frame.borrow().to_owned()
    }
}

impl From<image::Frame> for Frame {
    fn from(frame: image::Frame) -> Self {
        let mut duration = Duration::from(frame.delay());

        if duration.is_zero() {
            duration = Duration::from_millis(100);
        }

        let sample = frame.into_buffer().into_flat_samples();
        Self {
            texture: texture_from_data(
                &sample.samples,
                sample.layout,
                gdk::MemoryFormat::R8g8b8a8,
                image::ColorType::Rgba8.bytes_per_pixel(),
            )
            .upcast(),
            duration,
        }
    }
}

impl From<DynamicImage> for Frame {
    fn from(image: DynamicImage) -> Self {
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
                let sample = image.into_rgba32f().into_flat_samples();
                let bytes = sample
                    .samples
                    .into_iter()
                    .flat_map(|b| b.to_ne_bytes())
                    .collect::<Vec<_>>();
                texture_from_data(
                    &bytes,
                    sample.layout,
                    gdk::MemoryFormat::R32g32b32a32Float,
                    image::ColorType::Rgba32F.bytes_per_pixel(),
                )
            }
            color => {
                error!("Received image of unsupported color format: {color:?}");
                let sample = image.into_rgba8().into_flat_samples();
                texture_from_data(
                    &sample.samples,
                    sample.layout,
                    gdk::MemoryFormat::R8g8b8a8,
                    image::ColorType::Rgba8.bytes_per_pixel(),
                )
            }
        };

        Self {
            texture: texture.upcast(),
            duration: Duration::ZERO,
        }
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
