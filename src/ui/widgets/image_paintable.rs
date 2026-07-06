use std::{
    rc::Rc,
    time::Duration,
};

use anyhow::{
    Result,
    bail,
};
use gtk::{
    gdk,
    gio,
    glib,
    graphene,
    prelude::*,
    subclass::prelude::*,
};
use tracing::warn;

use crate::utils::spawn;

const DEFAULT_ANIMATION_FRAME_DELAY: Duration = Duration::from_millis(100);

mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default)]
    pub struct ImagePaintable {
        /// Glycin image handle used to asynchronously request subsequent animation frames.
        pub image: RefCell<Option<Rc<glycin::Image>>>,
        /// The currently displayed frame.
        pub frame: RefCell<Option<gdk::Texture>>,
        /// The source ID of the timeout to load the next frame, if any.
        pub timeout_source_id: RefCell<Option<glib::SourceId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImagePaintable {
        const NAME: &'static str = "ImagePaintable";
        type Type = super::ImagePaintable;
        type Interfaces = (gdk::Paintable,);
    }

    impl ObjectImpl for ImagePaintable {
        fn dispose(&self) {
            if let Some(source_id) = self.timeout_source_id.borrow_mut().take() {
                source_id.remove();
            }
            self.image.borrow_mut().take();
            self.frame.borrow_mut().take();
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
            gdk::PaintableFlags::STATIC_SIZE
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
}

glib::wrapper! {
    /// A paintable that displays an animated image decoded by glycin.
    ///
    /// `gtk::Picture` displays paintables but does not drive animation itself;
    /// this object owns the animation timer, requests the next glycin frame, and
    /// invalidates its contents when the current frame changes.
    pub struct ImagePaintable(ObjectSubclass<imp::ImagePaintable>)
        @implements gdk::Paintable;
}

pub async fn paintable_from_file(
    file: gio::File, cancellable: Option<gio::Cancellable>,
) -> Result<gdk::Paintable> {
    if cancellable.as_ref().is_some_and(|c| c.is_cancelled()) {
        bail!("image load cancelled");
    }

    let mut loader = glycin::Loader::new(file);
    if let Some(cancellable) = cancellable {
        loader.cancellable(cancellable);
    }

    let image = loader.load().await?;
    let frame = image.next_frame().await?;

    if frame.delay().is_some() {
        Ok(ImagePaintable::new(image, frame).upcast())
    } else {
        Ok(frame.texture().upcast())
    }
}

impl ImagePaintable {
    fn new(image: glycin::Image, frame: glycin::Frame) -> Self {
        let obj = glib::Object::new::<Self>();
        obj.imp().image.replace(Some(Rc::new(image)));
        obj.set_frame(frame);
        obj
    }

    fn set_frame(&self, frame: glycin::Frame) {
        let delay = frame
            .delay()
            .filter(|d| !d.is_zero())
            .unwrap_or(DEFAULT_ANIMATION_FRAME_DELAY);
        self.imp().frame.replace(Some(frame.texture()));
        self.invalidate_contents();
        self.schedule_next_frame(delay);
    }

    fn schedule_next_frame(&self, delay: Duration) {
        let imp = self.imp();
        if let Some(source_id) = imp.timeout_source_id.borrow_mut().take() {
            source_id.remove();
        }

        let source_id = glib::timeout_add_local_once(
            delay,
            glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move || {
                    obj.imp().timeout_source_id.borrow_mut().take();
                    obj.load_next_frame();
                }
            ),
        );
        imp.timeout_source_id.replace(Some(source_id));
    }

    fn load_next_frame(&self) {
        let Some(image) = self.imp().image.borrow().as_ref().cloned() else {
            return;
        };

        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                match image.next_frame().await {
                    Ok(frame) => obj.set_frame(frame),
                    Err(error) => warn!("Failed to load animated image frame with glycin: {error}"),
                }
            }
        ));
    }
}
